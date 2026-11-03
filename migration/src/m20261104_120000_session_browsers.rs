//! What a session says of itself on the account page
//! (src/models/sessions.rs), and when its person last proved who they are:
//!
//! - `browser`: the browser and system it was opened in, from its
//!   `User-Agent` at the sign-in ("Firefox on a Mac");
//! - `seen_at`: when it was last used, kept to the minute;
//! - `proven_at`: the sign-in, or the last password typed in it since. A
//!   sensitive step asks nothing within fifteen minutes of it, the password
//!   past them, so a session taken from an unlocked computer an hour later
//!   deletes no account (GitHub's sudo mode, OWASP's reauthentication). As
//!   an islands app on this stack keeps it.
//!
//! A session open before this counts from its sign-in.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        // Constant defaults, which `ALTER TABLE` asks; every row is given
        // its own times below, and a new one by `Session::open`.
        db.execute_unprepared(
            "ALTER TABLE sessions ADD COLUMN browser varchar NOT NULL DEFAULT ''",
        )
        .await?;
        for column in ["seen_at", "proven_at"] {
            db.execute_unprepared(&format!(
                "ALTER TABLE sessions ADD COLUMN {column} timestamp_with_timezone_text NOT NULL \
                 DEFAULT '1970-01-01 00:00:00+00:00'"
            ))
            .await?;
        }
        db.execute_unprepared("UPDATE sessions SET seen_at = created_at, proven_at = created_at")
            .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        for column in ["proven_at", "seen_at", "browser"] {
            db.execute_unprepared(&format!("ALTER TABLE sessions DROP COLUMN {column}"))
                .await?;
        }
        Ok(())
    }
}
