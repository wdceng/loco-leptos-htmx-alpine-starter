//! `/register`: the sign-up page and its form's post, which creates the
//! account and mails the link that verifies its address (`/verify`). A
//! plain HTML form, so it works without the islands bundle.
//!
//! The rules come first, so a refusal says the same whether the address is
//! taken or not; past them every sign-up gets the same page, "check your
//! mail", a taken address included: the answer never tells who has an
//! account. The post is on the auth bucket (`settings.rate_limit.auth`):
//! it mails any address.

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
    models::users::{self, RegisterParams, name_or_email_problem, password_problem},
    render::render_private_page,
    views::{
        layout::PageMeta,
        register::{RegisterForm, RegisterSent},
    },
};

/// The page's address and the form's target.
pub const PATH: &str = "/register";

/// What the form posts; empty when a field is missing.
#[derive(Debug, Deserialize)]
pub struct Submitted {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
}

fn meta() -> PageMeta {
    PageMeta {
        lang: "en",
        title: "Sign up".into(),
        description: "Make an account.".into(),
        robots: None,
    }
}

/// `GET /register`: the form.
#[debug_handler]
async fn page(State(ctx): State<AppContext>, req: Request) -> Result<Response> {
    render_private_page(&ctx, req, meta(), StatusCode::OK, || {
        view! { <RegisterForm name=String::new() email=String::new() problem=None /> }
    })
    .await
}

/// `POST /register`: a refused field shows the form again with the name
/// and email typed and the one reason, its title starting "Error: ".
/// Otherwise the account is made and its welcome mail sent, and the page
/// says to check the mail, as it does for a taken address.
async fn submit(
    State(ctx): State<AppContext>,
    parts: Parts,
    Form(form): Form<Submitted>,
) -> Result<Response> {
    if let Some(forbidden) = cross_site::refused(&parts.headers, "register") {
        return Ok(forbidden);
    }
    let req = Request::from_parts(parts, Body::empty());
    if let Some(problem) =
        password_problem(&form.password).or_else(|| name_or_email_problem(&form.name, &form.email))
    {
        let (name, email) = (form.name, form.email);
        return render_private_page(
            &ctx,
            req,
            meta().refused(),
            StatusCode::BAD_REQUEST,
            move || {
                view! {
                    <RegisterForm
                        name=name.clone()
                        email=email.clone()
                        problem=Some(problem.clone())
                    />
                }
            },
        )
        .await;
    }
    let params = RegisterParams {
        name: form.name,
        email: form.email,
        password: form.password,
    };
    match users::Model::create_with_password(&ctx.db, &params).await {
        Ok(user) => {
            let user = user
                .into_active_model()
                .set_email_verification_sent(&ctx.db)
                .await?;
            AuthMailer::send_welcome(&ctx, &user).await?;
            tracing::info!(pid = user.pid.to_string(), "signed up through the page");
        }
        // Never the address: personal data, and a taken one is exactly
        // what the answer keeps to itself.
        Err(err) => tracing::info!(message = err.to_string(), "could not register user"),
    }
    render_private_page(&ctx, req, meta(), StatusCode::OK, || {
        view! { <RegisterSent /> }
    })
    .await
}

/// The page, and its post on the auth bucket.
pub fn routes(ctx: &AppContext) -> Routes {
    Routes::new().add(PATH, get(page).merge(auth_limited(ctx, post(submit))))
}
