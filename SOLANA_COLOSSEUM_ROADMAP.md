# 🚀 Solana Colosseum Hackathon Master Roadmap: OmniDB TUI (Solana Dev Workspace)

This document contains the complete technical specification, feature breakdown, and execution strategy for transforming **OmniDB TUI** into the ultimate **Solana Terminal Developer Workspace** for the upcoming **September 2026 Colosseum Hackathon**.

---

## 🎯 Strategic Objective
Elevate **OmniDB TUI** from a local-first database/RPC reader to an indispensable, all-in-one terminal workspace (similar to `lazygit` or `k9s` for Kubernetes) specifically tailored for **Solana Smart Contract (Anchor) Developers**.

---

## 📋 Feature Breakdown & Architecture Specs

### Module 1: Smart Contract (Anchor IDL) Interaction Suite 🚀
- **1.1 Interactive Instruction Builder & Executor**
  - **Concept:** Parse Anchor IDL instruction definitions (`idl.instructions`).
  - **TUI UI:** Modal overlay allowing developers to select an instruction (e.g. `initialize`, `transfer`, `stake`).
  - **Input Form:** Dynamically render form fields based on argument types (`u64`, `Pubkey`, `String`, `bool`, etc.) and account constraints.
  - **Signing & Broadcast:** Read local keypair (`~/.config/solana/id.json` or custom keypair path), build transaction, sign, and broadcast directly to devnet/localnet/mainnet without leaving TUI.
- **1.2 Client Code Generator (`Ctrl + G`)**
  - **Concept:** Generate production-ready client snippets for any highlighted account or instruction.
  - **Output Formats:**
    - **TypeScript:** `@coral-xyz/anchor` / `@solana/web3.js` fetch snippet.
    - **Rust:** `anchor_client` account deserialization snippet.
  - **Clipboard Integration:** Auto-copy snippet to clipboard with notification badge.

---

### Module 2: Live Transaction Simulation & Debugging Suite 🔍
- **2.1 Compute Unit (CU) Meter & Pre-flight Simulator**
  - **Concept:** Before executing instructions live, run `simulateTransaction`.
  - **TUI Visual:** Render a visual progress bar indicating Compute Units consumed vs. the maximum CU budget (200,000 / 1,400,000 CU).
- **2.2 Colorized Program Log Trace Viewer**
  - **Concept:** Intercept transaction execution logs (`Program logs: ...`).
  - **Formatting:** Color-code log lines:
    - 🟩 `Program log: ...` (Info logs)
    - 🟨 `msg!(...)` (Custom program messages)
    - 🟥 `PANIC`, `Custom error: 0x...` (Highlighted errors with Anchor error code translation).
- **2.3 Account State Diff Viewer**
  - **Concept:** Split-screen side-by-side comparison of account state **Before** vs. **After** instruction execution.
  - **Visuals:** Highlight modified bytes/fields in green and removed fields in red.

---

### Module 3: Local Test Validator Controller 🛠️
- **3.1 Integrated `solana-test-validator` Manager**
  - **Concept:** Control the local Solana test validator process directly inside TUI.
  - **TUI Control Panel:**
    - `Start Validator` (Spawns `solana-test-validator` in background).
    - `Stop / Reset` (Clears ledger & restarts).
    - `Airdrop SOL` (Triggers `requestAirdrop` to default keypair).
  - **Log Stream:** Embedded log streaming panel to observe local validator slots and block production.

---

### Module 4: SPL Token & Metaplex NFT Visualizer 🎨
- **4.1 SPL Token & NFT Metadata Parser**
  - **Concept:** Automatically parse token accounts owned by queried public keys.
  - **Metadata Resolution:** Resolve SPL Token Mint decimals, symbol, supply, and Metaplex On-Chain/Off-Chain NFT JSON metadata (image URLs, attributes).
  - **TUI Display:** Display token portfolios and NFT collections as visual ASCII/ANSI cards inside the data grid.

---

### Module 5: Local AI (Ollama) Smart Assistant 🤖
- **5.1 One-Click Transaction Error Diagnoser**
  - **Concept:** When a transaction fails, send raw RPC error stack trace and program logs to local Ollama AI (`llama3` / `deepseek`).
  - **Output:** Plain-English explanation of why the transaction failed (e.g. *"ConstraintMut failed: Account is not writable"*) along with suggested fixes.
- **5.2 IDL Schema Summarizer**
  - **Concept:** Summarize complex Anchor IDLs into structured human-readable documentation directly inside the TUI.

---

## 🗓️ Execution Roadmap (July - September 2026)

| Phase | Timeline | Key Deliverables |
| :--- | :--- | :--- |
| **Phase 1: Foundation Refactoring** | *Completed* | Split `src/solana.rs`, added live RPC tests & Borsh deserializer. |
| **Phase 2: Local Validator & Code Generator** | *July 21 - Aug 5* | Implement `solana-test-validator` TUI controller & `Ctrl + G` code generator. |
| **Phase 3: Instruction Builder & Simulator** | *Aug 6 - Aug 25* | Implement Anchor Instruction Form builder, `simulateTransaction`, and CU Meter. |
| **Phase 4: Token/NFT Visualizer & AI Diagnoser** | *Aug 26 - Sept 15* | Implement SPL Token parser, Metaplex decoder, and Ollama transaction error explainer. |
| **Phase 5: Polish & Colosseum Launch** | *Sept 16 - Sept 28* | Package v1.0.0, record high-quality demo video, push to `crates.io`, submit to Colosseum. |

---

## 🛠️ Tech Stack & Dependencies

- **TUI Framework:** `ratatui` + `crossterm`
- **Async Engine:** `tokio`
- **Networking & RPC:** `reqwest`, `serde_json`
- **Codec & Crypto:** `bs58`, `base64`, `ed25519-dalek` (or lightweight keypair signer)
- **Local AI:** HTTP REST interface to `http://localhost:11434` (Ollama)
