use app::{
    app::App,
    models::{
        sessions::Session,
        users::{self, RegisterParams},
    },
    secret, sql,
};
use chrono::{Duration, Utc};
use loco_rs::{app::AppContext, testing::prelude::*};
use serial_test::serial;

/// The browser every test session is opened in.
const BROWSER: &str = "Firefox on a Mac";

/// A user to open sessions for, by `email`: their row's id.
async fn user(ctx: &AppContext, email: &str) -> i64 {
    let user = users::Model::create_with_password(
        &ctx.db,
        &RegisterParams {
            email: email.into(),
            password: "12341234".into(),
            name: "Session".into(),
        },
    )
    .await
    .expect("the user is created");
    user.id
}

#[tokio::test]
#[serial]
async fn an_open_session_is_found_by_its_secret_alone() {
    let boot = boot_test::<App>().await.expect("the app boots");
    let ctx = &boot.app_context;
    let pool = sql::pool(ctx);
    let user_id = user(ctx, "open@example.com").await;

    let (secret, opened) = Session::open(pool, user_id, BROWSER).await.expect("opened");
    assert!(secret::is_secret(&secret));
    assert_eq!(opened.user_id, user_id);
    let found = Session::find(pool, &secret).await.expect("read");
    assert_eq!(found, Some(opened));

    // Another secret, of the same shape, finds nothing.
    let other = secret::fresh();
    assert_eq!(Session::find(pool, &other).await.expect("read"), None);
    // A value without the shape is turned away before the database.
    assert_eq!(Session::find(pool, "nope").await.expect("read"), None);
}

/// The row keeps the secret's digest, never the secret: a copy of the
/// database signs nobody in.
#[tokio::test]
#[serial]
async fn the_row_holds_the_digest_not_the_secret() {
    let boot = boot_test::<App>().await.expect("the app boots");
    let ctx = &boot.app_context;
    let pool = sql::pool(ctx);
    let user_id = user(ctx, "digest@example.com").await;

    let (secret, _) = Session::open(pool, user_id, BROWSER).await.expect("opened");
    let digest = secret::digest(&secret);
    let kept = sqlx::query_scalar!("SELECT token_hash FROM sessions WHERE user_id = ?", user_id)
        .fetch_one(pool)
        .await
        .expect("the row");
    assert_eq!(kept, digest);
    assert_ne!(kept, secret);
}

/// A session past its time is found by nobody, and the next sign-in sweeps
/// its row away.
#[tokio::test]
#[serial]
async fn a_session_that_ran_out_is_not_found_and_swept() {
    let boot = boot_test::<App>().await.expect("the app boots");
    let ctx = &boot.app_context;
    let pool = sql::pool(ctx);
    let user_id = user(ctx, "late@example.com").await;

    let (secret, opened) = Session::open(pool, user_id, BROWSER).await.expect("opened");
    let past = Utc::now() - Duration::minutes(1);
    sqlx::query!(
        "UPDATE sessions SET expires_at = ? WHERE id = ?",
        past,
        opened.id
    )
    .execute(pool)
    .await
    .expect("moved into the past");
    assert_eq!(Session::find(pool, &secret).await.expect("read"), None);

    Session::open(pool, user_id, BROWSER)
        .await
        .expect("opened again");
    assert_eq!(
        Session::count(pool, user_id).await.expect("counted"),
        1,
        "the run-out row is gone, the new one stays"
    );
}

#[tokio::test]
#[serial]
async fn closing_takes_one_session_or_all_of_them() {
    let boot = boot_test::<App>().await.expect("the app boots");
    let ctx = &boot.app_context;
    let pool = sql::pool(ctx);
    let user_id = user(ctx, "close@example.com").await;
    let other_id = user(ctx, "other@example.com").await;

    let (first, opened) = Session::open(pool, user_id, BROWSER).await.expect("opened");
    let (second, _) = Session::open(pool, user_id, BROWSER).await.expect("opened");
    let (others, _) = Session::open(pool, other_id, BROWSER)
        .await
        .expect("opened");

    Session::close(pool, opened.id).await.expect("closed");
    assert_eq!(Session::find(pool, &first).await.expect("read"), None);
    assert!(Session::find(pool, &second).await.expect("read").is_some());

    Session::close_all(pool, user_id).await.expect("closed");
    assert_eq!(Session::count(pool, user_id).await.expect("counted"), 0);
    assert!(
        Session::find(pool, &others).await.expect("read").is_some(),
        "another user's session stays"
    );
}

/// The account page's list: each live session as its browser, the last
/// used first; one closed by its id among the user's own alone, and every
/// one but the one kept.
#[tokio::test]
#[serial]
async fn the_browsers_are_listed_and_closed_one_by_one_or_but_one() {
    let boot = boot_test::<App>().await.expect("the app boots");
    let ctx = &boot.app_context;
    let pool = sql::pool(ctx);
    let user_id = user(ctx, "browsers@example.com").await;
    let other_id = user(ctx, "stranger@example.com").await;

    let (_, mac) = Session::open(pool, user_id, BROWSER).await.expect("opened");
    let (_, phone) = Session::open(pool, user_id, "Safari on an iPhone")
        .await
        .expect("opened");
    let (_, laptop) = Session::open(pool, user_id, "Edge on Windows")
        .await
        .expect("opened");
    let (_, stranger) = Session::open(pool, other_id, BROWSER)
        .await
        .expect("opened");
    let earlier = Utc::now() - Duration::hours(1);
    sqlx::query!(
        "UPDATE sessions SET seen_at = ? WHERE id = ?",
        earlier,
        mac.id
    )
    .execute(pool)
    .await
    .expect("seen an hour ago");

    let listed = Session::browsers(pool, user_id).await.expect("listed");
    let names: Vec<&str> = listed.iter().map(|b| b.browser.as_str()).collect();
    assert_eq!(names, ["Edge on Windows", "Safari on an iPhone", BROWSER]);

    assert!(
        !Session::close_of(pool, user_id, stranger.id)
            .await
            .expect("asked"),
        "another user's session is not the user's to close"
    );
    assert!(
        Session::close_of(pool, user_id, phone.id)
            .await
            .expect("closed")
    );
    assert_eq!(Session::count(pool, user_id).await.expect("counted"), 2);

    Session::close_others(pool, user_id, laptop.id)
        .await
        .expect("closed");
    let listed = Session::browsers(pool, user_id).await.expect("listed");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, laptop.id);
    assert_eq!(Session::count(pool, other_id).await.expect("counted"), 1);
}

/// A session is proven at its sign-in, for fifteen minutes; a password
/// typed later proves it again.
#[tokio::test]
#[serial]
async fn a_proof_holds_fifteen_minutes() {
    let boot = boot_test::<App>().await.expect("the app boots");
    let ctx = &boot.app_context;
    let pool = sql::pool(ctx);
    let user_id = user(ctx, "proof@example.com").await;

    let (secret, opened) = Session::open(pool, user_id, BROWSER).await.expect("opened");
    assert!(opened.proven(), "proven at the sign-in");
    let past = Utc::now() - Duration::minutes(16);
    sqlx::query!(
        "UPDATE sessions SET proven_at = ? WHERE id = ?",
        past,
        opened.id
    )
    .execute(pool)
    .await
    .expect("moved into the past");
    let found = Session::find(pool, &secret)
        .await
        .expect("read")
        .expect("live");
    assert!(!found.proven(), "sixteen minutes later");

    Session::prove(pool, opened.id).await.expect("proven");
    let found = Session::find(pool, &secret)
        .await
        .expect("read")
        .expect("live");
    assert!(found.proven(), "a password typed again");
}
