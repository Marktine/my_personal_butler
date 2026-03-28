# 🤵 AI Butler

AI Butler is a lightweight, terminal-based AI assistant written in Rust. It leverages the Gemini API to provide a personalized, interactive chat experience directly in your terminal, equipped with native tool calling for system command execution.

## ✨ Features

-   **💬 Interactive TUI**: A terminal user interface built with `ratatui`, featuring a dedicated chat window and status dash.
-   **🛠️ Native Tool Calling**: The butler can suggest and execute shell commands on your system (with your explicit permission).
-   **🔐 Security Guardrails**: All system-altering commands require a manual `[Y/n]` confirmation before execution.
-   **📈 Usage Tracking**: Monitor your token consumption and request count in real-time to stay within API limits.

## 🚀 Getting Started

### Prerequisites

-   [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
-   A Gemini API Key (get one from [Google AI Studio](https://aistudio.google.com/))

### Installation

1.  Clone the repository:
    ```bash
    git clone https://github.com/yourusername/ai_butler.git
    cd ai_butler
    ```

2.  Set up your environment:
    Create a `.env` file in the root directory:
    ```env
    GEMINI_API_KEY=your_api_key_here
    GEMINI_API_URL=https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}
    ```

3.  Build and run:
    ```bash
    cargo run --release
    ```

### First Run

On your first launch, the butler will guide you through a brief onboarding process to set up your personal preferences. These settings are stored in `~/.config/ai_butler/config.json`.

## 🛠️ Usage

-   **Chatting**: Type your message in the input field at the bottom and press `Enter`.
-   **Tool Execution**: When the butler wants to run a command (e.g., `ls -la`), it will prompt you for confirmation. Press `y` to allow or `n` to deny.
-   **Exit**: Press `Esc` or `Ctrl+C` to quit the application gracefully.

## 🧰 Tech Stack

-   **Language**: Rust (2024 Edition)
-   **UI Library**: [Ratatui](https://ratatui.rs/)
-   **Async Runtime**: [Tokio](https://tokio.rs/)
-   **HTTP Client**: [Reqwest](https://github.com/seanmonstar/reqwest)
-   **Serialization**: [Serde](https://serde.rs/)
-   **API**: Google Gemini API

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
