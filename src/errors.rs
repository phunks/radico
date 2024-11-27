use thiserror::Error;
#[derive(Error, Debug)]
pub enum RadicoError {
    #[error("fetch AAC error")]
    RequestError(#[from] reqwest::Error),
    #[error("Operation was interrupted by the user")]
    OperationInterrupted,
    #[error("Station error")]
    StationError,
    #[error("Client error")]
    ClientError,
    #[error("Playlist error")]
    PlaylistError,
    #[error("Inquire Error")]
    InquireError,
    #[error("Auth Error")]
    AuthError,
    #[error("Forbidden")]
    Forbidden,
    #[error("Local time is negative {} ms", .0)]
    NegativeTime(i64),
    #[error("Quit")]
    Quit,
    #[error("Cancel")]
    Cancel,
}
