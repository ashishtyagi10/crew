#[derive(Debug, Clone, PartialEq)]
pub enum ApiProvider {
    Anthropic,
    OpenAiCompatible,
}

pub struct AiAgent {
    pub(super) api_key: Option<String>,
    pub(super) provider: ApiProvider,
    pub(super) base_url: String,
    pub(super) model: String,
    pub(super) max_tokens: u32,
}

impl AiAgent {
    pub fn new(
        provider: &str,
        base_url: String,
        model: String,
        max_tokens: u32,
        api_key_env: &str,
    ) -> Self {
        let api_key = std::env::var(api_key_env).ok();
        let provider = match provider {
            "anthropic" => ApiProvider::Anthropic,
            _ => ApiProvider::OpenAiCompatible,
        };
        Self {
            api_key,
            provider,
            base_url,
            model,
            max_tokens,
        }
    }

    pub fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    pub fn provider(&self) -> &ApiProvider {
        &self.provider
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    pub fn api_key_env_name(&self) -> &str {
        match self.provider {
            ApiProvider::Anthropic => "ANTHROPIC_API_KEY",
            ApiProvider::OpenAiCompatible => "OPENROUTER_API_KEY",
        }
    }
}
