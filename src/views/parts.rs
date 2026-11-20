//! The template's parts beyond a page's text: a dialog, a fold and a
//! badge, in Tailwind's default look, each the one way its job is done
//! (docs/one-job.md). HTML first: a dialog is the browser's modal
//! `<dialog>`, opened and closed by its buttons' commands, which the
//! `Navigation` island takes over (islands/dialogs.rs); a fold is
//! `<details>`.
//! What floats over what: `docs/look.md`. As an islands app on this stack
//! builds its own, in its look.

use leptos::{
    attr::{
        any_attribute::{AnyAttribute, IntoAnyAttribute},
        custom::custom_attribute,
    },
    prelude::*,
};

use crate::views::{
    layout::{BUTTON, LEAD, TITLE},
    pages,
};

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

/// The attributes of a button that opens `dialog`, the one way a [`Dialog`]
/// opens: the browser's own command, modal (`commandfor`,
/// `command="show-modal"`), which `Navigation` takes over in every browser,
/// the focus then on the dialog's Cancel (islands/dialogs.rs).
pub fn opens(dialog: impl Into<String>) -> Vec<AnyAttribute> {
    commands(dialog.into(), "show-modal")
}

/// The attributes of a button that closes `dialog` ([`opens`]):
/// [`DialogCancel`].
pub fn closes(dialog: impl Into<String>) -> Vec<AnyAttribute> {
    commands(dialog.into(), "close")
}

/// A button's command `command` on `dialog`.
fn commands(dialog: String, command: &'static str) -> Vec<AnyAttribute> {
    vec![
        custom_attribute("commandfor", dialog).into_any_attr(),
        custom_attribute("command", command).into_any_attr(),
    ]
}

/// A small dialog, `id` what its opening button names ([`opens`]) and
/// `title` its heading, `children` what it holds, a form as a rule, its
/// buttons ending with [`DialogCancel`]. The browser's modal `<dialog>`:
/// centred in the top layer over a veil, the page under it inert to the
/// keyboard and a screen reader alike, closed by Escape, a press outside
/// (`closedby`; islands/dialogs.rs where the browser lacks it) or Cancel.
/// Opened, the focus goes to Cancel (islands/dialogs.rs), so Enter never
/// does the step that can't be undone and no field's keyboard comes up on
/// a phone; closed, the browser hands it back to where it was: the opening
/// button when the keyboard opened it (Safari focuses no button a press
/// opened, so there it goes back to the page). No
/// `autofocus` inside: htmx focuses the first `[autofocus]` a swap brings,
/// a closed dialog's included.
#[component]
pub fn Dialog(
    #[prop(into)] id: String,
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    let heading = format!("{id}-title");
    let labelled_by = heading.clone();
    view! {
        <dialog id=id closedby="any" aria-labelledby=labelled_by class=DIALOG>
            <h2 id=heading class=DIALOG_TITLE>
                {title}
            </h2>
            {children()}
        </dialog>
    }
}

/// The button that closes the dialog `target` without doing its step,
/// where the focus goes as the dialog opens (`data-cancel`,
/// islands/dialogs.rs).
#[component]
pub fn DialogCancel(#[prop(into)] target: String) -> impl IntoView {
    view! {
        <button type="button" data-cancel="" {..closes(target)} class=QUIET_BUTTON>
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
/// fold. The dialog's Cancel inside a form, as a real dialog's is; the
/// form is never sent.
#[component]
pub fn PartsPage() -> impl IntoView {
    const DIALOG_ID: &str = "parts-dialog";
    view! {
        <h1 class=TITLE>{pages::PARTS}</h1>
        <p class=LEAD>"Messages " <Badge count=3 what="unread" /></p>
        <button type="button" {..opens(DIALOG_ID)} class=BUTTON>
            "Open the dialog"
        </button>
        <Dialog id=DIALOG_ID title="A dialog">
            <form method="post" hx-sync="this:drop">
                <p class="mt-4">"What it asks."</p>
                <DialogCancel target=DIALOG_ID />
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
