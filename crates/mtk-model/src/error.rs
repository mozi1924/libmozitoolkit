use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ModelError {
    #[error("Invalid BlockState syntax: '{0}'")]
    InvalidBlockStateSyntax(String),

    #[error("JSON deserialization error: {0}")]
    JsonError(String),

    #[error("Model parent hierarchy cycle detected: '{0}'")]
    CircularParentHierarchy(String),

    #[error("Texture variable unresolved: '#{0}'")]
    UnresolvedTextureVariable(String),

    #[error("Missing expected model element face for direction: '{0}'")]
    MissingFace(String),

    #[error("ThreadPool error: {0}")]
    ThreadPoolError(String),
}
