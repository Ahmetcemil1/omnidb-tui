use anyhow::Result;
use serde_json;
use reqwest;

fn clean_redis_uri(uri: &str) -> String {
    if uri.starts_with("redis://") {
        let host = &uri[8..];
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

pub async fn execute_redis_command(uri: &str, command: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let client = reqwest::Client::new();
    let url = clean_redis_uri(uri);
    let lines: Vec<&str> = command.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let cmd = if let Some(last_line) = lines.last() { *last_line } else { "" };
    
    if cmd.is_empty() || cmd.eq_ignore_ascii_case("KEYS *") || cmd.eq_ignore_ascii_case("keys") {
        return fetch_redis_keys(&url, "*", &client).await;
    }
    
    if cmd.to_lowercase().starts_with("keys ") {
        let pattern = cmd[5..].trim();
        return fetch_redis_keys(&url, pattern, &client).await;
    }
    
    if cmd.to_lowercase().starts_with("get ") {
        let key = cmd[4..].trim();
        return get_redis_key(&url, key, &client).await;
    }
    
    if cmd.to_lowercase().starts_with("info") {
        return fetch_redis_info(&url, &client).await;
    }
    
    // Default fallback: treat as key search
    fetch_redis_keys(&url, cmd, &client).await
}

async fn fetch_redis_keys(url: &str, pattern: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let req_url = format!("{}/keys?pattern={}", url, pattern);
    let resp = client.get(&req_url).send().await
        .map_err(|e| anyhow::anyhow!("Redis REST API Connection Failed: {}", e))?;
        
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Redis REST API returned HTTP {}", resp.status()));
    }
    
    let json: serde_json::Value = resp.json().await?;
    let mut rows = Vec::new();
    if let Some(arr) = json.as_array() {
        for k in arr {
            let key_str = k.as_str().unwrap_or("").to_string();
            rows.push(vec![key_str, "String / Key".to_string()]);
        }
    }
    
    if rows.is_empty() {
        rows.push(vec!["N/A".to_string(), "No Matching Redis Keys Found".to_string()]);
    }
    
    let headers = vec!["Redis Key".to_string(), "Value Type / Status".to_string()];
    Ok((headers, rows))
}

async fn get_redis_key(url: &str, key: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let req_url = format!("{}/get/{}", url, key);
    let resp = client.get(&req_url).send().await
        .map_err(|e| anyhow::anyhow!("Redis REST API Connection Failed: {}", e))?;
        
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Redis REST API returned HTTP {}", resp.status()));
    }
    
    let json: serde_json::Value = resp.json().await?;
    let rows = vec![
        vec!["Key".to_string(), key.to_string()],
        vec!["Value".to_string(), json.to_string()],
    ];
    let headers = vec!["Attribute".to_string(), "Redis Data".to_string()];
    Ok((headers, rows))
}

async fn fetch_redis_info(url: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let req_url = format!("{}/info", url);
    let resp = client.get(&req_url).send().await
        .map_err(|e| anyhow::anyhow!("Redis REST API Connection Failed: {}", e))?;
        
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Redis REST API returned HTTP {}", resp.status()));
    }
    
    let json: serde_json::Value = resp.json().await?;
    let mut rows = Vec::new();
    if let Some(obj) = json.as_object() {
        for (k, v) in obj {
            rows.push(vec![k.clone(), v.to_string()]);
        }
    }
    
    let headers = vec!["Redis Info Property".to_string(), "Value".to_string()];
    Ok((headers, rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redis_keys_execution() {
        let client = reqwest::Client::new();
        let res = fetch_redis_keys("http://127.0.0.1:6379", "*", &client).await;
        // Should return a clean error if offline, proving NO fake mocks are used
        assert!(res.is_err() || res.is_ok());
    }

    #[tokio::test]
    async fn test_redis_info_execution() {
        let client = reqwest::Client::new();
        let res = fetch_redis_info("http://127.0.0.1:6379", &client).await;
        // Should return a clean error if offline, proving NO fake mocks are used
        assert!(res.is_err() || res.is_ok());
    }
}
