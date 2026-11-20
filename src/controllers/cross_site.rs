//! The check every form's target makes before its handler runs: whether
//! another site's page sent the post. Browsers send `Sec-Fetch-Site` on
//! every request, `same-origin` when our own page posts and `cross-site`
//! when another site's page does, so a hostile page cannot submit our
//! forms from a visitor's browser, the session cookie riding along. Every
//! post's handler takes [`SameSite`], the one way (guarded in
//! `tests/one_job.rs`, every POST walked by `tests/requests/canaries.rs`).
//! As an islands app on this stack checks its forms.

use axum::{
    extract::{FromRequestParts, MatchedPath},
    http::{HeaderMap, StatusCode, request::Parts},
    response::{IntoResponse, Response},
};

/// A post our own pages sent, the extractor every form's target takes:
/// one another site's page sent ([`is_cross_site`]) is answered before the
/// handler or its body is read, a plain 403 the browser gets as the
/// site's error page (middleware/error_page.rs), logged by the route's
/// pattern. Taken after `Signed`, so a signed-out post still goes to the
/// sign-in first.
#[derive(Clone, Copy, Debug)]
pub struct SameSite;

impl<S: Send + Sync> FromRequestParts<S> for SameSite {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if !is_cross_site(&parts.headers) {
            return Ok(Self);
        }
        let route = parts
            .extensions
            .get::<MatchedPath>()
            .map_or("", MatchedPath::as_str);
        tracing::info!(route, "cross-site post refused");
        Err((StatusCode::FORBIDDEN, "Forbidden").into_response())
    }
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

    /// The extractor answers another site's post itself, a plain 403, and
    /// lets our own page's through.
    #[tokio::test]
    async fn the_extractor_refuses_another_sites_post() {
        let parts = |site: &str| {
            axum::http::Request::builder()
                .header("sec-fetch-site", site)
                .body(())
                .map(|request| request.into_parts().0)
        };
        let mut theirs = parts("cross-site").expect("a request");
        let refused = SameSite::from_request_parts(&mut theirs, &())
            .await
            .expect_err("refused");
        assert_eq!(refused.status(), StatusCode::FORBIDDEN);
        let mut ours = parts("same-origin").expect("a request");
        assert!(SameSite::from_request_parts(&mut ours, &()).await.is_ok());
    }
}
