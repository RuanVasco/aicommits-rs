use crate::providers::{gemini, openai_compat};
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
    #[serde(rename = "openai")]
    OpenAiCompatible {
        base_url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        api_key: Option<String>,
        model: String,
    },
}

impl fmt::Display for ProviderConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderConfig::Gemini { model, .. } => write!(f, "{model} (Gemini)"),
            ProviderConfig::OpenAiCompatible {
                model, base_url, ..
            } => write!(f, "{model} ({base_url})"),
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
        .context(t!("config.home_dir_error").to_string())?;

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
            println!("{}", t!("config.legacy_migrating"));
            let migrated = AppConfig {
                provider: ProviderConfig::Gemini {
                    api_key: legacy.api_key,
                    model: legacy.model,
                },
            };
            save_config(&migrated)?;
            return Ok(migrated);
        }

        anyhow::bail!(t!("config.corrupted_config").to_string());
    }

    println!("{}", t!("config.no_config_found"));
    run_setup().await
}

pub async fn run_setup() -> Result<AppConfig> {
    let theme = ColorfulTheme::default();

    println!("{}", t!("config.welcome"));

    let provider_options = vec![
        t!("config.provider_gemini_option").to_string(),
        t!("config.provider_openai_option").to_string(),
    ];

    let provider_selection = Select::with_theme(&theme)
        .with_prompt(t!("config.choose_provider").to_string())
        .default(0)
        .items(&provider_options)
        .interact()?;

    let config = match provider_selection {
        0 => setup_gemini(&theme).await?,
        _ => setup_openai_compatible(&theme).await?,
    };

    save_config(&config)?;
    println!("{}", t!("config.config_saved"));

    Ok(config)
}

async fn setup_gemini(theme: &ColorfulTheme) -> Result<AppConfig> {
    println!("{}", t!("config.get_gemini_key"));

    let api_key: String = Input::with_theme(theme)
        .with_prompt(t!("config.paste_gemini_key").to_string())
        .interact_text()?;

    let models = match gemini::list_models(&api_key).await {
        Ok(list) => list,
        Err(e) => {
            println!("{}", t!("config.models_list_failed", error = e));
            println!("{}", t!("config.using_fallback_list"));
            vec![
                "gemini-2.0-flash".to_string(),
                "gemini-1.5-flash".to_string(),
                "gemini-1.5-pro".to_string(),
            ]
        }
    };

    let selection = Select::with_theme(theme)
        .with_prompt(t!("config.choose_default_model").to_string())
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

// Every OpenAI-compatible server (Ollama, LM Studio, llama.cpp server, OpenAI
// itself) speaks the same chat-completions request/response shape, so there's
// nothing service-specific to ask beyond the URL - defaulting to Ollama's address
// just saves typing for the common case, it isn't a different code path.
const DEFAULT_BASE_URL: &str = "http://localhost:11434/v1";

async fn setup_openai_compatible(theme: &ColorfulTheme) -> Result<AppConfig> {
    println!(
        "{}",
        t!("config.base_url_hint", default_url = DEFAULT_BASE_URL)
    );

    let base_url: String = Input::with_theme(theme)
        .with_prompt(t!("config.base_url").to_string())
        .default(DEFAULT_BASE_URL.to_string())
        .interact_text()?;

    let api_key_input: String = Input::with_theme(theme)
        .with_prompt(t!("config.api_key_optional").to_string())
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
                .with_prompt(t!("config.choose_model").to_string())
                .default(0)
                .items(&list)
                .interact()?;
            list[selection].clone()
        }
        Ok(_) => {
            println!(
                "{}",
                t!("config.no_models_found", base_url = base_url.as_str())
            );
            Input::with_theme(theme)
                .with_prompt(t!("config.enter_model_name").to_string())
                .interact_text()?
        }
        Err(e) => {
            println!(
                "{}",
                t!(
                    "config.models_list_failed_at",
                    base_url = base_url.as_str(),
                    error = e
                )
            );
            println!("{}", t!("config.check_server_running"));
            Input::with_theme(theme)
                .with_prompt(t!("config.enter_model_name").to_string())
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
    }
}
