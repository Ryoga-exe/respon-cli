use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("authentication failed: {0}")]
    Authentication(String),
}

impl Error {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Authentication(_) => 2,
        }
    }
}
