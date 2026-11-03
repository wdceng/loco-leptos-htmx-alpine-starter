#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;
mod m20261102_120000_sessions;
mod m20261104_120000_session_browsers;
mod m20261104_130000_user_roles;
mod m20261104_140000_user_avatars;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20261102_120000_sessions::Migration),
            Box::new(m20261104_120000_session_browsers::Migration),
            Box::new(m20261104_130000_user_roles::Migration),
            Box::new(m20261104_140000_user_avatars::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
