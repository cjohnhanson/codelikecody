use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub struct ApiError(tisket::Error);

impl From<tisket::Error> for ApiError {
    fn from(err: tisket::Error) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            tisket::Error::IssueNotFound(_) | tisket::Error::ProjectNotFound(_) => {
                (StatusCode::NOT_FOUND, self.0.to_string())
            }
            tisket::Error::IssueClosed(_)
            | tisket::Error::IssueAlreadyClosed(_)
            | tisket::Error::IssueNotClosed(_)
            | tisket::Error::IssueAlreadyExists(_)
            | tisket::Error::ProjectAlreadyExists(_) => {
                (StatusCode::CONFLICT, self.0.to_string())
            }
            tisket::Error::InvalidStatus { .. } | tisket::Error::AmbiguousPrefix(_) => {
                (StatusCode::BAD_REQUEST, self.0.to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()),
        };

        let body = serde_json::json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}
