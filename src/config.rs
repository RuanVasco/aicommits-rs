use crate::providers::{gemini, openai_compat};
use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
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
    #[serde(rename = "openai")]
    OpenAiCompatible {
        base_url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        api_key: Option<String>,
        model: String,
    },
    #[serde(rename = "embedded")]
    Embedded,
}

impl fmt::Display for ProviderConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderConfig::Gemini { model, .. } => write!(f, "{model} (Gemini)"),
            ProviderConfig::OpenAiCompatible {
                model, base_url, ..
            } => write!(f, "{model} ({base_url})"),
            ProviderConfig::Embedded => write!(f, "TinyLlama-1.1B-Chat (embedded, local)"),
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

    let provider_options = vec![
        "Google Gemini (API na nuvem)",
        "Endpoint OpenAI-compatible (Ollama, LM Studio, llama.cpp server, OpenAI, etc.)",
        "Modelo local embutido (baixa um modelo pequeno automaticamente, sem API)",
    ];

    let provider_selection = Select::with_theme(&theme)
        .with_prompt("Qual provider você quer usar?")
        .default(0)
        .items(&provider_options)
        .interact()?;

    let config = match provider_selection {
        0 => setup_gemini(&theme).await?,
        1 => setup_openai_compatible(&theme).await?,
        _ => setup_embedded(&theme)?,
    };

    save_config(&config)?;
    println!("Configuração salva com sucesso!\n");

    Ok(config)
}

async fn setup_gemini(theme: &ColorfulTheme) -> Result<AppConfig> {
    println!("Obtenha sua chave em: https://aistudio.google.com/app/apikey\n");

    let api_key: String = Input::with_theme(theme)
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

    let selection = Select::with_theme(theme)
        .with_prompt("Escolha o modelo padrão")
        .default(0)
        .items(&models)
        .interact()?;

    Ok(AppConfig {
        provider: ProviderConfig::Gemini {
            api_key,
            model: models[selection].to_string(),
        },
    })
}

async fn setup_openai_compatible(theme: &ColorfulTheme) -> Result<AppConfig> {
    println!(
        "Informe a URL base da API (ex: http://localhost:11434/v1 para Ollama, \
        http://localhost:1234/v1 para LM Studio, https://api.openai.com/v1 para OpenAI).\n"
    );

    let base_url: String = Input::with_theme(theme)
        .with_prompt("URL base")
        .interact_text()?;

    let api_key_input: String = Input::with_theme(theme)
        .with_prompt("API Key (deixe em branco se o servidor local não exigir)")
        .allow_empty(true)
        .interact_text()?;

    let api_key = if api_key_input.trim().is_empty() {
        None
    } else {
        Some(api_key_input)
    };

    let model = match openai_compat::list_models(&base_url, api_key.as_deref()).await {
        Ok(list) if !list.is_empty() => {
            let selection = Select::with_theme(theme)
                .with_prompt("Escolha o modelo")
                .default(0)
                .items(&list)
                .interact()?;
            list[selection].clone()
        }
        _ => {
            println!("Não foi possível listar modelos automaticamente nesse endpoint.");
            Input::with_theme(theme)
                .with_prompt("Digite o nome do modelo")
                .interact_text()?
        }
    };

    Ok(AppConfig {
        provider: ProviderConfig::OpenAiCompatible {
            base_url,
            api_key,
            model,
        },
    })
}

fn setup_embedded(theme: &ColorfulTheme) -> Result<AppConfig> {
    println!(
        "O modelo local (TinyLlama 1.1B, ~700MB em Q4) será baixado automaticamente na \
        primeira vez que você gerar uma mensagem de commit, e fica em cache no seu computador \
        (não precisa de API key nem conexão depois disso)."
    );

    let confirmed = Confirm::with_theme(theme)
        .with_prompt("Continuar com o modelo local embutido?")
        .default(true)
        .interact()?;

    if !confirmed {
        anyhow::bail!("Setup cancelado.");
    }

    Ok(AppConfig {
        provider: ProviderConfig::Embedded,
    })
}

fn save_config(config: &AppConfig) -> Result<()> {
    let config_path = get_config_path()?;
    let toml_string = toml::to_string(config)?;
    fs::write(config_path, toml_string)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gemini_config_round_trips() {
        let config = AppConfig {
            provider: ProviderConfig::Gemini {
                api_key: "key".to_string(),
                model: "gemini-2.0-flash".to_string(),
            },
        };

        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("provider = \"gemini\""));

        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        match parsed.provider {
            ProviderConfig::Gemini { api_key, model } => {
                assert_eq!(api_key, "key");
                assert_eq!(model, "gemini-2.0-flash");
            }
            other => panic!("expected Gemini variant, got {other:?}"),
        }
    }

    #[test]
    fn openai_compatible_config_omits_api_key_when_none() {
        let config = AppConfig {
            provider: ProviderConfig::OpenAiCompatible {
                base_url: "http://localhost:11434/v1".to_string(),
                api_key: None,
                model: "llama3.1".to_string(),
            },
        };

        let toml_str = toml::to_string(&config).unwrap();
        assert!(!toml_str.contains("api_key"));
    }

    #[test]
    fn openai_compatible_config_round_trips_with_api_key() {
        let config = AppConfig {
            provider: ProviderConfig::OpenAiCompatible {
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: Some("sk-test".to_string()),
                model: "gpt-4o-mini".to_string(),
            },
        };

        let toml_str = toml::to_string(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        match parsed.provider {
            ProviderConfig::OpenAiCompatible { api_key, .. } => {
                assert_eq!(api_key.as_deref(), Some("sk-test"));
            }
            other => panic!("expected OpenAiCompatible variant, got {other:?}"),
        }
    }

    #[test]
    fn embedded_config_round_trips() {
        let config = AppConfig {
            provider: ProviderConfig::Embedded,
        };

        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("provider = \"embedded\""));

        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert!(matches!(parsed.provider, ProviderConfig::Embedded));
    }

    #[test]
    fn legacy_config_format_parses() {
        let legacy_toml = "api_key = \"old-key\"\nmodel = \"gemini-1.5-flash\"\n";
        let legacy: LegacyAppConfig = toml::from_str(legacy_toml).unwrap();
        assert_eq!(legacy.api_key, "old-key");
        assert_eq!(legacy.model, "gemini-1.5-flash");
    }

    #[test]
    fn display_formats_each_provider() {
        let gemini = ProviderConfig::Gemini {
            api_key: "x".to_string(),
            model: "gemini-2.0-flash".to_string(),
        };
        assert_eq!(gemini.to_string(), "gemini-2.0-flash (Gemini)");

        let openai = ProviderConfig::OpenAiCompatible {
            base_url: "http://localhost:11434/v1".to_string(),
            api_key: None,
            model: "llama3.1".to_string(),
        };
        assert_eq!(openai.to_string(), "llama3.1 (http://localhost:11434/v1)");

        assert_eq!(
            ProviderConfig::Embedded.to_string(),
            "TinyLlama-1.1B-Chat (embedded, local)"
        );
    }
}
