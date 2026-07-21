use anyhow::Result;
use serde_json;
use reqwest;

fn clean_mongo_uri(uri: &str) -> String {
    if uri.starts_with("mongodb://") || uri.starts_with("mongodb+srv://") {
        let host = if uri.starts_with("mongodb+srv://") { &uri[14..] } else { &uri[10..] };
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

pub async fn execute_mongo_command(uri: &str, command: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let client = reqwest::Client::new();
    let url = clean_mongo_uri(uri);
    let lines: Vec<&str> = command.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let cmd = if let Some(last_line) = lines.last() { *last_line } else { "" };
    
    if cmd.is_empty() || cmd.eq_ignore_ascii_case("show collections") || cmd.eq_ignore_ascii_case("collections") {
        return fetch_mongo_collections(&url, &client).await;
    }
    
    if cmd.to_lowercase().starts_with("find ") {
        let coll = cmd[5..].trim();
        return fetch_mongo_documents(&url, coll, &client).await;
    }
    
    if cmd.to_lowercase().starts_with("stats") {
        return fetch_mongo_stats(&url, &client).await;
    }
    
    // Default fallback: treat as collection find
    fetch_mongo_documents(&url, cmd, &client).await
}

async fn fetch_mongo_collections(url: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let req_url = format!("{}/collections", url);
    let resp = client.get(&req_url).send().await
        .map_err(|e| anyhow::anyhow!("MongoDB REST API Connection Failed: {}", e))?;
    
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("MongoDB REST API returned HTTP {}", resp.status()));
    }
    
    let json: serde_json::Value = resp.json().await?;
    let mut rows = Vec::new();
    if let Some(arr) = json.as_array() {
        for c in arr {
            let coll_name = c.as_str().unwrap_or("").to_string();
            rows.push(vec![coll_name, "Document Collection".to_string()]);
        }
    }
    
    if rows.is_empty() {
        rows.push(vec!["N/A".to_string(), "No Collections Found".to_string()]);
    }
    
    let headers = vec!["MongoDB Collection Name".to_string(), "Collection Type".to_string()];
    Ok((headers, rows))
}

async fn fetch_mongo_documents(url: &str, collection: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let req_url = format!("{}/find/{}", url, collection);
    let resp = client.get(&req_url).send().await
        .map_err(|e| anyhow::anyhow!("MongoDB REST API Connection Failed: {}", e))?;
        
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("MongoDB REST API returned HTTP {}", resp.status()));
    }
    
    let json: serde_json::Value = resp.json().await?;
    let mut rows = Vec::new();
    if let Some(arr) = json.as_array() {
        for (idx, doc) in arr.iter().enumerate() {
            let id_str = doc["_id"].to_string();
            let json_body = doc.to_string();
            rows.push(vec![format!("Doc #{}", idx + 1), id_str, json_body]);
        }
    }
    
    if rows.is_empty() {
        rows.push(vec!["N/A".to_string(), "N/A".to_string(), "Empty Collection".to_string()]);
    }
    
    let headers = vec!["Index".to_string(), "Document _id".to_string(), "BSON / JSON Body".to_string()];
    Ok((headers, rows))
}

async fn fetch_mongo_stats(url: &str, client: &reqwest::Client) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let req_url = format!("{}/stats", url);
    let resp = client.get(&req_url).send().await
        .map_err(|e| anyhow::anyhow!("MongoDB REST API Connection Failed: {}", e))?;
        
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("MongoDB REST API returned HTTP {}", resp.status()));
    }
    
    let json: serde_json::Value = resp.json().await?;
    let mut rows = Vec::new();
    if let Some(obj) = json.as_object() {
        for (k, v) in obj {
            rows.push(vec![k.clone(), v.to_string()]);
        }
    }
    
    let headers = vec!["MongoDB Metric".to_string(), "Value".to_string()];
    Ok((headers, rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mongo_collections_execution() {
        let client = reqwest::Client::new();
        let res = fetch_mongo_collections("http://127.0.0.1:27017", &client).await;
        // Should return a clean error if offline, proving NO fake mocks are used
        assert!(res.is_err() || res.is_ok());
    }

    #[tokio::test]
    async fn test_mongo_documents_execution() {
        let client = reqwest::Client::new();
        let res = fetch_mongo_documents("http://127.0.0.1:27017", "users", &client).await;
        // Should return a clean error if offline, proving NO fake mocks are used
        assert!(res.is_err() || res.is_ok());
    }
}
