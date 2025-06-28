use std::fmt;

pub type Result<T> = std::result::Result<T, GitLineageError>;

#[derive(Debug)]
pub enum GitLineageError {
    Git(gix::open::Error),
    Io(std::io::Error),
    Generic(String),
}

impl fmt::Display for GitLineageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitLineageError::Git(e) => write!(f, "Git error: {}", e),
            GitLineageError::Io(e) => write!(f, "IO error: {}", e),
            GitLineageError::Generic(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for GitLineageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GitLineageError::Git(e) => Some(e),
            GitLineageError::Io(e) => Some(e),
            GitLineageError::Generic(_) => None,
        }
    }
}

impl From<gix::open::Error> for GitLineageError {
    fn from(error: gix::open::Error) -> Self {
        GitLineageError::Git(error)
    }
}

impl From<std::io::Error> for GitLineageError {
    fn from(error: std::io::Error) -> Self {
        GitLineageError::Io(error)
    }
}

impl From<String> for GitLineageError {
    fn from(error: String) -> Self {
        GitLineageError::Generic(error)
    }
}

impl From<&str> for GitLineageError {
    fn from(error: &str) -> Self {
        GitLineageError::Generic(error.to_string())
    }
}

impl From<Box<dyn std::error::Error>> for GitLineageError {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        GitLineageError::Generic(error.to_string())
    }
}

impl From<serde_json::Error> for GitLineageError {
    fn from(error: serde_json::Error) -> Self {
        GitLineageError::Generic(error.to_string())
    }
}