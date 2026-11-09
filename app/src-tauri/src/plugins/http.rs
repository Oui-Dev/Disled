// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The local server of the plugins with events, on 127.0.0.1:47831 only:
//! `POST /v1/hooks/<id>`, with the token of the plugin in `X-Disled-Token`.
//! It answers at once, 204 with no body, since Claude Code would read a
//! JSON body as a decision, and the host handles the event afterwards. A
//! web page cannot reach it: the server wants its own host name and no
//! `Origin`, which browsers always send, and it has no CORS.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use serde_json::Value;
use subtle::ConstantTimeEq;
use tokio::sync::mpsc;

pub const PORT: u16 = 47831;
const TOKEN_HEADER: &str = "x-disled-token";
/// A `PostToolUse` of Claude Code can carry a large tool output.
const MAX_BODY: usize = 8 * 1024 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(5);
/// Events waiting for the host, for all the plugins.
pub const QUEUE: usize = 256;
/// A taken port is tried again this often.
const RETRY: Duration = Duration::from_secs(30);

/// What the server knows of a plugin with events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hook {
    pub token: String,
    /// A plugin turned off gets its events dropped without an error, so that
    /// Claude Code shows none.
    pub enabled: bool,
}

/// The approved plugins with events, which the host keeps up to date.
pub type Hooks = Arc<RwLock<HashMap<String, Hook>>>;

/// An event for the host: the plugin, and its JSON.
pub type Event = (String, Value);

#[derive(Clone)]
struct Server {
    hooks: Hooks,
    events: mpsc::Sender<Event>,
}

pub fn router(hooks: Hooks, events: mpsc::Sender<Event>) -> Router {
    Router::new()
        .route("/v1/hooks/{id}", post(hook))
        .with_state(Server { hooks, events })
}

/// A short text, never JSON, or no body at all.
fn reply(status: StatusCode, text: &'static str) -> Response {
    if text.is_empty() {
        status.into_response()
    } else {
        (status, text).into_response()
    }
}

/// The checks in the order of annex D, cheapest first.
async fn hook(
    State(server): State<Server>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let host = headers
        .get(header::HOST)
        .and_then(|host| host.to_str().ok());
    let own_host = [format!("127.0.0.1:{PORT}"), format!("localhost:{PORT}")];
    if !host.is_some_and(|host| own_host.iter().any(|own| own == host)) {
        return reply(StatusCode::FORBIDDEN, "wrong host");
    }
    if headers.contains_key(header::ORIGIN) {
        return reply(StatusCode::FORBIDDEN, "no browser requests");
    }
    let hook = server
        .hooks
        .read()
        .ok()
        .and_then(|hooks| hooks.get(&id).cloned());
    let Some(hook) = hook else {
        return reply(StatusCode::NOT_FOUND, "no such plugin");
    };
    let token = headers
        .get(TOKEN_HEADER)
        .map(|token| token.as_bytes())
        .unwrap_or_default();
    if !bool::from(token.ct_eq(hook.token.as_bytes())) {
        return reply(StatusCode::UNAUTHORIZED, "wrong token");
    }
    if !hook.enabled {
        return reply(StatusCode::NO_CONTENT, "");
    }
    let bytes = match tokio::time::timeout(READ_TIMEOUT, axum::body::to_bytes(body, MAX_BODY)).await
    {
        Err(_) => return reply(StatusCode::REQUEST_TIMEOUT, "too slow"),
        Ok(Err(_)) => return reply(StatusCode::PAYLOAD_TOO_LARGE, "too large"),
        Ok(Ok(bytes)) => bytes,
    };
    let Ok(event) = serde_json::from_slice::<Value>(&bytes) else {
        return reply(StatusCode::BAD_REQUEST, "not JSON");
    };
    match server.events.try_send((id, event)) {
        Ok(()) => reply(StatusCode::NO_CONTENT, ""),
        Err(_) => reply(StatusCode::SERVICE_UNAVAILABLE, "busy"),
    }
}

