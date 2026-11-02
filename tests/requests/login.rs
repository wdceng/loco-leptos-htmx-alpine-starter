//! The sign-in (src/controllers/login.rs), the way a person uses it: the
//! form, a refused try, the session's cookie, who is signed in, and the
//! sign-out.

use app::{app::App, models::sessions::Session, sql};
use loco_rs::{app::AppContext, testing::prelude::*};
use serial_test::serial;

use super::prepare_data::{self, PASSWORD, sign_in};

const EMAIL: &str = "signin@example.com";

/// A user with `email` who signed up, verified when `verified`: their
/// row's id.
async fn user(ctx: &AppContext, email: &str, verified: bool) -> i64 {
    prepare_data::user(ctx, email, verified).await.id
}

/// The session's secret from a sign-in's `Set-Cookie`.
fn secret_from(set_cookie: &str) -> String {
    set_cookie
        .strip_prefix("session=")
        .and_then(|rest| rest.split(';').next())
        .expect("the session cookie")
        .to_string()
}

#[tokio::test]
#[serial]
async fn the_form_signs_in_and_the_page_then_says_who() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = user(&ctx, EMAIL, true).await;

        let page = request.get("/login").await;
        assert_eq!(page.status_code(), 200);
        assert_eq!(page.header("cache-control"), "no-store");
        let body = page.text();
        for wanted in [
            r#"action="/login""#,
            r#"autocomplete="username""#,
            r#"autocomplete="current-password""#,
            r#"hx-sync="this:drop""#,
            // The ways on: a new password, an account, the mail again.
            r#"href="/forgot""#,
            r#"href="/register""#,
            r#"href="/verify""#,
        ] {
            assert!(body.contains(wanted), "{wanted}: {body}");
        }
        assert!(!body.contains("You're signed in"), "{body}");

        // Our own page's post, as a browser says it.
        let signed = request
            .post("/login")
            .add_header("sec-fetch-site", "same-origin")
            .form(&sign_in(EMAIL, PASSWORD))
            .await;
        assert_eq!(signed.status_code(), 303);
        assert_eq!(signed.header("location"), "/");
        let cookie = signed
            .header("set-cookie")
            .to_str()
            .expect("a cookie")
            .to_string();
        // Plain HTTP in the tests: no `Secure`, no `__Host-` prefix.
        assert_eq!(
            cookie,
            format!(
                "session={}; Path=/; Max-Age=43200; HttpOnly; SameSite=Lax",
                secret_from(&cookie)
            )
        );
        assert_eq!(
            Session::count(sql::pool(&ctx), user_id)
                .await
                .expect("counted"),
            1
        );

        let page = request
            .get("/login")
            .add_header("cookie", format!("session={}", secret_from(&cookie)))
            .await;
        assert_eq!(page.status_code(), 200);
        assert_eq!(page.header("cache-control"), "no-store");
        let body = page.text();
        assert!(body.contains("You're signed in"), "{body}");
        assert!(body.contains("As Ana."), "{body}");
        assert!(body.contains(r#"action="/logout""#), "{body}");
    })
    .await;
}

/// An unknown email, a wrong password and an unverified account get one
/// answer: nothing tells whether an address has an account.
#[tokio::test]
#[serial]
async fn every_refused_sign_in_answers_alike() {
    request::<App, _, _>(|request, ctx| async move {
        user(&ctx, EMAIL, true).await;
        let unverified = "unverified@example.com";
        let unverified_id = user(&ctx, unverified, false).await;

        for (email, password) in [
            ("nobody@example.com", PASSWORD),
            (EMAIL, "wrong-password"),
            (unverified, PASSWORD),
        ] {
            let refused = request.post("/login").form(&sign_in(email, password)).await;
            assert_eq!(refused.status_code(), 400, "{email}");
            assert!(refused.maybe_header("set-cookie").is_none(), "{email}");
            assert_eq!(refused.header("hx-push-url"), "false", "{email}");
            let body = refused.text();
            assert!(body.contains("<title>Error: Sign in"), "{email}: {body}");
            assert!(body.contains("match a verified account"), "{email}: {body}");
            assert!(body.contains(&format!(r#"value="{email}""#)), "{body}");
            assert!(body.contains(r#"aria-invalid="true""#), "{body}");
        }
        assert_eq!(
            Session::count(sql::pool(&ctx), unverified_id)
                .await
                .expect("counted"),
            0
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn signing_out_closes_the_session_and_takes_the_cookie() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = user(&ctx, EMAIL, true).await;
        let signed = request.post("/login").form(&sign_in(EMAIL, PASSWORD)).await;
        let secret = secret_from(signed.header("set-cookie").to_str().expect("a cookie"));
        let cookie = format!("session={secret}");

        let out = request
            .post("/logout")
            .add_header("cookie", cookie.clone())
            .await;
        assert_eq!(out.status_code(), 303);
        assert_eq!(out.header("location"), "/login");
        assert_eq!(
            out.header("set-cookie"),
            "session=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax"
        );
        assert_eq!(
            Session::count(sql::pool(&ctx), user_id)
                .await
                .expect("counted"),
            0
        );

        // The old cookie opens nothing now.
        let page = request.get("/login").add_header("cookie", cookie).await;
        assert!(!page.text().contains("You're signed in"));

        // Nobody signed in: the same way out.
        let out = request.post("/logout").await;
        assert_eq!(out.status_code(), 303);
        assert_eq!(out.header("location"), "/login");
    })
    .await;
}

/// Another site's page cannot sign a visitor in or out: the browser says
/// where the post came from (`Sec-Fetch-Site`).
#[tokio::test]
#[serial]
async fn another_sites_post_signs_nobody_in_or_out() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = user(&ctx, EMAIL, true).await;

        let refused = request
            .post("/login")
            .add_header("sec-fetch-site", "cross-site")
            .form(&sign_in(EMAIL, PASSWORD))
            .await;
        assert_eq!(refused.status_code(), 403);
        assert!(refused.maybe_header("set-cookie").is_none());
        assert_eq!(
            Session::count(sql::pool(&ctx), user_id)
                .await
                .expect("counted"),
            0
        );

        let signed = request.post("/login").form(&sign_in(EMAIL, PASSWORD)).await;
        let secret = secret_from(signed.header("set-cookie").to_str().expect("a cookie"));
        let refused = request
            .post("/logout")
            .add_header("sec-fetch-site", "cross-site")
            .add_header("cookie", format!("session={secret}"))
            .await;
        assert_eq!(refused.status_code(), 403);
        assert_eq!(
            Session::count(sql::pool(&ctx), user_id)
                .await
                .expect("counted"),
            1,
            "the session stays"
        );
    })
    .await;
}

/// The sign-in's post is on the auth bucket, `auth.burst: 10` in
/// config/test.yaml; visits to the page spend none of it.
#[tokio::test]
#[serial]
async fn the_eleventh_try_in_a_row_waits() {
    request::<App, _, _>(|request, _ctx| async move {
        for attempt in 1..=10 {
            let page = request.get("/login").await;
            assert_eq!(page.status_code(), 200, "visit {attempt}");
            let refused = request
                .post("/login")
                .form(&sign_in("nobody@example.com", "wrong-password"))
                .await;
            assert_eq!(refused.status_code(), 400, "try {attempt}");
        }
        let waits = request
            .post("/login")
            .form(&sign_in("nobody@example.com", "wrong-password"))
            .await;
        assert_eq!(waits.status_code(), 429);
        assert!(waits.maybe_header("retry-after").is_some());
    })
    .await;
}
