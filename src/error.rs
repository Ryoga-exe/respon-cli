use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("code must be exactly 9 ASCII digits")]
    InvalidCode,
}

impl Error {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::InvalidCode => 2,
        }
    }
}
