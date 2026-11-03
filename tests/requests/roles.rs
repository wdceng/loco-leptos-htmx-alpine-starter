//! A page for one role (`Admin`, src/controllers/signed.rs): nobody signed
//! in goes to the sign-in, a member gets the 403 page, an admin the page.
//! The template has no admin page, so the extractor is mounted on a router
//! with one, behind the error page layer.

use app::{
    app::App,
    controllers::signed::Admin,
    middleware::error_page::ErrorPages,
    models::{roles::Role, sessions::Session},
    sql,
};
use axum::{Router, routing::get};
use loco_rs::{TestServer, controller::middleware::MiddlewareLayer, testing::prelude::*};
use serial_test::serial;

use super::prepare_data::user;

const BROWSER: &str = "text/html,application/xhtml+xml,*/*;q=0.8";

#[tokio::test]
#[serial]
async fn an_admin_page_is_an_admins_alone() {
    let boot = boot_test::<App>()
        .await
        .expect("Failed to boot test application");
    let ctx = boot.app_context;
    let router = Router::new().route(
        "/admin-only",
        get(|Admin(signed): Admin| async move { format!("Hello, {}", signed.user.name) }),
    );
    let router = ErrorPages::from_context(&ctx)
        .apply(router)
        .expect("layer applies")
        .with_state(ctx.clone());
    let server = TestServer::new(router).expect("test server");
    let pool = sql::pool(&ctx);

    let nobody = server
        .get("/admin-only")
        .add_header("accept", BROWSER)
        .await;
    assert_eq!(nobody.status_code(), 303);
    assert_eq!(nobody.header("location"), "/login");

    let member = user(&ctx, "member@example.com", true).await;
    let (secret, _) = Session::open(pool, member.id, "Firefox on a Mac")
        .await
        .expect("opened");
    let refused = server
        .get("/admin-only")
        .add_header("accept", BROWSER)
        .add_header("cookie", format!("session={secret}"))
        .await;
    assert_eq!(refused.status_code(), 403);
    assert!(refused.text().contains("Not allowed here"), "the 403 page");

    let admin = user(&ctx, "admin@example.com", true).await;
    Role::Admin.give(pool, admin.id).await.expect("given");
    let (secret, _) = Session::open(pool, admin.id, "Firefox on a Mac")
        .await
        .expect("opened");
    let page = server
        .get("/admin-only")
        .add_header("accept", BROWSER)
        .add_header("cookie", format!("session={secret}"))
        .await;
    assert_eq!(page.status_code(), 200);
    assert_eq!(page.text(), "Hello, Ana");
}

/// Everyone who signs up is a member.
#[tokio::test]
#[serial]
async fn a_sign_up_is_a_member() {
    request::<App, _, _>(|_request, ctx| async move {
        let member = user(&ctx, "new@example.com", false).await;
        assert_eq!(
            Role::of(sql::pool(&ctx), member.id).await.expect("read"),
            Role::Member
        );
    })
    .await;
}
