//! Who is signed in: the user a request's `session` cookie names
//! (src/models/sessions.rs), read from the database on every request, so a
//! sign-out or a password reset holds at once, where a token would hold
//! until it ran out. [`Signed`] is an extractor: a handler that takes it
//! runs only for a signed-in user, anyone else being sent to the sign-in
//! page (a page request) or answered with one line (a script's request).
//! A page that shows something else to a visitor reads [`signed_in`]
//! itself, as the sign-in page does. Loco's own `auth::JWT` extractor is
//! not used: its 401 would show the error page rather than the sign-in, and
//! its token nothing could cancel. As an islands app on this stack does it.

use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, StatusCode, header::LOCATION, request::Parts},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;

use crate::{
    controllers::login,
    models::{_entities::users, sessions::Session},
    render::accepts_html,
    sql,
};

/// The signed-in user and their session, a handler's proof that someone
/// is signed in.
#[derive(Clone, Debug)]
pub struct Signed {
    pub user: users::Model,
    pub session: Session,
}

/// The user `headers`' cookie signs in, if any: a live session
/// (src/models/sessions.rs) and the user it belongs to.
///
/// # Errors
/// The database could not be read.
pub async fn signed_in(ctx: &AppContext, headers: &HeaderMap) -> Result<Option<Signed>> {
    let Some(secret) = Session::secret_from(headers) else {
        return Ok(None);
    };
    let Some(session) = Session::find(sql::pool(ctx), &secret)
        .await
        .map_err(Error::wrap)?
    else {
        return Ok(None);
    };
    let Some(user) = users::Entity::find_by_id(session.user_id)
        .one(&ctx.db)
        .await?
    else {
        return Ok(None);
    };
    Ok(Some(Signed { user, session }))
}

impl FromRequestParts<AppContext> for Signed {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, ctx: &AppContext) -> Result<Self, Response> {
        match signed_in(ctx, &parts.headers).await {
            Ok(Some(signed)) => Ok(signed),
            Ok(None) => Err(to_sign_in(&parts.headers)),
            Err(e) => {
                tracing::error!(error = %e, "signed: the session could not be read");
                Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
            }
        }
    }
}

/// Where a request with nobody signed in goes: a page asked for, a move
/// between pages in place too (`HX-Boosted`, and Back's
/// `HX-History-Restore-Request`: htmx follows the redirect and puts the
/// sign-in's page in place of the page), is sent to the sign-in. htmx's
/// other requests (a form inside a page, the session run out) get a 401
/// naming the sign-in in `HX-Redirect`, which htmx follows as a page load:
/// followed as a redirect, the sign-in's page would have been swapped into
/// the part. A script's request gets one line.
fn to_sign_in(headers: &HeaderMap) -> Response {
    let moves = ["hx-boosted", "hx-history-restore-request"]
        .iter()
        .any(|name| headers.contains_key(*name));
    if headers.contains_key("hx-request") && !moves {
        return (
            StatusCode::UNAUTHORIZED,
            [("hx-redirect", login::PATH)],
            "Unauthorized",
        )
            .into_response();
    }
    if !accepts_html(headers) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    (StatusCode::SEE_OTHER, [(LOCATION, login::PATH)]).into_response()
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(*name, HeaderValue::from_static(value));
        }
        headers
    }

    #[test]
    fn a_page_asked_for_is_sent_to_the_sign_in() {
        let res = to_sign_in(&headers(&[("accept", "text/html")]));
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(res.headers()[LOCATION], login::PATH);
    }

    #[test]
    fn a_move_in_place_is_sent_there_too() {
        for flag in ["hx-boosted", "hx-history-restore-request"] {
            let res = to_sign_in(&headers(&[
                ("accept", "text/html"),
                ("hx-request", "true"),
                (flag, "true"),
            ]));
            assert_eq!(res.status(), StatusCode::SEE_OTHER, "{flag}");
        }
    }

    #[test]
    fn htmx_inside_a_page_is_told_to_load_the_sign_in() {
        let res = to_sign_in(&headers(&[("accept", "text/html"), ("hx-request", "true")]));
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(res.headers()["hx-redirect"], login::PATH);
    }

    #[test]
    fn a_script_gets_one_line() {
        let res = to_sign_in(&headers(&[("accept", "application/json")]));
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert!(!res.headers().contains_key(LOCATION));
    }
}
