//! `/verify`: the page the welcome mail links to (`/verify?token=…`,
//! src/mailers/auth.rs), and its button's post, which verifies the address.
//! Opening the link verifies nothing: many inboxes open every link to scan
//! it, and a scanner's GET verified an account someone else had registered
//! with this address and a password of their own (docs/security.md, the
//! review of 2026-10-31, #1). Only the post does, which a person sends.
//! Loco's `GET /api/auth/verify/{token}` stays for an API client; no mail
//! links to it.
//!
//! The token arrives in the address, so both answers go out as every page
//! a mail's token opens (`render_token_page`): `no-referrer` and
//! `no-store`.

use axum::{
    Form,
    body::Body,
    extract::{Query, Request},
    http::{StatusCode, request::Parts},
};
// Named import on purpose: `leptos::prelude::*` also exports an `Error`
// type that would shadow Loco's.
use leptos::view;
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::{
    models::users,
    render::render_token_page,
    views::{
        layout::PageMeta,
        verify::{VerifyDone, VerifyExpired, VerifyForm},
    },
};

/// The page's address, the form's target and the mail link's start.
pub const PATH: &str = "/verify";

/// The mail link's query, and what the button posts: empty when missing.
#[derive(Debug, Deserialize)]
pub struct Link {
    #[serde(default)]
    pub token: String,
}

fn meta() -> PageMeta {
    PageMeta {
        lang: "en",
        title: "Verify your email".into(),
        description: "Finish signing up by verifying your email address.".into(),
        // A page reached from a mail, never from a search.
        robots: Some("noindex"),
    }
}

/// `GET /verify?token=…`: the button. The token isn't checked here: an
/// expired one is said after the press.
#[debug_handler]
async fn page(
    State(ctx): State<AppContext>,
    Query(link): Query<Link>,
    req: Request,
) -> Result<Response> {
    let token = link.token;
    render_token_page(&ctx, req, meta(), StatusCode::OK, move || {
        view! { <VerifyForm token=token.clone() /> }
    })
    .await
}

/// `POST /verify`: the token verifies the address, or says it no longer
/// works. An address verified already is said to be verified.
async fn submit(
    State(ctx): State<AppContext>,
    parts: Parts,
    Form(form): Form<Link>,
) -> Result<Response> {
    let req = Request::from_parts(parts, Body::empty());
    let Ok(user) = users::Model::find_by_verification_token(&ctx.db, &form.token).await else {
        return render_token_page(
            &ctx,
            req,
            meta(),
            StatusCode::BAD_REQUEST,
            || view! { <VerifyExpired /> },
        )
        .await;
    };
    if user.email_verified_at.is_none() {
        let user = user.into_active_model().verified(&ctx.db).await?;
        tracing::info!(pid = user.pid.to_string(), "user verified through the page");
    }
    render_token_page(
        &ctx,
        req,
        meta(),
        StatusCode::OK,
        || view! { <VerifyDone /> },
    )
    .await
}

pub fn routes() -> Routes {
    Routes::new().add(PATH, get(page).post(submit))
}
