#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("{0}")]
    IO(#[from] std::io::Error),

    #[error("{0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Tungstenite(#[from] tungstenite::Error),

    #[error("{0}")]
    ReadlineError(#[from] rustyline::error::ReadlineError),

    #[error("{0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("{0}")]
    Message(String),

    #[error("{0}")]
    MessageRef(&'static str),

    #[error("unknown error")]
    Unknown,
}
