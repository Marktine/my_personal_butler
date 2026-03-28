mod utils;
mod models;
mod config;
mod client;
mod app;

use std::env;
use std::io;
use dotenv::dotenv;
use tokio::sync::mpsc;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};

use crate::utils::AnyError;
use crate::config::load_or_prompt_config;
use crate::client::GeminiClient;
use crate::app::{App, UiEvent};

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    dotenv().ok();

    let api_key = match env::var("GEMINI_API_KEY") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("GEMINI_API_KEY environment variable is not set.");
            std::process::exit(1);
        }
    };

    let config = {
        let mut c = load_or_prompt_config();
        if let Ok(url) = env::var("GEMINI_API_URL") {
            c.api_url = url;
        }
        c
    };

    // Setup channels
    let (ui_tx, ui_rx) = mpsc::channel(100);
    let (api_tx, mut api_rx) = mpsc::channel::<String>(100);

    let mut gemini = GeminiClient::new(api_key, config.clone(), ui_tx.clone());

    // Spawn Gemini runtime
    tokio::spawn(async move {
        while let Some(prompt) = api_rx.recv().await {
            let _ = gemini.send_message(Some(&prompt)).await;
            let _ = gemini.ui_tx.send(UiEvent::FinishedLoading).await;
        }
    });

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(config, ui_rx, api_tx);
    let res = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}
