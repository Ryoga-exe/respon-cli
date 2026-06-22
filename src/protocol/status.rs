use url::Url;

use crate::protocol::page::CodeRejection;

pub enum AttendanceAccess {
    AuthenticationRequired { card_id: String, login_url: Url },
    ConfirmationAvailable { card_id: String, page_url: Url },
}

impl AttendanceAccess {
    pub fn card_id(&self) -> &str {
        match self {
            Self::AuthenticationRequired { card_id, .. }
            | Self::ConfirmationAvailable { card_id, .. } => card_id,
        }
    }
}

pub enum ProbeStatus {
    Available(AttendanceAccess),
    Unavailable(CodeRejection),
}
