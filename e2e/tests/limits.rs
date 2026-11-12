//! The server's limits that no browser shows: a body slower than the
//! site's time limit cut there ([`slow_post`]).

use app_e2e::{Browser, WebDriverError, browse, slow_post};

/// A request whose body comes slower than the site's 15 s limit
/// (`timeout_request`, config/e2e.yaml) is cut there with a 408, as a
/// phone on a weak network sending a form would be. Sent by hand, since a
/// browser's throttling hands the server the body whole; the server's
/// limit alone, so on one lane.
#[tokio::test]
async fn a_body_slower_than_the_sites_limit_is_cut_at_15_s() {
    browse(|page| async move {
        if page.browser != Browser::Safari {
            page.skip("the server's limit, one lane enough");
            return Ok(());
        }
        let base = page.url("");
        // 20 KB at 1 KB a second, under the 64 KB body limit: 20 s, longer
        // than the limit.
        let body = format!("email={}", "a".repeat(20_000));
        let headers = vec![(
            "Content-Type",
            "application/x-www-form-urlencoded".to_owned(),
        )];
        let answered = tokio::task::spawn_blocking(move || {
            slow_post(&base, "/forgot", &headers, body.as_bytes(), 1_000)
        })
        .await
        .map_err(|e| WebDriverError::RequestFailed(format!("the post's thread: {e}")))?
        .map_err(|e| WebDriverError::RequestFailed(format!("the post: {e}")))?;
        assert!(
            answered.status.contains(" 408 ") && (14.0..20.0).contains(&answered.seconds),
            "not cut at 15 s: {answered:?}"
        );
        Ok(())
    })
    .await;
}
