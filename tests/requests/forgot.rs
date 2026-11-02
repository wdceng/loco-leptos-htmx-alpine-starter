//! The `/forgot` page (src/controllers/forgot.rs), the way a person uses
//! it: the form, the reset link mailed, and an unknown address answered
//! alike.

use app::{app::App, models::users};
use loco_rs::testing::prelude::*;
use serial_test::serial;

use super::{
    mails::{assert_from_the_configured_sender, unfolded},
    prepare_data::{user, without_nonces},
};

#[tokio::test]
#[serial]
async fn the_form_mails_a_link_to_the_reset_page() {
    request::<App, _, _>(|request, ctx| async move {
        let email = "forgot@example.com";
        user(&ctx, email, true).await;

        let page = request.get("/forgot").await;
        assert_eq!(page.status_code(), 200);
        assert_eq!(page.header("cache-control"), "no-store");
        let body = page.text();
        for wanted in [
            r#"action="/forgot""#,
            r#"autocomplete="username""#,
            r#"hx-sync="this:drop""#,
            r#"x-bind:aria-busy="sending""#,
            r#"href="/login""#,
        ] {
            assert!(body.contains(wanted), "{wanted}: {body}");
        }

        let sent = request
            .post("/forgot")
            .add_header("sec-fetch-site", "same-origin")
            .form(&serde_json::json!({ "email": email }))
            .await;
        assert_eq!(sent.status_code(), 200);
        assert_eq!(sent.header("hx-push-url"), "false");
        assert!(sent.text().contains("Check your mail"));

        let user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("the user");
        let token = user.reset_token.expect("a reset token");
        assert!(user.reset_sent_at.is_some());

        let deliveries = ctx.mailer.expect("stub mailer").deliveries();
        assert_eq!(deliveries.count, 1, "the reset mail");
        // To the `/reset` page (controllers/reset.rs), from `server.host`.
        let mail = unfolded(&deliveries.messages[0]);
        assert_from_the_configured_sender(&mail);
        assert!(
            mail.contains(&format!("http://localhost:5150/reset?token={token}")),
            "the link does not start with server.host:\n{mail}"
        );
    })
    .await;
}

/// An address with no account gets the page an account's does, and no
/// mail goes anywhere.
#[tokio::test]
#[serial]
async fn an_unknown_address_is_answered_alike() {
    request::<App, _, _>(|request, ctx| async move {
        user(&ctx, "known@example.com", true).await;
        let known = request
            .post("/forgot")
            .form(&serde_json::json!({ "email": "known@example.com" }))
            .await;
        let unknown = request
            .post("/forgot")
            .form(&serde_json::json!({ "email": "nobody@example.com" }))
            .await;
        assert_eq!(known.status_code(), unknown.status_code());
        assert_eq!(
            without_nonces(&known.text()),
            without_nonces(&unknown.text())
        );
        assert_eq!(
            ctx.mailer.expect("stub mailer").deliveries().count,
            1,
            "a mail for the account alone"
        );
    })
    .await;
}

/// Another site's page cannot send an account's reset mail.
#[tokio::test]
#[serial]
async fn another_sites_post_mails_nothing() {
    request::<App, _, _>(|request, ctx| async move {
        user(&ctx, "known@example.com", true).await;
        let res = request
            .post("/forgot")
            .add_header("sec-fetch-site", "cross-site")
            .form(&serde_json::json!({ "email": "known@example.com" }))
            .await;
        assert_eq!(res.status_code(), 403);
        assert_eq!(ctx.mailer.expect("stub mailer").deliveries().count, 0);
    })
    .await;
}
