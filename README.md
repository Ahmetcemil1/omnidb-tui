<div align="center">

# 🚀 OmniDB TUI

**The Ultimate AI-Powered Multi-Database Client & Solana Terminal Developer Workspace**

[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Solana](https://img.shields.io/badge/Solana-Devnet%20%7C%20Mainnet-purple.svg)](https://solana.com/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)]()

<br>

**OmniDB TUI** is a blazingly fast, zero-latency terminal user interface (TUI) for managing **PostgreSQL, MySQL, SQLite, Redis, MongoDB, and Solana RPC networks**. 

Built for developers who demand native performance without memory-heavy Electron apps, it features built-in SSH tunneling, offline local AI assistance (Ollama), Anchor IDL decoders, transaction simulators, and full Vim keybindings.

<br>

</div>

---

## 🌟 Supported Platform Engines

| Platform | URI Protocol Format | Supported Features |
| :--- | :--- | :--- |
| **PostgreSQL** | `postgres://user:pass@host:5432/db` | Schema introspection, SQL editor, fuzzy search, inline cell edit |
| **MySQL** | `mysql://user:pass@host:3306/db` | Full DDL/DML, dynamic schema viewer, query history |
| **SQLite** | `sqlite://local.db` | Embedded file DBs, in-memory execution, schema export |
| **Solana RPC** | `solana://api.devnet.solana.com` | Anchor IDL decode, `simulateTransaction`, CU meter, `diff`, Metaplex NFT decoder, SPL tokens, `solana-test-validator` control |
| **Redis** | `redis://host:6379` | Key-Value reader, `KEYS *`, `GET <key>`, `INFO` cluster metrics |
| **MongoDB** | `mongodb://host:27017` | BSON / JSON document collection viewer, `find <coll>`, database stats |

---

## ⚡ Key Features

### 🚀 Solana Smart Contract (Anchor) Developer Suite
- 📜 **Anchor IDL Borsh Decoder (`idl`):** Decode raw Base64 account data into human-readable tables using local `idl.json` schemas.
- ⚡ **Compute Unit (CU) Meter & Pre-flight Simulator (`simulate`):** Run `simulateTransaction` and render visual progress meters for CU budget consumption (200,000 max CU).
- 🔍 **Account State Diff Viewer (`diff`):** Compare pre-transaction vs. post-transaction SOL balances and token states side-by-side with color-coded diffs.
- 🛠️ **Client Code Generator (`code`):** Generate 100% production-ready TypeScript (`@coral-xyz/anchor`) and Rust (`anchor_client`) snippets for any Anchor struct.
- 🎨 **Metaplex NFT & SPL Token Decoder (`nft` & `tokens`):** Resolve SPL token balances and Metaplex on-chain NFT metadata.
- 💻 **Integrated Validator Controller (`validator`):** Spawn, stop, reset `solana-test-validator`, and request local SOL airdrops directly inside TUI.

### 🤖 Local AI Assistant (Ollama)
- 🧠 **Natural Language to SQL (`Ctrl + Space`):** Convert English to SQL using local Ollama models (e.g., `qwen2.5-coder`, `llama3`).
- ⚡ **AI Query Explainer & Optimizer (`Ctrl + E`):** Analyze slow SQL queries and get index optimization advice.
- 🛠️ **AI Solana Transaction Error Diagnoser:** Send transaction failure logs to local AI to get plain-English root cause explanations and Anchor constraint fixes.
- 📋 **AI IDL Architecture Summarizer (`idl-summary`):** Summarize complex Anchor IDLs into structured executive documentation.

---

## ⚙️ Usage & Solana Commands

Run `omnidb-tui` and open a new connection tab (`Ctrl + N`).

### Solana Commands Reference

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

| Keybinding | Action |
| :--- | :--- |
| **`Ctrl + Space`** | Open AI Text-to-SQL Assistant |
| **`Ctrl + E`** | AI Explain & Optimize SQL Query |
| **`Ctrl + H`** | View Query History |
| **`Ctrl + N`** | Open New Connection Tab |
| **`Ctrl + X`** | Export Data Grid (CSV / JSON / Markdown) |
| **`h`, `j`, `k`, `l`** | Vim Navigation (Left, Down, Up, Right) |
| **`gg` / `G`** | Jump to Top / Bottom |
| **`/`** | Fuzzy Search Filter Data Grid |
| **`i`** | Inline Cell Edit Mode |
| **`Enter`** | Execute Query / Select Item |
| **`Esc` / `q`** | Close Modal / Quit Application |

---

## 🏗️ Getting Started & Installation

### Build from Source

```bash
git clone https://github.com/Ahmetcemil1/omnidb-tui.git
cd omnidb-tui
cargo build --release
sudo mv target/release/omnidb-tui /usr/local/bin/
```

### Local AI Prerequisites (Optional)
Install and run [Ollama](https://ollama.com/):
```bash
ollama serve
ollama pull qwen2.5-coder
```
OmniDB TUI automatically detects the local Ollama API on `http://localhost:11434`.

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
