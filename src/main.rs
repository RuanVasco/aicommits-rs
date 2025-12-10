mod config;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Parser)]
#[command(name = "aicommits")]
#[command(about = "Gera mensagens de commit usando IA", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Setup,
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

fn get_git_diff() -> Result<String> {
    let output = Command::new("git")
        .arg("diff")
        .arg("--staged")
        .output()
        .context("Falha ao executar o comando 'git'. O git está instalado e no PATH?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Erro ao rodar git diff: {}", stderr);
    }

    let diff =
        String::from_utf8(output.stdout).context("O output do git diff não é um UTF-8 válido")?;

    if diff.trim().is_empty() {
        anyhow::bail!(
            "Nenhuma alteração staged encontrada. Use 'git add' antes de rodar o gerador."
        );
    }

    Ok(diff)
}

async fn generate_commit(api_key: &str, model: &str, diff: &str) -> Result<String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let prompt_text = format!(
        "Act as a commit message generator. 
        Analyze the git diff below and generate a SINGLE, complete line of commit message following the Conventional Commits specification (e.g., feat, fix, chore, docs).
        The message must be concise, objective, and in English.
        Do not truncate the sentence. Do not use quotes or markdown code blocks.
        
        Diff:
        {}",
        diff
    );

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
        anyhow::bail!("Erro da API ({}): {}", model, err);
    }

    let response_json: GenerateContentResponse = res.json().await?;

    let text = response_json
        .candidates
        .first()
        .context("Sem resposta")?
        .content
        .parts
        .first()
        .context("Sem texto")?
        .text
        .clone();

    Ok(text.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Setup) = cli.command {
        config::run_setup().await?;
        return Ok(());
    }

    let cfg = config::load_or_setup().await?;

    println!("Analisando alterações no git...");
    let diff = get_git_diff()?;

    println!("Gerando mensagem de commit com {}...", cfg.model);

    let msg = generate_commit(&cfg.api_key, &cfg.model, &diff).await?;

    println!("\n--- Sugestão de Commit Message ---\n");
    println!("{}", msg);
    println!("\n----------------------------------");

    Ok(())
}
