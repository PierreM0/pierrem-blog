use axum::{response::IntoResponse, http::StatusCode};
use tracing::info;

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    NotFound,
    IoError,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::NotFound => {
                info!("Responding NotFound.");
                (StatusCode::NOT_FOUND, "Not found").into_response()
            }
            Error::IoError => {
                info!("Responding InternalServerError due to IoError.");
                (StatusCode::INTERNAL_SERVER_ERROR, "InternalServerError").into_response()
            }
        }
    }
}
