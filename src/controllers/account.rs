//! `/account`: the signed-in user's own page. Who they are, the browsers
//! they are signed in on (src/models/sessions.rs), each signed out on its
//! own or all but this one, a new password, and the account deleted. Plain
//! HTML forms, so they work without the islands bundle; nobody signed in is
//! sent to the sign-in (`Signed`).
//!
//! Deleting the account asks for the password, unless the person proved
//! who they are in this session within the last fifteen minutes
//! (`Session::proven`): a session taken from an unlocked computer later
//! deletes nothing. A new password needs the current one, and signs out
//! every other browser. The posts that take a password are on the auth
//! bucket (`settings.rate_limit.auth`). As an islands app on this stack has
//! its account page.

use axum::{
    Form,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Request, multipart::MultipartError},
    http::{StatusCode, header::LOCATION, request::Parts},
};
// Named import on purpose: `leptos::prelude::*` also exports an `Error`
// type that would shadow Loco's.
use leptos::view;
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::{
    app::stored,
    controllers::{cross_site, signed::Signed},
    cookies,
    middleware::rate_limit::auth_limited,
    models::{
        auth_events,
        avatars::{self, Avatars},
        sessions::{self, Session},
        sign_ins::SignIn,
        users::{self, password_problem},
    },
    render::render_private_page,
    sql, uploads,
    views::{
        account::{AccountDeleted, AccountPage, Browsers, Problem},
        layout::PageMeta,
    },
};

/// The page's address.
pub const PATH: &str = "/account";

/// The target that signs out every other browser.
pub const CLOSE_OTHERS: &str = "/account/browsers/close-others";

/// The target that signs out one browser, `{id}` its session's row.
pub const CLOSE_ONE: &str = "/account/browsers/{id}/close";

/// The new password's target.
pub const PASSWORD: &str = "/account/password";

/// The avatar's target.
pub const AVATAR: &str = "/account/avatar";

/// The deletion's target.
pub const DELETE: &str = "/account/delete";

/// Where a deleted account ends.
pub const DELETED: &str = "/account/deleted";

/// What is said when the password given isn't the account's.
const WRONG_PASSWORD: &str = "That isn't your current password.";

/// The address that signs out the browser of session `id`.
#[must_use]
pub fn close_one(id: i64) -> String {
    CLOSE_ONE.replace("{id}", &id.to_string())
}

/// The new password's form; empty when a field is missing.
#[derive(Debug, Deserialize)]
pub struct NewPassword {
    #[serde(default)]
    pub current: String,
    #[serde(default)]
    pub password: String,
}

/// The deletion's form: the password, unless the session is proven.
#[derive(Debug, Deserialize)]
pub struct Deletion {
    #[serde(default)]
    pub password: String,
}

fn meta() -> PageMeta {
    PageMeta {
        lang: "en",
        title: "Your account".into(),
        description: "Your account: where you are signed in, your password.".into(),
        // A page of one signed-in person, never a search's.
        robots: Some("noindex"),
        canonical: None,
    }
}

/// The page for `signed`, with what was refused or done, if anything.
async fn answer(
    ctx: &AppContext,
    req: Request,
    signed: Signed,
    status: StatusCode,
    problem: Option<Problem>,
    said: Option<&'static str>,
) -> Result<Response> {
    let pool = sql::pool(ctx);
    let browsers = Browsers::of(
        Session::browsers(pool, signed.user.id)
            .await
            .map_err(Error::wrap)?,
        signed.session.id,
    );
    let avatar = avatars::of(pool, signed.user.id)
        .await
        .map_err(Error::wrap)?
        .map(|name| format!("{}{name}", uploads::PREFIX));
    let meta = if problem.is_some() {
        meta().refused()
    } else {
        meta()
    };
    let (name, email, proven) = (
        signed.user.name.clone(),
        signed.user.email.clone(),
        signed.session.proven(),
    );
    render_private_page(ctx, req, meta, status, move || {
        view! {
            <AccountPage
                name=name.clone()
                email=email.clone()
                browsers=browsers.clone()
                avatar=avatar.clone()
                proven=proven
                problem=problem.clone()
                said=said
            />
        }
    })
    .await
}

/// `GET /account`.
#[debug_handler]
async fn page(State(ctx): State<AppContext>, signed: Signed, req: Request) -> Result<Response> {
    answer(&ctx, req, signed, StatusCode::OK, None, None).await
}

