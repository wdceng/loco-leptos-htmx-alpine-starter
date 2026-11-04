//! What happened at the sign-in, a row an event (src/models/auth_events.rs):
//! a second step's code wrong or its tries spent, a sign-in completed, a
//! password set or changed. What the wait after wrong codes counts, and the
//! trail of an account's sign-ins. No address and no IP; an account's rows
//! go with it, and every row after 180 days. As an islands app on this
//! stack keeps them.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        // On one line each: SQLite keeps the statement's text as the schema.
        // Raw SQL, so the table is its four columns, its time written by
        // the app, never Loco's `created_at` text.
        db.execute_unprepared(
            r#"CREATE TABLE "auth_events" ("id" integer NOT NULL PRIMARY KEY AUTOINCREMENT, "user_id" integer NOT NULL REFERENCES "users"("id") ON DELETE CASCADE ON UPDATE CASCADE, "event" varchar NOT NULL, "at" timestamp_with_timezone_text NOT NULL)"#,
        )
        .await?;
        // An account's rows, which the wait counts.
        db.execute_unprepared(
            r#"CREATE INDEX "idx-auth_events-user_id-at" ON "auth_events" ("user_id", "at")"#,
        )
        .await?;
        // The old ones, which the purge at every start deletes.
        db.execute_unprepared(r#"CREATE INDEX "idx-auth_events-at" ON "auth_events" ("at")"#)
            .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(r#"DROP TABLE "auth_events""#)
            .await?;
        Ok(())
    }
}
