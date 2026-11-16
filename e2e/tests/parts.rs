//! The parts of `views/parts.rs` in a real browser, on their page
//! (`/e2e/parts`, the browser tests' environment alone): a dialog opens
//! modal from its button, the page under it inert and the focus on Cancel,
//! and Cancel, a press outside and Escape each close it, a drag begun
//! inside keeping it, the focus handed back to its button; a fold opens
//! from its heading.

use app_e2e::{Key, browse, dialog_open, opener};
use serde_json::Value;

const PARTS: &str = "/e2e/parts";

/// The parts page's dialog.
const DIALOG: &str = "parts-dialog";

#[tokio::test]
async fn a_dialog_opens_modal_and_cancel_a_press_outside_or_escape_closes_it() {
    browse(|page| async move {
        let open = dialog_open(DIALOG);
        let closed = open.replace("return ", "return !");
        page.open(PARTS).await?;
        assert_eq!(
            page.eval(&open).await?,
            Value::Bool(false),
            "closed at first"
        );

        page.press(&opener(DIALOG)).await?;
        page.until("the dialog open", &open).await?;
        page.until(
            "the focus on Cancel",
            "return document.activeElement?.matches('#parts-dialog [data-cancel]')",
        )
        .await?;
        // The page under it inert: its opener takes no focus.
        assert_eq!(
            page.eval(&format!(
                "document.querySelector(\"{}\").focus();
                 return document.activeElement?.matches('#parts-dialog [data-cancel]')",
                opener(DIALOG)
            ))
            .await?,
            Value::Bool(true),
            "the page under it inert"
        );
        page.press("#parts-dialog [data-cancel]").await?;
        page.until("the dialog closed by Cancel", &closed).await?;

        // A press on the veil closes it where the browser does not
        // (`closedby`); a drag begun inside keeps it (islands/dialogs.rs).
        page.press(&opener(DIALOG)).await?;
        page.until("the dialog open again", &open).await?;
        assert_eq!(
            page.eval(
                "const d = document.getElementById('parts-dialog');
                 const box = d.getBoundingClientRect();
                 d.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true,
                     clientX: box.left + box.width / 2, clientY: box.top + box.height / 2 }));
                 d.dispatchEvent(new MouseEvent('click', { bubbles: true, clientX: 2, clientY: 2 }));
                 return d.open"
            )
            .await?,
            Value::Bool(true),
            "a drag from inside closed it"
        );
        page.eval(
            "const d = document.getElementById('parts-dialog');
             d.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, clientX: 2, clientY: 2 }));
             d.dispatchEvent(new MouseEvent('click', { bubbles: true, clientX: 2, clientY: 2 }))",
        )
        .await?;
        page.until("the dialog closed by a press outside", &closed)
            .await?;

        // Escape: a keyboard's, where the browser has one.
        if page.browser.is_safari() || page.browser.screen().is_some() {
            return Ok(());
        }
        page.press(&opener(DIALOG)).await?;
        page.until("the dialog open a third time", &open).await?;
        page.driver
            .action_chain()
            .send_keys(Key::Escape)
            .perform()
            .await?;
        page.until("the dialog closed by Escape", &closed).await?;
        Ok(())
    })
    .await;
}

/// Opened from the keyboard, the dialog takes the focus onto its Cancel,
/// and Escape hands it back to the button that opened it, the browser's
/// own modal dialog (WAI-ARIA's dialog pattern).
#[tokio::test]
async fn a_dialog_hands_the_focus_back_to_its_button() {
    browse(|page| async move {
        if page.browser.is_safari() || page.browser.screen().is_some() {
            page.skip("no keyboard to open and close the dialog with");
            return Ok(());
        }
        page.open(PARTS).await?;
        page.eval(&format!(
            "document.querySelector(\"{}\").focus()",
            opener(DIALOG)
        ))
        .await?;
        page.driver
            .action_chain()
            .send_keys(Key::Enter)
            .perform()
            .await?;
        page.until("the dialog open", &dialog_open(DIALOG)).await?;
        page.until(
            "the focus on Cancel",
            "return document.activeElement?.matches('#parts-dialog [data-cancel]')",
        )
        .await?;
        page.driver
            .action_chain()
            .send_keys(Key::Escape)
            .perform()
            .await?;
        page.until(
            "the focus back on the opening button",
            &format!(
                "return !document.getElementById('parts-dialog').open
                     && document.activeElement?.matches(\"{}\")",
                opener(DIALOG)
            ),
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
