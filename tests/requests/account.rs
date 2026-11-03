//! The account page (src/controllers/account.rs), the way a person uses
//! it: their browsers, each signed out on its own or all but this one, a
//! new password, and the account deleted.

use app::{
    app::App,
    models::{sessions::Session, users},
    sql,
};
use chrono::{Duration, Utc};
use loco_rs::testing::prelude::*;
use serial_test::serial;

use super::prepare_data::{PASSWORD, session_cookie, signs_in, user};

const EMAIL: &str = "account@example.com";
const MAC: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:140.0) Gecko/20100101 Firefox/140.0";
const IPHONE: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";

/// The session id behind `cookie` (`session=…`).
async fn session_id(ctx: &loco_rs::app::AppContext, cookie: &str) -> i64 {
    let secret = cookie.strip_prefix("session=").expect("the session cookie");
    Session::find(sql::pool(ctx), secret)
        .await
        .expect("read")
        .expect("live")
        .id
}

#[tokio::test]
#[serial]
async fn nobody_signed_in_is_sent_to_the_sign_in() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request
            .get("/account")
            .add_header("accept", "text/html")
            .await;
        assert_eq!(res.status_code(), 303);
        assert_eq!(res.header("location"), "/login");
    })
    .await;
}

/// The page lists where the user is signed in, this browser marked; one
/// other signed out by its button, then every other.
#[tokio::test]
#[serial]
async fn the_browsers_are_listed_and_signed_out() {
    request::<App, _, _>(|request, ctx| async move {
        let user = user(&ctx, EMAIL, true).await;
        let here = session_cookie(&request, EMAIL, MAC).await;
        let phone = session_cookie(&request, EMAIL, IPHONE).await;
        let third = session_cookie(&request, EMAIL, MAC).await;

        let page = request
            .get("/account")
            .add_header("cookie", here.clone())
            .await;
        assert_eq!(page.status_code(), 200);
        assert_eq!(page.header("cache-control"), "no-store");
        let body = page.text();
        for wanted in [
            "Firefox on a Mac (this browser)",
            "Safari on an iPhone",
            r#"name="robots" content="noindex""#,
            r#"action="/account/browsers/close-others""#,
            r#"action="/logout""#,
        ] {
            assert!(body.contains(wanted), "{wanted}: {body}");
        }

        let phone_id = session_id(&ctx, &phone).await;
        let closed = request
            .post(&format!("/account/browsers/{phone_id}/close"))
            .add_header("cookie", here.clone())
            .await;
        assert_eq!(closed.status_code(), 303);
        assert_eq!(closed.header("location"), "/account");
        let pool = sql::pool(&ctx);
        assert_eq!(Session::count(pool, user.id).await.expect("counted"), 2);

        let rest = request
            .post("/account/browsers/close-others")
            .add_header("cookie", here.clone())
            .await;
        assert_eq!(rest.status_code(), 303);
        assert_eq!(Session::count(pool, user.id).await.expect("counted"), 1);
        let gone = request
            .get("/account")
            .add_header("cookie", third)
            .add_header("accept", "text/html")
            .await;
        assert_eq!(gone.status_code(), 303, "the third browser is signed out");
    })
    .await;
}

/// Another user's session id closes nothing.
#[tokio::test]
#[serial]
async fn another_users_browser_is_not_yours_to_sign_out() {
    request::<App, _, _>(|request, ctx| async move {
        user(&ctx, EMAIL, true).await;
        user(&ctx, "other@example.com", true).await;
        let mine = session_cookie(&request, EMAIL, MAC).await;
        let theirs = session_cookie(&request, "other@example.com", MAC).await;
        let theirs_id = session_id(&ctx, &theirs).await;

        request
            .post(&format!("/account/browsers/{theirs_id}/close"))
            .add_header("cookie", mine)
            .await;
        let still = request.get("/account").add_header("cookie", theirs).await;
        assert_eq!(still.status_code(), 200, "their session stays");
    })
    .await;
}

