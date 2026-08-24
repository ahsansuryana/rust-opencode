//! Ported dari packages/opencode/src/provider/provider.ts:1116-1168
//! (error classes) — pesan `message` getter direplikasi persis.

/// Ported from: provider.ts:1116-1131 (ModelNotFoundError)
#[derive(Debug, Clone)]
pub struct ModelNotFoundError {
    pub provider_id: String,
    pub model_id: String,
    pub suggestions: Option<Vec<String>>,
}

impl std::fmt::Display for ModelNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let suggestions = match &self.suggestions {
            Some(list) if !list.is_empty() => format!(" Did you mean: {}?", list.join(", ")),
            _ => String::new(),
        };
        write!(
            f,
            "Model not found: {}/{}.{suggestions}",
            self.provider_id, self.model_id
        )
    }
}

/// Ported from: provider.ts:1132-1144 (InitError)
#[derive(Debug, Clone)]
pub struct InitError {
    pub provider_id: String,
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to initialize provider: {}", self.provider_id)
    }
}

/// Ported from: provider.ts:1145-1154 (NoProvidersError)
#[derive(Debug)]
pub struct NoProvidersError;

impl std::fmt::Display for NoProvidersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No providers are available")
    }
}

/// Ported from: provider.ts:1155-1166 (NoModelsError)
#[derive(Debug, Clone)]
pub struct NoModelsError {
    pub provider_id: String,
}

impl std::fmt::Display for NoModelsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "No models are available for provider: {}",
            self.provider_id
        )
    }
}

/// Ported from: provider.ts:1168 (type Error)
#[derive(Debug)]
pub enum Error {
    ModelNotFound(ModelNotFoundError),
    Init(InitError),
    NoProviders(NoProvidersError),
    NoModels(NoModelsError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ModelNotFound(e) => write!(f, "{e}"),
            Error::Init(e) => write!(f, "{e}"),
            Error::NoProviders(e) => write!(f, "{e}"),
            Error::NoModels(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ModelNotFoundError> for Error {
    fn from(value: ModelNotFoundError) -> Self {
        Error::ModelNotFound(value)
    }
}
