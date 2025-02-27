use crate::agent::AgentPurpose;

#[derive(Debug, Clone)]
pub enum AgentProvider {
    Anthropic,
    Groq,
    LocalAI,
    Mistral,
    Ollama,
    OpenAI,
    OpenAICompat,
    OpenRouter,
    TogetherAI,
}

impl AgentProvider {
    pub fn choices() -> Vec<&'static Self> {
        vec![
            &Self::Anthropic,
            &Self::Groq,
            &Self::LocalAI,
            &Self::Mistral,
            &Self::Ollama,
            &Self::OpenAI,
            &Self::OpenAICompat,
            &Self::OpenRouter,
            &Self::TogetherAI,
        ]
    }

    pub fn to_static_str(&self) -> &'static str {
        match &self {
            Self::Anthropic => "anthropic",
            Self::Groq => "groq",
            Self::LocalAI => "localai",
            Self::Mistral => "mistral",
            Self::Ollama => "ollama",
            Self::OpenAI => "openai",
            Self::OpenAICompat => "openai-compatible",
            Self::OpenRouter => "openrouter",
            Self::TogetherAI => "together-ai",
        }
    }

    pub fn from_string(s: &str) -> Result<Self, &'static str> {
        match s {
            "anthropic" => Ok(Self::Anthropic),
            "groq" => Ok(Self::Groq),
            "localai" => Ok(Self::LocalAI),
            "mistral" => Ok(Self::Mistral),
            "ollama" => Ok(Self::Ollama),
            "openai" => Ok(Self::OpenAI),
            "openai-compatible" => Ok(Self::OpenAICompat),
            "openrouter" => Ok(Self::OpenRouter),
            "together-ai" => Ok(Self::TogetherAI),
            _ => Err("Unexpected string value"),
        }
    }

    pub fn info(&self) -> AgentProviderInfo {
        match &self {
            Self::Anthropic => AgentProviderInfo {
                id: Self::Anthropic.to_static_str(),
                name: "Anthropic",
                description: "Anthropic is an American AI company founded by former OpenAI engineers and providing powerful language models.",
                homepage_url: Some("https://www.anthropic.com/"),
                wiki_url: Some("https://en.wikipedia.org/wiki/Anthropic"),
                sign_up_url: Some("https://console.anthropic.com/"),
                models_list_url: Some("https://docs.anthropic.com/en/docs/about-claude/models"),
                supported_purposes: vec![AgentPurpose::TextGeneration],
            },
            Self::Groq => AgentProviderInfo {
                id: Self::Groq.to_static_str(),
                name: "Groq",
                description: "Groq is an American company developing optimized Language Processing Units (LPU) and offering cloud service which runs various models (built by others) with very high performance.",
                homepage_url: Some("https://groq.com/"),
                wiki_url: Some("https://en.wikipedia.org/wiki/Groq"),
                sign_up_url: Some("https://console.groq.com/login"),
                models_list_url: Some("https://console.groq.com/docs/models"),
                supported_purposes: vec![AgentPurpose::TextGeneration, AgentPurpose::SpeechToText],
            },
            Self::LocalAI => AgentProviderInfo {
                id: Self::LocalAI.to_static_str(),
                name: "LocalAI",
                description: "LocalAI is the free, Open Source OpenAI alternative. LocalAI act as a drop-in replacement REST API that’s compatible with OpenAI API specifications for local inferencing. It allows you to run LLMs, generate images, audio (and not only) locally or on-prem with consumer grade hardware, supporting multiple model families and architectures.",
                homepage_url: Some("https://localai.io/"),
                wiki_url: None,
                sign_up_url: None,
                models_list_url: Some("https://localai.io/gallery.html"),
                supported_purposes: vec![
                    AgentPurpose::TextGeneration,
                    AgentPurpose::TextToSpeech,
                    AgentPurpose::SpeechToText,
                ],
            },
            Self::Mistral => AgentProviderInfo {
                id: Self::Mistral.to_static_str(),
                name: "Mistral",
                description: "Mistral AI is a research lab based in Europe (France) which produces their own language models.",
                homepage_url: Some("https://mistral.ai/"),
                wiki_url: Some("https://en.wikipedia.org/wiki/Mistral_AI"),
                sign_up_url: Some("https://auth.mistral.ai/ui/registration"),
                models_list_url: Some("https://docs.mistral.ai/getting-started/models/"),
                supported_purposes: vec![AgentPurpose::TextGeneration],
            },
            Self::Ollama => AgentProviderInfo {
                id: Self::Ollama.to_static_str(),
                name: "Ollama",
                description: "Ollama lets you run various models in a [self-hosted](https://github.com/ollama/ollama?tab=readme-ov-file#ollama) way. This is more advanced and requires powerful hardware for running some of the better models, but ensures your data stays with you.",
                homepage_url: Some("https://ollama.com/"),
                wiki_url: None,
                sign_up_url: None,
                models_list_url: Some("https://ollama.com/library"),
                supported_purposes: vec![AgentPurpose::TextGeneration],
            },
            Self::OpenAI => AgentProviderInfo {
                id: Self::OpenAI.to_static_str(),
                name: "OpenAI",
                description: "OpenAI is an American AI company providing powerful language models.\n\nUse this provider either with the OpenAI API or with other OpenAI-compatible API services which **fully** adhere to the [OpenAI API spec](https://github.com/openai/openai-openapi/).\nFor services which are not fully compatible with the OpenAI API, consider using the **OpenAI Compatible** provider.",
                homepage_url: Some("https://openai.com/"),
                wiki_url: Some("https://en.wikipedia.org/wiki/OpenAI"),
                sign_up_url: Some("https://platform.openai.com/signup"),
                models_list_url: Some("https://platform.openai.com/docs/models"),
                supported_purposes: vec![
                    AgentPurpose::ImageGeneration,
                    AgentPurpose::TextGeneration,
                    AgentPurpose::TextToSpeech,
                    AgentPurpose::SpeechToText,
                ],
            },
            Self::OpenAICompat => AgentProviderInfo {
                id: Self::OpenAICompat.to_static_str(),
                name: "OpenAI Compatible",
                description: "This provider allows you to use OpenAI-compatible API services like [OpenRouter](https://openrouter.ai/), [Together AI](https://www.together.ai/), etc.\n\nSome of these popular services already have **shortcut** providers (leading to this one behind the scenes) - this make it easier to get started.\n\nThis provider just as featureful as the **OpenAI** provider, but is more compatible with services which do not fully adhere to the [OpenAI API spec](https://github.com/openai/openai-openapi/).",
                homepage_url: None,
                wiki_url: None,
                sign_up_url: None,
                models_list_url: None,
                supported_purposes: vec![
                    AgentPurpose::ImageGeneration,
                    AgentPurpose::TextGeneration,
                    AgentPurpose::TextToSpeech,
                    AgentPurpose::SpeechToText,
                ],
            },
            Self::OpenRouter => AgentProviderInfo {
                id: Self::OpenRouter.to_static_str(),
                name: "OpenRouter",
                description: "OpenRouter is a unified interface for LLMs. The platform scouts for the lowest prices and best latencies/throughputs across dozens of providers, and lets you choose how to [prioritize](https://openrouter.ai/docs/provider-routing) them.",
                homepage_url: Some("https://openrouter.ai/"),
                wiki_url: None,
                sign_up_url: Some("https://openrouter.ai/"),
                models_list_url: Some("https://openrouter.ai/models"),
                supported_purposes: vec![AgentPurpose::TextGeneration],
            },
            Self::TogetherAI => AgentProviderInfo {
                id: Self::TogetherAI.to_static_str(),
                name: "Together AI",
                description: "Together AI makes it easy to run or [fine-tune](https://docs.together.ai/docs/fine-tuning-overview) leading open source models with only a few lines of code.",
                homepage_url: Some("https://www.together.ai/"),
                wiki_url: None,
                sign_up_url: Some("https://api.together.ai/signup"),
                models_list_url: Some("https://api.together.xyz/models"),
                supported_purposes: vec![AgentPurpose::TextGeneration],
            },
        }
    }
}

impl std::fmt::Display for AgentProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_static_str())
    }
}

pub struct AgentProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub homepage_url: Option<&'static str>,
    pub wiki_url: Option<&'static str>,
    pub sign_up_url: Option<&'static str>,
    pub models_list_url: Option<&'static str>,
    pub supported_purposes: Vec<AgentPurpose>,
}
