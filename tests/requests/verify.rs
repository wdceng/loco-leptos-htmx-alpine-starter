//! The `/verify` page (src/controllers/verify.rs), the way a person uses
//! it: the welcome mail's link, the button, the address verified, and the
//! link spent. Opening the link verifies nothing: a mail scanner opens
//! every link (docs/security.md, the review of 2026-10-31, #1).

use app::{app::App, models::users};
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn the_verify_page_verifies_on_its_button_alone() {
    request::<App, _, _>(|request, ctx| async move {
        let email = "new@example.com";
        let password = "12341234";
        request
            .post("/api/auth/register")
            .json(&serde_json::json!({ "name": "new", "email": email, "password": password }))
            .await;
        let token = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("registered")
            .email_verification_token
            .expect("a verification token");

        // The mail's link, opened as a scanner would: the button, the token
        // in it, the address kept out of referrers, caches and searches,
        // and nothing verified.
        let page = request.get(&format!("/verify?token={token}")).await;
        assert_eq!(page.status_code(), 200);
        assert_eq!(page.header("cache-control"), "no-store");
        assert_eq!(page.header("referrer-policy"), "no-referrer");
        let body = page.text();
        for wanted in [
            r#"action="/verify""#,
            &format!(r#"value="{token}""#),
            r#"name="robots" content="noindex""#,
            r#"hx-sync="this:drop""#,
            r#"x-bind:aria-busy="sending""#,
            "busy:delay-200",
            "Verify my email",
        ] {
            assert!(body.contains(wanted), "{wanted}: {body}");
        }
        let user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("registered");
        assert!(
            user.email_verified_at.is_none(),
            "opening the link verified the account"
        );
        let login = serde_json::json!({ "email": email, "password": password });
        let refused = request.post("/api/auth/login").json(&login).await;
        assert_eq!(refused.status_code(), 401, "unverified, no sign-in");

        // The press: verified, and the account signs in.
        let done = request
            .post("/verify")
            .form(&serde_json::json!({ "token": token }))
            .await;
        assert_eq!(done.status_code(), 200);
        // A page answered to a post keeps the address the browser is at.
        assert_eq!(done.header("hx-push-url"), "false");
        assert!(done.text().contains("Your email is verified"));
        let signed_in = request.post("/api/auth/login").json(&login).await;
        assert_eq!(signed_in.status_code(), 200, "verified, signed in");

        // The link is spent.
        let again = request
            .post("/verify")
            .form(&serde_json::json!({ "token": token }))
            .await;
        assert_eq!(again.status_code(), 400);
        assert!(again.text().contains("This link no longer works"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn an_unknown_or_missing_token_is_a_link_that_no_longer_works() {
    request::<App, _, _>(|request, _ctx| async move {
        for form in [
            serde_json::json!({ "token": "no-such-token" }),
            serde_json::json!({}),
        ] {
            let res = request.post("/verify").form(&form).await;
            assert_eq!(res.status_code(), 400, "{form}");
            assert_eq!(res.header("cache-control"), "no-store", "{form}");
            assert!(res.text().contains("This link no longer works"), "{form}");
        }
    })
    .await;
}
