<div align="center">

# 🚀 OmniDB TUI

**The Fastest AI-Powered Terminal Database Client**

[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)]()

<br>
OmniDB TUI is a blazingly fast, zero-latency terminal user interface (TUI) for managing your databases. Forget resource-heavy Electron apps; experience native performance, built-in SSH tunneling, and offline AI SQL assistance right from your terminal.
<br>

</div>

## ✨ Key Features

- 🦀 **Native Performance:** Written in 100% Rust. Zero JVM or Electron bloat. Starts instantly and uses minimal RAM.
- 🤖 **Local AI SQL Assistant:** Forget sending your schemas to the cloud. Integrated with Ollama, you get Text-to-SQL and Query Explanation features completely offline and private!
- 🚇 **Built-in SSH Tunneling:** Connect securely to production databases without leaving the interface. No need for separate CLI background processes.
- ⌨️ **Vim Navigations:** Full support for `h`, `j`, `k`, `l`, `gg`, `G`. Power users never need to touch the mouse.
- ⚡ **Multi-Tab Asynchronous Architecture:** Powered by `Tokio`, run heavy queries in one tab while managing another database in a different tab without UI freezing.
- 📑 **Query History & Bookmarks:** Press `Ctrl+H` for history. Save long connection strings securely as bookmarks.
- 📦 **Data Export:** Export your query results directly to CSV, JSON, or GitHub-flavored Markdown.

## 🚀 Quick Start

### Installation

Ensure you have Rust and Cargo installed, then run:

```bash
git clone https://github.com/Ahmetcemil1/omnidb-tui.git
cd omnidb-tui
cargo build --release
```

The executable will be located in `target/release/omnidb-tui`.

### AI Integration (Ollama)
For the Text-to-SQL features to work, ensure [Ollama](https://ollama.com/) is installed and running locally with your preferred model. 
```bash
ollama run llama3
```

## 🎮 Keyboard Shortcuts

| Keybinding | Action |
| --- | --- |
| `Ctrl + Space` | Open AI Text-to-SQL Assistant |
| `Ctrl + E` | Explain / Optimize Current Query via AI |
| `Ctrl + H` | View Query History |
| `Ctrl + N` | Open New Tab |
| `h, j, k, l` | Vim-style Navigation |
| `gg / G` | Jump to Top / Bottom |
| `Ctrl + E` | Export Data (CSV/JSON/MD) |

## 🤝 Contributing

Contributions are completely welcome! Whether it's adding support for new database engines, optimizing the ratatui renderer, or writing tests. Please feel free to open a Pull Request.

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
