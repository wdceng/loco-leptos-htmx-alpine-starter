//! `/login`: the sign-in page and its form's post, which opens a session
//! (src/models/sessions.rs) and hands its cookie over; `/logout`, whose
//! post closes it. Plain HTML forms, so they work without the islands
//! bundle. Signed in already, the page says as whom, with the way out.
//!
//! Every refusal is the same answer after the same work: an email with no
//! account checks the password against a dummy hash (src/hashing.rs), so
//! neither the answer nor its time tells whether the address is registered.
//! An account whose email isn't verified yet is refused too. The post is on
//! the auth bucket (`settings.rate_limit.auth`), the page's visits never.

use axum::{
    Form,
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode, header::LOCATION, request::Parts},
};
// Named import on purpose: `leptos::prelude::*` also exports an `Error`
// type that would shadow Loco's.
use leptos::view;
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::{
    controllers::{cross_site, signed::signed_in},
    cookies, hashing,
    middleware::rate_limit::auth_limited,
    models::{sessions::Session, users},
    render::render_private_page,
    sql,
    views::{
        layout::PageMeta,
        login::{LoginForm, SignedIn},
    },
};

/// The page's address and the form's target.
pub const PATH: &str = "/login";

/// The sign-out's target.
pub const LOGOUT: &str = "/logout";

/// What every refused sign-in is told.
const REFUSED: &str = "That email and password don't match a verified account. Not verified yet? Use the link in the welcome mail.";

/// What the form posts; empty when a field is missing.
#[derive(Debug, Deserialize)]
pub struct Submitted {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
}

fn meta() -> PageMeta {
    PageMeta {
        lang: "en",
        title: "Sign in".into(),
        description: "Sign in to your account.".into(),
        robots: None,
        canonical: None,
    }
}

/// `GET /login`: the form, or, signed in already, who and the way out.
#[debug_handler]
async fn page(State(ctx): State<AppContext>, req: Request) -> Result<Response> {
    match signed_in(&ctx, req.headers()).await? {
        Some(signed) => {
            let name = signed.user.name;
            render_private_page(&ctx, req, meta(), StatusCode::OK, move || {
                view! { <SignedIn name=name.clone() /> }
            })
            .await
        }
        None => {
            render_private_page(&ctx, req, meta(), StatusCode::OK, || {
                view! { <LoginForm email=String::new() problem=None /> }
            })
            .await
        }
    }
}

/// `POST /login`: the session opened and its cookie handed over, then on
/// to the home page; refused, the form again with the email typed and the
/// one reason, its title starting "Error: ".
async fn submit(
    State(ctx): State<AppContext>,
    parts: Parts,
    Form(form): Form<Submitted>,
) -> Result<Response> {
    if let Some(forbidden) = cross_site::refused(&parts.headers, "login") {
        return Ok(forbidden);
    }
    let user = users::Model::find_by_email(&ctx.db, &form.email).await.ok();
    let valid = match &user {
        Some(user) => user.verify_password(&form.password).await,
        None => {
            hashing::verify_dummy(&form.password).await;
            false
        }
    };
    let Some(user) = user
        .filter(|_| valid)
        .filter(|user| user.email_verified_at.is_some())
    else {
        // Which of the three it was stays out of the log: the address is
        // personal data, and the count is what matters.
        tracing::info!("sign-in refused");
        let req = Request::from_parts(parts, Body::empty());
        let email = form.email;
        return render_private_page(
            &ctx,
            req,
            meta().refused(),
            StatusCode::BAD_REQUEST,
            move || view! { <LoginForm email=email.clone() problem=Some(REFUSED.to_string()) /> },
        )
        .await;
    };
    let (secret, _) = Session::open(sql::pool(&ctx), user.id)
        .await
        .map_err(Error::wrap)?;
    tracing::info!(pid = user.pid.to_string(), "signed in");
    let mut res = (StatusCode::SEE_OTHER, [(LOCATION, "/")]).into_response();
    cookies::store(&mut res, &Session::cookie(&secret, cookies::secure(&ctx)))?;
    Ok(res)
}

/// `POST /logout`: the session closed and the cookie taken away, then the
/// sign-in page. Nobody signed in, the same answer.
async fn logout(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    if let Some(forbidden) = cross_site::refused(&headers, "logout") {
        return Ok(forbidden);
    }
    if let Some(signed) = signed_in(&ctx, &headers).await? {
        Session::close(sql::pool(&ctx), signed.session.id)
            .await
            .map_err(Error::wrap)?;
        tracing::info!(pid = signed.user.pid.to_string(), "signed out");
    }
    let mut res = (StatusCode::SEE_OTHER, [(LOCATION, PATH)]).into_response();
    cookies::store(&mut res, &Session::cookie_deleted(cookies::secure(&ctx)))?;
    Ok(res)
}

/// The page, its post on the auth bucket (`auth_limited`), and the
/// sign-out.
pub fn routes(ctx: &AppContext) -> Routes {
    Routes::new()
        .add(PATH, get(page).merge(auth_limited(ctx, post(submit))))
        .add(LOGOUT, post(logout))
}
