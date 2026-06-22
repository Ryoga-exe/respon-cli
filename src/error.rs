use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("authentication failed: {0}")]
    Authentication(String),

    #[error("attendance code was rejected: {0}")]
    Rejected(String),

    #[error("respon protocol changed or returned an unsupported page: {0}")]
    Protocol(String),

    #[error("network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),

    #[error("could not read input: {0}")]
    Input(#[from] std::io::Error),

    #[error("interactive prompt failed: {0}")]
    Prompt(#[from] dialoguer::Error),
}

impl Error {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Authentication(_) => 2,
            Self::Rejected(_) => 3,
            Self::Protocol(_) => 4,
            Self::Network(_) | Self::Url(_) => 5,
            Self::Input(_) | Self::Prompt(_) => 1,
        }
    }
}
