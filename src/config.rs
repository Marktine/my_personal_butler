use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

fn default_token_limit() -> usize {
    4_000_000
}

fn default_request_limit() -> usize {
    1_500
}

fn default_api_url() -> String {
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ButlerConfig {
    pub user_name: String,
    pub ai_name: String,
    pub vibe: String,
    #[serde(default)]
    pub tokens_used: usize,
    #[serde(default = "default_token_limit")]
    pub token_limit: usize,
    #[serde(default)]
    pub requests_made: usize,
    #[serde(default = "default_request_limit")]
    pub request_limit: usize,
    #[serde(default = "default_api_url")]
    pub api_url: String,
}

fn get_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("ai_butler");
    fs::create_dir_all(&path).ok();
    path.push("config.json");
    path
}

pub fn save_config(config: &ButlerConfig) {
    let path = get_config_path();
    let serialized = serde_json::to_string_pretty(config).unwrap_or_default();
    fs::write(&path, serialized).unwrap_or_else(|e| {
        eprintln!("Failed to save config to {:?}: {}", path, e);
    });
}

pub fn load_or_prompt_config() -> ButlerConfig {
    let path = get_config_path();

    if path.exists() {
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&contents) {
                return config;
            }
        }
    }

    println!("🤵 Ah, a new presence. The butler's quarters have been quiet for too long.");
    println!("🖋️ Before I can begin my service, we must establish the foundations of our bond.\n");

    let stdin = io::stdin();
    
    print!("\x1b[1;34m👤\x1b[0m Whom shall I have the honor of serving? (\x1b[36mYour name\x1b[0m): ");
    io::stdout().flush().unwrap();
    let mut user_name = String::new();
    stdin.read_line(&mut user_name).unwrap();
    let user_name = user_name.trim().to_string();

    print!("\x1b[1;35m🧠\x1b[0m And what shall we call the mind that will dwell within this machine? (\x1b[36mAI name\x1b[0m): ");
    io::stdout().flush().unwrap();
    let mut ai_name = String::new();
    stdin.read_line(&mut ai_name).unwrap();
    let ai_name = ai_name.trim().to_string();

    print!("\x1b[1;33m🔮\x1b[0m Tell me, what kind of spirit shall I summon for you? Give this entity its '\x1b[36msoul\x1b[0m' (Vibe/Personality): ");
    io::stdout().flush().unwrap();
    let mut vibe = String::new();
    stdin.read_line(&mut vibe).unwrap();
    let vibe = vibe.trim().to_string();

    let config = ButlerConfig {
        user_name,
        ai_name,
        vibe,
        tokens_used: 0,
        token_limit: default_token_limit(),
        requests_made: 0,
        request_limit: default_request_limit(),
        api_url: default_api_url(),
    };

    save_config(&config);
    println!("\n✨ The ink is dry. The bond is sealed. Welcome, \x1b[1;36m{}\x1b[0m. \x1b[1;35m{}\x1b[0m is ready for you.\n", config.user_name, config.ai_name);
    config
}
