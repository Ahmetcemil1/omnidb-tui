use anyhow::Result;
use sqlx::{any::AnyPoolOptions, AnyPool, Column, Row};
use serde_json;
use reqwest;
use bs58;
use base64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbType {
    Postgres,
    MySql,
    Sqlite,
    Solana,
}

pub fn detect_db_type(uri: &str) -> DbType {
    if uri.starts_with("postgres") || uri.starts_with("postgresql") {
        DbType::Postgres
    } else if uri.starts_with("mysql") {
        DbType::MySql
    } else if uri.starts_with("solana") {
        DbType::Solana
    } else {
        DbType::Sqlite
    }
}

pub fn escape_identifier(name: &str, db_type: DbType) -> String {
    match db_type {
        DbType::MySql => format!("`{}`", name.replace('`', "``")),
        _ => format!("\"{}\"", name.replace('"', "\"\"")),
    }
}

pub async fn connect(uri: &str) -> Result<AnyPool> {
    // Install default SQLx drivers (required for the dynamic 'any' driver)
    sqlx::any::install_default_drivers();
    
    let pool = AnyPoolOptions::new()
        .max_connections(5)
        .connect(uri)
        .await?;
    Ok(pool)
}

pub async fn get_tables(pool: &AnyPool, db_type: DbType) -> Result<Vec<String>> {
    if db_type == DbType::Solana {
        return Ok(vec![]);
    }
    let query_str = match db_type {
        DbType::Postgres => {
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name;"
        }
        DbType::MySql => {
            "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE() ORDER BY table_name;"
        }
        DbType::Sqlite => {
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name;"
        }
        DbType::Solana => unreachable!(),
    };

    let rows = sqlx::query(query_str).fetch_all(pool).await?;
    let mut tables = Vec::new();
    for row in rows {
        let table_name: String = row.try_get(0)?;
        tables.push(table_name);
    }
    Ok(tables)
}

pub async fn get_schema(pool: &AnyPool, db_type: DbType) -> Result<String> {
    if db_type == DbType::Solana {
        return Ok("Account Info, Recent Transactions, Borsh IDL Parser".to_string());
    }
    let mut schema_desc = String::new();

    match db_type {
        DbType::Postgres | DbType::MySql => {
            let query_str = if db_type == DbType::Postgres {
                "SELECT table_name, column_name, data_type FROM information_schema.columns WHERE table_schema = 'public' ORDER BY table_name, ordinal_position;"
            } else {
                "SELECT table_name, column_name, data_type FROM information_schema.columns WHERE table_schema = DATABASE() ORDER BY table_name, ordinal_position;"
            };

            let rows = sqlx::query(query_str).fetch_all(pool).await?;
            let mut current_table = String::new();
            for row in rows {
                let table: String = row.try_get(0)?;
                let column: String = row.try_get(1)?;
                let data_type: String = row.try_get(2)?;

                if table != current_table {
                    if !current_table.is_empty() {
                        schema_desc.push_str("), ");
                    }
                    current_table = table.clone();
                    schema_desc.push_str(&format!("{}({}", table, column));
                } else {
                    schema_desc.push_str(&format!(", {}:{}", column, data_type));
                }
            }
            if !current_table.is_empty() {
                schema_desc.push_str(")");
            }
        }
        DbType::Sqlite => {
            let tables = get_tables(pool, DbType::Sqlite).await?;
            for (idx, table) in tables.iter().enumerate() {
                let info_query = format!("PRAGMA table_info({});", table);
                let rows = sqlx::query(&info_query).fetch_all(pool).await?;
                
                schema_desc.push_str(&format!("{}(", table));
                let cols: Vec<String> = rows.iter().map(|row| {
                    let name: String = row.try_get("name").unwrap_or_default();
                    let data_type: String = row.try_get("type").unwrap_or_default();
                    format!("{}:{}", name, data_type)
                }).collect();
                
                schema_desc.push_str(&cols.join(", "));
                schema_desc.push_str(")");
                if idx < tables.len() - 1 {
                    schema_desc.push_str(", ");
                }
            }
        }
        DbType::Solana => unreachable!(),
    }

    Ok(schema_desc)
}

