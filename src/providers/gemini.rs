use super::{CommitProvider, build_prompt};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct GeminiProvider {
    api_key: String,
    model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

#[derive(Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
    generation_config: GenerationConfig,
}

#[derive(Serialize, Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Content,
}

#[async_trait]
impl CommitProvider for GeminiProvider {
    async fn generate(&self, diff: &str, language: &str) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let prompt_text = build_prompt(diff, language);

        let body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt_text }],
            }],
            generation_config: GenerationConfig {
                max_output_tokens: 1024,
                temperature: 0.2,
            },
        };

        let client = Client::new();
        let res = client.post(&url).json(&body).send().await?;

        if !res.status().is_success() {
            let err = res.text().await?;
            anyhow::bail!(t!("provider.api_error", model = self.model, err = err).to_string());
        }

        let response_json: GenerateContentResponse = res.json().await?;

        let text = response_json
            .candidates
            .first()
            .context(t!("provider.no_response").to_string())?
            .content
            .parts
            .first()
            .context(t!("provider.no_text").to_string())?
            .text
            .clone();

        Ok(text.trim().to_string())
    }
}

#[derive(Deserialize)]
struct ListModelsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    name: String,
    #[serde(rename = "supportedGenerationMethods")]
    supported_methods: Option<Vec<String>>,
}

pub async fn list_models(api_key: &str) -> Result<Vec<String>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );

    let client = Client::new();
    let res = client.get(&url).send().await?;

    if !res.status().is_success() {
        anyhow::bail!(t!("provider.list_models_failed", status = res.status()).to_string());
    }

    let list: ListModelsResponse = res.json().await?;

    let mut model_names = Vec::new();

    for model in list.models {
        if let Some(methods) = model.supported_methods {
            if methods.contains(&"generateContent".to_string()) {
                let clean_name = model.name.replace("models/", "");
                model_names.push(clean_name);
            }
        }
    }

    model_names.sort();
    model_names.reverse();

    if model_names.is_empty() {
        anyhow::bail!(t!("provider.no_compatible_model").to_string());
    }

    Ok(model_names)
}
