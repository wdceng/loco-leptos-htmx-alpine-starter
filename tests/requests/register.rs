//! The `/register` page (src/controllers/register.rs), the way a person
//! uses it: the form, a refused field, the account made and its welcome
//! mail, and a taken address answered like a new one.

use app::{app::App, models::users, views::layout::APP_NAME};
use loco_rs::testing::prelude::*;
use serial_test::serial;

use super::{
    mails::{assert_from_the_configured_sender, html_part, subject},
    prepare_data::{signs_in, without_nonces},
};

fn sign_up(name: &str, email: &str, password: &str) -> serde_json::Value {
    serde_json::json!({ "name": name, "email": email, "password": password })
}

#[tokio::test]
#[serial]
async fn the_form_makes_an_account_and_mails_its_link() {
    request::<App, _, _>(|request, ctx| async move {
        let page = request.get("/register").await;
        assert_eq!(page.status_code(), 200);
        assert_eq!(page.header("cache-control"), "no-store");
        let body = page.text();
        for wanted in [
            r#"action="/register""#,
            r#"autocomplete="name""#,
            r#"autocomplete="email""#,
            r#"autocomplete="new-password""#,
            r#"minlength="8""#,
            r#"maxlength="128""#,
            r#"hx-sync="this:drop""#,
            r#"x-bind:aria-busy="sending""#,
            r#"href="/login""#,
        ] {
            assert!(body.contains(wanted), "{wanted}: {body}");
        }
        assert!(!body.contains("<title>Error: "), "{body}");

        let email = "new@example.com";
        let done = request
            .post("/register")
            .add_header("sec-fetch-site", "same-origin")
            .form(&sign_up("loco", email, "12341234"))
            .await;
        assert_eq!(done.status_code(), 200);
        assert_eq!(done.header("cache-control"), "no-store");
        // A page answered to a post keeps the address the browser is at.
        assert_eq!(done.header("hx-push-url"), "false");
        assert!(done.text().contains("Check your mail"));

        let user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("the account");
        assert_eq!(user.name, "loco");
        assert!(user.email_verified_at.is_none(), "not verified yet");
        let token = user.email_verification_token.expect("a verification token");
        assert!(
            !signs_in(&request, email, "12341234").await,
            "unverified, no sign-in"
        );

        let deliveries = ctx.mailer.expect("stub mailer").deliveries();
        assert_eq!(deliveries.count, 1, "the welcome mail");
        // The link starts with `server.host` as configured, port included,
        // never `host:port` (src/mailers/auth.rs), and leads to the page
        // whose button verifies (controllers/verify.rs).
        let mail = &deliveries.messages[0];
        assert_from_the_configured_sender(mail);
        assert!(
            mail.contains(&format!("http://localhost:5150/verify?token={token}")),
            "the link does not start with server.host:\n{mail}"
        );
    })
    .await;
}

/// 8 to 128 characters (`users::PASSWORD_MIN`, `PASSWORD_MAX`), checked
/// before anything else.
#[tokio::test]
#[serial]
async fn passwords_outside_8_to_128_characters_are_refused() {
    request::<App, _, _>(|request, ctx| async move {
        for (password, status) in [
            ("a".repeat(7), 400),
            ("a".repeat(129), 400),
            ("a".repeat(8), 200),
            ("a".repeat(128), 200),
        ] {
            let email = format!("p{}@example.com", password.len());
            let res = request
                .post("/register")
                .form(&sign_up("loco", &email, &password))
                .await;
            assert_eq!(res.status_code(), status, "{} characters", password.len());
            let exists = users::Model::find_by_email(&ctx.db, &email).await.is_ok();
            assert_eq!(exists, status == 200, "{} characters", password.len());
        }
    })
    .await;
}

