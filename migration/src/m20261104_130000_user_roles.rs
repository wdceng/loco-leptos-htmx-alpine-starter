//! A user's role (src/models/roles.rs): `member`, everyone who signs up,
//! or `admin`, given from the command line (`user:create … role:admin`).
//! Read by the app's own SQLx queries; Loco's users entity, as generated,
//! leaves the column to them.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE users ADD COLUMN role varchar NOT NULL DEFAULT 'member'",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("ALTER TABLE users DROP COLUMN role")
            .await?;
        Ok(())
    }
}
