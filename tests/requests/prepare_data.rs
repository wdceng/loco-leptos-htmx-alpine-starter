//! Users for the request tests, made through the model: the site's own
//! pages are what the tests exercise, and Loco's JSON auth API that once
//! made them is unmounted (src/app.rs).

use app::models::users::{self, RegisterParams};
use loco_rs::app::AppContext;
use sea_orm::IntoActiveModel;

/// Every test user's password: at least 8 characters
/// (`users::PASSWORD_MIN`).
pub const PASSWORD: &str = "12341234";

/// A user with `email` and [`PASSWORD`], named Ana, who signed up, verified
/// when `verified`.
pub async fn user(ctx: &AppContext, email: &str, verified: bool) -> users::Model {
    let user = users::Model::create_with_password(
        &ctx.db,
        &RegisterParams {
            email: email.into(),
            password: PASSWORD.into(),
            name: "Ana".into(),
        },
    )
    .await
    .expect("the user is created");
    if !verified {
        return user;
    }
    user.into_active_model()
        .verified(&ctx.db)
        .await
        .expect("verified")
}

/// The sign-in form's fields.
pub fn sign_in(email: &str, password: &str) -> serde_json::Value {
    serde_json::json!({ "email": email, "password": password })
}

/// A page with its CSP nonces blanked: the one thing that differs between
/// two answers alike, being new on every request (src/render.rs).
pub fn without_nonces(page: &str) -> String {
    const NONCE: &str = "nonce=\"";
    let mut kept = String::with_capacity(page.len());
    let mut rest = page;
    while let Some(start) = rest.find(NONCE) {
        kept.push_str(&rest[..start + NONCE.len()]);
        rest = &rest[start + NONCE.len()..];
        rest = rest.find('"').map_or("", |end| &rest[end..]);
    }
    kept.push_str(rest);
    kept
}

/// Whether `email` and `password` sign in through the page: a 303 with the
/// session's cookie, where a refusal is the form again, a 400.
pub async fn signs_in(request: &loco_rs::TestServer, email: &str, password: &str) -> bool {
    let res = request.post("/login").form(&sign_in(email, password)).await;
    match res.status_code().as_u16() {
        303 => true,
        400 => false,
        other => panic!("the sign-in answered {other}: {}", res.text()),
    }
}