/// A new password needs the current one and the rule; it signs out every
/// other browser and is the one that signs in.
#[tokio::test]
#[serial]
async fn a_new_password_needs_the_current_one_and_signs_out_the_rest() {
    request::<App, _, _>(|request, ctx| async move {
        let user = user(&ctx, EMAIL, true).await;
        let here = session_cookie(&request, EMAIL, MAC).await;
        session_cookie(&request, EMAIL, IPHONE).await;

        let wrong = request
            .post("/account/password")
            .add_header("cookie", here.clone())
            .form(&serde_json::json!({ "current": "not-it", "password": "a-new-password" }))
            .await;
        assert_eq!(wrong.status_code(), 400);
        let body = wrong.text();
        assert!(body.contains("<title>Error: Your account | "), "{body}");
        assert!(body.contains("That isn't your current password."), "{body}");

        let short = request
            .post("/account/password")
            .add_header("cookie", here.clone())
            .form(&serde_json::json!({ "current": PASSWORD, "password": "short" }))
            .await;
        assert_eq!(short.status_code(), 400);
        assert!(short.text().contains("must be 8 to 128 characters"));

        let done = request
            .post("/account/password")
            .add_header("cookie", here.clone())
            .form(&serde_json::json!({ "current": PASSWORD, "password": "a-new-password" }))
            .await;
        assert_eq!(done.status_code(), 200);
        assert_eq!(done.header("hx-push-url"), "false");
        assert!(done.text().contains("Your password is changed"));
        assert_eq!(
            Session::count(sql::pool(&ctx), user.id)
                .await
                .expect("counted"),
            1,
            "this browser alone"
        );
        assert!(signs_in(&request, EMAIL, "a-new-password").await);
        assert!(!signs_in(&request, EMAIL, PASSWORD).await);
    })
    .await;
}

