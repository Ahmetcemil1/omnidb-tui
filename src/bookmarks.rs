use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SshTunnelConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: String,
    pub remote_db_host: String,
    pub remote_db_port: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Bookmark {
    pub name: String,
    pub connection_uri: String,
    pub ssh_tunnel: Option<SshTunnelConfig>,
}

pub fn get_config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/zenhor".to_string());
    let mut p = PathBuf::from(home);
    p.push(".config");
    p.push("omnidb");
    p
}

pub fn load_bookmarks() -> Result<Vec<Bookmark>> {
    let config_dir = get_config_dir();
    let mut path = config_dir.clone();
    path.push("connections.json");

    if !path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&path)
        .context("Failed to read bookmarks config file")?;
    let bookmarks: Vec<Bookmark> = serde_json::from_str(&content)
        .context("Failed to parse bookmarks JSON")?;
    Ok(bookmarks)
}

pub fn save_bookmarks(bookmarks: &[Bookmark]) -> Result<()> {
    let config_dir = get_config_dir();
    fs::create_dir_all(&config_dir)
        .context("Failed to create config directory")?;
    
    let mut path = config_dir;
    path.push("connections.json");

    let content = serde_json::to_string_pretty(bookmarks)
        .context("Failed to serialize bookmarks to JSON")?;
    fs::write(&path, content)
        .context("Failed to write bookmarks to config file")?;
    Ok(())
}

pub fn get_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .unwrap_or(28392)
}

pub fn parse_host_port_from_uri(uri: &str, default_port: u16) -> (String, u16) {
    if let Some(at_idx) = uri.find('@') {
        let after_at = &uri[at_idx + 1..];
        let end_idx = after_at.find('/').or_else(|| after_at.find('?')).unwrap_or(after_at.len());
        let host_port = &after_at[..end_idx];
        if let Some(colon_idx) = host_port.rfind(':') {
            let host = &host_port[..colon_idx];
            let port_str = &host_port[colon_idx + 1..];
            if let Ok(port) = port_str.parse::<u16>() {
                return (host.to_string(), port);
            }
        }
        return (host_port.to_string(), default_port);
    }
    ("127.0.0.1".to_string(), default_port)
}

pub fn rewrite_uri_for_ssh(uri: &str, local_port: u16) -> String {
    if let Some(at_idx) = uri.find('@') {
        let after_at = &uri[at_idx + 1..];
        let end_idx = after_at.find('/').or_else(|| after_at.find('?')).unwrap_or(after_at.len());
        let mut new_uri = uri[..at_idx + 1].to_string();
        new_uri.push_str(&format!("127.0.0.1:{}", local_port));
        new_uri.push_str(&after_at[end_idx..]);
        new_uri
    } else {
        uri.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_host_port_from_uri() {
        let (host, port) = parse_host_port_from_uri("postgres://user:pass@1.2.3.4:5432/db", 5432);
        assert_eq!(host, "1.2.3.4");
        assert_eq!(port, 5432);

        let (host2, port2) = parse_host_port_from_uri("postgres://user:pass@mydb/db", 5432);
        assert_eq!(host2, "mydb");
        assert_eq!(port2, 5432);
    }

    #[test]
    fn test_rewrite_uri_for_ssh() {
        let rewritten = rewrite_uri_for_ssh("postgres://user:pass@1.2.3.4:5432/db?sslmode=disable", 12345);
        assert_eq!(rewritten, "postgres://user:pass@127.0.0.1:12345/db?sslmode=disable");
    }
}


