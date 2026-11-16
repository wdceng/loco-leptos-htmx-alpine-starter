//! The not-found page, rendered by the router's fallback
//! (controllers/not_found.rs) with status 404. The only 404 page there is:
//! no static `404.html` sits in `public/`.

use leptos::prelude::*;

use super::{layout::Notice, pages};

/// Rendered inside the shell's <main>, like every page.
#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! { <Notice heading=pages::NOT_FOUND lead="The page you asked for does not exist or has moved." /> }
}