/// `POST /account/avatar`: the picture sent made into the avatar
/// (src/uploads.rs), off the async threads, then the page again; the one
/// it replaces gone once nobody shows it. Refused, the page with the
/// reason, its title starting "Error: ".
async fn avatar(
    State(ctx): State<AppContext>,
    signed: Signed,
    parts: Parts,
    mut form: Multipart,
) -> Result<Response> {
    if let Some(forbidden) = cross_site::refused(&parts.headers, "avatar") {
        return Ok(forbidden);
    }
    let req = Request::from_parts(parts, Body::empty());
    let made = match avatar_field(&mut form).await {
        Ok(upload) if upload.is_empty() => Err("Choose a picture first."),
        Ok(upload) => tokio::task::spawn_blocking(move || uploads::avatar(&upload))
            .await
            .map_err(Error::wrap)?,
        Err(reason) => Err(reason),
    };
    let jpeg = match made {
        Ok(jpeg) => jpeg,
        Err(reason) => {
            let problem = Problem::Avatar(reason.to_string());
            return answer(
                &ctx,
                req,
                signed,
                StatusCode::BAD_REQUEST,
                Some(problem),
                None,
            )
            .await;
        }
    };
    let Avatars(folder) = stored(&ctx)?;
    let name = uploads::keep(&folder, &jpeg).await.map_err(Error::wrap)?;
    let pool = sql::pool(&ctx);
    let before = avatars::set(pool, signed.user.id, Some(&name))
        .await
        .map_err(Error::wrap)?;
    if let Some(before) = before.filter(|before| *before != name) {
        forget_unused(&ctx, &folder, &before).await?;
    }
    tracing::info!(pid = signed.user.pid.to_string(), "avatar changed");
    let said = "Your picture is changed.";
    answer(&ctx, req, signed, StatusCode::OK, None, Some(said)).await
}

/// The bytes of the form's `avatar` field, empty where none came. Refused
/// where the body ran over the route's limit ([`uploads::TOO_LARGE`]) or
/// broke off ([`uploads::NOT_A_PICTURE`]), never read as no picture.
async fn avatar_field(form: &mut Multipart) -> std::result::Result<Vec<u8>, &'static str> {
    let refusal = |e: MultipartError| {
        if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
            uploads::TOO_LARGE
        } else {
            uploads::NOT_A_PICTURE
        }
    };
    let mut upload = Vec::new();
    while let Some(field) = form.next_field().await.map_err(refusal)? {
        if field.name() == Some("avatar") {
            upload = field.bytes().await.map_err(refusal)?.to_vec();
        }
    }
    Ok(upload)
}

/// The avatar `name` taken away from `folder` once no user shows it.
async fn forget_unused(ctx: &AppContext, folder: &std::path::Path, name: &str) -> Result<()> {
    if !avatars::in_use(sql::pool(ctx), name)
        .await
        .map_err(Error::wrap)?
    {
        uploads::forget(folder, name).await.map_err(Error::wrap)?;
    }
    Ok(())
}

/// `POST /account/browsers/{id}/close`: that browser signed out, if it is
/// one of the user's, then the page again.
async fn close(
    State(ctx): State<AppContext>,
    signed: Signed,
    Path(id): Path<i64>,
    parts: Parts,
) -> Result<Response> {
    if let Some(forbidden) = cross_site::refused(&parts.headers, "close a browser") {
        return Ok(forbidden);
    }
    if Session::close_of(sql::pool(&ctx), signed.user.id, id)
        .await
        .map_err(Error::wrap)?
    {
        tracing::info!(pid = signed.user.pid.to_string(), "a browser signed out");
    }
    Ok((StatusCode::SEE_OTHER, [(LOCATION, PATH)]).into_response())
}

/// `POST /account/browsers/close-others`: every browser but this one
/// signed out, then the page again.
async fn close_others(
    State(ctx): State<AppContext>,
    signed: Signed,
    parts: Parts,
) -> Result<Response> {
    if let Some(forbidden) = cross_site::refused(&parts.headers, "close the other browsers") {
        return Ok(forbidden);
    }
    Session::close_others(sql::pool(&ctx), signed.user.id, signed.session.id)
        .await
        .map_err(Error::wrap)?;
    tracing::info!(
        pid = signed.user.pid.to_string(),
        "the other browsers signed out"
    );
    Ok((StatusCode::SEE_OTHER, [(LOCATION, PATH)]).into_response())
}

