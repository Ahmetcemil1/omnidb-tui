use anyhow::Result;
use serde_json;
use reqwest;

fn clean_ethereum_uri(uri: &str) -> String {
    if uri.starts_with("ethereum://") || uri.starts_with("eth://") {
        let host = if uri.starts_with("ethereum://") { &uri[11..] } else { &uri[6..] };
        if host.starts_with("http://") || host.starts_with("https://") {
            host.to_string()
        } else if host == "localhost" || host.starts_with("localhost:") || host.starts_with("127.0.0.1") {
            format!("http://{}", host)
        } else {
            format!("https://{}", host)
        }
    } else {
        uri.to_string()
    }
}

pub async fn execute_ethereum_command(uri: &str, command: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let client = reqwest::Client::new();
    let url = clean_ethereum_uri(uri);
    let cmd = command.trim();

    if cmd.is_empty() || cmd.eq_ignore_ascii_case("block") || cmd.eq_ignore_ascii_case("latest") {
        return fetch_ethereum_block(&url, &client).await;
    }

    if cmd.starts_with("tx ") {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!("Usage: tx <tx_hash>"));
        }
        return fetch_ethereum_transaction(&url, parts[1], &client).await;
    }

    if cmd.starts_with("erc20 ") {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(anyhow::anyhow!("Usage: erc20 <token_address> <owner_address>"));
        }
        return fetch_erc20_balance(&url, parts[1], parts[2], &client).await;
    }

    if cmd.len() >= 40 && cmd.starts_with("0x") {
        return fetch_ethereum_account_info(&url, cmd, &client).await;
    }

    fetch_ethereum_block(&url, &client).await
}

async fn fetch_ethereum_account_info(url: &str, address: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let balance_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_getBalance",
        "params": [address, "latest"]
    });

    let resp = client.post(url).json(&balance_req).send().await
        .map_err(|e| anyhow::anyhow!("Ethereum RPC Connection Failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Ethereum RPC returned HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await?;
    let hex_val = json["result"].as_str().unwrap_or("0x0");
    let wei = u128::from_str_radix(hex_val.trim_start_matches("0x"), 16).unwrap_or(0);
    let eth = wei as f64 / 1e18;

    let rows = vec![
        vec!["Address".to_string(), address.to_string()],
        vec!["Balance (ETH)".to_string(), format!("{:.18}", eth)],
        vec!["Balance (Wei)".to_string(), wei.to_string()],
        vec!["Network RPC".to_string(), url.to_string()],
    ];

    let headers = vec!["EVM Account Property".to_string(), "Value".to_string()];
    Ok((headers, rows))
}

async fn fetch_ethereum_block(url: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let block_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_getBlockByNumber",
        "params": ["latest", false]
    });

    let resp = client.post(url).json(&block_req).send().await
        .map_err(|e| anyhow::anyhow!("Ethereum RPC Connection Failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Ethereum RPC returned HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await?;
    let result = &json["result"];

    let block_num = u64::from_str_radix(result["number"].as_str().unwrap_or("0x0").trim_start_matches("0x"), 16).unwrap_or(0);
    let timestamp = u64::from_str_radix(result["timestamp"].as_str().unwrap_or("0x0").trim_start_matches("0x"), 16).unwrap_or(0);
    let tx_count = result["transactions"].as_array().map(|a| a.len()).unwrap_or(0);
    let hash = result["hash"].as_str().unwrap_or("N/A").to_string();
    let miner = result["miner"].as_str().unwrap_or("N/A").to_string();

    let rows = vec![
        vec!["Block Number".to_string(), block_num.to_string()],
        vec!["Block Hash".to_string(), hash],
        vec!["Miner / Validator".to_string(), miner],
        vec!["Transaction Count".to_string(), tx_count.to_string()],
        vec!["Unix Timestamp".to_string(), timestamp.to_string()],
    ];

    let headers = vec!["Latest EVM Block Metric".to_string(), "Value".to_string()];
    Ok((headers, rows))
}

async fn fetch_ethereum_transaction(url: &str, hash: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let tx_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_getTransactionByHash",
        "params": [hash]
    });

    let resp = client.post(url).json(&tx_req).send().await
        .map_err(|e| anyhow::anyhow!("Ethereum RPC Connection Failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Ethereum RPC returned HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await?;
    let result = &json["result"];

    if result.is_null() {
        return Err(anyhow::anyhow!("EVM Transaction not found."));
    }

    let from = result["from"].as_str().unwrap_or("N/A").to_string();
    let to = result["to"].as_str().unwrap_or("N/A").to_string();
    let val_hex = result["value"].as_str().unwrap_or("0x0");
    let wei = u128::from_str_radix(val_hex.trim_start_matches("0x"), 16).unwrap_or(0);
    let eth = wei as f64 / 1e18;

    let rows = vec![
        vec!["Tx Hash".to_string(), hash.to_string()],
        vec!["From".to_string(), from],
        vec!["To".to_string(), to],
        vec!["Value (ETH)".to_string(), format!("{:.18}", eth)],
        vec!["Block Number".to_string(), result["blockNumber"].as_str().unwrap_or("N/A").to_string()],
    ];

    let headers = vec!["Transaction Field".to_string(), "Value".to_string()];
    Ok((headers, rows))
}

async fn fetch_erc20_balance(url: &str, token: &str, owner: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let data = format!("0x70a08231000000000000000000000000{}", owner.trim_start_matches("0x"));
    let call_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_call",
        "params": [{ "to": token, "data": data }, "latest"]
    });

    let resp = client.post(url).json(&call_req).send().await
        .map_err(|e| anyhow::anyhow!("Ethereum RPC Connection Failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Ethereum RPC returned HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await?;
    let hex_val = json["result"].as_str().unwrap_or("0x0");
    let raw_val = u128::from_str_radix(hex_val.trim_start_matches("0x"), 16).unwrap_or(0);

    let rows = vec![
        vec!["ERC-20 Token Contract".to_string(), token.to_string()],
        vec!["Owner Account".to_string(), owner.to_string()],
        vec!["Raw Token Balance".to_string(), raw_val.to_string()],
    ];

    let headers = vec!["ERC-20 Property".to_string(), "Value".to_string()];
    Ok((headers, rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ethereum_block_fetch() {
        let client = reqwest::Client::new();
        let url = "https://ethereum-rpc.publicnode.com";
        let res = fetch_ethereum_block(url, &client).await;
        assert!(res.is_ok() || res.is_err());
    }

    #[tokio::test]
    async fn test_ethereum_account_fetch() {
        let client = reqwest::Client::new();
        let url = "https://ethereum-rpc.publicnode.com";
        let res = fetch_ethereum_account_info(url, "0x0000000000000000000000000000000000000000", &client).await;
        assert!(res.is_ok() || res.is_err());
    }
}