/// Serves until aborted. A port that another program holds, or that
/// Windows reserves, is tried again every 30 s; `taken` tells of it.
pub async fn serve(router: Router, taken: impl Fn(bool)) {
    loop {
        match tokio::net::TcpListener::bind(("127.0.0.1", PORT)).await {
            Ok(listener) => {
                taken(false);
                let _ = axum::serve(listener, router.clone()).await;
            }
            Err(_) => taken(true),
        }
        tokio::time::sleep(RETRY).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;

    const TOKEN: &str = "dl_Q7mX2vK9pR4tW8zNabcdefgh";

    fn server(enabled: bool, queue: usize) -> (Router, mpsc::Receiver<Event>) {
        let hooks: Hooks = Arc::default();
        hooks.write().expect("hooks").insert(
            "claude-code".into(),
            Hook {
                token: TOKEN.into(),
                enabled,
            },
        );
        let (events, received) = mpsc::channel(queue);
        (router(hooks, events), received)
    }

    fn request(method: &str, path: &str) -> axum::http::request::Builder {
        Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, "127.0.0.1:47831")
            .header(TOKEN_HEADER, TOKEN)
            .header(header::CONTENT_TYPE, "application/json")
    }

    async fn send(router: &Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
        let response = router.clone().oneshot(request).await.expect("response");
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        (status, body.to_vec())
    }

    fn hook(body: &str) -> Request<Body> {
        request("POST", "/v1/hooks/claude-code")
            .body(Body::from(body.to_owned()))
            .expect("request")
    }

    #[tokio::test]
    async fn an_event_gets_204_with_no_body_and_reaches_the_host() {
        let (router, mut received) = server(true, QUEUE);
        let (status, body) = send(&router, hook(r#"{"hook_event_name":"Stop"}"#)).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(body.is_empty());
        let (id, event) = received.recv().await.expect("event");
        assert_eq!(id, "claude-code");
        assert_eq!(event["hook_event_name"], "Stop");
    }

    #[tokio::test]
    async fn every_check_has_its_status() {
        let (router, _received) = server(true, QUEUE);
        let mut wrong_token = hook("{}");
        wrong_token
            .headers_mut()
            .insert(TOKEN_HEADER, "dl_wrong".parse().expect("header"));
        assert_eq!(send(&router, wrong_token).await.0, StatusCode::UNAUTHORIZED);

        let mut no_token = hook("{}");
        no_token.headers_mut().remove(TOKEN_HEADER);
        assert_eq!(send(&router, no_token).await.0, StatusCode::UNAUTHORIZED);

        let mut browser = hook("{}");
        browser.headers_mut().insert(
            header::ORIGIN,
            "https://example.com".parse().expect("header"),
        );
        assert_eq!(send(&router, browser).await.0, StatusCode::FORBIDDEN);

        let mut rebinding = hook("{}");
        rebinding
            .headers_mut()
            .insert(header::HOST, "evil.example:47831".parse().expect("header"));
        assert_eq!(send(&router, rebinding).await.0, StatusCode::FORBIDDEN);

        let get = request("GET", "/v1/hooks/claude-code")
            .body(Body::empty())
            .expect("request");
        assert_eq!(send(&router, get).await.0, StatusCode::METHOD_NOT_ALLOWED);

        let unknown = request("POST", "/v1/hooks/pomodoro")
            .body(Body::from("{}"))
            .expect("request");
        assert_eq!(send(&router, unknown).await.0, StatusCode::NOT_FOUND);
        let elsewhere = request("POST", "/v1/other")
            .body(Body::from("{}"))
            .expect("request");
        assert_eq!(send(&router, elsewhere).await.0, StatusCode::NOT_FOUND);

        let (status, body) = send(&router, hook("not json")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!body.starts_with(b"{"), "errors are short texts");
    }

    #[tokio::test]
    async fn a_large_tool_output_passes_and_a_huge_body_does_not() {
        let (router, _received) = server(true, QUEUE);
        let large = format!(r#"{{"tool_response":"{}"}}"#, "a".repeat(3 * 1024 * 1024));
        assert_eq!(send(&router, hook(&large)).await.0, StatusCode::NO_CONTENT);
        let huge = format!(r#"{{"tool_response":"{}"}}"#, "a".repeat(9 * 1024 * 1024));
        assert_eq!(
            send(&router, hook(&huge)).await.0,
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[tokio::test]
    async fn a_plugin_turned_off_drops_its_events_silently() {
        let (router, mut received) = server(false, QUEUE);
        let (status, body) = send(&router, hook("{}")).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(body.is_empty());
        assert!(received.try_recv().is_err());
    }

    #[tokio::test]
    async fn a_full_queue_answers_503() {
        let (router, _received) = server(true, 1);
        assert_eq!(send(&router, hook("{}")).await.0, StatusCode::NO_CONTENT);
        assert_eq!(
            send(&router, hook("{}")).await.0,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
