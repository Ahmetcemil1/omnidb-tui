use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    system: String,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

#[derive(Deserialize)]
struct OllamaModelList {
    models: Vec<OllamaModelItem>,
}

#[derive(Deserialize)]
struct OllamaModelItem {
    name: String,
}

/// Retrieve the first available model in local Ollama, defaulting to "qwen2.5-coder"
async fn get_active_model(client: &reqwest::Client) -> String {
    let url = "http://localhost:11434/api/tags";
    if let Ok(resp) = client.get(url).send().await {
        if let Ok(model_list) = resp.json::<OllamaModelList>().await {
            if !model_list.models.is_empty() {
                // Return the first available model
                return model_list.models[0].name.clone();
            }
        }
    }
    "qwen2.5-coder".to_string()
}

pub async fn generate_sql(prompt: &str, schema: &str, db_type: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let model = get_active_model(&client).await;

    let system_prompt = format!(
        "You are an expert SQL generation assistant for {}. \
        Given the database schema provided, generate only a valid, raw SQL query matching the user request. \
        Do NOT wrap the SQL in code blocks (no ```sql or ```). Do NOT include any explanations, introduction, or comments. \
        Output ONLY the raw SQL query.",
        db_type
    );

    let full_prompt = format!(
        "Database Schema:\n{}\n\nUser Request: {}\n\nGenerated SQL:",
        schema, prompt
    );

    let req_payload = OllamaGenerateRequest {
        model,
        prompt: full_prompt,
        stream: false,
        system: system_prompt,
    };

    let url = "http://localhost:11434/api/generate";
    let resp = client
        .post(url)
        .json(&req_payload)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to connect to local Ollama API: {}. Make sure Ollama is running ('ollama serve').", e))?;

    if !resp.status().is_success() {
        return Err(anyhow!("Ollama server returned status code: {}", resp.status()));
    }

    let result = resp.json::<OllamaGenerateResponse>().await?;
    let sql_clean = result.response.trim().trim_matches('`').trim().to_string();
    
    Ok(sql_clean)
}

pub async fn explain_sql(sql: &str, schema: &str, db_type: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?;

    let model = get_active_model(&client).await;

    let system_prompt = format!(
        "You are an expert database administrator and query optimizer for {}. \
        Explain the given SQL query, analyze its performance, point out any bottlenecks, and suggest index additions or query rewrites to make it faster. \
        Be concise, professional, and clear. Format the output in clean, readable text.",
        db_type
    );

    let full_prompt = format!(
        "Database Schema:\n{}\n\nSQL Query:\n{}\n\nOptimization & Explanation:",
        schema, sql
    );

    let req_payload = OllamaGenerateRequest {
        model,
        prompt: full_prompt,
        stream: false,
        system: system_prompt,
    };

    let url = "http://localhost:11434/api/generate";
    let resp = client
        .post(url)
        .json(&req_payload)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to connect to local Ollama API: {}. Make sure Ollama is running.", e))?;

    if !resp.status().is_success() {
        return Err(anyhow!("Ollama server returned status code: {}", resp.status()));
    }

    let result = resp.json::<OllamaGenerateResponse>().await?;
    Ok(result.response.trim().to_string())
}