pub async fn execute_query(pool: &AnyPool, sql: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let sql_trimmed = sql.trim().to_lowercase();
    let is_query = sql_trimmed.starts_with("select")
        || sql_trimmed.starts_with("show")
        || sql_trimmed.starts_with("explain")
        || sql_trimmed.starts_with("pragma")
        || sql_trimmed.starts_with("describe")
        || sql_trimmed.starts_with("with");

    if !is_query {
        // Execute DML/DDL command
        let result = sqlx::query(sql).execute(pool).await?;
        let headers = vec!["Execution Status".to_string(), "Rows Affected".to_string()];
        let rows = vec![vec!["Query OK".to_string(), result.rows_affected().to_string()]];
        return Ok((headers, rows));
    }

    let rows = sqlx::query(sql).fetch_all(pool).await?;
    if rows.is_empty() {
        return Ok((vec![], vec![]));
    }

    // Extract headers
    let headers: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    // Extract rows and convert values to String dynamically
    let mut result_rows = Vec::new();
    for row in rows {
        let mut row_values = Vec::new();
        for i in 0..row.columns().len() {
            let val_str = match row.try_get::<String, _>(i) {
                Ok(s) => s,
                Err(_) => match row.try_get::<i64, _>(i) {
                    Ok(n) => n.to_string(),
                    Err(_) => match row.try_get::<f64, _>(i) {
                        Ok(f) => f.to_string(),
                        Err(_) => match row.try_get::<bool, _>(i) {
                            Ok(b) => b.to_string(),
                            Err(_) => match row.try_get::<i32, _>(i) {
                                Ok(n) => n.to_string(),
                                Err(_) => "NULL".to_string(),
                            }
                        }
                    }
                }
            };
            row_values.push(val_str);
        }
        result_rows.push(row_values);
    }

    Ok((headers, result_rows))
}

pub async fn execute_update(pool: &AnyPool, sql: &str) -> Result<u64> {
    let result = sqlx::query(sql).execute(pool).await?;
    Ok(result.rows_affected())
}

fn clean_solana_uri(uri: &str) -> String {
    if uri.starts_with("solana://") {
        let host = &uri[9..];
        if host == "localhost" || host.starts_with("localhost:") || host.starts_with("127.0.0.1") {
            format!("http://{}", host)
        } else {
            format!("https://{}", host)
        }
    } else {
        uri.to_string()
    }
}

pub async fn execute_solana_command(uri: &str, command: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let client = reqwest::Client::new();
    let url = clean_solana_uri(uri);
    let cmd = command.trim();
    
    if cmd.starts_with("idl ") {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(anyhow::anyhow!("Usage: idl <path_to_idl.json> <pubkey> <struct_name>"));
        }
        let idl_path = parts[1];
        let pubkey = parts[2];
        let struct_name = parts[3];
        return run_solana_idl_decode(&url, idl_path, pubkey, struct_name, &client).await;
    }
    
    if cmd.starts_with("tx ") {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!("Usage: tx <signature>"));
        }
        let sig = parts[1];
        return fetch_solana_transaction(&url, sig, &client).await;
    }
    
    if cmd.starts_with("history ") {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!("Usage: history <pubkey>"));
        }
        let pubkey = parts[1];
        return fetch_solana_history(&url, pubkey, &client).await;
    }
    
    if cmd.len() >= 32 && cmd.len() <= 44 && !cmd.contains(' ') {
        return fetch_solana_account_info(&url, cmd, &client).await;
    }
    
    Err(anyhow::anyhow!("Unknown Solana command. Use a pubkey, 'tx <signature>', 'history <pubkey>', or 'idl <path> <pubkey> <struct>'."))
}

