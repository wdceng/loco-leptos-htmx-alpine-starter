//! The site with one of its scripts refused, as a blocker, a firewall or a
//! failed download would, in Chrome and Edge, which can refuse a file
//! ([`Page::block`]): links and forms are plain underneath (CLAUDE.md,
//! "Islands, htmx and Alpine"), so without htmx a link is a page load and
//! a form posts as a page; without Alpine or the WASM, htmx still moves in
//! place and a refused form still shows in the region. The refused file's
//! dependants say so in the console: those errors are expected
//! ([`Page::allow_errors`]). As an islands app on this stack tests it.

use app_e2e::{Page, RESET, WebDriverResult, browse};
use serde_json::Value;

/// Whether this browser can refuse a file; the test skips where not.
fn can_block(page: &Page) -> bool {
    if !page.browser.has_devtools() {
        page.skip("no DevTools: no file to refuse");
    }
    page.browser.has_devtools()
}

/// On the reset page, a password too short sent: the refusal, with its
/// words, in place or as a page of its own as `in_place` says.
async fn refused(page: &Page, in_place: bool) -> WebDriverResult<()> {
    page.mark().await?;
    page.eval(
        "const field = document.getElementById('password');
         field.value = 'short'; field.form.requestSubmit()",
    )
    .await?;
    page.until(
        "the refusal",
        "return document.readyState === 'complete'
             && document.body.innerText.includes('must be 8 to 128')",
    )
    .await?;
    assert_eq!(page.in_place().await?, in_place, "in place: {in_place}");
    Ok(())
}

#[tokio::test]
async fn without_htmx_a_link_and_a_form_are_page_loads() {
    browse(|page| async move {
        if !can_block(&page) {
            return Ok(());
        }
        page.allow_errors();
        page.block(&["*/vendor/htmx-*", "*/vendor/hx-*"]).await?;
        page.load("/").await?;
        assert_eq!(
            page.eval("return typeof window.htmx").await?,
            Value::from("undefined"),
            "htmx refused"
        );
        page.mark().await?;
        page.add_link(RESET).await?;
        page.press("#e2e-link").await?;
        page.until(
            "the reset page loaded",
            &format!(
                "return location.pathname + location.search === {RESET:?}
                     && document.readyState === 'complete'"
            ),
        )
        .await?;
        assert!(!page.in_place().await?, "a page load, htmx refused");
        refused(&page, false).await
    })
    .await;
}

#[tokio::test]
async fn without_alpine_a_move_and_a_refused_form_stay_in_place() {
    browse(|page| async move {
        if !can_block(&page) {
            return Ok(());
        }
        page.allow_errors();
        page.block(&["*/vendor/alpinejs-*"]).await?;
        page.load("/").await?;
        page.until("htmx", "return !!window.htmx").await?;
        assert_eq!(
            page.eval("return typeof window.Alpine").await?,
            Value::from("undefined"),
            "Alpine refused"
        );
        page.mark().await?;
        page.add_link(RESET).await?;
        page.press("#e2e-link").await?;
        page.until(
            "the reset page in place",
            &format!("return location.pathname + location.search === {RESET:?}"),
        )
        .await?;
        assert!(page.in_place().await?, "a move in place without Alpine");
        refused(&page, true).await
    })
    .await;
}

#[tokio::test]
async fn without_the_wasm_a_move_and_a_refused_form_stay_in_place() {
    browse(|page| async move {
        if !can_block(&page) {
            return Ok(());
        }
        page.allow_errors();
        page.block(&["*/pkg/*.wasm*"]).await?;
        page.load("/").await?;
        page.until("htmx and Alpine", "return !!(window.htmx && window.Alpine)")
            .await?;
        page.mark().await?;
        page.add_link(RESET).await?;
        page.press("#e2e-link").await?;
        page.until(
            "the reset page in place",
            &format!("return location.pathname + location.search === {RESET:?}"),
        )
        .await?;
        assert!(page.in_place().await?, "a move in place without the WASM");
        refused(&page, true).await
    })
    .await;
}
