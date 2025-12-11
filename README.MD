# aicommits-rs

![Status](https://img.shields.io/badge/Status-Active-success)
![Language](https://img.shields.io/badge/Rust-2024-orange)
![License](https://img.shields.io/badge/License-GPLv3-blue)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-yellow)

**aicommits-rs** (executable: `aic`) is a CLI tool written in Rust that automates your git workflow. It uses **Google Gemini AI** to analyze your staged changes (`git diff`) and generates concise, semantic commit messages following the **Conventional Commits** specification.

It handles the generation, commit, and push process for you, saving time and ensuring a standardized history.

## Installation

Go to the [Releases Page](https://github.com/RuanVasco/aicommits-rs/releases) and download the version for your system:

- **Windows:** Download `aic.exe`. Add it to your PATH or drop it in a folder like `C:\Windows`.
- **Linux (Debian/Ubuntu):** Download the `.deb` file and install:
  ```bash
  sudo dpkg -i aic_0.1.0_amd64.deb
  ```
- **Linux (Arch):** Users on Arch Linux, Manjaro, or other derivatives can install the latest version via an AUR helper (like `yay` or `paru`):
  ```bash
  yay -S aicommits-rs
  ```

## Usage

### 1. First Time Setup

Before using the tool, you need a Google Gemini API Key. Get one for free at Google AI Studio.
Run the setup command:

```bash
aic setup
```

_The tool will guide you through entering your API Key and selecting the preferred AI Model from a dynamic list._

### 2. Generating a Commit (Standard Flow)

The standard flow automates the entire process: Analyze -> Generate -> Commit -> Push.

- Stage your files normally (especially new files):
  ```bash
  git add .
  ```
- Run aic. The tool will present a generated message and ask for confirmation before proceeding.
  ```bash
  aic
  ```
- You will see an interactive prompt:
  ```bash
    Sugestão: feat: introduce AI-powered commit generation loop
    O que deseja fazer?
        > Confirmar (Commit & Push)
        Gerar Novamente
        Cancelar
  ```
  _If you select Confirm, the tool commits the message and runs git push._

### 3. Using Flags

| Flag           | Short | Type     | Description                                                                                                       |
| :------------- | :---- | :------- | :---------------------------------------------------------------------------------------------------------------- |
| `--all`        | `-a`  | `bool`   | Automatically runs `git add .` before analyzing changes. Useful for tracking **new files**.                       |
| `--print-only` | `-p`  | `bool`   | **Dry Run mode.** Generates the message and prints it, but **does not commit or push**.                           |
| `--language`   | `-l`  | `String` | Specifies the language for the commit message (e.g., `Portuguese`, `Spanish`, `Japanese`). Defaults to `English`. |

### 4. Configuration Reference

The configuration file is saved in the following locations:

    Linux: ~/.config/aicommits-rs/config.toml

    Windows: %APPDATA%\aicommits\aicommits-rs\config\config.toml

You can manually edit this file to adjust settings or run aic setup to change them interactively.
