//! A lane's boot, run alone before its tests (e2e/run.sh): a session opened
//! and the home page loaded until it is ready, nothing more. A lane whose
//! browser or driver can't start says so here within a minute, not test by
//! test, each waiting out its time (as an islands app on this stack found).

use app_e2e::browse;

#[tokio::test]
async fn the_lane_opens_its_home_page() {
    browse(|page| async move {
        page.open("/").await?;
        assert!(
            !page.driver.title().await?.is_empty(),
            "the home page's title"
        );
        Ok(())
    })
    .await;
}
