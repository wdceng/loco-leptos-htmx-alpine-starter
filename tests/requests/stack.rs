//! The middleware stack the app builds (`Hooks::middlewares`, src/app.rs):
//! the site's own layers in their places. Loco's stack helpers only log a
//! warning for a name they do not find, `replace` then changing nothing and
//! `insert_after` appending at the far end, so a renamed Loco middleware
//! would quietly bring Loco's own logger back, with the query in its lines,
//! or put the error pages outside the security headers.

use app::app::App;
use loco_rs::{app::Hooks, testing::prelude::*};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn the_sites_own_layers_stand_where_they_must() {
    request::<App, _, _>(|_request, ctx| async move {
        let stack = App::middlewares(&ctx);
        let names: Vec<&str> = stack.iter().map(|layer| layer.name()).collect();
        let at = |name: &str| {
            names
                .iter()
                .position(|n| *n == name)
                .unwrap_or_else(|| panic!("{name} missing: {names:?}"))
        };
        // The site's files and misses first, so every layer wraps them.
        assert_eq!(at("site"), 0, "{names:?}");
        assert!(!names.contains(&"static") && !names.contains(&"fallback"));
        // The limiter just outside remote_ip, which it reads the visitor by.
        assert_eq!(at("rate_limit"), at("remote_ip") + 1, "{names:?}");
        // The error pages just outside the time limit, inside the headers,
        // the request log and the request id.
        assert_eq!(at("error_page"), at("timeout_request") + 1, "{names:?}");
        assert!(
            at("error_page") < at("secure_headers")
                && at("secure_headers") < at("logger")
                && at("logger") < at("request_id"),
            "{names:?}"
        );
        // The request log is the site's (middleware/request_log.rs): Loco's
        // own nests its settings under `config`, ours has none.
        let logger = stack
            .iter()
            .find(|layer| layer.name() == "logger")
            .and_then(|layer| layer.config().ok())
            .expect("the logger's config");
        assert!(logger.get("config").is_none(), "Loco's logger: {logger}");
        assert_eq!(logger["enable"], true, "{logger}");
    })
    .await;
}
