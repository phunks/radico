use thiserror::Error;

#[derive(Error, Debug)]
pub enum RadicoError {
    #[error("fetch AAC error")]
    RequestError(#[from] reqwest::Error),
    #[error("Operation was interrupted by the user")]
    OperationInterrupted,
    #[error("Station error")]
    StationError,
    #[error("Playlist error")]
    PlaylistError,
    #[error("Inquire Error")]
    InquireError,
    #[error("Auth Error")]
    AuthError,
    #[error("Forbidden")]
    Forbidden,
    #[error("Quit")]
    Quit,
}
