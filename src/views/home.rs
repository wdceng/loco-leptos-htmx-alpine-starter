//! Home page. Server-rendered: a plain `#[component]`, not an island, so
//! none of it reaches the browser bundle but the `IslandsCheck` island.

use leptos::prelude::*;

use crate::{
    islands::IslandsCheck,
    views::layout::{APP_NAME, LEAD, TITLE},
};

/// Landing page content, rendered inside the shell's <main>.
#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <h1 class=TITLE>{APP_NAME}</h1>
        <p class=LEAD>"TODO: what the app does for its visitor, in a line."</p>
        <IslandsCheck/>
    }
}
