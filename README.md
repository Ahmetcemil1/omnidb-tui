<div align="center">

# 🚀 OmniDB TUI

### *The Zero-Latency Multi-Database Client & Solana Developer Terminal Workspace*

[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Solana](https://img.shields.io/badge/Solana-Devnet%20%7C%20Mainnet-purple.svg?style=for-the-badge&logo=solana)](https://solana.com/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg?style=for-the-badge)]()
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=for-the-badge)]()

<br>

**OmniDB TUI** is an ultra-fast, keyboard-driven terminal workspace designed to unify traditional relational databases, NoSQL document/key-value stores, and Web3 blockchain nodes into a single, zero-latency interface.

Built 100% in Rust with `ratatui` and `tokio`, it eliminates memory-heavy Electron apps while giving developers instant access to **PostgreSQL, MySQL, SQLite, Redis, MongoDB, and Solana RPC networks**.

<br>

</div>

---

## 🏗️ High-Level Architecture Diagram

```mermaid
graph TD
    A[Terminal User Interface - Ratatui / Crossterm] --> B[Async Event Loop - Tokio & MPSC Channel]
    
    subgraph Multi-Database Engine Core
        B --> C[SQL Driver Core: SQLx - Postgres / MySQL / SQLite]
        B --> D[Solana RPC Client: Anchor IDL & Borsh Decoder]
        B --> E[NoSQL Redis Client: Key-Value & Cluster Info]
        B --> F[NoSQL MongoDB Client: BSON / JSON Collections]
    end

    subgraph Intelligence & Tunneling Layer
        B --> G[Local AI Assistant: Ollama REST API - qwen2.5-coder / llama3]
        B --> H[SSH Bastion Tunneling Engine: Native Port Forwarding]
    end
```

---

## 🌟 Supported Database & Blockchain Platforms

| Engine | Protocol Format | Supported Features |
| :--- | :--- | :--- |
| 🐘 **PostgreSQL** | `postgres://user:pass@host:5432/db` | Schema introspection, full DDL/DML, fuzzy search filter, inline cell editing |
| 🐬 **MySQL** | `mysql://user:pass@host:3306/db` | Dynamic schema navigation, query history logging, export to CSV/JSON/MD |
| 📁 **SQLite** | `sqlite://local.db` | Embedded local file DBs, in-memory execution, schema export |
| ⚡ **Redis** | `redis://host:6379` | Key-Value explorer, `KEYS *` pattern search, `GET <key>`, `INFO` memory metrics |
| 🍃 **MongoDB** | `mongodb://host:27017` | BSON / JSON document collection viewer, `find <coll>`, database cluster stats |
| 🟣 **Solana RPC** | `solana://api.devnet.solana.com` | Anchor IDL decode, `simulateTransaction` CU budget meter, `diff` state viewer, `nft` Metaplex decoder, `tokens` portfolio, `solana-test-validator` manager |

---

## 🚀 Solana Smart Contract (Anchor) Developer Suite

OmniDB TUI transforms your terminal into a complete **Solana Developer Workspace**:

- 📜 **Anchor IDL Borsh Account Decoder (`idl`):** Read raw Base64 account data on-chain and deserialize it into clean, structured TUI tables using local `idl.json` schemas.
- ⚡ **Compute Unit (CU) Budget Meter & Pre-Flight Simulator (`simulate`):** Run `simulateTransaction` before broadcast and render a visual progress bar indicating Compute Units consumed vs. maximum budget limit (200,000 max CU).
- 🔍 **Account State Diff Viewer (`diff`):** Inspect pre-transaction vs. post-transaction account state, showing color-coded SOL balance diffs (`🟩 +0.050 SOL`, `🟥 -0.005 SOL`).
- 🛠️ **Anchor Client Code Generator (`code`):** Generate 100% production-ready TypeScript (`@coral-xyz/anchor`) and Rust (`anchor_client`) client code snippets for any Anchor struct.
- 🎨 **Metaplex NFT & SPL Token Visualizer (`nft` & `tokens`):** Query SPL token balances and Metaplex on-chain NFT metadata (supply, decimals, non-fungible status).
- 💻 **Integrated Validator Controller (`validator`):** Spawn, stop, reset `solana-test-validator`, check node health, and request test SOL airdrops directly inside TUI.
- 📋 **Local IDL Architecture Summarizer (`idl-summary`):** Generate human-readable architectural summaries of Anchor instructions, structs, and account constraints.

