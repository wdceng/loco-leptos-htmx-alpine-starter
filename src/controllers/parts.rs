//! `/e2e/parts`: every part of `views/parts.rs` on one page, for the
//! browser tests alone. Mounted in the `e2e` environment only (app.rs):
//! the template has no pages of its own, and the parts must still pass the
//! browser's checks (`e2e/tests/health.rs`: tap targets, no sideways
//! scroll, the accessibility audit). Beside it `/e2e/slow`, an answer held
//! back, for a request still on its way when the page is left
//! (`e2e/tests/moves.rs`).

use std::time::Duration;

use axum::{extract::Request, http::StatusCode};
// Named import on purpose: `leptos::prelude::*` also exports an `Error`
// type that would shadow Loco's.
use leptos::view;
use loco_rs::{environment::Environment, prelude::*};

use crate::{
    render::render_private_page,
    settings,
    views::{layout::PageMeta, parts::PartsPage},
};

/// The page's address.
pub const PATH: &str = "/e2e/parts";

/// Whether `env` is the browser tests' environment, the one this page
/// is mounted in.
#[must_use]
pub fn mounted_in(env: &Environment) -> bool {
    matches!(env, Environment::Any(name) if name == settings::E2E)
}

#[debug_handler]
async fn page(State(ctx): State<AppContext>, req: Request) -> Result<Response> {
    let meta = PageMeta {
        lang: "en",
        title: "Parts".into(),
        description: "The template's parts, for the browser tests.".into(),
        robots: Some("noindex"),
        canonical: None,
    };
    render_private_page(&ctx, req, meta, StatusCode::OK, || {
        view! { <PartsPage /> }
    })
    .await
}

/// The held-back answer's address.
pub const SLOW_PATH: &str = "/e2e/slow";

/// How long the held-back answer waits.
const HELD: Duration = Duration::from_secs(3);

#[debug_handler]
async fn slow() -> Result<Response> {
    tokio::time::sleep(HELD).await;
    format::text("slow")
}

pub fn routes() -> Routes {
    Routes::new().add(PATH, get(page)).add(SLOW_PATH, get(slow))
}
