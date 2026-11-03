//! `/uploads/avatars/{name}`: an avatar's file from the avatars' folder
//! (src/uploads.rs), outside the site's. Its name is its contents' hash,
//! so it is kept for a year; a name of any other shape, or one with no
//! file, is a miss. Limited like any route.

use axum::{
    extract::Path,
    http::{
        StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
};
use loco_rs::prelude::*;

use crate::{app::stored, models::avatars::Avatars, uploads};

/// The route's pattern.
const ROUTE: &str = "/uploads/avatars/{name}";

/// `GET /uploads/avatars/{name}`.
#[debug_handler]
async fn avatar(State(ctx): State<AppContext>, Path(name): Path<String>) -> Result<Response> {
    if !uploads::is_name(&name) {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let Avatars(folder) = stored(&ctx)?;
    match tokio::fs::read(folder.join(&name)).await {
        Ok(bytes) => Ok((
            [
                (CONTENT_TYPE, "image/jpeg"),
                (CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            bytes,
        )
            .into_response()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(StatusCode::NOT_FOUND.into_response())
        }
        Err(e) => Err(Error::wrap(e)),
    }
}

pub fn routes() -> Routes {
    Routes::new().add(ROUTE, get(avatar))
}
