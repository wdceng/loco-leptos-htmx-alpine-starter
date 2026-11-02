//! The cookies the server writes and reads, one shape for all: for the
//! whole site, out of reach of scripts (`HttpOnly`), sent along when
//! another site links here but not when another site's page posts
//! (`SameSite=Lax`), and over HTTPS only where the site is served over it
//! (`Secure`). Today that's the session's (src/models/sessions.rs). As an
//! islands app on this stack writes its own.
//!
//! Where the site is served over HTTPS every name carries the `__Host-`
//! prefix, so the browser keeps the cookie to this host alone and no other
//! host of the domain can set one in its place: a sibling host's cookie
//! could otherwise sign a visitor into another's session. There a cookie
//! without the prefix is never read. Plain HTTP (development, the tests)
//! keeps the plain names, which a browser refuses the prefix on. Which it
//! is, the boot says once ([`serve_over_https`]): one process serves one
//! host.

use std::sync::OnceLock;

use axum::http::{HeaderMap, HeaderValue, header::SET_COOKIE};
use loco_rs::prelude::{cookie::CookieJar, *};

/// The prefix a cookie's name carries where the site is served over HTTPS.
const HOST_PREFIX: &str = "__Host-";

/// Whether the process serves its site over HTTPS, as the boot said;
/// plain HTTP until it says.
static OVER_HTTPS: OnceLock<bool> = OnceLock::new();

/// The boot's word, once, whether the site is served over HTTPS
/// (`server.host`): what [`value`] reads the names by. A second word, a
/// second app booted in the same process (the tests), keeps the first.
pub fn serve_over_https(ctx: &AppContext) {
    let _ = OVER_HTTPS.set(secure(ctx));
}

/// `name` as the browser holds it: with the `__Host-` prefix where
/// `secure`, plain otherwise.
fn named(name: &str, secure: bool) -> String {
    if secure {
        format!("{HOST_PREFIX}{name}")
    } else {
        name.to_string()
    }
}

/// The value of the cookie `name` a request carries, if it carries one; by
/// its `__Host-` name alone where the site is served over HTTPS.
#[must_use]
pub fn value(headers: &HeaderMap, name: &str) -> Option<String> {
    let over_https = OVER_HTTPS.get().copied().unwrap_or(false);
    CookieJar::from_headers(headers)
        .get(&named(name, over_https))
        .map(|cookie| cookie.value().to_string())
}

/// Whether a cookie is marked `Secure`: where the site is served over
/// https (`server.host`); the development server is plain HTTP.
#[must_use]
pub fn secure(ctx: &AppContext) -> bool {
    ctx.config.server.host.starts_with("https://")
}

/// A `Set-Cookie` value: `name` holding `value` for `max_age` seconds
/// (0 takes the cookie away), in the shape the module doc gives: `Path=/`
/// and no `Domain`, which the `__Host-` prefix asks.
#[must_use]
pub fn set_cookie(name: &str, value: &str, max_age: i64, secure: bool) -> String {
    let name = named(name, secure);
    let secure = if secure { "; Secure" } else { "" };
    format!("{name}={value}; Path=/; Max-Age={max_age}; HttpOnly; SameSite=Lax{secure}")
}

/// `cookie`, a `Set-Cookie` value, added to `res`.
///
/// # Errors
/// The value holds a character a header cannot; one [`set_cookie`] wrote
/// never does.
pub fn store(res: &mut Response, cookie: &str) -> Result<()> {
    let value =
        HeaderValue::from_str(cookie).map_err(|e| Error::Message(format!("cookie: {e}")))?;
    res.headers_mut().append(SET_COOKIE, value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header::COOKIE};

    use super::{named, set_cookie, value};

    #[test]
    fn a_cookie_is_read_by_its_name() {
        let mut headers = HeaderMap::new();
        assert_eq!(value(&headers, "session"), None);
        headers.insert(COOKIE, HeaderValue::from_static("other=1; session=abc"));
        assert_eq!(value(&headers, "session").as_deref(), Some("abc"));
        assert_eq!(value(&headers, "missing"), None);
    }

    #[test]
    fn a_name_carries_the_host_prefix_over_https_alone() {
        assert_eq!(named("session", true), "__Host-session");
        assert_eq!(named("session", false), "session");
    }

    #[test]
    fn a_cookie_has_the_sites_shape() {
        assert_eq!(
            set_cookie("session", "abc", 60, false),
            "session=abc; Path=/; Max-Age=60; HttpOnly; SameSite=Lax"
        );
        // Over HTTPS, the `__Host-` prefix, the browser keeping the cookie
        // to this host alone.
        assert_eq!(
            set_cookie("session", "", 0, true),
            "__Host-session=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax; Secure"
        );
    }
}
