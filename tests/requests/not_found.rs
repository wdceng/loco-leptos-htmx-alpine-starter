//! Unmatched paths (src/controllers/not_found.rs): a real 404, the page for
//! a browser, one line for anything else, never cached, and limited like a
//! route.

use app::{app::App, controllers::parts, views::layout::APP_NAME};
use loco_rs::testing::prelude::*;
use serial_test::serial;

use super::prepare_data::WantsPage;

/// The parts' page and the held-back answer are the browser tests' alone
/// (src/controllers/parts.rs): anywhere else they are no page at all.
#[tokio::test]
#[serial]
async fn the_parts_page_is_the_browser_tests_alone() {
    request::<App, _, _>(|request, _ctx| async move {
        for path in [parts::PATH, parts::SLOW_PATH] {
            let res = request.get(path).await;
            assert_eq!(res.status_code(), 404, "{path}");
        }
    })
    .await;
}

#[tokio::test]
#[serial]
async fn unknown_path_is_a_404_page_for_a_browser_and_a_line_for_a_probe() {
    request::<App, _, _>(|request, _ctx| async move {
        let page = request
            .get("/nope")
            .add_header("accept", "text/html,application/xhtml+xml")
            .await;
        assert_eq!(page.status_code(), 404);
        assert_eq!(
            page.header("cache-control").to_str().expect("text"),
            "no-store",
            "a miss is never cached"
        );
        let body = page.text();
        assert!(body.contains("Page not found"), "{body}");
        assert!(
            body.contains(&format!("<title>Page not found | {APP_NAME}</title>")),
            "the page's name, then the app's, from the shell:\n{body}"
        );
        assert!(
            body.contains("id=\"content\"") && body.contains("/pkg/app.css"),
            "the page is inside the shell, with the stylesheet:\n{body}"
        );
        assert!(
            body.contains(r#"<meta name="robots" content="noindex">"#),
            "the not-found page says noindex:\n{body}"
        );

        // A probe that does not ask for HTML costs a string, not a render.
        let probe = request.get("/wp-login.php").await;
        assert_eq!(probe.status_code(), 404);
        assert_eq!(
            probe.header("cache-control").to_str().expect("text"),
            "no-store"
        );
        let body = probe.text();
        assert!(!body.contains("<html"), "no page for a probe:\n{body}");
    })
    .await;
}

/// The fallback sits inside Loco's middleware stack (`not_found::Site`,
/// first in `Hooks::middlewares`), so a miss gets what a route gets.
#[tokio::test]
#[serial]
async fn a_miss_gets_the_security_headers_and_a_request_id() {
    request::<App, _, _>(|request, _ctx| async move {
        for accept in ["text/html", "*/*"] {
            let res = request.get("/nope").add_header("accept", accept).await;
            assert_eq!(res.status_code(), 404, "{accept}");
            assert_eq!(
                res.header("x-frame-options").to_str().expect("text"),
                "DENY",
                "{accept}"
            );
            assert_eq!(
                res.header("x-content-type-options").to_str().expect("text"),
                "nosniff",
                "{accept}"
            );
            assert!(
                res.maybe_header("strict-transport-security").is_some(),
                "{accept}: HSTS"
            );
            assert!(
                res.maybe_header("x-request-id").is_some(),
                "{accept}: request id"
            );
        }
        // The page keeps its own nonce CSP: the fallback one is only added
        // where a response has none.
        let page = request.get("/nope").wants_page().await;
        assert!(
            page.header("content-security-policy")
                .to_str()
                .expect("text")
                .contains("'nonce-"),
            "the not-found page's own CSP"
        );
    })
    .await;
}

/// Loco's starter auth API stays unmounted (src/app.rs, `Hooks::routes`):
/// the site signs in with cookie sessions through its own pages, and the
/// configs carry no JWT key, so its addresses are misses and no token or
/// `api_key` opens anything. As an islands app on this stack keeps it out.
#[tokio::test]
#[serial]
async fn the_starters_auth_api_is_not_there() {
    request::<App, _, _>(|request, _ctx| async move {
        for path in [
            "/api/auth/register",
            "/api/auth/login",
            "/api/auth/forgot",
            "/api/auth/reset",
            "/api/auth/magic-link",
            "/api/auth/resend-verification-mail",
        ] {
            let res = request.post(path).json(&serde_json::json!({})).await;
            assert_eq!(res.status_code(), 404, "POST {path}");
        }
        for path in [
            "/api/auth/current",
            "/api/auth/verify/any-token",
            "/api/auth/magic-link/any-token",
        ] {
            let res = request
                .get(path)
                .add_header(
                    "authorization",
                    "Bearer lo-00000000-0000-0000-0000-000000000000",
                )
                .await;
            assert_eq!(res.status_code(), 404, "GET {path}");
        }
    })
    .await;
}

/// Misses have a bucket of their own with the site-wide numbers: the
/// site-wide limiter covers matched routes only.
#[tokio::test]
#[serial]
async fn misses_past_the_burst_are_refused() {
    request::<App, _, _>(|request, _ctx| async move {
        // The site-wide numbers from config/test.yaml: a burst of 20.
        for n in 1..=20 {
            let response = request.get(&format!("/missing-{n}")).await;
            assert_eq!(response.status_code(), 404, "miss {n} is answered");
        }
        let response = request.get("/missing-21").await;
        assert_eq!(response.status_code(), 429, "the 21st miss is refused");
        assert!(
            response.maybe_header("retry-after").is_some(),
            "retry-after on the 429"
        );
        // A bucket of its own: the routes still answer.
        assert_eq!(request.get("/robots.txt").await.status_code(), 200);
    })
    .await;
}
