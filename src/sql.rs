//! The app's own queries run on SQLx (`sqlx::query!`, checked at compile
//! time), on the pool Sea-ORM opened for Loco: one set of connections, with
//! the PRAGMAs Loco applied. Sea-ORM keeps the migrations and Loco's own
//! models (`docs/architecture.md`, "Sea-ORM and SQLx").

use loco_rs::app::AppContext;
use sqlx::{Sqlite, SqlitePool, Transaction};

/// The SQLite pool behind `ctx.db`, for `sqlx::query!` and friends.
///
/// # Panics
/// Never in this app: Sea-ORM panics for a database other than SQLite, and
/// SQLite is the only driver compiled in (`Cargo.toml`).
#[must_use]
pub fn pool(ctx: &AppContext) -> &SqlitePool {
    ctx.db.get_sqlite_connection_pool()
}

/// Every SQLx transaction: `BEGIN IMMEDIATE`, the write lock taken at the
/// start, so a transaction that reads and then writes never fails at its
/// write, `SQLITE_BUSY_SNAPSHOT`, which no timeout waits out, whenever
/// another connection wrote in between (WAL mode).
///
/// A connection SQLx still counts inside a transaction SQLite already
/// ended refuses every `BEGIN IMMEDIATE` until it is closed: after a full
/// disk (`SQLITE_FULL`), SQLite rolls the transaction back itself, SQLx's
/// own rollback then fails, and its count stays at one (launchbadge/sqlx
/// #4434, open, 0.9.0). Every write on that connection, every sign-in
/// among them, would fail until a restart, the disk long since freed. So
/// on that refusal the idle connections stuck so are closed, the pool
/// opening fresh ones, and the write is tried again, as many times as the
/// pool has connections; said in the log once it went on. As an islands
/// app on this stack writes.
///
/// # Errors
/// The write lock not had within the busy timeout, or no connection; the
/// refusal still there after every try.
pub async fn write(pool: &SqlitePool) -> sqlx::Result<Transaction<'static, Sqlite>> {
    let mut tries = pool.options().get_max_connections();
    let mut closed = 0;
    loop {
        match pool.begin_with("BEGIN IMMEDIATE").await {
            Err(sqlx::Error::InvalidSavePointStatement) if tries > 0 => {
                tries -= 1;
                closed += close_stuck(pool).await;
            }
            begun => {
                if closed > 0 {
                    tracing::warn!(
                        closed,
                        "sql: connections left inside a transaction SQLite had ended (a full disk?), closed"
                    );
                }
                return begun;
            }
        }
    }
}

/// The idle connections taken out of the pool and each asked for a plain
/// `BEGIN`, which takes no lock: one SQLx refuses as still inside a
/// transaction is closed, a healthy one rolled back and handed back. How
/// many were closed. SQLx hands a refused connection back to the pool in a
/// task of its own, so it is waited for, 10 ms at a time, 100 ms at most,
/// this path being the rare one. A connection busy in another request is
/// not reached: it heals when its own next write is refused.
async fn close_stuck(pool: &SqlitePool) -> usize {
    use sqlx::Connection;

    let mut closed = 0;
    for _ in 0..10 {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let mut idle = Vec::new();
        while let Some(connection) = pool.try_acquire() {
            idle.push(connection);
        }
        for mut connection in idle {
            let stuck = match connection.begin_with("BEGIN").await {
                // A failed rollback leaves the count up: closed too.
                Ok(probe) => probe.rollback().await.is_err(),
                Err(sqlx::Error::InvalidSavePointStatement) => true,
                Err(_) => false,
            };
            if stuck {
                // The pool opens a fresh one in its place on demand.
                let _ = connection.close().await;
                closed += 1;
            }
        }
        if closed > 0 {
            return closed;
        }
    }
    closed
}
