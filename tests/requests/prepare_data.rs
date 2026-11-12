//! Users for the request tests, made through the model: the site's own
//! pages are what the tests exercise, and Loco's JSON auth API that once
//! made them is unmounted (src/app.rs).

use app::models::users::{self, RegisterParams};
use axum::http::HeaderValue;
use loco_rs::app::AppContext;
use sea_orm::IntoActiveModel;

/// A request for a page, as a browser asks for one: `Accept: text/html`,
/// which makes the site answer with a page, not the one line a script's
/// request gets (`render::accepts_html`). Every page request's one way.
pub trait WantsPage {
    #[must_use]
    fn wants_page(self) -> Self;
}

impl WantsPage for axum_test::TestRequest {
    fn wants_page(self) -> Self {
        self.add_header("accept", "text/html")
    }
}

/// That no id stands twice in `body`: a second one makes a label, an
/// `aria-labelledby`, a `popovertarget` or a script find the first.
pub fn assert_unique_ids(body: &str) {
    let mut ids: Vec<&str> = body
        .split(r#" id=""#)
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect();
    ids.sort_unstable();
    let twice: Vec<&str> = ids
        .windows(2)
        .filter(|pair| pair[0] == pair[1])
        .map(|pair| pair[0])
        .collect();
    assert!(twice.is_empty(), "ids twice: {twice:?}");
}

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

/// The header a page's nonce is read from ([`without_nonce`]).
pub const CSP: &str = "content-security-policy";

/// A page with its CSP nonce blanked, read from the answer's own policy:
/// the one thing two answers alike differ in, new on every request
/// (src/render.rs), and stamped by Leptos in more than `nonce=`, its
/// preload links' `crossorigin` too.
pub fn without_nonce(csp: &HeaderValue, page: &str) -> String {
    let nonce = csp
        .to_str()
        .ok()
        .and_then(|csp| csp.split("'nonce-").nth(1))
        .and_then(|rest| rest.split('\'').next())
        .expect("the page's CSP names its nonce");
    page.replace(nonce, "")
}

/// The `Cookie` header of a session `email` and [`PASSWORD`] opened
/// through the sign-in page, in a browser named by `user_agent`.
pub async fn session_cookie(
    request: &loco_rs::TestServer,
    email: &str,
    user_agent: &str,
) -> String {
    let res = request
        .post("/login")
        .add_header("user-agent", user_agent.to_owned())
        .form(&sign_in(email, PASSWORD))
        .await;
    assert_eq!(res.status_code(), 303, "signed in: {}", res.text());
    let set = res.header("set-cookie");
    let set = set.to_str().expect("a cookie");
    set.split(';').next().expect("the pair").to_string()
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
