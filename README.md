<div align="center">

# 🚀 OmniDB TUI

**The Ultimate, AI-Powered, Zero-Latency Terminal Database Client**

[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)]()
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)]()

<br>
OmniDB TUI is a blazingly fast terminal user interface (TUI) for managing databases. Designed for power users who demand native performance, it completely eliminates the need for resource-heavy Electron or JVM apps. With built-in SSH tunneling, offline AI SQL assistance (Ollama), and full Vim-keybindings, it brings modern database management directly to your terminal.
<br>

</div>

---

## 🌟 Why OmniDB TUI?

Traditional database clients consume gigabytes of RAM and force you to break your terminal workflow. OmniDB TUI is built with **Rust** and **Ratatui** to provide an instant, zero-latency interface. 

### ✨ Core Features

- 🦀 **Native & Blazingly Fast:** Written in 100% Rust. Starts instantly, consumes minimal memory, and renders thousands of rows without stuttering.
- 🤖 **Local AI SQL Assistant:** Generate complex SQL queries using plain English (`Ctrl + Space`). Powered by your local Ollama models (e.g., Llama 3) ensuring your database schemas never leave your machine!
- 🧠 **AI Query Explainer & Optimizer:** Highlight a slow query, press `Ctrl + E`, and let the local AI explain the execution plan or suggest indexes.
- 🚇 **Built-in SSH Tunneling:** Connect securely to production databases hidden behind bastions. No need for separate CLI background processes; OmniDB handles the tunneling natively.
- ⌨️ **Vim Navigations:** Full support for `h`, `j`, `k`, `l`, `gg`, `G`. Real developers don't touch the mouse.
- ⚡ **Multi-Tab Asynchronous Architecture:** Powered by `Tokio`. Run a 5-minute aggregation query in one tab while managing a completely different database in another tab without UI freezing.
- 📑 **Query History & Secure Bookmarks:** Press `Ctrl+H` for query history. Save long, complex connection strings securely as local bookmarks.
- 📦 **Data Export:** Export query results directly to CSV, JSON, or GitHub-flavored Markdown.

---

## 🚀 Getting Started

### Prerequisites

- [Rust & Cargo](https://rustup.rs/) (v1.70+)
- [Ollama](https://ollama.com/) (Required only if you want to use the AI Assistant features)

### Installation

Clone the repository and build it locally:

```bash
git clone https://github.com/Ahmetcemil1/omnidb-tui.git
cd omnidb-tui
cargo build --release
```

Move the executable to your path:
```bash
sudo mv target/release/omnidb-tui /usr/local/bin/
```

---

## ⚙️ Configuration & Usage

Start the application by running:
```bash
omnidb-tui
```

### Connection Bookmarks
Connections are automatically saved locally and can be easily managed. They are stored in `~/.config/omnidb/connections.json`. You can manage SSH keys, usernames, and passwords directly from the TUI.

### AI Integration Setup
To use the Text-to-SQL or Query Explain features, ensure Ollama is running in the background:
```bash
ollama run llama3
```
OmniDB TUI will automatically detect the local Ollama API (http://localhost:11434).

---

## 🎮 Keyboard Shortcuts

OmniDB TUI is designed to keep your hands on the keyboard.

| Keybinding | Action |
| :--- | :--- |
| **`Ctrl + Space`** | Open AI Text-to-SQL Assistant |
| **`Ctrl + E`** | Explain / Optimize Current Query via AI |
| **`Ctrl + H`** | View Query History |
| **`Ctrl + N`** | Open New Tab |
| **`Ctrl + X`** | Export Data (CSV/JSON/MD) |
| **`h`, `j`, `k`, `l`** | Vim-style Navigation (Left, Down, Up, Right) |
| **`gg` / `G`** | Jump to Top / Jump to Bottom |
| **`Enter`** | Execute Query / Select Item |
| **`Esc` / `q`** | Close Modal / Quit Application |

---

## 🏗️ Architecture

- **UI Framework:** [Ratatui](https://github.com/ratatui/ratatui) + Crossterm
- **Async Runtime:** [Tokio](https://tokio.rs/)
- **Database Drivers:** Native Rust SQLx drivers
- **AI Integration:** Local HTTP calls to Ollama REST API

---

## ❤️ Support & Donate

If OmniDB TUI has saved your life (or at least your RAM), consider supporting the development! Your support helps me spend more time adding new database drivers, optimizing the renderer, and keeping this project 100% free and open-source.

**Crypto Donations:**
- **Bitcoin (BTC):** `bc1qetuu2ehsltezy2t7f7pgr7gl388494cd9duxnd`
- **Ethereum (ETH):** `0x9Da009aE0C9366d5944FA041dD43Dc89528DB289`

---

## 🤝 Contributing

We welcome all contributions! Whether it's adding support for Oracle/SQL Server, optimizing the Ratatui renderer, or writing unit tests:
1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
