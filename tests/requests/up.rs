use app::{app::App, controllers::up};
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// The app's start page reads this from another origin, so it
/// allows any, and it's never cached, so every check asks the app.
#[tokio::test]
#[serial]
async fn up_answers_ok_to_any_origin_and_is_never_cached() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get(up::PATH).await;

        assert_eq!(res.status_code(), 200);
        assert_eq!(res.text(), up::BODY);
        assert_eq!(res.header("access-control-allow-origin"), "*");
        assert_eq!(res.header("cache-control"), "no-store");
    })
    .await;
}
