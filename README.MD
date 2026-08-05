# aicommits-rs

![Status](https://img.shields.io/badge/Status-Active-success)
![Language](https://img.shields.io/badge/Rust-2024-orange)
![License](https://img.shields.io/badge/License-GPLv3-blue)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-yellow)

**aicommits-rs** (executable: `aic`) is a CLI tool written in Rust that automates your git workflow. It analyzes your staged changes (`git diff`) and generates concise, semantic commit messages following the **Conventional Commits** specification, using whichever AI provider you configure: **Google Gemini**, any **OpenAI-compatible endpoint** (Ollama, LM Studio, llama.cpp server, OpenAI itself), or a small **embedded local model** that needs no API key at all.

It handles the generation, commit, and push process for you, saving time and ensuring a standardized history.

## Installation

Go to the [Releases Page](https://github.com/RuanVasco/aicommits-rs/releases) and download the version for your system:

- **Windows:** Download `aic.exe` **and** `libstdc++-6.dll` and keep them in the same folder (the DLL is required by the embedded local model's tokenizer). Add that folder to your PATH.
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

Run the setup command and pick a provider:

```bash
aic setup
```

- **Google Gemini** — needs a free API key from [Google AI Studio](https://aistudio.google.com/app/apikey). The wizard lists available models for you to pick from.
- **OpenAI-compatible endpoint** — point it at any server that speaks the OpenAI chat-completions API: a local [Ollama](https://ollama.com/) (`http://localhost:11434/v1`), [LM Studio](https://lmstudio.ai/) (`http://localhost:1234/v1`), a `llama.cpp` server, or OpenAI itself. The API key is optional (most local servers don't need one). The wizard tries to list models from the endpoint automatically; if the server doesn't support that, you can type the model name manually.
- **Embedded local model** — no API key, no server to run. The first time you generate a commit message, `aic` downloads a small quantized model (TinyLlama 1.1B, ~700MB) and caches it locally, then runs inference entirely on your machine (CPU). Slower and lower quality than the cloud options, but fully offline and "plug and play" after the first download.

### 2. Generating a Commit (Standard Flow)

The standard flow automates the process: Analyze -> Generate -> Commit. Pushing is **opt-in** (see the `-y`/`--push` flag below) so you can batch several commits before sending them.

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
        > Confirmar (Commit)
        Gerar Novamente
        Cancelar
  ```
  _If you select Confirm, the tool commits the message. Add `-y`/`--push` to also push once the commit succeeds (the prompt then reads "Confirmar (Commit & Push)")._

### 3. Using Flags

| Flag           | Short | Type     | Description                                                                                                       |
| :------------- | :---- | :------- | :---------------------------------------------------------------------------------------------------------------- |
| `--all`        | `-a`  | `bool`   | Automatically runs `git add .` before analyzing changes. Useful for tracking **new files**.                       |
| `--print-only` | `-p`  | `bool`   | **Dry Run mode.** Generates the message and prints it, but **does not commit or push**.                           |
| `--language`   | `-l`  | `String` | Specifies the language for the commit message (e.g., `Portuguese`, `Spanish`, `Japanese`). Defaults to `English`. |
| `--push`       | `-y`  | `bool`   | Runs `git push` automatically after a successful commit. Without it, the commit is left unpushed.                 |

Flags can be combined, e.g. `aic -ay` stages everything and pushes after commit.

### 4. Configuration Reference

The configuration file is saved in the following locations:

    Linux: ~/.config/aicommits-rs/config.toml

    Windows: %APPDATA%\aicommits\aicommits-rs\config\config.toml

You can manually edit this file to adjust settings or run aic setup to change them interactively.

Since v0.2.0 the config selects a provider explicitly. The shape depends on which one you picked in `aic setup`:

```toml
# Google Gemini
provider = "gemini"
api_key = "..."
model = "gemini-2.0-flash"
```

```toml
# OpenAI-compatible endpoint (Ollama, LM Studio, llama.cpp server, OpenAI...)
provider = "openai"
base_url = "http://localhost:11434/v1"
model = "llama3.1"
# api_key = "..." # optional, omit for servers that don't require auth
```

```toml
# Embedded local model
provider = "embedded"
```

Config files written by older versions (no `provider` key) are detected and migrated to the Gemini format automatically the first time you run `aic`.
