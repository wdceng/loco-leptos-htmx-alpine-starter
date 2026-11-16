//! The parts of `views/parts.rs` in a real browser, on their page
//! (`/e2e/parts`, the browser tests' environment alone): a dialog opens
//! from its button, and Escape and its Cancel each close it, the focus
//! handed back to its button, the browser's
//! own popover with no script of the site's, Cancel inside a form as a real
//! dialog's is; a fold opens from its heading.

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

        // Cancel inside the dialog's form, owned by none: Safari 17 runs no
        // popovertarget on a button a form owns (views/parts.rs).
        assert_eq!(
            page.eval("return document.querySelector('#parts-dialog button[popovertargetaction=hide]').form")
                .await?,
            Value::Null,
            "Cancel owned by no form"
        );
        page.press("button[popovertarget=parts-dialog]").await?;
        page.until("the dialog open", OPEN).await?;
        // The focus in the dialog: the opening button, left with it, is
        // outside what `aria-modal` lets a screen reader reach.
        page.until(
            "the focus in the dialog",
            "return document.getElementById('parts-dialog').contains(document.activeElement)",
        )
        .await?;
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

/// Opened from the keyboard, the dialog takes the focus onto its Cancel,
/// and Escape hands it back to the button that opened it, the browser's
/// own popover (WAI-ARIA's dialog pattern).
#[tokio::test]
async fn a_dialog_hands_the_focus_back_to_its_button() {
    browse(|page| async move {
        if page.browser.is_safari() || page.browser.screen().is_some() {
            page.skip("no keyboard to open and close the dialog with");
            return Ok(());
        }
        page.open(PARTS).await?;
        page.eval("document.querySelector('button[popovertarget=parts-dialog]').focus()")
            .await?;
        page.driver
            .action_chain()
            .send_keys(Key::Enter)
            .perform()
            .await?;
        page.until("the dialog open", OPEN).await?;
        page.until(
            "the focus on Cancel",
            "return document.activeElement?.matches('#parts-dialog button[popovertargetaction=hide]')",
        )
        .await?;
        page.driver
            .action_chain()
            .send_keys(Key::Escape)
            .perform()
            .await?;
        page.until(
            "the focus back on the opening button",
            "return !document.getElementById('parts-dialog').matches(':popover-open')
                 && document.activeElement?.matches('button[popovertarget=parts-dialog]')",
        )
        .await
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
