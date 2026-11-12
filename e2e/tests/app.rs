//! The Android app's own tests, the app lane's alone (`e2e/run.sh app`):
//! what the app does that no browser lane sees, its start page handing
//! over to the site, and another site's link leaving the app for the
//! system while the app stays on its page (src-tauri/src/lib.rs,
//! `destination`). Each starts the app afresh and attaches to its webview;
//! the system's side through adb (e2e/src/device.rs). Skipped in every
//! other lane. As an islands app on this stack tests its own.

use std::time::Duration;

use app_e2e::{Page, WebDriverResult, app_package, browse_app};

/// The page at `path` on the lane's site, its islands awake: where the
/// app's start page hands over to.
async fn arrives(page: &Page, path: &str) -> WebDriverResult<()> {
    let site = page.url(path);
    page.until_within(
        &format!("the site's {path}"),
        &format!("return location.origin + location.pathname === {site:?}"),
        Duration::from_secs(60),
    )
    .await?;
    page.ready().await
}

/// The app opened from the home screen: its start page, then the site's
/// home page within a minute, in the app's webview (the `wv` Android
/// writes in a webview's user agent).
#[tokio::test]
async fn the_app_opens_the_site_from_its_start_page() {
    browse_app(|page| async move {
        arrives(&page, "/").await?;
        let inside = page
            .eval("return navigator.userAgent.includes('; wv)')")
            .await?;
        assert_eq!(inside, true, "the site inside the app's webview");
        Ok(())
    })
    .await;
}

/// With no network the start page says so, and goes on to the site once
/// the network is back (src-tauri/offline/index.html): the start page
/// opened again in the app's webview, the device's airplane mode on, and
/// off again whatever happened.
#[tokio::test]
async fn the_start_page_says_no_internet_and_goes_on_once_its_back() {
    browse_app(|page| async move {
        arrives(&page, "/").await?;
        page.airplane(true).await?;
        let offline = async {
            page.eval("setTimeout(() => location.assign('http://tauri.localhost/index.html'))")
                .await?;
            page.until_within(
                "the start page saying no internet",
                "const problem = document.getElementById('problem');
                 return !!problem && !problem.hidden
                     && document.getElementById('problem-title').textContent === 'No internet connection'",
                Duration::from_secs(30),
            )
            .await
        }
        .await;
        page.airplane(false).await?;
        offline?;
        arrives(&page, "/").await
    })
    .await;
}

/// Another site's link goes to the system, Android's browser on top, and
/// the app stays on its page: Back brings the app back as it was (src/lib.rs,
/// `destination`; `links.sh` taps the rest by hand).
#[tokio::test]
async fn another_sites_link_goes_to_the_system_and_the_app_stays() {
    browse_app(|page| async move {
        arrives(&page, "/").await?;
        page.add_link("https://example.com/").await?;
        page.press("#e2e-link").await?;
        let app = app_package();
        let browser = page
            .until_on_top(
                "another app",
                |top| !top.is_empty() && top != app,
                Duration::from_secs(45),
            )
            .await?;
        assert!(
            browser.contains("chrome") || browser.contains("browser"),
            "a browser on top: {browser}"
        );
        page.system_back().await?;
        page.until_on_top("the app", |top| top == app, Duration::from_secs(20))
            .await?;
        assert_eq!(
            page.path().await?,
            "/",
            "the app still on the page the link was on"
        );
        Ok(())
    })
    .await;
}
