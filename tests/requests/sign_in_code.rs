//! An admin's second step (src/models/sign_ins.rs, controllers/login.rs):
//! after the password, six digits by mail and no session until they're
//! right; tries counted, a new code at most once a minute. A member signs
//! in with the password alone (tests/requests/login.rs).

use app::{
    app::App,
    models::{
        auth_events::{self, Event},
        roles::Role,
        sessions::Session,
    },
    sql,
};
use loco_rs::{TestServer, app::AppContext, testing::prelude::*};
use serial_test::serial;

use super::prepare_data::{PASSWORD, session_cookie, sign_in, user};

const EMAIL: &str = "admin@example.com";

/// An admin, verified, by [`EMAIL`]: their row's id.
async fn admin(ctx: &AppContext) -> i64 {
    let admin = user(ctx, EMAIL, true).await;
    Role::Admin
        .give(sql::pool(ctx), admin.id)
        .await
        .expect("given");
    admin.id
}

/// The `signin` cookie's pair from a sign-in's answer.
fn signin_cookie(set_cookie: &str) -> String {
    set_cookie.split(';').next().expect("the pair").to_string()
}

/// The code in the last mail sent.
fn last_code(ctx: &AppContext) -> String {
    let mails = ctx
        .mailer
        .as_ref()
        .expect("stub mailer")
        .deliveries()
        .messages;
    let mail = mails.last().expect("a mail");
    mail.split("Your sign-in code: ")
        .nth(1)
        .map(|rest| rest.chars().take(6).collect())
        .unwrap_or_else(|| panic!("no code in:\n{mail}"))
}

/// The password, then the admin's sign-in waiting: its cookie.
async fn password_step(request: &TestServer) -> String {
    let res = request.post("/login").form(&sign_in(EMAIL, PASSWORD)).await;
    assert_eq!(res.status_code(), 303);
    assert_eq!(res.header("location"), "/login/code");
    let set = res.header("set-cookie");
    let set = set.to_str().expect("a cookie");
    assert!(set.starts_with("signin="), "no session yet: {set}");
    signin_cookie(set)
}

#[tokio::test]
#[serial]
async fn an_admin_signs_in_with_the_mailed_code() {
    request::<App, _, _>(|request, ctx| async move {
        let id = admin(&ctx).await;
        let waiting = password_step(&request).await;
        assert_eq!(
            Session::count(sql::pool(&ctx), id).await.expect("counted"),
            0,
            "no session before the code"
        );
        let mails = ctx
            .mailer
            .as_ref()
            .expect("stub mailer")
            .deliveries()
            .messages;
        let subject = mails
            .last()
            .and_then(|mail| mail.lines().find(|line| line.starts_with("Subject: ")))
            .expect("a subject")
            .to_string();
        let code = last_code(&ctx);
        assert!(
            !subject.contains(&code),
            "the code stays out of the subject"
        );

        let page = request
            .get("/login/code")
            .add_header("cookie", waiting.clone())
            .await;
        assert_eq!(page.status_code(), 200);
        let body = page.text();
        for wanted in [
            r#"autocomplete="one-time-code""#,
            r#"inputmode="numeric""#,
            r#"action="/login/code/again""#,
        ] {
            assert!(body.contains(wanted), "{wanted}: {body}");
        }

        let wrong = request
            .post("/login/code")
            .add_header("cookie", waiting.clone())
            .form(&serde_json::json!({ "code": "000000" }))
            .await;
        assert_eq!(wrong.status_code(), 400);
        assert!(wrong.text().contains("4 tries left"), "{}", wrong.text());

        let right = request
            .post("/login/code")
            .add_header("cookie", waiting)
            .form(&serde_json::json!({ "code": code }))
            .await;
        assert_eq!(right.status_code(), 303);
        assert_eq!(right.header("location"), "/");
        let cookies: Vec<String> = right
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|value| value.to_str().expect("text").to_string())
            .collect();
        assert!(
            cookies.iter().any(|c| c.starts_with("session=")),
            "{cookies:?}"
        );
        assert!(
            cookies.iter().any(|c| c.starts_with("signin=;")),
            "the sign-in's cookie taken away: {cookies:?}"
        );
        assert_eq!(
            Session::count(sql::pool(&ctx), id).await.expect("counted"),
            1
        );
    })
    .await;
}

/// Five wrong codes and the password is asked again.
#[tokio::test]
#[serial]
async fn five_wrong_codes_ask_for_the_password_again() {
    request::<App, _, _>(|request, ctx| async move {
        admin(&ctx).await;
        let waiting = password_step(&request).await;
        for _ in 0..4 {
            let wrong = request
                .post("/login/code")
                .add_header("cookie", waiting.clone())
                .form(&serde_json::json!({ "code": "000000" }))
                .await;
            assert_eq!(wrong.status_code(), 400);
        }
        let spent = request
            .post("/login/code")
            .add_header("cookie", waiting.clone())
            .form(&serde_json::json!({ "code": "000000" }))
            .await;
        assert_eq!(spent.status_code(), 303);
        assert_eq!(spent.header("location"), "/login");
        let gone = request
            .get("/login/code")
            .add_header("cookie", waiting)
            .await;
        assert_eq!(gone.status_code(), 303, "the sign-in is closed");
    })
    .await;
}

