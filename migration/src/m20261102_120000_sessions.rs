//! A signed-in user's sessions (src/models/sessions.rs): a row per sign-in,
//! the secret its cookie carries kept as its digest, and when it runs out.
//! The rows go with the user (`user_id`, cascade).

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "sessions",
            &[
                ("id", ColType::PkAuto),
                ("token_hash", ColType::StringUniq),
                ("expires_at", ColType::TimestampWithTimeZone),
            ],
            &[("user", "")],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "sessions").await?;
        Ok(())
    }
}