/// The name's 2 to 100 characters and the email's form (`users::Validator`)
/// are refused with their reason, the form again with what was typed, its
/// title starting "Error: ". Checked before anything else like the
/// password's, so the answer says nothing about who exists. They register
/// nobody and send nothing.
#[tokio::test]
#[serial]
async fn a_bad_name_or_email_is_refused_with_its_reason() {
    request::<App, _, _>(|request, ctx| async move {
        for (name, email, reason) in [
            ("a".repeat(101), "long@example.com", "Name must be"),
            ("a".to_owned(), "short@example.com", "Name must be"),
            ("loco".to_owned(), "not-an-email", "invalid email"),
        ] {
            let res = request
                .post("/register")
                .form(&sign_up(&name, email, "12341234"))
                .await;
            assert_eq!(res.status_code(), 400, "{email}");
            let body = res.text();
            for wanted in [
                reason,
                "<title>Error: Sign up | ",
                r#"id="sign-up-problem" role="alert""#,
                &format!(r#"value="{email}""#),
            ] {
                assert!(body.contains(wanted), "{email}: {wanted}: {body}");
            }
            assert!(
                users::Model::find_by_email(&ctx.db, email).await.is_err(),
                "{email}: no user was created"
            );
        }
        assert_eq!(
            ctx.mailer.expect("stub mailer").deliveries().count,
            0,
            "no mail was sent"
        );
    })
    .await;
}

/// A taken email answers like a new one, in whatever case it is typed: the
/// same status and page, no second account, and no second mail.
#[tokio::test]
#[serial]
async fn a_taken_address_signs_up_like_a_new_one() {
    request::<App, _, _>(|request, ctx| async move {
        let first = request
            .post("/register")
            .form(&sign_up("loco", "Mixed.Case@Example.com", "12341234"))
            .await;
        let second = request
            .post("/register")
            .form(&sign_up("other", "MIXED.CASE@example.com", "abcdefgh"))
            .await;
        assert_eq!(first.status_code(), second.status_code());
        assert_eq!(
            without_nonces(&first.text()),
            without_nonces(&second.text())
        );
        assert_eq!(
            ctx.mailer.expect("stub mailer").deliveries().count,
            1,
            "one welcome mail, for the new account only"
        );

        // One mailbox, one account: stored lowercase, found in any case.
        let user = users::Model::find_by_email(&ctx.db, "MIXED.case@example.COM")
            .await
            .expect("found in any case");
        assert_eq!(user.email, "mixed.case@example.com", "stored lowercase");
        assert_eq!(user.name, "loco", "the second sign-up changed nothing");
    })
    .await;
}

/// The name is typed by whoever signs up, and the welcome mail goes to any
/// address they give: markup in it arrives as text, and the subject is the
/// app's, never the name.
#[tokio::test]
#[serial]
async fn the_welcome_mail_shows_the_name_as_text_and_leaves_it_out_of_the_subject() {
    request::<App, _, _>(|request, ctx| async move {
        let name = "<b>Ana</b> <a href=\"https://evil.example\">click</a>";
        let res = request
            .post("/register")
            .form(&sign_up(name, "ana@example.com", "12341234"))
            .await;
        assert_eq!(res.status_code(), 200);

        let mails = ctx.mailer.expect("stub mailer").deliveries().messages;
        assert_eq!(mails.len(), 1, "the welcome mail");
        assert_eq!(
            subject(&mails[0]),
            format!("Subject: Welcome to {APP_NAME}")
        );
        let html = html_part(&mails[0]);
        assert!(
            html.contains("&lt;b&gt;Ana&lt;&#x2F;b&gt;") || html.contains("&lt;b&gt;Ana&lt;/b&gt;"),
            "the markup is shown as text:\n{html}"
        );
        for raw in ["<b>Ana", "<a href=\"https://evil.example\""] {
            assert!(!html.contains(raw), "raw `{raw}` in the HTML part:\n{html}");
        }
    })
    .await;
}

/// The body limit (`limit_payload`, 64 KB in every config): a larger body is
/// refused before any handler parses it.
#[tokio::test]
#[serial]
async fn a_body_past_the_limit_is_refused() {
    request::<App, _, _>(|request, ctx| async move {
        let res = request
            .post("/register")
            .form(&sign_up("loco", "big@example.com", &"a".repeat(70_000)))
            .await;
        assert_eq!(res.status_code(), 413, "a body past 64 KB");
        assert!(
            users::Model::find_by_email(&ctx.db, "big@example.com")
                .await
                .is_err()
        );
    })
    .await;
}

/// Another site's page cannot sign anyone up, nor mail an address through
/// the site.
#[tokio::test]
#[serial]
async fn another_sites_post_signs_nobody_up() {
    request::<App, _, _>(|request, ctx| async move {
        let res = request
            .post("/register")
            .add_header("sec-fetch-site", "cross-site")
            .form(&sign_up("loco", "victim@example.com", "12341234"))
            .await;
        assert_eq!(res.status_code(), 403);
        assert!(
            users::Model::find_by_email(&ctx.db, "victim@example.com")
                .await
                .is_err()
        );
        assert_eq!(ctx.mailer.expect("stub mailer").deliveries().count, 0);
    })
    .await;
}
