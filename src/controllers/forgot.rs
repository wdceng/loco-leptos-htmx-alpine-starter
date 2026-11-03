//! `/forgot`: the page that asks for a password-reset mail, and its form's
//! post, which sends the link to the reset page (`/reset`). A plain HTML
//! form, so it works without the islands bundle.
//!
//! Every post gets the same page, "check your mail", an address with no
//! account included: the answer never tells who has one. The post is on
//! the auth bucket (`settings.rate_limit.auth`): it mails any account.

use axum::{
    Form,
    body::Body,
    extract::Request,
    http::{StatusCode, request::Parts},
};
// Named import on purpose: `leptos::prelude::*` also exports an `Error`
// type that would shadow Loco's.
use leptos::view;
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::{
    controllers::cross_site,
    mailers::auth::AuthMailer,
    middleware::rate_limit::auth_limited,
    models::users,
    render::render_private_page,
    views::{
        forgot::{ForgotForm, ForgotSent},
        layout::PageMeta,
    },
};

/// The page's address and the form's target.
pub const PATH: &str = "/forgot";

/// What the form posts; empty when the field is missing.
#[derive(Debug, Deserialize)]
pub struct Submitted {
    #[serde(default)]
    pub email: String,
}

fn meta() -> PageMeta {
    PageMeta {
        lang: "en",
        title: "Forgot your password".into(),
        description: "Get a link to choose a new password.".into(),
        robots: None,
        canonical: None,
    }
}

/// `GET /forgot`: the form.
#[debug_handler]
async fn page(State(ctx): State<AppContext>, req: Request) -> Result<Response> {
    render_private_page(&ctx, req, meta(), StatusCode::OK, || {
        view! { <ForgotForm /> }
    })
    .await
}

/// `POST /forgot`: an account with the address gets the reset link; every
/// post gets the same page.
async fn submit(
    State(ctx): State<AppContext>,
    parts: Parts,
    Form(form): Form<Submitted>,
) -> Result<Response> {
    if let Some(forbidden) = cross_site::refused(&parts.headers, "forgot") {
        return Ok(forbidden);
    }
    let req = Request::from_parts(parts, Body::empty());
    if let Ok(user) = users::Model::find_by_email(&ctx.db, &form.email).await {
        let user = user
            .into_active_model()
            .set_forgot_password_sent(&ctx.db)
            .await?;
        AuthMailer::forgot_password(&ctx, &user).await?;
        tracing::info!(pid = user.pid.to_string(), "reset link sent");
    }
    render_private_page(&ctx, req, meta(), StatusCode::OK, || {
        view! { <ForgotSent /> }
    })
    .await
}

/// The page, and its post on the auth bucket.
pub fn routes(ctx: &AppContext) -> Routes {
    Routes::new().add(PATH, get(page).merge(auth_limited(ctx, post(submit))))
}
