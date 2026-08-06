#[macro_use]
extern crate rust_i18n;

i18n!("locales", fallback = "en");

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

fn init_locale() {
    let system_locale = sys_locale::get_locale().unwrap_or_default();
    let locale = if system_locale.to_lowercase().starts_with("pt") {
        "pt-BR"
    } else {
        "en"
    };
    rust_i18n::set_locale(locale);
}

fn get_git_diff() -> Result<String> {
    let output = Command::new("git")
        .arg("diff")
        .arg("--staged")
        .output()
        .context(t!("main.git_not_found").to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(t!("main.git_diff_failed", stderr = stderr).to_string());
    }

    let diff = String::from_utf8(output.stdout).context(t!("main.diff_not_utf8").to_string())?;

    if diff.trim().is_empty() {
        anyhow::bail!(t!("main.no_staged_changes").to_string());
    }

    Ok(diff)
}

#[tokio::main]
async fn main() -> Result<()> {
    init_locale();

    let cli = Cli::parse();

    if let Some(Commands::Setup) = cli.command {
        config::run_setup().await?;
        return Ok(());
    }

    let cfg = config::load_or_setup().await?;

    if cli.all {
        println!("{}", t!("main.adding_all"));
        let add_all_status = Command::new("git")
            .arg("add")
            .arg(".")
            .status()
            .context(t!("main.git_add_context").to_string())?;

        if !add_all_status.success() {
            anyhow::bail!(t!("main.git_add_failed").to_string());
        }
    }

    println!("{}", t!("main.analyzing"));
    let diff = get_git_diff()?;

    let provider = providers::build(&cfg.provider)?;

    let final_msg: String;

    loop {
        println!("{}", t!("main.generating", provider = cfg.provider));
        let msg = provider.generate(&diff, &cli.language).await?;

        if cli.print_only {
            println!("{}", t!("main.suggestion_header"));
            println!("{}", msg);
            println!("{}", t!("main.suggestion_footer"));
            return Ok(());
        }

        println!(
            "\n{}: \x1b[1;32m{}\x1b[0m\n",
            t!("main.suggestion_label"),
            msg
        );

        let confirm_label = if cli.push {
            t!("main.confirm_commit_push")
        } else {
            t!("main.confirm_commit")
        };
        let options = vec![
            confirm_label.to_string(),
            t!("main.regenerate").to_string(),
            t!("main.cancel").to_string(),
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(t!("main.what_to_do").to_string())
            .default(0)
            .items(&options)
            .interact()?;

        match selection {
            0 => {
                final_msg = msg;
                break;
            }
            1 => {
                println!("{}", t!("main.retrying"));
                continue;
            }
            _ => {
                println!("{}", t!("main.cancelled"));
                return Ok(());
            }
        }
    }

    let commit_status = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(&final_msg)
        .status()
        .context(t!("main.git_commit_context").to_string())?;

    if !commit_status.success() {
        anyhow::bail!(t!("main.commit_failed").to_string());
    }

    println!("{}", t!("main.commit_success"));

    if !cli.push {
        println!("{}", t!("main.push_not_executed"));
        return Ok(());
    }

    println!("{}", t!("main.pushing"));

    let push_status = Command::new("git")
        .arg("push")
        .status()
        .context(t!("main.git_push_context").to_string())?;

    if push_status.success() {
        println!("{}", t!("main.push_success"));
        return Ok(());
    }

    anyhow::bail!(t!("main.push_failed").to_string());
}
