mod config;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dialoguer::{Select, theme::ColorfulTheme};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Parser)]
#[command(name = "aic")]
#[command(version)]
#[command(about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long)]
    all: bool,

    #[arg(short, long)]
    print_only: bool,

    #[arg(short, long, default_value = "English")]
    language: String,
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

async fn generate_commit(api_key: &str, model: &str, diff: &str, language: &str) -> Result<String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let prompt_text = format!(
        "Act as a commit message generator. 
        Analyze the git diff below and generate a SINGLE, complete line of commit message following the Conventional Commits specification (e.g., feat, fix, chore, docs).
        The message must be concise, objective, and in {language}.
        Do not truncate the sentence. Do not use quotes or markdown code blocks.
        
        Diff:
        {diff}",
        language = language,
        diff = diff
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

    if cli.all {
        println!("Adicionando todos os arquivos (git add .)...");
        let add_all_status = Command::new("git")
            .arg("add")
            .arg(".")
            .status()
            .context("Falha ao executar git add")?;

        if !add_all_status.success() {
            anyhow::bail!("O git add falhou.");
        }
    }

    println!("Analisando alterações no git...");
    let diff = get_git_diff()?;

    let final_msg: String;

    loop {
        println!("Gerando mensagem de commit com {}...", cfg.model);
        let msg = generate_commit(&cfg.api_key, &cfg.model, &diff, &cli.language).await?;

        if cli.print_only {
            println!("\n--- Sugestão de Commit Message ---\n");
            println!("{}", msg);
            println!("\n----------------------------------");
            return Ok(());
        }

        println!("\nSugestão: \x1b[1;32m{}\x1b[0m\n", msg);

        let options = vec!["Confirmar (Commit & Push)", "Gerar Novamente", "Cancelar"];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("O que deseja fazer?")
            .default(0)
            .items(&options)
            .interact()?;

        match selection {
            0 => {
                final_msg = msg;
                break;
            }
            1 => {
                println!("Tentando outra opção...\n");
                continue;
            }
            _ => {
                println!("Operação cancelada pelo usuário.");
                return Ok(());
            }
        }
    }

    let commit_status = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(&final_msg)
        .status()
        .context("Falha ao executar git commit")?;

    if !commit_status.success() {
        anyhow::bail!("O git commit falhou. Verifique se há arquivos staged.");
    }

    println!("Executando git push...");

    let push_status = Command::new("git")
        .arg("push")
        .status()
        .context("Falha ao executar git push")?;

    if push_status.success() {
        println!("Sucesso! Alterações enviadas.");
        return Ok(());
    }

    anyhow::bail!("O git push falhou. Verifique sua conexão ou permissões.");
}
