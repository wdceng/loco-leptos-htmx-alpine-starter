//! The template's parts beyond a page's text: a dialog, a fold and a
//! badge, in Tailwind's default look, each the one way its job is done
//! (docs/one-job.md). HTML first, no island: a dialog is a native popover
//! the browser opens, closes on Escape or a tap outside and keeps in the
//! top layer, with or without the islands bundle; a fold is `<details>`.
//! What floats over what: `docs/look.md`. As an islands app on this stack
//! builds its own, in its look.

use leptos::prelude::*;

use crate::views::layout::{BUTTON, LEAD, TITLE};

/// The dialog's box: centred by the browser in the top layer over a dim
/// veil, a form's width, 384px at most, 16px inside the screen; taller than
/// the screen, it scrolls inside itself, never the page behind it.
pub const DIALOG: &str = "m-auto w-[min(100%-2rem,24rem)] max-h-[calc(100dvh-2rem)] overflow-y-auto overscroll-contain rounded-md border border-ink-muted bg-surface p-6 text-ink backdrop:bg-gray-950/40";

/// A dialog's heading.
pub const DIALOG_TITLE: &str = "text-xl font-bold tracking-tight";

/// A button beside a form's main one that leaves without doing it: a
/// dialog's "Cancel". The main button's height, its outline alone.
pub const QUIET_BUTTON: &str =
    "mt-6 min-h-11 rounded-md border border-ink-muted px-4 py-2 font-semibold text-ink";

/// The main button of a step that can't be undone: `BUTTON`'s shape in the
/// danger colour, its busy look too.
pub const DANGER_BUTTON: &str = "mt-6 min-h-11 rounded-md bg-danger px-4 py-2 font-semibold text-white transition-opacity busy:pointer-events-none busy:opacity-60 busy:delay-200";

/// A small dialog, `id` what its opening button's `popovertarget` names and
/// `title` its heading, `children` what it holds, a form as a rule, its
/// buttons ending with [`DialogCancel`]. A native popover (`popover="auto"`):
/// the browser centres it over a veil and closes it on Escape, a tap
/// outside or the cancel button, no script of the site's. The focus goes in
/// only to an `autofocus` element inside, so one is always marked: the
/// first field, or Cancel where there is none, never the step that can't be
/// undone. Without it the focus stayed on the opening button, outside what
/// `aria-modal` lets a screen reader reach (Chrome, Firefox, Edge, the
/// browser tests, 2026-11-04).
#[component]
pub fn Dialog(
    #[prop(into)] id: String,
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    let heading = format!("{id}-title");
    let labelled_by = heading.clone();
    view! {
        <div
            id=id
            popover="auto"
            role="dialog"
            aria-modal="true"
            aria-labelledby=labelled_by
            class=DIALOG
        >
            <h2 id=heading class=DIALOG_TITLE>
                {title}
            </h2>
            {children()}
        </div>
    }
}

/// The button that closes the dialog `target` without doing its step.
///
/// No form owner (`form=""`): it sits inside the dialog's form as a rule,
/// and Safari 17 runs no `popovertarget` on a button inside a form, so
/// Cancel did nothing on iOS 17.5, the low bar; the others follow the spec
/// either way (an islands app on this stack, a probe, 2026-11-04).
/// `autofocus` where the dialog has no field to take the focus ([`Dialog`]).
#[component]
pub fn DialogCancel(
    #[prop(into)] target: String,
    #[prop(optional)] autofocus: bool,
) -> impl IntoView {
    view! {
        <button
            type="button"
            form=""
            popovertarget=target
            popovertargetaction="hide"
            autofocus=autofocus
            class=QUIET_BUTTON
        >
            "Cancel"
        </button>
    }
}

/// A part folded away under its heading, `summary`, until opened: the
/// native `<details>`, opened and closed by the browser, its state found
/// by a page's search. 44px tall to tap.
#[component]
pub fn Fold(#[prop(into)] summary: String, children: Children) -> impl IntoView {
    view! {
        <details class="mt-6 border-b border-ink-muted">
            <summary class="flex min-h-11 cursor-pointer items-center font-semibold">
                {summary}
            </summary>
            <div class="pb-4">{children()}</div>
        </details>
    }
}

/// Every part on one page, for the browser tests (`/e2e/parts`,
/// controllers/parts.rs): a badge, a dialog with its opening button, a
/// fold. The dialog's Cancel inside a form, as a real dialog's is (Safari
/// 17, [`DialogCancel`]); the form is never sent.
#[component]
pub fn PartsPage() -> impl IntoView {
    const DIALOG_ID: &str = "parts-dialog";
    view! {
        <h1 class=TITLE>"Parts"</h1>
        <p class=LEAD>"Messages " <Badge count=3 what="unread" /></p>
        <button type="button" popovertarget=DIALOG_ID class=BUTTON>
            "Open the dialog"
        </button>
        <Dialog id=DIALOG_ID title="A dialog">
            <form method="post" hx-sync="this:drop">
                <p class="mt-4">"What it asks."</p>
                <DialogCancel target=DIALOG_ID autofocus=true />
            </form>
        </Dialog>
        <Fold summary="A fold">
            <p>"What it holds."</p>
        </Fold>
    }
}

/// A count beside what it counts, `what` read after it by a screen reader
/// ("3 unread"): the number to the eye, both to the ear.
#[component]
pub fn Badge(count: u32, #[prop(into)] what: String) -> impl IntoView {
    view! {
        <span class="inline-flex min-w-6 items-center justify-center rounded-full bg-primary px-1.5 text-sm font-semibold text-white">
            {count}
            <span class="sr-only">{format!(" {what}")}</span>
        </span>
    }
}
