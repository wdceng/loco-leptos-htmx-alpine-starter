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
use chrono::Utc;
// Named import on purpose: `leptos::prelude::*` also exports an `Error`
// type that would shadow Loco's.
use leptos::view;
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::{
    controllers::{cross_site, signed::signed_in},
    cookies, hashing,
    mailers::auth::AuthMailer,
    middleware::rate_limit::auth_limited,
    models::{
        auth_events::{self, Event},
        roles::Role,
        sessions::{Session, label_of},
        sign_ins::{Checked, SignIn},
        users,
    },
    render::render_private_page,
    sql,
    views::{
        layout::PageMeta,
        login::{CodeForm, CodeProblem, LoginForm, SignedIn},
    },
};

/// The page's address and the form's target.
pub const PATH: &str = "/login";

/// The second step's page and form's target: the mailed code.
pub const CODE: &str = "/login/code";

/// The target that mails a new code.
pub const CODE_AGAIN: &str = "/login/code/again";

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
    let pool = sql::pool(&ctx);
    // An admin's second step: a code by mail, the session only once it is
    // right (src/models/sign_ins.rs). Within the wait after wrong codes no
    // code goes: the code's page says how long (src/models/auth_events.rs).
    if Role::of(pool, user.id).await.map_err(Error::wrap)? == Role::Admin {
        let mailed = auth_events::code_waiting(pool, user.id, Utc::now())
            .await
            .map_err(Error::wrap)?
            .is_none();
        let (secret, code, _) = SignIn::open(pool, user.id, mailed)
            .await
            .map_err(Error::wrap)?;
        if mailed {
            AuthMailer::sign_in_code(&ctx, &user, &code).await?;
            tracing::info!(pid = user.pid.to_string(), "sign-in code mailed");
        }
        let mut res = (StatusCode::SEE_OTHER, [(LOCATION, CODE)]).into_response();
        cookies::store(&mut res, &SignIn::cookie(&secret, cookies::secure(&ctx)))?;
        return Ok(res);
    }
    open_session(&ctx, &user, &parts.headers).await
}

/// A session opened for `user`, who passed every step, in the browser that
/// sent `headers`: its cookie handed over, then on to the home page. The
/// one way a sign-in ends, after the password or after its code; the count
/// of wrong codes starts again from it.
async fn open_session(
    ctx: &AppContext,
    user: &users::Model,
    headers: &HeaderMap,
) -> Result<Response> {
    let pool = sql::pool(ctx);
    let (secret, _) = Session::open(pool, user.id, &label_of(headers))
        .await
        .map_err(Error::wrap)?;
    auth_events::record(pool, user.id, Event::SignedIn)
        .await
        .map_err(Error::wrap)?;
    tracing::info!(pid = user.pid.to_string(), "signed in");
    let mut res = (StatusCode::SEE_OTHER, [(LOCATION, "/")]).into_response();
    cookies::store(&mut res, &Session::cookie(&secret, cookies::secure(ctx)))?;
    Ok(res)
}

/// The sign-in waiting for its code, by the `signin` cookie; none, the
/// password again.
async fn waiting(ctx: &AppContext, headers: &HeaderMap) -> Result<Option<(String, SignIn)>> {
    let Some(secret) = SignIn::secret_from(headers) else {
        return Ok(None);
    };
    Ok(SignIn::find(sql::pool(ctx), &secret)
        .await
        .map_err(Error::wrap)?
        .map(|sign_in| (secret, sign_in)))
}

fn code_meta() -> PageMeta {
    PageMeta {
        title: "Your sign-in code".into(),
        description: "The code we mailed you, to finish signing in.".into(),
        ..meta()
    }
}

/// Back to the password: no sign-in waits, or its tries are spent. The
/// `signin` cookie taken away.
fn to_password(ctx: &AppContext) -> Result<Response> {
    let mut res = (StatusCode::SEE_OTHER, [(LOCATION, PATH)]).into_response();
    cookies::store(&mut res, &SignIn::cookie_deleted(cookies::secure(ctx)))?;
    Ok(res)
}

/// The code's page: `problem` what was refused, `said` what was done,
/// `mailed` whether a code went out at all.
async fn show_code(
    ctx: &AppContext,
    req: Request,
    status: StatusCode,
    problem: Option<CodeProblem>,
    said: Option<&'static str>,
    mailed: bool,
) -> Result<Response> {
    let meta = if problem.is_some() {
        code_meta().refused()
    } else {
        code_meta()
    };
    render_private_page(ctx, req, meta, status, move || {
        view! { <CodeForm problem=problem.clone() said=said mailed=mailed /> }
    })
    .await
}

/// What the code's page says while the account waits after wrong codes.
fn wait_line(seconds: i64) -> CodeProblem {
    let unit = if seconds == 1 { "second" } else { "seconds" };
    CodeProblem::NotNow(format!(
        "Too many wrong codes. Try again in {seconds} {unit}."
    ))
}

/// The seconds the account of `sign_in` waits before a code is checked or
/// mailed (src/models/auth_events.rs); none where it doesn't.
async fn code_wait(ctx: &AppContext, sign_in: &SignIn) -> Result<Option<i64>> {
    auth_events::code_waiting(sql::pool(ctx), sign_in.user_id, Utc::now())
        .await
        .map_err(Error::wrap)
}

/// `GET /login/code`: the code's form, for the sign-in waiting, saying at
/// once how long the account waits, if it does: the password reached it
/// with no code mailed.
#[debug_handler]
async fn code_page(State(ctx): State<AppContext>, req: Request) -> Result<Response> {
    let Some((_, sign_in)) = waiting(&ctx, req.headers()).await? else {
        return to_password(&ctx);
    };
    let problem = code_wait(&ctx, &sign_in).await?.map(wait_line);
    let mailed = sign_in.code_mailed();
    show_code(&ctx, req, StatusCode::OK, problem, None, mailed).await
}

