use super::{CommitProvider, build_prompt};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct OpenAiCompatProvider {
    base_url: String,
    api_key: Option<String>,
    model: String,
}

impl OpenAiCompatProvider {
    pub fn new(base_url: String, api_key: Option<String>, model: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
        }
    }
}

#[derive(Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[async_trait]
impl CommitProvider for OpenAiCompatProvider {
    async fn generate(&self, diff: &str, language: &str) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url);
        let prompt_text = build_prompt(diff, language);

        let body = ChatCompletionRequest {
            model: &self.model,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt_text,
            }],
            temperature: 0.2,
            max_tokens: 1024,
        };

        let client = Client::new();
        let mut req = client.post(&url).json(&body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let res = req
            .send()
            .await
            .context(t!("provider.connect_failed").to_string())?;

        if !res.status().is_success() {
            let err = res.text().await?;
            anyhow::bail!(t!("provider.api_error", model = self.model, err = err).to_string());
        }

        let response_json: ChatCompletionResponse = res.json().await?;

        let text = response_json
            .choices
            .into_iter()
            .next()
            .context(t!("provider.no_response").to_string())?
            .message
            .content;

        Ok(text.trim().to_string())
    }
}

#[derive(Deserialize)]
struct ListModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    id: String,
}

pub async fn list_models(base_url: &str, api_key: Option<&str>) -> Result<Vec<String>> {
    let base_url = base_url.trim_end_matches('/');
    let url = format!("{}/models", base_url);

    let client = Client::new();
    let mut req = client.get(&url);
    if let Some(key) = api_key {
        req = req.bearer_auth(key);
    }

    let res = req.send().await?;

    if !res.status().is_success() {
        anyhow::bail!(t!("provider.list_models_failed", status = res.status()).to_string());
    }

    let list: ListModelsResponse = res.json().await?;

    if list.data.is_empty() {
        anyhow::bail!(t!("provider.no_model_found").to_string());
    }

    let mut names: Vec<String> = list.data.into_iter().map(|m| m.id).collect();
    names.sort();

    Ok(names)
}
