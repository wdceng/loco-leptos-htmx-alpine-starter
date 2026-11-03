//! The parts of `views/parts.rs` in a real browser, on their page
//! (`/e2e/parts`, the browser tests' environment alone): a dialog opens
//! from its button, and Escape and its Cancel each close it, the browser's
//! own popover with no script of the site's; a fold opens from its heading.

use app_e2e::{Key, browse};
use serde_json::Value;

const PARTS: &str = "/e2e/parts";

/// Whether the dialog is open.
const OPEN: &str = "return document.getElementById('parts-dialog').matches(':popover-open')";

#[tokio::test]
async fn a_dialog_opens_and_escape_or_cancel_closes_it() {
    browse(|page| async move {
        page.open(PARTS).await?;
        assert_eq!(
            page.eval(OPEN).await?,
            Value::Bool(false),
            "closed at first"
        );

        page.press("button[popovertarget=parts-dialog]").await?;
        page.until("the dialog open", OPEN).await?;
        page.press("#parts-dialog button[popovertargetaction=hide]")
            .await?;
        page.until(
            "the dialog closed by Cancel",
            &OPEN.replace("return ", "return !"),
        )
        .await?;

        // Escape: a keyboard's, where the browser has one.
        if page.browser.is_safari() || page.browser.screen().is_some() {
            return Ok(());
        }
        page.press("button[popovertarget=parts-dialog]").await?;
        page.until("the dialog open again", OPEN).await?;
        page.driver
            .action_chain()
            .send_keys(Key::Escape)
            .perform()
            .await?;
        page.until(
            "the dialog closed by Escape",
            &OPEN.replace("return ", "return !"),
        )
        .await?;
        Ok(())
    })
    .await;
}

#[tokio::test]
async fn a_fold_opens_from_its_heading() {
    browse(|page| async move {
        page.open(PARTS).await?;
        assert_eq!(
            page.eval("return document.querySelector('details').open")
                .await?,
            Value::Bool(false)
        );
        page.press("details summary").await?;
        page.until(
            "the fold open",
            "return document.querySelector('details').open",
        )
        .await?;
        Ok(())
    })
    .await;
}