/// `POST /account/password`: the current password, then the new one's
/// rule; a new password signs out every other browser and proves this
/// session. Refused, the page again with the reason, its title starting
/// "Error: ".
async fn password(
    State(ctx): State<AppContext>,
    mut signed: Signed,
    parts: Parts,
    Form(form): Form<NewPassword>,
) -> Result<Response> {
    if let Some(forbidden) = cross_site::refused(&parts.headers, "new password") {
        return Ok(forbidden);
    }
    let req = Request::from_parts(parts, Body::empty());
    let refused = if signed.user.verify_password(&form.current).await {
        password_problem(&form.password).map(Problem::New)
    } else {
        Some(Problem::Current(WRONG_PASSWORD.to_string()))
    };
    if let Some(problem) = refused {
        return answer(
            &ctx,
            req,
            signed,
            StatusCode::BAD_REQUEST,
            Some(problem),
            None,
        )
        .await;
    }
    let pool = sql::pool(&ctx);
    let user = signed
        .user
        .clone()
        .into_active_model()
        .reset_password(&ctx.db, &form.password)
        .await?;
    let kept = signed.session.id;
    Session::after_new_password(pool, user.id, sessions::NewPassword::Changed { kept })
        .await
        .map_err(Error::wrap)?;
    Session::prove(pool, signed.session.id)
        .await
        .map_err(Error::wrap)?;
    signed.session.proven_at = chrono::Utc::now();
    tracing::info!(
        pid = user.pid.to_string(),
        "password changed on the account page"
    );
    let said = "Your password is changed, and every other browser is signed out.";
    answer(&ctx, req, signed, StatusCode::OK, None, Some(said)).await
}

/// `POST /account/delete`: the password, unless the session is proven;
/// then the user's sessions and the user are gone, the cookie with them,
/// and the browser goes to [`DELETED`]. The sessions are deleted first, by
/// hand: SQLite's cascade holds only on a connection whose foreign keys
/// are on, and Loco turns them on for one of the pool's.
async fn delete(
    State(ctx): State<AppContext>,
    signed: Signed,
    parts: Parts,
    Form(form): Form<Deletion>,
) -> Result<Response> {
    if let Some(forbidden) = cross_site::refused(&parts.headers, "delete the account") {
        return Ok(forbidden);
    }
    if !signed.session.proven() && !signed.user.verify_password(&form.password).await {
        let req = Request::from_parts(parts, Body::empty());
        let problem = Problem::Delete(WRONG_PASSWORD.to_string());
        return answer(
            &ctx,
            req,
            signed,
            StatusCode::BAD_REQUEST,
            Some(problem),
            None,
        )
        .await;
    }
    let pool = sql::pool(&ctx);
    Session::close_all(pool, signed.user.id)
        .await
        .map_err(Error::wrap)?;
    SignIn::close_all(pool, signed.user.id)
        .await
        .map_err(Error::wrap)?;
    auth_events::forget(pool, signed.user.id)
        .await
        .map_err(Error::wrap)?;
    let avatar = avatars::of(pool, signed.user.id)
        .await
        .map_err(Error::wrap)?;
    users::Entity::delete_by_id(signed.user.id)
        .exec(&ctx.db)
        .await?;
    // The picture with the account, once nobody else shows it.
    if let Some(avatar) = avatar {
        let Avatars(folder) = stored(&ctx)?;
        forget_unused(&ctx, &folder, &avatar).await?;
    }
    tracing::info!(pid = signed.user.pid.to_string(), "account deleted");
    let mut res = (StatusCode::SEE_OTHER, [(LOCATION, DELETED)]).into_response();
    cookies::store(&mut res, &Session::cookie_deleted(cookies::secure(&ctx)))?;
    Ok(res)
}

/// `GET /account/deleted`: the account is gone; nobody is signed in.
#[debug_handler]
async fn deleted(State(ctx): State<AppContext>, req: Request) -> Result<Response> {
    let meta = PageMeta {
        title: "Account deleted".into(),
        ..meta()
    };
    render_private_page(&ctx, req, meta, StatusCode::OK, || {
        view! { <AccountDeleted /> }
    })
    .await
}

/// The page and its posts, the ones that take a password on the auth
/// bucket.
pub fn routes(ctx: &AppContext) -> Routes {
    Routes::new()
        .add(PATH, get(page))
        .add(CLOSE_ONE, post(close))
        .add(CLOSE_OTHERS, post(close_others))
        .add(PASSWORD, auth_limited(ctx, post(password)))
        // The upload's own body limit, over the site's 64 KB: axum's
        // `DefaultBodyLimit`, which Loco's `limit_payload` sets for every
        // route and this route's layer, the inner one, overrides.
        .add(
            AVATAR,
            auth_limited(ctx, post(avatar)).route_layer(DefaultBodyLimit::max(uploads::MAX_BYTES)),
        )
        .add(DELETE, auth_limited(ctx, post(delete)))
        .add(DELETED, get(deleted))
}
