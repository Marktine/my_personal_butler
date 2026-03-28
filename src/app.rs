use std::io;
use tokio::sync::{mpsc, oneshot};
use ratatui::{
    crossterm::event::{Event, KeyCode},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, List, ListItem},
};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::config::ButlerConfig;
use crate::utils::format_with_commas;

pub enum UiEvent {
    AppendMessage(String, String), // role, text
    AiError(String),
    UsageUpdate(usize, usize, usize, usize),
    ToolPrompt(String, String, oneshot::Sender<bool>), // ai_name, cmd_str, confirmation_channel
    ToolExecuted(String),                              // message
    FinishedLoading,
}

/// Semantic colour palette for the TUI.
/// To restyle any element, change only the Color returned by its variant here.
enum ThemeColor {
    // -- Message list --
    UserMessage,       // Role header for the human user
    AiMessage,         // Role header for the AI assistant
    SystemMessage,     // Role header for system/tool notices
    MessageBody,       // Text body of every message
    HeaderPunctuation, // The colon after a role name

    // -- Status bar --
    StatusBarBg,       // Background of the status bar strip
    StatusBarLabel,    // Dim labels ("Tokens:", "Requests:")
    StatusBarMuted,    // Muted values (limits, separators)
    TokensNormal,      // Token count when usage is low
    TokensWarning,     // Token count when usage is moderate
    TokensCritical,    // Token count when usage is high
    RequestCount,      // Current request count
    LoadingIndicator,  // Spinner + "Thinking..." text

    // -- Input box --
    InputBorder, // Border and title of the message input
    InputText,   // Text typed by the user

    // -- Tool confirmation panel --
    ToolAccent,        // Border, title, and header of the confirmation panel
    ToolCommandPrefix, // The "$ " prompt prefix
    ToolCommand,       // The shell command being proposed
    AllowAction,       // "Y / Enter" keybinding label
    DenyAction,        // "any other key" keybinding label
    HintText,          // Supporting prose around keybinding hints
}

impl ThemeColor {
    fn color(&self) -> Color {
        match self {
            Self::UserMessage       => Color::Cyan,
            Self::AiMessage         => Color::LightMagenta,
            Self::SystemMessage     => Color::Yellow,
            Self::MessageBody       => Color::White,
            Self::HeaderPunctuation => Color::Gray,

            Self::StatusBarBg       => Color::Indexed(234),
            Self::StatusBarLabel    => Color::Gray,
            Self::StatusBarMuted    => Color::DarkGray,
            Self::TokensNormal      => Color::Green,
            Self::TokensWarning     => Color::Yellow,
            Self::TokensCritical    => Color::Red,
            Self::RequestCount      => Color::Cyan,
            Self::LoadingIndicator  => Color::LightYellow,

            Self::InputBorder       => Color::Blue,
            Self::InputText         => Color::White,

            Self::ToolAccent        => Color::Indexed(214), // Soft amber/orange
            Self::ToolCommandPrefix => Color::DarkGray,
            Self::ToolCommand       => Color::Yellow,
            Self::AllowAction       => Color::Green,
            Self::DenyAction        => Color::Red,
            Self::HintText          => Color::Gray,
        }
    }
}

enum AppState {
    Normal,
    WaitingForTool {
        ai_name: String,
        cmd: String,
        tx: Option<oneshot::Sender<bool>>,
    },
}

pub struct App {
    messages: Vec<(String, String)>,
    input: Input,
    config: ButlerConfig,
    is_loading: bool,
    state: AppState,
    ui_rx: mpsc::Receiver<UiEvent>,
    api_tx: mpsc::Sender<String>,
    frame: usize, // Animation frame counter
}

impl App {
    pub fn new(config: ButlerConfig, ui_rx: mpsc::Receiver<UiEvent>, api_tx: mpsc::Sender<String>) -> Self {
        Self {
            messages: Vec::new(),
            input: Input::default(),
            config,
            is_loading: false,
            state: AppState::Normal,
            ui_rx,
            api_tx,
            frame: 0,
        }
    }