/// A new code goes at most once a minute; nobody waiting is sent to the
/// password.
#[tokio::test]
#[serial]
async fn a_new_code_waits_a_minute_and_nobody_waiting_goes_to_the_password() {
    request::<App, _, _>(|request, ctx| async move {
        admin(&ctx).await;
        let waiting = password_step(&request).await;
        let again = request
            .post("/login/code/again")
            .add_header("cookie", waiting)
            .await;
        assert_eq!(again.status_code(), 200);
        assert!(
            again.text().contains("You can ask for a new code in"),
            "{}",
            again.text()
        );
        assert_eq!(
            ctx.mailer.as_ref().expect("stub mailer").deliveries().count,
            1,
            "the first code alone"
        );

        let nobody = request.get("/login/code").await;
        assert_eq!(nobody.status_code(), 303);
        assert_eq!(nobody.header("location"), "/login");
    })
    .await;
}

/// How many mails went, and the subject of the last.
fn mails(ctx: &AppContext) -> (usize, String) {
    let deliveries = ctx.mailer.as_ref().expect("stub mailer").deliveries();
    let subject = deliveries
        .messages
        .last()
        .and_then(|mail| mail.lines().find(|line| line.starts_with("Subject: ")))
        .unwrap_or_default()
        .to_string();
    (deliveries.count, subject)
}

/// Five wrong codes and the person is told by mail; the wrong codes count
/// on across a new sign-in, where the right password once brought five
/// fresh tries: no code goes within the wait, the page says how long, and a
/// code posted is not checked.
#[tokio::test]
#[serial]
async fn wrong_codes_count_across_sign_ins_and_the_fifth_is_mailed() {
    request::<App, _, _>(|request, ctx| async move {
        let id = admin(&ctx).await;
        let waiting = password_step(&request).await;
        for _ in 0..auth_events::FREE_TRIES {
            request
                .post("/login/code")
                .add_header("cookie", waiting.clone())
                .form(&serde_json::json!({ "code": "000000" }))
                .await;
        }
        let (sent, subject) = mails(&ctx);
        assert_eq!(sent, 2, "the code, then the wrong codes' mail");
        assert!(subject.contains("Wrong sign-in codes"), "{subject}");

        // More wrong codes since, so the wait outlasts the test.
        for _ in 0..8 {
            auth_events::record(sql::pool(&ctx), id, Event::CodeWrong)
                .await
                .expect("recorded");
        }
        let waiting = password_step(&request).await;
        assert_eq!(mails(&ctx).0, sent, "no code within the wait");
        let page = request
            .get("/login/code")
            .add_header("cookie", waiting.clone())
            .await;
        assert_eq!(page.status_code(), 200);
        assert!(
            page.text().contains("Too many wrong codes. Try again in"),
            "{}",
            page.text()
        );
        let posted = request
            .post("/login/code")
            .add_header("cookie", waiting.clone())
            .form(&serde_json::json!({ "code": "000000" }))
            .await;
        assert_eq!(posted.status_code(), 400);
        assert!(posted.text().contains("Too many wrong codes"));
        let again = request
            .post("/login/code/again")
            .add_header("cookie", waiting)
            .await;
        assert!(again.text().contains("Too many wrong codes"));
        assert_eq!(mails(&ctx).0, sent, "no code asked again within the wait");
    })
    .await;
}

/// A new password on the account page ends a sign-in waiting for its code
/// elsewhere, as one through the mailed link does: whoever knew the old
/// password is left with nothing.
#[tokio::test]
#[serial]
async fn a_new_password_ends_a_sign_in_waiting_for_its_code() {
    request::<App, _, _>(|request, ctx| async move {
        // Signed in as a member, then made an admin: the next sign-in asks
        // for a code.
        let id = user(&ctx, EMAIL, true).await.id;
        let here = session_cookie(&request, EMAIL, "Firefox on a Mac").await;
        Role::Admin.give(sql::pool(&ctx), id).await.expect("given");
        let waiting = password_step(&request).await;
        let changed = request
            .post("/account/password")
            .add_header("cookie", here)
            .form(&serde_json::json!({ "current": PASSWORD, "password": "a-new-password" }))
            .await;
        assert_eq!(changed.status_code(), 200, "{}", changed.text());
        let gone = request
            .get("/login/code")
            .add_header("cookie", waiting)
            .await;
        assert_eq!(gone.status_code(), 303, "the sign-in is closed");
        assert_eq!(
            Session::count(sql::pool(&ctx), id).await.expect("counted"),
            1,
            "this browser's session kept"
        );
    })
    .await;
}
