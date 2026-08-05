mod config;
mod providers;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dialoguer::{Select, theme::ColorfulTheme};
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

    #[arg(short = 'y', long = "push")]
    push: bool,
}

#[derive(Subcommand)]
enum Commands {
    Setup,
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

    let provider = providers::build(&cfg.provider)?;

    let final_msg: String;

    loop {
        println!("Gerando mensagem de commit com {}...", cfg.provider);
        let msg = provider.generate(&diff, &cli.language).await?;

        if cli.print_only {
            println!("\n--- Sugestão de Commit Message ---\n");
            println!("{}", msg);
            println!("\n----------------------------------");
            return Ok(());
        }

        println!("\nSugestão: \x1b[1;32m{}\x1b[0m\n", msg);

        let confirm_label = if cli.push {
            "Confirmar (Commit & Push)"
        } else {
            "Confirmar (Commit)"
        };
        let options = vec![confirm_label, "Gerar Novamente", "Cancelar"];

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

    println!("Commit realizado com sucesso.");

    if !cli.push {
        println!("Push não executado. Use -y/--push para enviar automaticamente após o commit.");
        return Ok(());
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
