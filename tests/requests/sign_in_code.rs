//! An admin's second step (src/models/sign_ins.rs, controllers/login.rs):
//! after the password, six digits by mail and no session until they're
//! right; tries counted, a new code at most once a minute. A member signs
//! in with the password alone (tests/requests/login.rs).

use app::{
    app::App,
    models::{roles::Role, sessions::Session},
    sql,
};
use loco_rs::{TestServer, app::AppContext, testing::prelude::*};
use serial_test::serial;

use super::prepare_data::{PASSWORD, sign_in, user};

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
            again.text().contains("less than a minute ago"),
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
