//! `/up`: the site itself answering, `ok`. For the app's start page
//! (src-tauri/offline/index.html), which opens the site only once this
//! answers: an error page from Cloudflare or Caddy in front of a stopped app
//! is not `ok`, so the start page says the site is not reachable instead of
//! showing it.
//!
//! Loco's `/_ping` says the same, but the start page is not the site's
//! origin and may read an answer only if it allows any origin to. This one
//! does (`Access-Control-Allow-Origin: *`): two letters, no data, nothing a
//! credential would unlock. The site's `Cross-Origin-Resource-Policy:
//! same-origin` does not stand in the way: it governs loads without CORS,
//! and this one is read with it. Never cached, so every check asks the app.

use axum::http::header::{ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL};
use loco_rs::prelude::*;

/// The address the start page checks.
pub const PATH: &str = "/up";

/// What `/up` answers.
pub const BODY: &str = "ok";

#[debug_handler]
async fn up() -> Result<Response> {
    format::render()
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(CACHE_CONTROL, "no-store")
        .text(BODY)
}

pub fn routes() -> Routes {
    Routes::new().add(PATH, get(up))
}
