//! Home page. Loco owns the URL; Leptos renders the document.

use axum::extract::Request;
// Named import on purpose: `leptos::prelude::*` also exports an `Error`
// type that would shadow Loco's.
use leptos::view;
use loco_rs::prelude::*;

use crate::{
    render::render_page,
    views::{home::HomePage, layout::PageMeta},
};

/// The site in one sentence: the home page's meta description, the link
/// preview's, the manifest's, and the summary line of `/llms.txt`
/// (controllers/llms.rs). A `TODO` left in it refuses production's boot
/// (src/deploy_checks.rs).
pub const DESCRIPTION: &str = "TODO: what the site is, in one sentence.";

#[debug_handler]
async fn index(State(ctx): State<AppContext>, req: Request) -> Result<Response> {
    let meta = PageMeta {
        lang: "en",
        // The app's name alone (`full_title`).
        title: String::new(),
        description: DESCRIPTION.into(),
        robots: None,
        canonical: None,
    };
    render_page(&ctx, req, meta, || view! { <HomePage/> }).await
}

pub fn routes() -> Routes {
    Routes::new().add("/", get(index))
}
