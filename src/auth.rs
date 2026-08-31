use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header::AUTHORIZATION, HeaderMap, Request, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{http::StatusCode, Json};
use serde_json::json;
use subtle::ConstantTimeEq;
use url::form_urlencoded;

use crate::AppState;

#[derive(Clone)]
pub struct AuthState {
    expected: Arc<Vec<u8>>,
}

impl AuthState {
    pub fn new(expected: &str) -> Self {
        Self {
            expected: Arc::new(expected.as_bytes().to_vec()),
        }
    }

    fn matches(&self, candidate: &str) -> bool {
        self.expected.as_slice().ct_eq(candidate.as_bytes()).into()
    }
}

enum CandidateToken {
    Missing,
    InvalidHeader,
    Present(String),
}

fn candidate_token(headers: &HeaderMap, uri: &Uri) -> CandidateToken {
    if let Some(value) = headers.get(AUTHORIZATION) {
        return match value.to_str() {
            Ok(value) => CandidateToken::Present(value.replace("Bearer ", "")),
            Err(_) => CandidateToken::InvalidHeader,
        };
    }

    let value = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes())
        .filter(|(key, _)| key == "token")
        .map(|(_, value)| value.into_owned())
        .last();
    value.map_or(CandidateToken::Missing, CandidateToken::Present)
}

pub async fn require_token(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let candidate = candidate_token(request.headers(), request.uri());
    match candidate {
        CandidateToken::Missing => unauthorized("Authentication token required"),
        CandidateToken::InvalidHeader => unauthorized("Invalid authentication token"),
        CandidateToken::Present(token) if !state.auth.matches(&token) => {
            unauthorized("Invalid authentication token")
        }
        CandidateToken::Present(_) => next.run(request).await,
    }
}

fn unauthorized(detail: &str) -> Response {
    (StatusCode::UNAUTHORIZED, Json(json!({ "detail": detail }))).into_response()
}
