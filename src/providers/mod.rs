pub mod embedded;
pub mod gemini;
pub mod openai_compat;

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
        OpenAiCompatible {
            base_url,
            api_key,
            model,
        } => Ok(Box::new(openai_compat::OpenAiCompatProvider::new(
            base_url.clone(),
            api_key.clone(),
            model.clone(),
        ))),
        Embedded => Ok(Box::new(embedded::EmbeddedProvider::new())),
    }
}

#[cfg(test)]
mod tests {
    use super::build_prompt;

    #[test]
    fn build_prompt_includes_diff_and_language() {
        let prompt = build_prompt("diff --git a/foo b/foo", "Portuguese");

        assert!(prompt.contains("diff --git a/foo b/foo"));
        assert!(prompt.contains("Portuguese"));
        assert!(prompt.contains("Conventional Commits"));
    }
}
