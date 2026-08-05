use super::{CommitProvider, build_prompt};
use anyhow::{Context, Result};
use async_trait::async_trait;
use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_llama::ModelWeights;
use hf_hub::HFClientSync;
use std::path::PathBuf;
use tokenizers::Tokenizer;

const MODEL_REPO_OWNER: &str = "TheBloke";
const MODEL_REPO_NAME: &str = "TinyLlama-1.1B-Chat-v1.0-GGUF";
const MODEL_FILE: &str = "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf";

const TOKENIZER_REPO_OWNER: &str = "TinyLlama";
const TOKENIZER_REPO_NAME: &str = "TinyLlama-1.1B-Chat-v1.0";
const TOKENIZER_FILE: &str = "tokenizer.json";

const MAX_NEW_TOKENS: usize = 120;
const SAMPLING_SEED: u64 = 299792458;

pub struct EmbeddedProvider;

impl EmbeddedProvider {
    pub fn new() -> Self {
        Self
    }
}

fn download_files() -> Result<(PathBuf, PathBuf)> {
    let client = HFClientSync::new().context("Falha ao inicializar cliente do Hugging Face Hub")?;

    let model_repo = client.model(MODEL_REPO_OWNER, MODEL_REPO_NAME);
    let model_path = model_repo
        .download_file()
        .filename(MODEL_FILE)
        .send()
        .context("Falha ao baixar o modelo local (.gguf)")?;

    let tokenizer_repo = client.model(TOKENIZER_REPO_OWNER, TOKENIZER_REPO_NAME);
    let tokenizer_path = tokenizer_repo
        .download_file()
        .filename(TOKENIZER_FILE)
        .send()
        .context("Falha ao baixar o tokenizer do modelo local")?;

    Ok((model_path, tokenizer_path))
}

fn generate_blocking(diff: &str, language: &str) -> Result<String> {
    println!("Baixando/verificando modelo local (pode levar alguns minutos na primeira vez)...");
    let (model_path, tokenizer_path) = download_files()?;

    println!("Carregando modelo em memória...");
    let device = Device::Cpu;

    let mut file = std::fs::File::open(&model_path)
        .with_context(|| format!("Não foi possível abrir {}", model_path.display()))?;
    let content = gguf_file::Content::read(&mut file)?;
    let mut model = ModelWeights::from_gguf(content, &mut file, &device)?;

    let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(anyhow::Error::msg)?;

    let prompt_text = build_prompt(diff, language);
    let prompt = format!(
        "<|system|>\nYou are a helpful assistant that writes git commit messages.</s>\n<|user|>\n{prompt_text}</s>\n<|assistant|>\n"
    );

    let encoding = tokenizer
        .encode(prompt, true)
        .map_err(anyhow::Error::msg)?;
    let mut all_tokens: Vec<u32> = encoding.get_ids().to_vec();

    let eos_token = tokenizer.token_to_id("</s>").unwrap_or(2);

    let mut logits_processor = LogitsProcessor::from_sampling(
        SAMPLING_SEED,
        Sampling::TopP {
            p: 0.9,
            temperature: 0.2,
        },
    );

    let input = Tensor::new(all_tokens.as_slice(), &device)?.unsqueeze(0)?;
    let logits = model.forward(&input, 0)?;
    let logits = logits.squeeze(0)?;
    let mut next_token = logits_processor.sample(&logits)?;

    let mut generated_tokens = Vec::new();

    for _ in 0..MAX_NEW_TOKENS {
        if next_token == eos_token {
            break;
        }

        generated_tokens.push(next_token);
        let index_pos = all_tokens.len();
        all_tokens.push(next_token);

        let input = Tensor::new(&[next_token], &device)?.unsqueeze(0)?;
        let logits = model.forward(&input, index_pos)?;
        let logits = logits.squeeze(0)?;
        next_token = logits_processor.sample(&logits)?;
    }

    let text = tokenizer
        .decode(&generated_tokens, true)
        .map_err(anyhow::Error::msg)?;

    let first_line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();

    Ok(first_line.to_string())
}

#[async_trait]
impl CommitProvider for EmbeddedProvider {
    async fn generate(&self, diff: &str, language: &str) -> Result<String> {
        let diff = diff.to_string();
        let language = language.to_string();

        tokio::task::spawn_blocking(move || generate_blocking(&diff, &language))
            .await
            .context("Falha ao rodar inferência local")?
    }
}
