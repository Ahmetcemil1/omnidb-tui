# Solana Integration & Deserialization Test Report

This document outlines the testing strategy, test descriptions, and instructions for verifying the Solana RPC integration and Borsh deserialization features in **OmniDB TUI**.

---

## 🛠️ How to Run the Tests

To run the automated test suite on your local machine, execute the following command in the root of the project:

```bash
cargo test
```

For detailed test execution logging and standard output capturing:

```bash
cargo test -- --nocapture
```

---

## 📋 Test Suite Structure

The Solana test suite is implemented natively in Rust within [src/solana.rs](file:///home/zenhor/Masaüstü/proje2/src/solana.rs) and consists of two main categories of tests:

### 1. Unit Test: `test_solana_borsh_deserialization`
*   **Purpose:** Verifies that our custom Borsh deserializer accurately decodes binary account payloads using standard Anchor IDL JSON schemas without depending on external Solana CLI/SDK binaries.
*   **What it does:**
    1.  Constructs a mock Anchor IDL JSON structure containing a variety of data types (`u64`, `publicKey`, `string`, `bool`).
    2.  Creates a mock binary payload (representing the serialized account state) containing matching test values: an Anchor account discriminator, the integer `42`, a 32-byte public key of all `1`s, a 5-byte string `"hello"`, and a boolean `true`.
    3.  Runs the `decode_borsh_using_idl` function and asserts that each field name, data type, and decoded value are parsed exactly as expected.
*   **Execution Time:** < 1 ms (Pure memory operations).

### 2. Live Integration Test: `test_solana_live_rpc`
*   **Purpose:** Proves that the HTTP JSON-RPC client connects, authenticates, and retrieves real account data from live Solana nodes.
*   **What it does:**
    1.  Spawns an asynchronous HTTP client pointing directly to the official **Solana Devnet RPC endpoint** (`https://api.devnet.solana.com`).
    2.  Sends real JSON-RPC requests (`getBalance` and `getAccountInfo`) for a known valid Devnet public key (`vines1Yue2Cx6GPJ8zb8T27221KszrrK46j35cSL2uR`).
    3.  Asserts that the connection is successful and verifies that the returned dataset correctly parses the account's SOL balance.
*   **Execution Time:** ~200 - 500 ms (Depends on network latency to the Solana RPC).

---

## 🖥️ Sample Successful Output

Below is the expected console output when running the test suite:

```text
$ cargo test
   Compiling omnidb-tui v0.1.0 (/home/zenhor/Masaüstü/proje2)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.16s
     Running unittests src/main.rs (target/debug/deps/omnidb_tui-e1ccf37bfe0ea90a)

running 5 tests
test bookmarks::tests::test_parse_host_port_from_uri ... ok
test bookmarks::tests::test_rewrite_uri_for_ssh ... ok
test db::tests::test_sqlite_integration ... ok
test solana::tests::test_solana_borsh_deserialization ... ok
test solana::tests::test_solana_live_rpc ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
```

---

## 🔍 Manual Verification Scenarios (Inside TUI)

If you wish to demo or manually verify the functionality inside the terminal user interface:

1.  Launch the application using a Solana RPC URL as the database target:
    ```bash
    cargo run -- solana://api.devnet.solana.com
    ```
2.  **Verify Account Query:** Input any valid Solana wallet address (e.g., `vines1Yue2Cx6GPJ8zb8T27221KszrrK46j35cSL2uR`) in the query input and press `Enter`. The grid will display:
    *   `Address`
    *   `Balance (SOL)`
    *   `Owner Program`
    *   `Executable` (true/false)
    *   `Rent Epoch`
3.  **Verify Account History:** Input `history <pubkey>` and press `Enter` to retrieve the last 20 transaction signatures, blocks, slots, and block times.
4.  **Verify Anchor IDL Borsh Decoding:** If you have a local Anchor state account, run:
    ```text
    idl /path/to/idl.json <account_pubkey> <state_struct_name>
    ```
    This will instantly deserialize the binary state account and display the decoded structure.
