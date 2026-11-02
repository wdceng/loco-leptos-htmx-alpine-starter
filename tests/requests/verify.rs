//! The `/verify` page (src/controllers/verify.rs), the way a person uses
//! it: the welcome mail's link, the button, the address verified, and the
//! link spent. Opening the link verifies nothing: a mail scanner opens
//! every link (docs/security.md, the review of 2026-10-31, #1).

use app::{app::App, models::users};
use chrono::{Duration, offset::Local};
use loco_rs::{app::AppContext, testing::prelude::*};
use sea_orm::{ActiveModelTrait, ActiveValue, IntoActiveModel};
use serial_test::serial;

use super::prepare_data::{PASSWORD, signs_in, user, without_nonces};

/// A user who signed up through the page and was mailed a link: its token.
async fn signed_up(request: &loco_rs::TestServer, ctx: &AppContext, email: &str) -> String {
    request
        .post("/register")
        .form(&serde_json::json!({ "name": "new", "email": email, "password": PASSWORD }))
        .await;
    users::Model::find_by_email(&ctx.db, email)
        .await
        .expect("registered")
        .email_verification_token
        .expect("a verification token")
}

#[tokio::test]
#[serial]
async fn the_verify_page_verifies_on_its_button_alone() {
    request::<App, _, _>(|request, ctx| async move {
        let email = "new@example.com";
        let token = signed_up(&request, &ctx, email).await;

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
        assert!(
            !signs_in(&request, email, PASSWORD).await,
            "unverified, no sign-in"
        );

        // The press: verified, and the account signs in.
        let done = request
            .post("/verify")
            .form(&serde_json::json!({ "token": token }))
            .await;
        assert_eq!(done.status_code(), 200);
        // A page answered to a post keeps the address the browser is at.
        assert_eq!(done.header("hx-push-url"), "false");
        assert!(done.text().contains("Your email is verified"));
        assert!(
            signs_in(&request, email, PASSWORD).await,
            "verified, signed in"
        );

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

/// Another site's page cannot press the button for the visitor.
#[tokio::test]
#[serial]
async fn another_sites_post_verifies_nothing() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request
            .post("/verify")
            .add_header("sec-fetch-site", "cross-site")
            .form(&serde_json::json!({ "token": "no-such-token" }))
            .await;
        assert_eq!(res.status_code(), 403);
    })
    .await;
}

/// `/verify` without a token: the form that sends the welcome mail again.
/// A new mail carries a fresh link and the old one stops working; a mail
/// whose day is over is answered by a new one.
#[tokio::test]
#[serial]
async fn the_mail_comes_again_with_a_fresh_link() {
    request::<App, _, _>(|request, ctx| async move {
        let email = "late@example.com";
        let old_token = signed_up(&request, &ctx, email).await;

        let page = request.get("/verify").await;
        assert_eq!(page.status_code(), 200);
        assert_eq!(page.header("cache-control"), "no-store");
        let body = page.text();
        for wanted in [
            r#"action="/verify/resend""#,
            r#"autocomplete="username""#,
            r#"hx-sync="this:drop""#,
            r#"href="/login""#,
        ] {
            assert!(body.contains(wanted), "{wanted}: {body}");
        }

        // The link's day is over (`users::EMAIL_VERIFICATION_EXPIRATION_HOURS`).
        let user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("registered");
        let mut sent_a_day_ago = user.into_active_model();
        sent_a_day_ago.email_verification_sent_at = ActiveValue::Set(Some(
            (Local::now()
                - Duration::hours(users::EMAIL_VERIFICATION_EXPIRATION_HOURS)
                - Duration::minutes(1))
            .into(),
        ));
        sent_a_day_ago.update(&ctx.db).await.expect("updated");
        let late = request
            .post("/verify")
            .form(&serde_json::json!({ "token": old_token }))
            .await;
        assert_eq!(late.status_code(), 400, "a day and a minute late");

        let sent = request
            .post("/verify/resend")
            .add_header("sec-fetch-site", "same-origin")
            .form(&serde_json::json!({ "email": email }))
            .await;
        assert_eq!(sent.status_code(), 200);
        assert_eq!(sent.header("hx-push-url"), "false");
        assert!(sent.text().contains("Check your mail"));
        assert_eq!(
            ctx.mailer.expect("stub mailer").deliveries().count,
            2,
            "the welcome mail and the new one"
        );

        let new_token = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("registered")
            .email_verification_token
            .expect("a new token");
        assert_ne!(new_token, old_token);
        let fresh = request
            .post("/verify")
            .form(&serde_json::json!({ "token": new_token }))
            .await;
        assert_eq!(fresh.status_code(), 200, "the new link");
        assert!(signs_in(&request, email, PASSWORD).await, "verified");
    })
    .await;
}

/// A verified account and an address with no account get the page a
/// waiting one does, and no mail.
#[tokio::test]
#[serial]
async fn a_verified_or_unknown_address_is_answered_alike() {
    request::<App, _, _>(|request, ctx| async move {
        user(&ctx, "waiting@example.com", false).await;
        user(&ctx, "verified@example.com", true).await;
        let mut answers = Vec::new();
        for email in [
            "waiting@example.com",
            "verified@example.com",
            "nobody@example.com",
        ] {
            let res = request
                .post("/verify/resend")
                .form(&serde_json::json!({ "email": email }))
                .await;
            assert_eq!(res.status_code(), 200, "{email}");
            answers.push(without_nonces(&res.text()));
        }
        assert!(answers.windows(2).all(|pair| pair[0] == pair[1]));
        assert_eq!(
            ctx.mailer.expect("stub mailer").deliveries().count,
            1,
            "a mail for the waiting account alone"
        );
    })
    .await;
}

/// Another site's page cannot send an account's mail again.
#[tokio::test]
#[serial]
async fn another_sites_post_resends_nothing() {
    request::<App, _, _>(|request, ctx| async move {
        user(&ctx, "waiting@example.com", false).await;
        let res = request
            .post("/verify/resend")
            .add_header("sec-fetch-site", "cross-site")
            .form(&serde_json::json!({ "email": "waiting@example.com" }))
            .await;
        assert_eq!(res.status_code(), 403);
        assert_eq!(ctx.mailer.expect("stub mailer").deliveries().count, 0);
    })
    .await;
}
