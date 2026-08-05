use crate::providers::gemini;
use anyhow::{Context, Result};
use dialoguer::{Input, Select, theme::ColorfulTheme};
use directories::ProjectDirs;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    #[serde(flatten)]
    pub provider: ProviderConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "provider")]
pub enum ProviderConfig {
    #[serde(rename = "gemini")]
    Gemini { api_key: String, model: String },
}

impl fmt::Display for ProviderConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderConfig::Gemini { model, .. } => write!(f, "{model} (Gemini)"),
        }
    }
}

#[derive(Deserialize)]
struct LegacyAppConfig {
    api_key: String,
    model: String,
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

        if let Ok(config) = toml::from_str::<AppConfig>(&content) {
            return Ok(config);
        }

        if let Ok(legacy) = toml::from_str::<LegacyAppConfig>(&content) {
            println!("Formato de configuração antigo detectado. Migrando para Gemini automaticamente...");
            let migrated = AppConfig {
                provider: ProviderConfig::Gemini {
                    api_key: legacy.api_key,
                    model: legacy.model,
                },
            };
            save_config(&migrated)?;
            return Ok(migrated);
        }

        anyhow::bail!("Arquivo de configuração corrompido. Rode 'aic setup' novamente.");
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

    let models = match gemini::list_models(&api_key).await {
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
        provider: ProviderConfig::Gemini {
            api_key,
            model: models[selection].to_string(),
        },
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