/// Deleting asks for the password once the sign-in is older than fifteen
/// minutes, and not before; the account and its sessions go, the cookie
/// with them.
#[tokio::test]
#[serial]
async fn the_account_is_deleted_with_the_password_or_a_fresh_proof() {
    request::<App, _, _>(|request, ctx| async move {
        let user = user(&ctx, EMAIL, true).await;
        let here = session_cookie(&request, EMAIL, MAC).await;
        let pool = sql::pool(&ctx);

        // Fresh from the sign-in: no password field. The step asked in a
        // dialog (views/parts.rs), its button opening it.
        let page = request
            .get("/account")
            .add_header("cookie", here.clone())
            .await;
        let body = page.text();
        assert!(!body.contains(r#"id="delete-password""#));
        for wanted in [
            r#"popovertarget="delete-account""#,
            r#"id="delete-account" popover="auto" role="dialog""#,
            r#"popovertargetaction="hide""#,
        ] {
            assert!(body.contains(wanted), "{wanted}: {body}");
        }

        // An hour later: the password, and a wrong one keeps the account.
        let past = Utc::now() - Duration::hours(1);
        sqlx::query!(
            "UPDATE sessions SET proven_at = ? WHERE user_id = ?",
            past,
            user.id
        )
        .execute(pool)
        .await
        .expect("proved an hour ago");
        let page = request
            .get("/account")
            .add_header("cookie", here.clone())
            .await;
        assert!(page.text().contains(r#"id="delete-password""#));
        let wrong = request
            .post("/account/delete")
            .add_header("cookie", here.clone())
            .form(&serde_json::json!({ "password": "not-it" }))
            .await;
        assert_eq!(wrong.status_code(), 400);
        assert!(users::Model::find_by_email(&ctx.db, EMAIL).await.is_ok());

        let deleted = request
            .post("/account/delete")
            .add_header("cookie", here.clone())
            .form(&serde_json::json!({ "password": PASSWORD }))
            .await;
        assert_eq!(deleted.status_code(), 303);
        assert_eq!(deleted.header("location"), "/account/deleted");
        assert!(
            deleted
                .header("set-cookie")
                .to_str()
                .expect("a cookie")
                .contains("Max-Age=0")
        );
        assert!(users::Model::find_by_email(&ctx.db, EMAIL).await.is_err());
        assert_eq!(Session::count(pool, user.id).await.expect("counted"), 0);

        let after = request.get("/account/deleted").await;
        assert_eq!(after.status_code(), 200);
        assert!(after.text().contains("Your account is deleted"));
    })
    .await;
}

/// Another site's page can't sign out, change a password or delete.
#[tokio::test]
#[serial]
async fn another_sites_post_changes_nothing() {
    request::<App, _, _>(|request, ctx| async move {
        user(&ctx, EMAIL, true).await;
        let here = session_cookie(&request, EMAIL, MAC).await;
        for (path, form) in [
            ("/account/browsers/close-others", serde_json::json!({})),
            (
                "/account/password",
                serde_json::json!({ "current": PASSWORD, "password": "a-new-password" }),
            ),
            (
                "/account/delete",
                serde_json::json!({ "password": PASSWORD }),
            ),
        ] {
            let res = request
                .post(path)
                .add_header("cookie", here.clone())
                .add_header("sec-fetch-site", "cross-site")
                .form(&form)
                .await;
            assert_eq!(res.status_code(), 403, "{path}");
        }
        assert!(users::Model::find_by_email(&ctx.db, EMAIL).await.is_ok());
        assert!(signs_in(&request, EMAIL, PASSWORD).await);
    })
    .await;
}

/// A PNG of `width` by `height`, one colour.
fn png(width: u32, height: u32, shade: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::RgbImage::from_pixel(width, height, image::Rgb([shade, 0, 0]))
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .expect("a PNG");
    bytes
}

/// The avatar's form as a browser sends it, `picture` in its file field.
fn avatar_form(picture: &[u8]) -> (String, axum::body::Bytes) {
    const BOUNDARY: &str = "picture-boundary";
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"me.png\"\r\nContent-Type: image/png\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(picture);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    (
        format!("multipart/form-data; boundary={BOUNDARY}"),
        body.into(),
    )
}

/// The avatar's address on the account page.
fn avatar_src(page: &str) -> String {
    page.split(r#"<img src=""#)
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("the avatar shown")
        .to_string()
}

/// A picture sent becomes the avatar, served for a year; a new one takes
/// its place and the old file goes; what isn't a picture is refused; the
/// account's deletion takes the file away.
#[tokio::test]
#[serial]
async fn the_avatar_is_made_served_replaced_and_taken_away() {
    request::<App, _, _>(|request, ctx| async move {
        user(&ctx, EMAIL, true).await;
        let here = session_cookie(&request, EMAIL, MAC).await;

        let page = request
            .get("/account")
            .add_header("cookie", here.clone())
            .await
            .text();
        for wanted in [
            r#"enctype="multipart/form-data""#,
            r#"data-shrink="1024""#,
            r#"data-shrink-under="1000000""#,
        ] {
            assert!(page.contains(wanted), "{wanted}: {page}");
        }
        assert!(!page.contains("<img src="), "no avatar yet");

        let (kind, body) = avatar_form(&png(640, 480, 10));
        let sent = request
            .post("/account/avatar")
            .add_header("cookie", here.clone())
            .bytes(body)
            .content_type(&kind)
            .await;
        assert_eq!(sent.status_code(), 200, "{}", sent.text());
        let first = avatar_src(&sent.text());
        assert!(first.starts_with("/uploads/avatars/"), "{first}");
        let file = request.get(&first).await;
        assert_eq!(file.status_code(), 200);
        assert_eq!(file.header("content-type"), "image/jpeg");
        assert_eq!(
            file.header("cache-control"),
            "public, max-age=31536000, immutable"
        );
        let made = image::load_from_memory(file.as_bytes()).expect("a picture");
        assert_eq!((made.width(), made.height()), (256, 256));

        let (kind, body) = avatar_form(&png(500, 500, 200));
        let again = request
            .post("/account/avatar")
            .add_header("cookie", here.clone())
            .bytes(body)
            .content_type(&kind)
            .await;
        let second = avatar_src(&again.text());
        assert_ne!(first, second);
        assert_eq!(
            request.get(&first).await.status_code(),
            404,
            "the old file gone"
        );

        let (kind, body) = avatar_form(b"not a picture");
        let refused = request
            .post("/account/avatar")
            .add_header("cookie", here.clone())
            .bytes(body)
            .content_type(&kind)
            .await;
        assert_eq!(refused.status_code(), 400);
        assert!(refused.text().contains("That isn't a picture we can use"));

        let deleted = request
            .post("/account/delete")
            .add_header("cookie", here)
            .form(&serde_json::json!({}))
            .await;
        assert_eq!(deleted.status_code(), 303);
        assert_eq!(
            request.get(&second).await.status_code(),
            404,
            "gone with the account"
        );
    })
    .await;
}

/// Only a name the pictures are given is served: nothing outside the
/// folder.
#[tokio::test]
#[serial]
async fn only_an_avatars_own_name_is_served() {
    request::<App, _, _>(|request, _ctx| async move {
        for path in [
            "/uploads/avatars/..%2F..%2FCargo.toml",
            "/uploads/avatars/abc.jpg",
        ] {
            assert_eq!(request.get(path).await.status_code(), 404, "{path}");
        }
    })
    .await;
}
