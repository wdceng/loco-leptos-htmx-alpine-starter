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

    let (secret, opened) = Session::open(pool, user_id).await.expect("opened");
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

    let (secret, _) = Session::open(pool, user_id).await.expect("opened");
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

    let (secret, opened) = Session::open(pool, user_id).await.expect("opened");
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

    Session::open(pool, user_id).await.expect("opened again");
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

    let (first, opened) = Session::open(pool, user_id).await.expect("opened");
    let (second, _) = Session::open(pool, user_id).await.expect("opened");
    let (others, _) = Session::open(pool, other_id).await.expect("opened");

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
