//! A sign-in whose password passed and whose second step is still due
//! (src/models/sign_ins.rs): its secret's digest (the `signin` cookie),
//! the mailed code's digest, when it runs out, when the last code went,
//! the tries so far, and the step it asks. The rows go with the user
//! (`user_id`, cascade). As an islands app on this stack keeps them.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "sign_ins",
            &[
                ("id", ColType::PkAuto),
                ("token_hash", ColType::StringUniq),
                ("code_hash", ColType::String),
                ("expires_at", ColType::TimestampWithTimeZone),
                ("code_sent_at", ColType::TimestampWithTimeZone),
                ("tries", ColType::IntegerWithDefault(0)),
                ("method", ColType::String),
            ],
            &[("user", "")],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "sign_ins").await?;
        Ok(())
    }
}