async fn fetch_solana_account_info(url: &str, pubkey: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let balance_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [pubkey]
    });
    
    let info_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [pubkey, { "encoding": "base64" }]
    });
    
    let balance_res: serde_json::Value = client.post(url).json(&balance_req).send().await?.json().await?;
    let info_res: serde_json::Value = client.post(url).json(&info_req).send().await?.json().await?;
    
    if let Some(err) = info_res.get("error") {
        return Err(anyhow::anyhow!("RPC Error: {}", err["message"].as_str().unwrap_or("Unknown")));
    }
    
    let balance = balance_res["result"]["value"].as_u64().unwrap_or(0) as f64 / 1_000_000_000.0;
    
    let mut rows = vec![
        vec!["Address".to_string(), pubkey.to_string()],
        vec!["Balance (SOL)".to_string(), format!("{:.9}", balance)],
    ];
    
    if let Some(val) = info_res["result"]["value"].as_object() {
        let owner = val.get("owner").and_then(|v| v.as_str()).unwrap_or("");
        let executable = val.get("executable").and_then(|v| v.as_bool()).unwrap_or(false);
        let rent_epoch = val.get("rentEpoch").and_then(|v| v.as_u64()).unwrap_or(0);
        let data_arr = val.get("data").and_then(|v| v.as_array());
        
        let (data_base64, data_len) = if let Some(arr) = data_arr {
            let data_str = arr.get(0).and_then(|v| v.as_str()).unwrap_or("");
            (data_str.to_string(), data_str.len())
        } else {
            (String::new(), 0)
        };
        
        rows.push(vec!["Owner Program".to_string(), owner.to_string()]);
        rows.push(vec!["Executable".to_string(), executable.to_string()]);
        rows.push(vec!["Rent Epoch".to_string(), rent_epoch.to_string()]);
        rows.push(vec!["Data Length".to_string(), data_len.to_string()]);
        rows.push(vec!["Data (Base64)".to_string(), data_base64]);
    } else {
        rows.push(vec!["Status".to_string(), "Account does not exist (unallocated)".to_string()]);
    }
    
    let headers = vec!["Property".to_string(), "Value".to_string()];
    Ok((headers, rows))
}

async fn fetch_solana_history(url: &str, pubkey: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getSignaturesForAddress",
        "params": [pubkey, { "limit": 20 }]
    });
    
    let res: serde_json::Value = client.post(url).json(&req).send().await?.json().await?;
    if let Some(err) = res.get("error") {
        return Err(anyhow::anyhow!("RPC Error: {}", err["message"].as_str().unwrap_or("Unknown")));
    }
    
    let mut rows = Vec::new();
    if let Some(arr) = res["result"].as_array() {
        for item in arr {
            let sig = item["signature"].as_str().unwrap_or("").to_string();
            let slot = item["slot"].as_u64().unwrap_or(0).to_string();
            let block_time = item["blockTime"].as_i64().map(|t| {
                let seconds = t % 60;
                let minutes = (t / 60) % 60;
                let hours = (t / 3600) % 24;
                format!("Unix: {} ({:02}:{:02}:{:02} UTC)", t, hours, minutes, seconds)
            }).unwrap_or_else(|| "N/A".to_string());
            
            let status = if item["err"].is_null() {
                "Success"
            } else {
                "Error"
            }.to_string();
            
            rows.push(vec![sig, slot, block_time, status]);
        }
    }
    
    let headers = vec!["Signature".to_string(), "Slot".to_string(), "Block Time".to_string(), "Status".to_string()];
    Ok((headers, rows))
}

