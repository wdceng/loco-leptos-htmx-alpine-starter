use app::app::App;
use axum::http::HeaderValue;
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// `config/test.yaml` sets `settings.rate_limit.burst` to this and keys on
/// the TCP peer, which the harness provides (it serves over a real loopback
/// port), so every request here shares the `127.0.0.1` bucket. All requests
/// finish in milliseconds, well inside the one-second refill.
const BURST: u32 = 20;

/// `settings.rate_limit.auth.burst` in `config/test.yaml`: the bucket on
/// the auth forms' posts alone, inside the site-wide one.
const AUTH_BURST: u32 = 10;

/// The bucket's state, never shown: a bot would read its tries left and
/// pace itself just under the limit.
const BUCKET_STATE: [&str; 2] = ["x-ratelimit-limit", "x-ratelimit-remaining"];

/// A `retry-after` value, in whole seconds.
fn retry_after_secs(value: &HeaderValue) -> u64 {
    value
        .to_str()
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("retry-after is not a number of seconds: {value:?}"))
}

#[tokio::test]
#[serial]
async fn request_past_the_burst_is_rejected_with_a_page() {
    request::<App, _, _>(|request, _ctx| async move {
        // Within the burst: allowed, and the bucket's state never shown.
        for _ in 0..BURST {
            let res = request.get("/robots.txt").await;
            assert_eq!(res.status_code(), 200);
            for told in BUCKET_STATE {
                assert!(
                    res.maybe_header(told).is_none(),
                    "{told} on an allowed answer"
                );
            }
        }

        // Bucket empty: 429 with the rendered page and the retry headers.
        let res = request.get("/robots.txt").await;
        assert_eq!(res.status_code(), 429);
        // The bucket refills once a second, which tower_governor rounds down
        // to 0; `Retry-After: 0` would send the client straight back.
        let retry_after = res.header("retry-after");
        assert_eq!(
            retry_after_secs(&retry_after),
            1,
            "a wait under a second is 1, never 0"
        );
        assert_eq!(res.header("x-ratelimit-after"), retry_after);
        for told in BUCKET_STATE {
            assert!(res.maybe_header(told).is_none(), "{told} on the 429");
        }
        assert_eq!(res.header("cache-control"), "no-store");
        assert!(
            res.header("content-type")
                .to_str()
                .unwrap_or("")
                .starts_with("text/html"),
            "429 must be an HTML page"
        );
        // Where the limiter sits in the stack (`Hooks::middlewares` in
        // src/app.rs): inside `secure_headers` and `request_id`. So a 429
        // carries the security headers, the strict CSP for non-page
        // responses and a request id.
        assert_eq!(res.header("x-frame-options"), "DENY");
        assert_eq!(res.header("x-content-type-options"), "nosniff");
        assert!(
            res.maybe_header("strict-transport-security").is_some(),
            "HSTS on the 429"
        );
        assert!(
            res.header("content-security-policy")
                .to_str()
                .unwrap_or("")
                .starts_with("default-src 'none';"),
            "the strict CSP on the 429"
        );
        assert!(
            res.maybe_header("x-request-id").is_some(),
            "request id on the 429"
        );
        let body = res.text();
        assert!(
            body.starts_with("<!DOCTYPE html>"),
            "not a document:\n{body}"
        );
        assert!(body.contains("Too many requests"));
        assert!(
            !body.contains("{wait}") && !body.contains("{stylesheet}"),
            "placeholder not replaced:\n{body}"
        );
        // The stylesheet resolved at boot (plain here, hashed in a release
        // deploy), and the wait in words.
        assert!(
            body.contains(r#"href="/pkg/app.css""#),
            "stylesheet link:\n{body}"
        );
        assert!(body.contains("in 1 second."), "the wait:\n{body}");

        // Unmatched paths have a bucket of their own
        // (controllers/not_found.rs), untouched by the routes: still 404.
        let res = request.get("/nope").await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}

/// The auth forms' posts share a second, stricter bucket
/// (`rate_limit::auth_limited`): the sign-in takes a password, the
/// sign-up, the forgotten password and a new welcome mail mail any address.
/// A post there spends a token from both buckets.
#[tokio::test]
#[serial]
async fn the_auth_forms_share_one_stricter_bucket() {
    request::<App, _, _>(|request, _ctx| async move {
        let email = "nobody@example.com";
        let posts = [
            (
                "/login",
                serde_json::json!({ "email": email, "password": "wrong" }),
            ),
            (
                "/register",
                serde_json::json!({ "name": "loco", "email": email, "password": "short" }),
            ),
            ("/forgot", serde_json::json!({ "email": email })),
            ("/verify/resend", serde_json::json!({ "email": email })),
        ];

        // Within the auth burst, the four forms taking turns: each answered
        // as itself, not as too many, and neither bucket's state shown.
        for attempt in 0..AUTH_BURST as usize {
            let (path, form) = &posts[attempt % posts.len()];
            let res = request.post(path).form(form).await;
            assert_ne!(res.status_code(), 429, "{path}, post {attempt}");
            for told in BUCKET_STATE {
                assert!(
                    res.maybe_header(told).is_none(),
                    "{told} on {path}, post {attempt}"
                );
            }
        }

        // The shared bucket is empty for every one of them: the 429 page,
        // with the wait until its next token.
        for (path, form) in &posts {
            let res = request.post(path).form(form).await;
            assert_eq!(res.status_code(), 429, "{path}");
        }
        let (path, form) = &posts[0];
        let res = request.post(path).form(form).await;
        assert_eq!(res.status_code(), 429);
        // The wait is the auth bucket's, one token per 30 s, not the
        // site-wide bucket's second.
        let wait = retry_after_secs(&res.header("retry-after"));
        assert!(
            (2..=31).contains(&wait),
            "retry-after {wait} is not the auth bucket's wait"
        );
        assert_eq!(res.header("cache-control"), "no-store");
        assert!(res.text().contains("Too many requests"));

        // The site-wide bucket still has tokens (twenty, minus the fifteen
        // auth posts and this one), so everything else still answers.
        let res = request.get("/robots.txt").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}
