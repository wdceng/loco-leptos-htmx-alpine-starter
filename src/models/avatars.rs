//! A user's avatar (`users.avatar`, migration/src): the name of its file in
//! the avatars' folder (src/uploads.rs), none until one is sent. The folder
//! itself, made at boot, is in the shared store as [`Avatars`].

use std::path::PathBuf;

use sqlx::SqlitePool;

/// The avatars' folder (`uploads::avatars_folder`), made at boot and
/// parked in the shared store.
#[derive(Clone, Debug)]
pub struct Avatars(pub PathBuf);

/// The avatar's file of the user whose row is `user_id`, if they sent one.
///
/// # Errors
/// The database could not be read.
pub async fn of(pool: &SqlitePool, user_id: i64) -> sqlx::Result<Option<String>> {
    Ok(
        sqlx::query_scalar!("SELECT avatar FROM users WHERE id = ?", user_id)
            .fetch_optional(pool)
            .await?
            .flatten(),
    )
}

/// The user whose row is `user_id` given the avatar `name`, or none: the
/// one they had before.
///
/// # Errors
/// The database refused the write.
pub async fn set(
    pool: &SqlitePool,
    user_id: i64,
    name: Option<&str>,
) -> sqlx::Result<Option<String>> {
    let before = of(pool, user_id).await?;
    sqlx::query!("UPDATE users SET avatar = ? WHERE id = ?", name, user_id)
        .execute(pool)
        .await?;
    Ok(before)
}

/// Whether any user still has the avatar `name`: the same picture is one
/// file, whoever sent it, so a file goes only when nobody shows it.
///
/// # Errors
/// The database could not be read.
pub async fn in_use(pool: &SqlitePool, name: &str) -> sqlx::Result<bool> {
    let count = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!: i64" FROM users WHERE avatar = ?"#,
        name
    )
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}