---

## 🤖 Local AI Assistant (Ollama Enriched)

OmniDB TUI integrates directly with local **Ollama** LLMs (e.g., `qwen2.5-coder:14b`, `llama3`), ensuring 100% privacy with **zero cloud dependencies**:

- 🧠 **Natural Language to SQL (`Ctrl + Space`):** Type English requests (e.g. *"Show top 5 active users by transaction count"*) and get valid, schema-aware raw SQL.
- ⚡ **AI Query Explainer & Optimizer (`Ctrl + E`):** Highlight slow SQL queries to get instant performance analysis, bottleneck warnings, and index suggestions.
- 🛠️ **AI Solana Transaction Error Diagnoser:** Send raw transaction error traces and logs to local AI to receive plain-English root cause explanations and Anchor constraint code fixes.
- 🔒 **100% Private & Free:** Runs on `http://localhost:11434`. No API keys required, zero data leaves your local machine.

---

## ⚙️ Solana Terminal Commands Quick Reference

Run `omnidb-tui` and open a Solana connection tab (`Ctrl + N` -> `solana://api.devnet.solana.com`).

```text
vines1Yue2Cx6GPJ8zb8T27221KszrrK46j35cSL2uR               Fetch SOL balance, owner program, and data
code <idl_path> <struct_name> [ts|rust]                   Generate TypeScript / Rust client code snippet
idl <idl_path> <pubkey> <struct_name>                     Decode account data using Anchor IDL schema
tx <signature>                                            Fetch transaction logs, slot, fee, and status
simulate <signature>                                      Simulate tx & render Compute Unit (CU) budget meter
diff <signature>                                          Render pre-tx vs post-tx account balance diffs
nft <mint_pubkey>                                         Decode Metaplex NFT metadata & mint info
tokens <owner_pubkey>                                     Fetch all SPL token accounts owned by public key
validator [start|stop|status|airdrop <pubkey> <sol>]      Control local solana-test-validator instance
idl-summary <idl_path>                                    Summarize Anchor IDL architecture locally
```

---

## 🎮 Keyboard Shortcuts

OmniDB TUI is engineered for keyboard-driven efficiency with full Vim support.

| Keybinding | Action |
| :--- | :--- |
| **`Ctrl + Space`** | Open AI Text-to-SQL Assistant Modal |
| **`Ctrl + E`** | AI Explain & Optimize Current SQL Query |
| **`Ctrl + H`** | Open Query History Modal |
| **`Ctrl + N`** | Open Connection Modal / New Tab |
| **`Ctrl + X`** | Open Data Grid Export Modal (CSV / JSON / Markdown) |
| **`h`, `j`, `k`, `l`** | Vim-style Navigation (Left, Down, Up, Right) |
| **`gg` / `G`** | Jump to Top / Bottom of List |
| **`/`** | Activate Fuzzy Search Filter Mode on Data Grid |
| **`i`** | Activate Inline Cell Editing Mode |
| **`Tab` / `BackTab`** | Switch Active Connection Tab |
| **`Enter`** | Execute Selected Query / Confirm Action |
| **`Esc` / `q`** | Close Active Modal / Quit Application |

---

## 💻 Installation & Quick Start

### Build from Source

Ensure you have Rust (v1.75+) installed:

```bash
git clone https://github.com/Ahmetcemil1/omnidb-tui.git
cd omnidb-tui
cargo build --release
sudo mv target/release/omnidb-tui /usr/local/bin/
```

### Run Application

```bash
omnidb-tui
```

### Local AI Setup (Optional)

Install and run [Ollama](https://ollama.com/):

```bash
ollama serve
ollama pull qwen2.5-coder:14b
```
OmniDB TUI automatically detects the running Ollama model on `http://localhost:11434`.

---

## 📜 License & Acknowledgments

Distributed under the **MIT License**. See [LICENSE](LICENSE) for more information.

Built with ❤️ for the **Solana Colosseum Hackathon** using [Ratatui](https://github.com/ratatui/ratatui) and [SQLx](https://github.com/launchbadge/sqlx).
