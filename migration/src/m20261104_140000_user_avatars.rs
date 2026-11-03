//! A user's avatar (src/uploads.rs): the name of its file in the uploads'
//! folder, none until one is sent. Read and written by the app's own SQLx
//! queries, as the role is.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("ALTER TABLE users ADD COLUMN avatar varchar NULL")
            .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("ALTER TABLE users DROP COLUMN avatar")
            .await?;
        Ok(())
    }
}
