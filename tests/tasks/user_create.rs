use app::{
    app::App,
    models::{roles::Role, users},
    sql,
};
use loco_rs::{task, testing::prelude::*};

use loco_rs::boot::run_task;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_can_run_user_create() {
    let boot = boot_test::<App>().await.unwrap();

    let email = "test@example.com";
    let user = users::Model::find_by_email(&boot.app_context.db, email).await;
    assert!(user.is_err());

    let vars = task::Vars::from_cli_args(vec![
        ("email".to_string(), email.to_string()),
        ("name".to_string(), "Test User".to_string()),
        ("password".to_string(), "securepassword".to_string()),
    ]);
    assert!(
        run_task::<App>(&boot.app_context, Some(&"user:create".to_string()), &vars)
            .await
            .is_ok()
    );

    let deliveries = boot.app_context.mailer.unwrap().deliveries();
    assert_eq!(deliveries.count, 1, "Exactly one email should be sent");

    let user = users::Model::find_by_email(&boot.app_context.db, email).await;
    assert!(user.is_ok());
}

/// `role:admin` makes an admin (src/models/roles.rs); a word that is no
/// role creates nobody.
#[tokio::test]
#[serial]
async fn the_task_gives_the_role_it_is_told() {
    let boot = boot_test::<App>().await.unwrap();
    let ctx = &boot.app_context;

    let vars = |email: &str, role: &str| {
        task::Vars::from_cli_args(vec![
            ("email".to_string(), email.to_string()),
            ("name".to_string(), "Admin User".to_string()),
            ("password".to_string(), "securepassword".to_string()),
            ("role".to_string(), role.to_string()),
        ])
    };
    run_task::<App>(
        ctx,
        Some(&"user:create".to_string()),
        &vars("admin@example.com", "admin"),
    )
    .await
    .expect("created");
    let admin = users::Model::find_by_email(&ctx.db, "admin@example.com")
        .await
        .expect("the admin");
    assert_eq!(
        Role::of(sql::pool(ctx), admin.id).await.expect("read"),
        Role::Admin
    );

    let err = run_task::<App>(
        ctx,
        Some(&"user:create".to_string()),
        &vars("owner@example.com", "owner"),
    )
    .await
    .expect_err("no such role");
    assert_eq!(err.to_string(), "role:owner is not a role");
    assert!(
        users::Model::find_by_email(&ctx.db, "owner@example.com")
            .await
            .is_err()
    );
}

/// A role typed without its colon (`role admin`) reaches the task unread,
/// as Loco 1.3 hands over every argument: the user is a member, the least a
/// role can do, and the task's summary says so (`Role: member`).
#[tokio::test]
#[serial]
async fn a_role_without_its_colon_makes_a_member() {
    let boot = boot_test::<App>().await.unwrap();
    let ctx = &boot.app_context;
    let vars = task::Vars::from_args(
        [
            "email:typo@example.com",
            "name:Typo",
            "password:securepassword",
            "role",
            "admin",
        ]
        .map(String::from)
        .to_vec(),
    );
    run_task::<App>(ctx, Some(&"user:create".to_string()), &vars)
        .await
        .expect("created");
    let user = users::Model::find_by_email(&ctx.db, "typo@example.com")
        .await
        .expect("the user");
    assert_eq!(
        Role::of(sql::pool(ctx), user.id).await.expect("read"),
        Role::Member
    );
}

/// Each of the three arguments is asked for by name, and none missing
/// creates anybody or sends a mail.
#[tokio::test]
#[serial]
async fn a_missing_argument_is_named_and_creates_nobody() {
    let boot = boot_test::<App>().await.unwrap();
    let ctx = &boot.app_context;
    let all = [
        ("email", "missing@example.com"),
        ("name", "Missing"),
        ("password", "securepassword"),
    ];
    for left_out in ["email", "name", "password"] {
        let vars = task::Vars::from_cli_args(
            all.iter()
                .filter(|(key, _)| *key != left_out)
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
        );
        let err = run_task::<App>(ctx, Some(&"user:create".to_string()), &vars)
            .await
            .expect_err("refused");
        assert_eq!(err.to_string(), format!("{left_out} is mandatory"));
    }
    assert!(
        users::Model::find_by_email(&ctx.db, "missing@example.com")
            .await
            .is_err()
    );
    let deliveries = ctx.mailer.as_ref().expect("stub mailer").deliveries();
    assert_eq!(deliveries.count, 0, "no mail");
}

#[tokio::test]
#[serial]
async fn test_user_email_already_exists() {
    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    let email = "user1@example.com";

    let vars = task::Vars::from_cli_args(vec![
        ("email".to_string(), email.to_string()),
        ("name".to_string(), "Test User".to_string()),
        ("password".to_string(), "securepassword".to_string()),
    ]);
    let err = run_task::<App>(&boot.app_context, Some(&"user:create".to_string()), &vars)
        .await
        .expect_err("err");

    assert_eq!(
        err.to_string(),
        "Failed to create user. err: Entity already exists"
    );

    let deliveries = boot.app_context.mailer.unwrap().deliveries();
    assert_eq!(deliveries.count, 0, "No email should be sent");
}
