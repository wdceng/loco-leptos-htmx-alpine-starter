//! Canaries over every route the app mounts, read from its own table
//! (`Hooks::routes`), so a route added later is checked the day it is,
//! with nobody having to remember a test for it: every account page asks
//! for a signed-in person (`Signed`, controllers/signed.rs), and every POST
//! refuses another site's page (controllers/cross_site.rs).
//!
//! The routes are served without the site-wide middleware, each request
//! from a visitor of its own ([`server`]): the site's bucket would answer
//! 429 before the walk ends, and what is checked here is the handlers' own.
//! The forms' own buckets stay, being the routes' (`rate_limit::auth_limited`).

use std::{
    net::SocketAddr,
    sync::atomic::{AtomicU32, Ordering},
};

use app::{app::App, controllers::account};
use axum::{
    Router,
    extract::{ConnectInfo, Request},
    http::{Method, StatusCode},
    middleware::{Next, from_fn},
};
use loco_rs::{
    TestServer,
    app::{AppContext, Hooks},
    testing::prelude::*,
};
use serial_test::serial;

use super::prepare_data::{WantsPage, assert_unique_ids, session_cookie, user};

/// The account pages anyone may open: the line after an account's
/// deletion, whose session is gone with it.
const OPEN: [&str; 1] = [account::DELETED];

/// The next visitor's address, one a request.
static VISITOR: AtomicU32 = AtomicU32::new(1);

/// Every route the app mounts, its placeholders filled (`{id}` as `1`),
/// with its methods.
fn routes(ctx: &AppContext) -> Vec<(String, Vec<Method>)> {
    App::routes(ctx)
        .collect()
        .into_iter()
        .map(|route| {
            let uri = route
                .uri
                .split('/')
                .map(|part| if part.starts_with('{') { "1" } else { part })
                .collect::<Vec<_>>()
                .join("/");
            (uri, route.actions)
        })
        .collect()
}

/// The app's routes alone, without the site-wide middleware, each request
/// from a visitor of its own (the module doc).
fn server(ctx: &AppContext) -> TestServer {
    let mut router = Router::new();
    for route in App::routes(ctx).collect() {
        router = router.route(&route.uri, route.method);
    }
    let router =
        router
            .with_state(ctx.clone())
            .layer(from_fn(|mut req: Request, next: Next| async move {
                let [_, _, high, low] = VISITOR.fetch_add(1, Ordering::Relaxed).to_be_bytes();
                req.extensions_mut()
                    .insert(ConnectInfo(SocketAddr::from(([10, 9, high, low], 4321))));
                next.run(req).await
            }));
    TestServer::new(router).expect("the test server")
}

#[tokio::test]
#[serial]
async fn every_account_page_asks_a_signed_in_person() {
    request::<App, _, _>(|_request, ctx| async move {
        let server = server(&ctx);
        let mut checked = 0;
        for (uri, methods) in routes(&ctx) {
            let signed = uri == account::PATH || uri.starts_with(&format!("{}/", account::PATH));
            if !signed || OPEN.contains(&uri.as_str()) {
                continue;
            }
            for method in methods {
                let res = server
                    .method(method.clone(), &uri)
                    .wants_page()
                    .await;
                let to = res
                    .maybe_header("location")
                    .map(|to| to.to_str().unwrap_or_default().to_string());
                assert!(
                    res.status_code() == StatusCode::SEE_OTHER && to.as_deref() == Some("/login"),
                    "{method} {uri} signed out: {} {to:?}, not the sign-in",
                    res.status_code()
                );
                checked += 1;
            }
        }
        // The table read as expected: the page and its five posts
        // (2026-11-12), not none.
        assert!(checked >= 6, "only {checked} pages checked");
    })
    .await;
}

/// No page has an id twice, signed out and signed in: a second one makes a
/// label, an `aria-labelledby` or a dialog's button find the first.
#[tokio::test]
#[serial]
async fn no_page_has_an_id_twice() {
    request::<App, _, _>(|request, ctx| async move {
        let email = user(&ctx, "ids@example.com", true).await.email;
        let cookie = session_cookie(&request, &email, "Firefox on a Mac").await;
        let server = server(&ctx);
        let mut checked = 0;
        for (uri, methods) in routes(&ctx) {
            if !methods.contains(&Method::GET) {
                continue;
            }
            for signed_in in [false, true] {
                let mut ask = server.get(&uri).wants_page();
                if signed_in {
                    ask = ask.add_header("cookie", cookie.clone());
                }
                let res = ask.await;
                let page = res
                    .maybe_header("content-type")
                    .is_some_and(|kind| kind.to_str().is_ok_and(|kind| kind.starts_with("text/html")));
                if page {
                    assert_unique_ids(&res.text());
                    checked += 1;
                }
            }
        }
        // Every page, not none.
        assert!(checked >= 10, "only {checked} pages checked");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn every_post_refuses_another_sites_page() {
    request::<App, _, _>(|request, ctx| async move {
        // Signed in, so a post behind `Signed` reaches the handler's own
        // first line.
        let email = user(&ctx, "canary@example.com", true).await.email;
        let cookie = session_cookie(&request, &email, "Firefox on a Mac").await;
        let server = server(&ctx);
        let mut checked = 0;
        for (uri, methods) in routes(&ctx) {
            if !methods.contains(&Method::POST) {
                continue;
            }
            // The body kind the route reads: a form's, or a photo's
            // multipart, whose extractor refuses a form before the
            // handler runs.
            let mut statuses = Vec::new();
            for kind in [
                "application/x-www-form-urlencoded",
                "multipart/form-data; boundary=canary",
            ] {
                let body = if kind.starts_with("multipart") {
                    "--canary--\r\n"
                } else {
                    ""
                };
                let res = server
                    .post(&uri)
                    .wants_page()
                    .add_header("cookie", cookie.clone())
                    .add_header("sec-fetch-site", "cross-site")
                    // After the body, whose own type it replaces.
                    .text(body)
                    .content_type(kind)
                    .await;
                let start: String = res.text().chars().take(40).collect();
                statuses.push((res.status_code(), start));
            }
            // `cross_site::refused`'s own answer, a plain 403, for the kind
            // the route reads: the handler checks it first (no extractor
            // does), so the other kind is its body extractor's 4xx, and
            // neither gets through.
            assert!(
                statuses
                    .iter()
                    .any(|(status, body)| *status == StatusCode::FORBIDDEN && body == "Forbidden")
                    && statuses.iter().all(|(status, _)| status.is_client_error()),
                "POST {uri} from another site's page: {statuses:?}, not refused as cross-site"
            );
            checked += 1;
        }
        // Every form's target (14 on 2026-11-12), not none.
        assert!(checked >= 14, "only {checked} posts checked");
    })
    .await;
}
