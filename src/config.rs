use anyhow::{Context, Result};
use dialoguer::{Input, Select, theme::ColorfulTheme};
use directories::ProjectDirs;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub api_key: String,
    pub model: String,
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

async fn get_models(api_key: &str) -> Result<Vec<String>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );

    let client = Client::new();
    let res = client.get(&url).send().await?;

    if !res.status().is_success() {
        anyhow::bail!("Falha ao listar modelos: {}", res.status());
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
        anyhow::bail!("Nenhum modelo compatível encontrado.");
    }

    Ok(model_names)
}

fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "aicommits", "aicommits-rs")
        .context("Não foi possível determinar o diretório home do usuário")?;

    let config_dir = proj_dirs.config_dir();

    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }

    Ok(config_dir.join("config.toml"))
}

pub async fn load_or_setup() -> Result<AppConfig> {
    let config_path = get_config_path()?;

    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        let config: AppConfig = toml::from_str(&content)
            .context("Arquivo de configuração corrompido. Tente rodar com --reset")?;
        return Ok(config);
    }

    println!("Nenhuma configuração encontrada. Iniciando setup...");
    run_setup().await
}

pub async fn run_setup() -> Result<AppConfig> {
    let theme = ColorfulTheme::default();

    println!("\nBem-vindo ao AI Commits RS! Vamos configurar.");
    println!("Obtenha sua chave em: https://aistudio.google.com/app/apikey\n");

    let api_key: String = Input::with_theme(&theme)
        .with_prompt("Cole sua Google Gemini API Key")
        .interact_text()?;

    let models = match get_models(&api_key).await {
        Ok(list) => list,
        Err(e) => {
            println!("Não foi possível listar modelos automaticamente: {}", e);
            println!("Usando lista padrão de fallback.");
            vec![
                "gemini-2.0-flash".to_string(),
                "gemini-1.5-flash".to_string(),
                "gemini-1.5-pro".to_string(),
            ]
        }
    };

    let selection = Select::with_theme(&theme)
        .with_prompt("Escolha o modelo padrão")
        .default(0)
        .items(&models)
        .interact()?;

    let config = AppConfig {
        api_key,
        model: models[selection].to_string(),
    };

    save_config(&config)?;
    println!("Configuração salva com sucesso!\n");

    Ok(config)
}

fn save_config(config: &AppConfig) -> Result<()> {
    let config_path = get_config_path()?;
    let toml_string = toml::to_string(config)?;
    fs::write(config_path, toml_string)?;
    Ok(())
}