/// What the code's form posts; empty when the field is missing.
#[derive(Debug, Deserialize)]
pub struct Code {
    #[serde(default)]
    pub code: String,
}

/// The user a sign-in is for.
async fn user_of(ctx: &AppContext, sign_in: &SignIn) -> Result<users::Model> {
    users::Entity::find_by_id(sign_in.user_id)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::NotFound)
}

/// `POST /login/code`: within the account's wait, nothing checked; the code
/// right, the session opened and the sign-in closed; wrong, the form again
/// with the tries left; spent, the password again. A wrong or spent one
/// counts toward the account's wait.
async fn code_submit(
    State(ctx): State<AppContext>,
    parts: Parts,
    Form(form): Form<Code>,
) -> Result<Response> {
    if let Some(forbidden) = cross_site::refused(&parts.headers, "sign-in code") {
        return Ok(forbidden);
    }
    let Some((secret, sign_in)) = waiting(&ctx, &parts.headers).await? else {
        return to_password(&ctx);
    };
    if let Some(seconds) = code_wait(&ctx, &sign_in).await? {
        tracing::info!(seconds, "sign-in code within the wait");
        let req = Request::from_parts(parts, Body::empty());
        let problem = Some(wait_line(seconds));
        let mailed = sign_in.code_mailed();
        return show_code(&ctx, req, StatusCode::BAD_REQUEST, problem, None, mailed).await;
    }
    let pool = sql::pool(&ctx);
    let user = user_of(&ctx, &sign_in).await?;
    match sign_in
        .check(pool, &secret, &form.code)
        .await
        .map_err(Error::wrap)?
    {
        Checked::Right => {
            sign_in.close(pool).await.map_err(Error::wrap)?;
            let mut res = open_session(&ctx, &user, &parts.headers).await?;
            cookies::store(&mut res, &SignIn::cookie_deleted(cookies::secure(&ctx)))?;
            Ok(res)
        }
        Checked::Wrong { left } => {
            auth_events::record(pool, user.id, Event::CodeWrong)
                .await
                .map_err(Error::wrap)?;
            tell_of_failures(&ctx, &user, &parts.headers).await?;
            let req = Request::from_parts(parts, Body::empty());
            let problem = format!(
                "That code isn't right. {left} {} left.",
                if left == 1 { "try" } else { "tries" }
            );
            let problem = Some(CodeProblem::Wrong(problem));
            show_code(&ctx, req, StatusCode::BAD_REQUEST, problem, None, true).await
        }
        Checked::Spent => {
            auth_events::record(pool, user.id, Event::CodeSpent)
                .await
                .map_err(Error::wrap)?;
            tell_of_failures(&ctx, &user, &parts.headers).await?;
            sign_in.close(pool).await.map_err(Error::wrap)?;
            tracing::info!("sign-in code tries spent");
            to_password(&ctx)
        }
    }
}

/// The person told by mail at the [`auth_events::FREE_TRIES`]th wrong code
/// in a row, once a streak: a second step failing after a right password
/// means the password is known (OWASP). The browser named in the mail.
async fn tell_of_failures(
    ctx: &AppContext,
    user: &users::Model,
    headers: &HeaderMap,
) -> Result<()> {
    let failures = auth_events::code_failures(sql::pool(ctx), user.id)
        .await
        .map_err(Error::wrap)?;
    if failures == auth_events::FREE_TRIES {
        tracing::info!(
            pid = user.pid.to_string(),
            failures,
            "wrong sign-in codes, the person told"
        );
        AuthMailer::code_failures(ctx, user, failures, &label_of(headers)).await?;
    }
    Ok(())
}

/// `POST /login/code/again`: a new code mailed, unless the account waits
/// after wrong codes or one went within the last minute; the form again,
/// saying which.
async fn code_again(State(ctx): State<AppContext>, parts: Parts) -> Result<Response> {
    if let Some(forbidden) = cross_site::refused(&parts.headers, "sign-in code again") {
        return Ok(forbidden);
    }
    let Some((secret, sign_in)) = waiting(&ctx, &parts.headers).await? else {
        return to_password(&ctx);
    };
    let req = Request::from_parts(parts, Body::empty());
    if let Some(seconds) = code_wait(&ctx, &sign_in).await? {
        let problem = Some(wait_line(seconds));
        let mailed = sign_in.code_mailed();
        return show_code(&ctx, req, StatusCode::OK, problem, None, mailed).await;
    }
    let renewed = sign_in
        .renew_code(sql::pool(&ctx), &secret)
        .await
        .map_err(Error::wrap)?;
    if let Some(code) = renewed {
        let user = user_of(&ctx, &sign_in).await?;
        AuthMailer::sign_in_code(&ctx, &user, &code).await?;
        let said = if sign_in.code_mailed() {
            "A new code is on its way; the one before no longer works."
        } else {
            "A code is on its way."
        };
        return show_code(&ctx, req, StatusCode::OK, None, Some(said), true).await;
    }
    let wait = sign_in.code_wait().unwrap_or(1);
    let problem = Some(CodeProblem::NotNow(format!(
        "You can ask for a new code in {wait} seconds."
    )));
    let mailed = sign_in.code_mailed();
    show_code(&ctx, req, StatusCode::OK, problem, None, mailed).await
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
        .add(
            CODE,
            get(code_page).merge(auth_limited(ctx, post(code_submit))),
        )
        .add(CODE_AGAIN, auth_limited(ctx, post(code_again)))
        .add(LOGOUT, post(logout))
}
