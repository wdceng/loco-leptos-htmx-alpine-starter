//! The request log: Loco's `logger`, one span per request with its method,
//! user agent, environment and request id, naming the request by its
//! route's pattern instead of its address.
//!
//! A token travels in the address: the reset and verification links' query
//! (`/reset?token=…`, `/verify?token=…`), the magic link's path. Loco's own
//! logger writes the whole URI into the journal, where a reset token still
//! works for half an hour. Here a matched route is logged as its pattern
//! (`/api/auth/verify/{token}`), anything else (a file, a miss) by its path
//! alone, never a query. It takes Loco's `logger` place in the stack under
//! the same name, switched by the same `logger.enable` (`Hooks::middlewares`,
//! src/app.rs).

use axum::{
    Router,
    extract::MatchedPath,
    http::{Request, header::USER_AGENT},
};
use loco_rs::{
    Result,
    app::AppContext,
    controller::middleware::{MiddlewareLayer, request_id::LocoRequestId},
    environment::Environment,
};
use serde::Serialize;
use tower_http::trace::TraceLayer;

/// What the log names `request` by: its route's pattern where a route
/// matched, its path where none did; never its query. axum sets the
/// pattern before a route's layers run, so this layer sees it.
#[must_use]
pub fn logged_path<B>(request: &Request<B>) -> &str {
    request
        .extensions()
        .get::<MatchedPath>()
        .map_or_else(|| request.uri().path(), MatchedPath::as_str)
}

/// The layer, see the module doc.
#[derive(Debug, Serialize)]
pub struct RequestLog {
    enable: bool,
    environment: Environment,
}

impl RequestLog {
    #[must_use]
    pub fn from_context(ctx: &AppContext) -> Self {
        Self {
            // Loco's default: on unless the config switches it off.
            enable: ctx
                .config
                .server
                .middlewares
                .logger
                .as_ref()
                .is_none_or(|logger| logger.enable),
            environment: ctx.environment.clone(),
        }
    }
}

impl MiddlewareLayer for RequestLog {
    fn name(&self) -> &'static str {
        "logger"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Loco's span, field for field, with `http.path` from [`logged_path`]
    /// in place of `http.uri`.
    fn apply(&self, app: Router<AppContext>) -> Result<Router<AppContext>> {
        let environment = self.environment.to_string();
        Ok(app.layer(
            TraceLayer::new_for_http().make_span_with(move |request: &Request<_>| {
                let request_id = request
                    .extensions()
                    .get::<LocoRequestId>()
                    .map_or("req-id-none", LocoRequestId::get);
                let user_agent = request
                    .headers()
                    .get(USER_AGENT)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("");
                tracing::error_span!(
                    "http-request",
                    "http.method" = tracing::field::display(request.method()),
                    "http.path" = tracing::field::display(logged_path(request)),
                    "http.version" = tracing::field::debug(request.version()),
                    "http.user_agent" = tracing::field::display(user_agent),
                    "environment" = tracing::field::display(&environment),
                    request_id = tracing::field::display(request_id),
                )
            }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        extract::Request,
        http::HeaderValue,
        middleware::{self, Next},
        response::Response,
        routing::get,
    };
    use loco_rs::TestServer;

    use super::*;

    /// Sends back what the log would name the request by.
    async fn echo(request: Request, next: Next) -> Response {
        let logged = logged_path(&request).to_owned();
        let mut response = next.run(request).await;
        if let Ok(value) = HeaderValue::from_str(&logged) {
            response.headers_mut().insert("x-logged", value);
        }
        response
    }

    #[tokio::test]
    async fn a_request_is_logged_by_its_route_never_its_token() {
        let app = Router::new()
            .route("/api/auth/verify/{token}", get(|| async {}))
            .route("/reset", get(|| async {}))
            .fallback(|| async {})
            .layer(middleware::from_fn(echo));
        let server = TestServer::new(app).expect("a test server");
        for (address, logged) in [
            ("/api/auth/verify/s3cret", "/api/auth/verify/{token}"),
            ("/reset?token=s3cret", "/reset"),
            ("/pkg/app.css?v=s3cret", "/pkg/app.css"),
        ] {
            let response = server.get(address).await;
            assert_eq!(response.header("x-logged"), logged, "{address}");
        }
    }
}