async fn fetch_solana_transaction(url: &str, sig: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [sig, { "encoding": "json", "maxSupportedTransactionVersion": 0 }]
    });
    
    let res: serde_json::Value = client.post(url).json(&req).send().await?.json().await?;
    if let Some(err) = res.get("error") {
        return Err(anyhow::anyhow!("RPC Error: {}", err["message"].as_str().unwrap_or("Unknown")));
    }
    
    if res["result"].is_null() {
        return Err(anyhow::anyhow!("Transaction not found."));
    }
    
    let slot = res["result"]["slot"].as_u64().unwrap_or(0).to_string();
    let fee = res["result"]["meta"]["fee"].as_u64().unwrap_or(0).to_string();
    let logs = res["result"]["meta"]["logMessages"].as_array().map(|arr| {
        arr.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect::<Vec<_>>().join(" | ")
    }).unwrap_or_default();
    
    let err_status = if res["result"]["meta"]["err"].is_null() {
        "Success".to_string()
    } else {
        format!("{:?}", res["result"]["meta"]["err"])
    };
    
    let rows = vec![
        vec!["Signature".to_string(), sig.to_string()],
        vec!["Slot".to_string(), slot],
        vec!["Fee (Lamports)".to_string(), fee],
        vec!["Status".to_string(), err_status],
        vec!["Logs".to_string(), logs],
    ];
    
    let headers = vec!["Field".to_string(), "Value".to_string()];
    Ok((headers, rows))
}

async fn run_solana_idl_decode(
    url: &str,
    idl_path: &str,
    pubkey: &str,
    struct_name: &str,
    client: &reqwest::Client,
) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let idl_content = std::fs::read_to_string(idl_path)?;
    let idl: serde_json::Value = serde_json::from_str(&idl_content)?;
    
    let mut target_struct = None;
    if let Some(accounts) = idl["accounts"].as_array() {
        for acc in accounts {
            if acc["name"].as_str() == Some(struct_name) {
                target_struct = Some(acc.clone());
                break;
            }
        }
    }
    
    if target_struct.is_none() {
        if let Some(types) = idl["types"].as_array() {
            for t in types {
                if t["name"].as_str() == Some(struct_name) {
                    target_struct = Some(t.clone());
                    break;
                }
            }
        }
    }
    
    let target_struct = target_struct.ok_or_else(|| {
        anyhow::anyhow!("Struct '{}' not found in IDL accounts or types.", struct_name)
    })?;
    
    let info_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [pubkey, { "encoding": "base64" }]
    });
    let info_res: serde_json::Value = client.post(url).json(&info_req).send().await?.json().await?;
    if let Some(err) = info_res.get("error") {
        return Err(anyhow::anyhow!("RPC Error: {}", err["message"].as_str().unwrap_or("Unknown")));
    }
    
    let val = &info_res["result"]["value"];
    if val.is_null() {
        return Err(anyhow::anyhow!("Account not found."));
    }
    
    let data_arr = val["data"].as_array().ok_or_else(|| anyhow::anyhow!("No data field found."))?;
    let data_b64 = data_arr.get(0).and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("Invalid data format."))?;
    
    use base64::{Engine as _, engine::general_purpose};
    let data_bytes = general_purpose::STANDARD.decode(data_b64)?;
    
    let rows = decode_borsh_using_idl(&data_bytes, &target_struct)?;
    
    let headers = vec!["Field Name".to_string(), "Type".to_string(), "Value (Parsed)".to_string()];
    Ok((headers, rows))
}

fn decode_borsh_using_idl(data: &[u8], idl_struct: &serde_json::Value) -> Result<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    let fields = idl_struct["type"]["fields"].as_array().ok_or_else(|| {
        anyhow::anyhow!("Struct definition has no fields.")
    })?;
    
    let mut cursor = 0;
    
    // Heuristic: skip 8-byte discriminator for Anchor accounts if the data is large enough
    if data.len() >= 8 {
        cursor = 8;
    }
    
    for field in fields {
        let name = field["name"].as_str().unwrap_or("").to_string();
        let field_type = &field["type"];
        
        let (val_str, bytes_read) = parse_borsh_field(&data[cursor..], field_type)?;
        cursor += bytes_read;
        
        let type_str = if field_type.is_string() {
            field_type.as_str().unwrap_or("").to_string()
        } else {
            format!("{}", field_type)
        };
        
        rows.push(vec![name, type_str, val_str]);
        
        if cursor >= data.len() {
            break;
        }
    }
    
    Ok(rows)
}

