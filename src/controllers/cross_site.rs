//! The check every form's target makes first: whether another site's page
//! sent the post. Browsers send `Sec-Fetch-Site` on every request,
//! `same-origin` when our own page posts and `cross-site` when another
//! site's page does, so a hostile page cannot submit our forms from a
//! visitor's browser, the session cookie riding along: signing in and out
//! (controllers/login.rs), the verify button (controllers/verify.rs) and
//! the new password (controllers/reset.rs), each through [`refused`]. As an
//! islands app on this stack checks its forms.

use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

/// The answer to a post another site's page sent ([`is_cross_site`]): a
/// plain 403, no page of ours having asked for it, which a browser gets as
/// the site's error page (middleware/error_page.rs), logged under `form`'s
/// name. None for every other post, which the handler goes on with.
pub fn refused(headers: &HeaderMap, form: &str) -> Option<Response> {
    is_cross_site(headers).then(|| {
        tracing::info!("{form}: cross-site post refused");
        (StatusCode::FORBIDDEN, "Forbidden").into_response()
    })
}

/// Whether the browser says this request was made by another site's page.
/// `same-origin` is our own form, `none` a navigation the person started
/// (a bookmark, a typed address); anything else, `cross-site` or a sibling
/// site, is refused. No header at all is not a browser making the claim,
/// so it passes: the check closes the browser hole and is not a bot filter.
fn is_cross_site(headers: &HeaderMap) -> bool {
    headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|site| !matches!(site.trim(), "same-origin" | "none"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with_site(site: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(site) = site {
            headers.insert("sec-fetch-site", site.parse().expect("header value"));
        }
        headers
    }

    #[test]
    fn only_a_browser_claiming_another_site_is_cross_site() {
        assert!(
            !is_cross_site(&headers_with_site(None)),
            "no header: allowed"
        );
        assert!(
            !is_cross_site(&headers_with_site(Some("same-origin"))),
            "our own form"
        );
        assert!(
            !is_cross_site(&headers_with_site(Some("none"))),
            "a navigation the person started"
        );
        assert!(is_cross_site(&headers_with_site(Some("cross-site"))));
        assert!(
            is_cross_site(&headers_with_site(Some("same-site"))),
            "a sibling site is not our page either"
        );
    }
}
