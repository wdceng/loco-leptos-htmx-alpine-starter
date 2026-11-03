use app::{
    app::App,
    models::users::{self, RegisterParams},
    sql,
};
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// SQLx runs on the pool Sea-ORM opened (`src/sql.rs`): a row Sea-ORM writes
/// is one SQLx reads. `query_scalar!` on purpose, so the compile-time check
/// and the `.sqlx/` cache are exercised from the first commit on.
#[tokio::test]
#[serial]
async fn sqlx_reads_what_sea_orm_wrote() {
    let boot = boot_test::<App>()
        .await
        .expect("Failed to boot test application");
    let ctx = &boot.app_context;

    users::Model::create_with_password(
        &ctx.db,
        &RegisterParams {
            email: "pool@example.com".into(),
            password: "12341234".into(),
            name: "Pool".into(),
        },
    )
    .await
    .expect("Sea-ORM writes the user");

    let email = "pool@example.com";
    let count = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM users WHERE email = ?"#,
        email
    )
    .fetch_one(sql::pool(ctx))
    .await
    .expect("SQLx reads through the same pool");
    assert_eq!(count, 1);
}

/// A full disk's leftover, made by hand: SQLite's transaction ended behind
/// SQLx's back (a plain `ROLLBACK` run through it, a fixture, the one SQL
/// here not checked by the macros), then the transaction dropped, SQLx's
/// own rollback failing. The test pool's one connection then refuses
/// `BEGIN IMMEDIATE` (sqlx #4434; should that assertion fail, sqlx fixed
/// it and `close_stuck` can go), and `sql::write` closes it and writes on
/// a fresh one, twice over. As an islands app on this stack tests it.
#[tokio::test]
#[serial]
async fn a_connection_stuck_in_a_transaction_is_closed_and_the_write_goes_on() {
    let boot = boot_test::<App>()
        .await
        .expect("Failed to boot test application");
    let pool = sql::pool(&boot.app_context);
    let mut stuck = sql::write(pool).await.expect("begun");
    sqlx::query("ROLLBACK")
        .execute(&mut *stuck)
        .await
        .expect("ended behind SQLx's back");
    drop(stuck);
    assert!(
        matches!(
            pool.begin_with("BEGIN IMMEDIATE").await,
            Err(sqlx::Error::InvalidSavePointStatement)
        ),
        "the connection is not stuck: sqlx #4434 fixed?"
    );
    for n in 1..=2 {
        sql::write(pool)
            .await
            .unwrap_or_else(|e| panic!("write {n}: {e}"))
            .commit()
            .await
            .unwrap_or_else(|e| panic!("commit {n}: {e}"));
    }
}