    pub fn run(mut self, terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>) -> io::Result<()> {
        loop {
            // Handle async events from GeminiClient
            while let Ok(event) = self.ui_rx.try_recv() {
                match event {
                    UiEvent::AppendMessage(role, text) => {
                        self.messages.push((role, text));
                    }
                    UiEvent::AiError(err) => {
                        self.messages.push(("System".to_string(), err));
                    }
                    UiEvent::UsageUpdate(tu, tl, ru, rl) => {
                        self.config.tokens_used = tu;
                        self.config.token_limit = tl;
                        self.config.requests_made = ru;
                        self.config.request_limit = rl;
                    }
                    UiEvent::ToolPrompt(ai_name, cmd, tx) => {
                        self.state = AppState::WaitingForTool {
                            ai_name,
                            cmd,
                            tx: Some(tx),
                        };
                    }
                    UiEvent::ToolExecuted(msg) => {
                        self.messages.push(("System".to_string(), msg));
                    }
                    UiEvent::FinishedLoading => {
                        self.is_loading = false;
                    }
                }
            }

            terminal.draw(|f| self.ui(f))?;
            self.frame = self.frame.wrapping_add(1);

            if ratatui::crossterm::event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = ratatui::crossterm::event::read()? {
                    match &mut self.state {
                        AppState::Normal => {
                            if key.code == KeyCode::Esc {
                                return Ok(());
                            }
                            if key.code == KeyCode::Enter && !self.is_loading {
                                let val = self.input.value().to_string();
                                if !val.is_empty() {
                                    if val.eq_ignore_ascii_case("exit") {
                                        return Ok(());
                                    }
                                    self.api_tx.try_send(val).ok();
                                    self.input.reset();
                                    self.is_loading = true;
                                }
                            } else {
                                self.input.handle_event(&Event::Key(key));
                            }
                        }
                        AppState::WaitingForTool { tx: tx_opt, .. } => {
                            if let Some(tx) = tx_opt.take() {
                                if key.code == KeyCode::Char('y') || key.code == KeyCode::Char('Y') || key.code == KeyCode::Enter {
                                    let _ = tx.send(true);
                                } else {
                                    let _ = tx.send(false);
                                }
                            }
                            self.state = AppState::Normal;
                        }
                    }
                }
            }
        }
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        let input_height = match &self.state {
            AppState::WaitingForTool { .. } => 6,
            AppState::Normal => 3,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(0),               // Messages list
                Constraint::Length(1),            // Divider / Status
                Constraint::Length(input_height), // Input or Tool Confirmation
            ])
            .split(f.area());

        // Messages
        let mut list_items = Vec::new();
        for (role, msg) in &self.messages {
            let (icon, color) = if role == &self.config.user_name {
                ("👤", ThemeColor::UserMessage.color())
            } else if role == "System" {
                ("⚙️", ThemeColor::SystemMessage.color())
            } else {
                ("🤖", ThemeColor::AiMessage.color())
            };

            let header = Line::from(vec![
                Span::styled(format!("{} ", icon), Style::default()),
                Span::styled(role, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(":", Style::default().fg(ThemeColor::HeaderPunctuation.color())),
            ]);

            let body = Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(msg, Style::default().fg(ThemeColor::MessageBody.color())),
            ]);

            list_items.push(ListItem::new(vec![header, body, Line::from("")]));
        }

        let messages_list = List::new(list_items).block(Block::default().borders(Borders::NONE));
        f.render_widget(messages_list, chunks[0]);

        // Status Divider
        let usage_color = if self.config.tokens_used > self.config.token_limit * 8 / 10 {
            ThemeColor::TokensCritical.color()
        } else if self.config.tokens_used > self.config.token_limit / 2 {
            ThemeColor::TokensWarning.color()
        } else {
            ThemeColor::TokensNormal.color()
        };

        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner = if self.is_loading {
            spinner_frames[(self.frame / 2) % spinner_frames.len()]
        } else {
            ""
        };
        let loading_msg = if self.is_loading {
            format!(" │ {} Thinking...", spinner)
        } else {
            "".to_string()
        };

        let status_line = Line::from(vec![
            Span::styled(" 📊 Tokens: ", Style::default().fg(ThemeColor::StatusBarLabel.color())),
            Span::styled(format!("{}", format_with_commas(self.config.tokens_used)), Style::default().fg(usage_color).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" / {} ", format_with_commas(self.config.token_limit)), Style::default().fg(ThemeColor::StatusBarMuted.color())),
            Span::styled(" │ 📨 Requests: ", Style::default().fg(ThemeColor::StatusBarLabel.color())),
            Span::styled(format!("{}", format_with_commas(self.config.requests_made)), Style::default().fg(ThemeColor::RequestCount.color()).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" / {} ", format_with_commas(self.config.request_limit)), Style::default().fg(ThemeColor::StatusBarMuted.color())),
            Span::styled(loading_msg, Style::default().fg(ThemeColor::LoadingIndicator.color()).add_modifier(Modifier::ITALIC)),
        ]);

        let divider = Paragraph::new(status_line)
            .style(Style::default().bg(ThemeColor::StatusBarBg.color()))
            .block(Block::default());
        f.render_widget(divider, chunks[1]);

        // Input / Tool Confirmation
        match &self.state {
            AppState::Normal => {
                let input_widget = Paragraph::new(self.input.value())
                    .style(Style::default().fg(ThemeColor::InputText.color()))
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(ThemeColor::InputBorder.color()))
                        .title(Span::styled(" ✉️ Message ", Style::default().fg(ThemeColor::InputBorder.color()).add_modifier(Modifier::BOLD))));
                f.render_widget(input_widget, chunks[2]);
                f.set_cursor_position(ratatui::layout::Position::new(
                    chunks[2].x + self.input.cursor() as u16 + 1,
                    chunks[2].y + 1,
                ));
            }
            AppState::WaitingForTool { ai_name, cmd, .. } => {
                let accent = ThemeColor::ToolAccent.color();
                let content = vec![
                    Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(
                            format!("{} wants to execute a shell command:", ai_name),
                            Style::default().fg(accent).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  $ ", Style::default().fg(ThemeColor::ToolCommandPrefix.color()).add_modifier(Modifier::BOLD)),
                        Span::styled(cmd.as_str(), Style::default().fg(ThemeColor::ToolCommand.color()).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  Press ", Style::default().fg(ThemeColor::HintText.color())),
                        Span::styled("Y / Enter", Style::default().fg(ThemeColor::AllowAction.color()).add_modifier(Modifier::BOLD)),
                        Span::styled(" to allow, ", Style::default().fg(ThemeColor::HintText.color())),
                        Span::styled("any other key", Style::default().fg(ThemeColor::DenyAction.color()).add_modifier(Modifier::BOLD)),
                        Span::styled(" to deny.", Style::default().fg(ThemeColor::HintText.color())),
                    ]),
                ];
                let prompt_widget = Paragraph::new(content)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(accent))
                        .title(Span::styled(" 🔔 Tool Confirmation ", Style::default().fg(accent).add_modifier(Modifier::BOLD))));
                f.render_widget(prompt_widget, chunks[2]);
            }
        }
    }
}