fn parse_borsh_field(data: &[u8], field_type: &serde_json::Value) -> Result<(String, usize)> {
    if data.is_empty() {
        return Ok(("EOF".to_string(), 0));
    }
    
    if let Some(t_str) = field_type.as_str() {
        match t_str {
            "u8" => {
                if data.len() < 1 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                Ok((data[0].to_string(), 1))
            }
            "i8" => {
                if data.len() < 1 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                Ok(((data[0] as i8).to_string(), 1))
            }
            "u16" => {
                if data.len() < 2 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                let val = u16::from_le_bytes([data[0], data[1]]);
                Ok((val.to_string(), 2))
            }
            "i16" => {
                if data.len() < 2 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                let val = i16::from_le_bytes([data[0], data[1]]);
                Ok((val.to_string(), 2))
            }
            "u32" => {
                if data.len() < 4 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                let val = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                Ok((val.to_string(), 4))
            }
            "i32" => {
                if data.len() < 4 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                let val = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                Ok((val.to_string(), 4))
            }
            "u64" => {
                if data.len() < 8 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                let val = u64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
                Ok((val.to_string(), 8))
            }
            "i64" => {
                if data.len() < 8 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                let val = i64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
                Ok((val.to_string(), 8))
            }
            "publicKey" => {
                if data.len() < 32 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                let pk_bytes = &data[0..32];
                let pk_str = bs58::encode(pk_bytes).into_string();
                Ok((pk_str, 32))
            }
            "bool" => {
                if data.len() < 1 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                let val = data[0] != 0;
                Ok((val.to_string(), 1))
            }
            "string" => {
                if data.len() < 4 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
                if data.len() < 4 + len { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
                let s = String::from_utf8_lossy(&data[4..4+len]).into_owned();
                Ok((s, 4 + len))
            }
            _ => {
                Ok((format!("Hex: {:?}", &data[0..std::cmp::min(data.len(), 8)]), 0))
            }
        }
    } else if let Some(obj) = field_type.as_object() {
        if let Some(opt_type) = obj.get("option") {
            if data.len() < 1 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
            if data[0] == 0 {
                Ok(("None".to_string(), 1))
            } else {
                let (val, read) = parse_borsh_field(&data[1..], opt_type)?;
                Ok((format!("Some({})", val), 1 + read))
            }
        } else if let Some(vec_type) = obj.get("vec") {
            if data.len() < 4 { return Err(anyhow::anyhow!("Borsh parse error: EOF")); }
            let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
            let mut cursor = 4;
            let mut elements = Vec::new();
            for _ in 0..len {
                if cursor >= data.len() { break; }
                let (val, read) = parse_borsh_field(&data[cursor..], vec_type)?;
                if read == 0 { break; }
                elements.push(val);
                cursor += read;
            }
            Ok((format!("[{}]", elements.join(", ")), cursor))
        } else if let Some(arr_val) = obj.get("array") {
            if let Some(arr) = arr_val.as_array() {
                if arr.len() >= 2 {
                    let elem_type = &arr[0];
                    let len = arr[1].as_u64().unwrap_or(0) as usize;
                    let mut cursor = 0;
                    let mut elements = Vec::new();
                    for _ in 0..len {
                        if cursor >= data.len() { break; }
                        let (val, read) = parse_borsh_field(&data[cursor..], elem_type)?;
                        if read == 0 { break; }
                        elements.push(val);
                        cursor += read;
                    }
                    Ok((format!("[{}]", elements.join(", ")), cursor))
                } else {
                    Ok((format!("Invalid array spec: {:?}", arr_val), 0))
                }
            } else {
                Ok((format!("Invalid array spec: {:?}", arr_val), 0))
            }
        } else if let Some(defined) = obj.get("defined") {
            let type_name = defined.as_str().unwrap_or("defined");
            Ok((format!("Nested Struct: {}", type_name), 0))
        } else {
            Ok((format!("Unsupported complex type: {:?}", field_type), 0))
        }
    } else {
        Ok(("Unknown type".to_string(), 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_integration() {
        // 1. Test database connection with max_connections(1) for in-memory SQLite schema sharing
        sqlx::any::install_default_drivers();
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        
        // 2. Test DDL / command execution
        execute_update(&pool, "CREATE TABLE test_users (id INTEGER PRIMARY KEY, name TEXT);").await.unwrap();
        
        // 3. Test insert command
        execute_update(&pool, "INSERT INTO test_users VALUES (1, 'Alice');").await.unwrap();
        
        // 4. Test table metadata retrieval
        let tables = get_tables(&pool, DbType::Sqlite).await.unwrap();
        assert_eq!(tables, vec!["test_users".to_string()]);
        
        // 5. Test schema extraction for AI context
        let schema = get_schema(&pool, DbType::Sqlite).await.unwrap();
        assert!(schema.contains("test_users(id:INTEGER, name:TEXT)"));

        // 6. Test select query execution & data formatting
        let (headers, rows) = execute_query(&pool, "SELECT * FROM test_users;").await.unwrap();
        assert_eq!(headers, vec!["id".to_string(), "name".to_string()]);
        assert_eq!(rows, vec![vec!["1".to_string(), "Alice".to_string()]]);
        
        // 7. Test identifier escaping and safe inline update
        let esc_table = escape_identifier("test_users", DbType::Sqlite);
        let esc_col = escape_identifier("name", DbType::Sqlite);
        let esc_pk = escape_identifier("id", DbType::Sqlite);
        assert_eq!(esc_table, "\"test_users\"");
        assert_eq!(esc_col, "\"name\"");
        assert_eq!(esc_pk, "\"id\"");

        let update_sql = format!(
            "UPDATE {} SET {} = 'Bob' WHERE {} = '1';",
            esc_table, esc_col, esc_pk
        );
        let rows_affected = execute_update(&pool, &update_sql).await.unwrap();
        assert_eq!(rows_affected, 1);
        
        // 8. Verify data updated successfully
        let (_, new_rows) = execute_query(&pool, "SELECT name FROM test_users;").await.unwrap();
        assert_eq!(new_rows, vec![vec!["Bob".to_string()]]);
    }

    #[test]
    fn test_solana_borsh_deserialization() {
        let idl_struct = serde_json::json!({
            "name": "MyState",
            "type": {
                "kind": "struct",
                "fields": [
                    { "name": "count", "type": "u64" },
                    { "name": "authority", "type": "publicKey" },
                    { "name": "label", "type": "string" },
                    { "name": "active", "type": "bool" }
                ]
            }
        });

        // 8 bytes discriminator + 8 bytes count (42) + 32 bytes authority (all 1s) + 4 bytes string length (5) + "hello" + 1 byte bool (true)
        let mut data = vec![0; 8];
        data.extend_from_slice(&42u64.to_le_bytes());
        data.extend_from_slice(&[1; 32]);
        data.extend_from_slice(&5u32.to_le_bytes());
        data.extend_from_slice(b"hello");
        data.extend_from_slice(&[1]);

        let decoded = decode_borsh_using_idl(&data, &idl_struct).unwrap();
        assert_eq!(decoded.len(), 4);
        assert_eq!(decoded[0], vec!["count".to_string(), "u64".to_string(), "42".to_string()]);
        assert_eq!(decoded[2], vec!["label".to_string(), "string".to_string(), "hello".to_string()]);
        assert_eq!(decoded[3], vec!["active".to_string(), "bool".to_string(), "true".to_string()]);
    }
}
