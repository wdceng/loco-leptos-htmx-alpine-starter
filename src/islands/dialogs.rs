//! A dialog's buttons (`opens` and `closes`, views/parts.rs), armed by the
//! `Navigation` island on every page: a button naming its dialog
//! (`commandfor`) shows it modal, the focus on its Cancel (`data-cancel`),
//! or closes it. The browser's own command is taken over in every browser,
//! so each opens it the same way; the low bar has none (Chrome 135,
//! Firefox 144, Safari 26.2 first), so there only this opens a dialog. A
//! press outside an open dialog closes it where the browser doesn't
//! (`closedby`), only where the press began outside too, as the browser's
//! own: a drag from a field's words that ends on the veil keeps it. As an
//! islands app on this stack opens its dialogs.

use std::cell::Cell;

use leptos::{prelude::*, wasm_bindgen::JsCast};

use super::focus_quietly;

thread_local! {
    /// Whether the latest press began outside an open dialog's box.
    static BEGAN_OUTSIDE: Cell<bool> = const { Cell::new(false) };
}

/// A press begins: outside an open dialog's box, or not.
pub(super) fn press_began(ev: &web_sys::Event) {
    BEGAN_OUTSIDE.set(outside(ev).is_some());
}

/// A click: a dialog's button runs its command, the browser's own held
/// back; a press begun and ended outside an open dialog closes it.
pub(super) fn pressed(ev: &web_sys::Event) {
    let began_outside = BEGAN_OUTSIDE.replace(false);
    let Some(target) = ev
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    else {
        return;
    };
    if target.is_instance_of::<web_sys::HtmlDialogElement>() {
        if began_outside && let Some(dialog) = outside(ev) {
            dialog.close();
        }
        return;
    }
    let Some(button) = target.closest("button[commandfor][command]").ok().flatten() else {
        return;
    };
    let Some(dialog) = button
        .get_attribute("commandfor")
        .and_then(|named| document().get_element_by_id(&named))
        .and_then(|dialog| dialog.dyn_into::<web_sys::HtmlDialogElement>().ok())
    else {
        return;
    };
    ev.prevent_default();
    match button.get_attribute("command").as_deref() {
        Some("show-modal") => show(&dialog),
        Some("close") => dialog.close(),
        _ => {}
    }
}

/// `dialog` shown modal from its top, the focus on its Cancel, moved
/// without scrolling.
fn show(dialog: &web_sys::HtmlDialogElement) {
    if dialog.open() || dialog.show_modal().is_err() {
        return;
    }
    dialog.set_scroll_top(0);
    if let Some(cancel) = dialog
        .query_selector("[data-cancel]")
        .ok()
        .flatten()
        .and_then(|cancel| cancel.dyn_into::<web_sys::HtmlElement>().ok())
    {
        focus_quietly(&cancel);
    }
}

/// The open dialog a press landed on outside its box: its veil, which the
/// browser draws behind a modal dialog and counts as the dialog. A press
/// inside lands on the box's content, or on its padding, inside its edges.
fn outside(ev: &web_sys::Event) -> Option<web_sys::HtmlDialogElement> {
    let dialog = ev.target()?.dyn_into::<web_sys::HtmlDialogElement>().ok()?;
    let press = ev.dyn_ref::<web_sys::MouseEvent>()?;
    let edges = dialog.get_bounding_client_rect();
    let (x, y) = (f64::from(press.client_x()), f64::from(press.client_y()));
    let inside = x >= edges.left() && x <= edges.right() && y >= edges.top() && y <= edges.bottom();
    (dialog.open() && !inside).then_some(dialog)
}
