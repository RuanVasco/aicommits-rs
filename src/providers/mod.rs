pub mod gemini;

use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait CommitProvider {
    async fn generate(&self, diff: &str, language: &str) -> Result<String>;
}

pub(crate) fn build_prompt(diff: &str, language: &str) -> String {
    format!(
        "Act as a commit message generator. \
        Analyze the git diff below and generate a SINGLE, complete line of commit message following the Conventional Commits specification (e.g., feat, fix, chore, docs). \
        The message must be concise, objective, and in {language}. \
        Do not truncate the sentence. Do not use quotes or markdown code blocks.\n\nDiff:\n{diff}"
    )
}

pub fn build(cfg: &crate::config::ProviderConfig) -> Result<Box<dyn CommitProvider>> {
    use crate::config::ProviderConfig::*;
    match cfg {
        Gemini { api_key, model } => Ok(Box::new(gemini::GeminiProvider::new(
            api_key.clone(),
            model.clone(),
        ))),
    }
}
