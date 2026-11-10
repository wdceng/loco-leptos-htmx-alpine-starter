//! Islands: the only components that are compiled into the browser bundle
//! and hydrated there. Everything else in the app is server-rendered HTML.
//!
//! This module is compiled in both builds: it is the one module in `lib.rs`
//! without the `ssr` gate, so the server renders an island's HTML and the
//! browser half brings it to life. A component marked `#[island]` anywhere
//! else, in `views/` for instance, compiles for the server only and never
//! hydrates.
//!
//! Rules that keep the bundle small:
//! - Keep an island a thin wrapper and pass the real content in as
//!   `children`: children are rendered on the server only and never reach
//!   the wasm.
//! - Props are plain data (`String`, numbers, structs deriving `Serialize`
//!   and `Deserialize`); nothing server-only (database, config) inside.
//! - Check both halves: `cargo clippy --all-targets` and
//!   `cargo clippy --lib --target wasm32-unknown-unknown --no-default-features --features hydrate`.
//!
//! Leptos wraps an island in `<leptos-island>` and its children in
//! `<leptos-children>`; `style/tailwind.css` sets both to `display: contents`
//! so they do not disturb grid or flex layouts.
//!
//! Lazy islands (`#[island(lazy)]`) are split out of the main bundle into a
//! chunk of their own, fetched only by a page that renders one. That needs
//! `--split` on every cargo-leptos command; without it no island on any page
//! runs, and the server refuses to boot (`src/assets.rs`). Make an island
//! lazy when only some pages use it, and render it only where it is needed:
//! one rendered on every page is fetched on every page.
//!
//! Which tool for which job: htmx for whatever needs the server (a page
//! moved to in place, a form sent, a part fetched), Alpine's CSP build for
//! small state in the page (open or closed, focus, keys), an island only
//! for what neither can (Rust shared with the server, a browser API Alpine
//! can't reach, state that must outlive the server's answers).
//!
//! **An island is freed when htmx takes it off the page:** Leptos 0.9 keeps
//! its state behind `$$dispose` on its `<leptos-island>` element (#4953),
//! which `public/js/htmx-islands.js` calls for every island a swap removes;
//! its owner's `on_cleanup` runs then, where an island removes its window
//! listeners.
//!
//! **No Leptos timer helper in an island** (`set_timeout`, `set_interval`,
//! `request_animation_frame`, `request_idle_callback`): Leptos 0.9 hands
//! each call's callback to the owner current at the call, and in an islands
//! page that is the root, kept as long as the tab, so a call per scroll or
//! per move would stay in memory for the whole visit. A job scheduled again
//! and again makes its callback once ([`Job`]). A test below refuses the
//! helpers.

use std::{cell::RefCell, time::Duration};

use leptos::{
    prelude::*,
    wasm_bindgen::{JsCast, JsValue, closure::Closure},
};
use web_sys::{
    ScrollBehavior, ScrollRestoration, ScrollToOptions,
    js_sys::{Function, Object, Reflect},
};

mod shrink;

/// The page's region, the part a move between pages replaces (the shell's
/// `#page`, views/layout.rs), by its id.
pub const PAGE_REGION: &str = "page";

/// The polite live region outside the page's region that reads the new
/// page's title after a move (views/layout.rs).
pub const PAGE_ANNOUNCE: &str = "page-announce";

/// The statuses the app answers with a page of its own, which a move in
/// place shows in the region: a request refused (400, 405, 413, 415), a
/// page another role's (403, `Admin`), a page not found (404), a slow
/// answer (408), the server's own fault (500). Anything else is loaded
/// plainly ([`Navigation`]): the 429, a page outside the shell, and a
/// gateway's page in front of the app, a restart's 502 above all.
pub const PAGE_STATUSES: [u16; 8] = [400, 403, 404, 405, 408, 413, 415, 500];

/// The page's `<meta>` naming the build it loaded (views/layout.rs).
pub const BUILD_META: &str = "build";

/// The answer header naming the build it came from, which every page sends
/// (src/render.rs) and a move's answer is compared by: one name for both
/// halves.
pub const BUILD_HEADER: &str = "x-build";

/// The key a history entry's state keeps its page's scroll under, beside
/// htmx's own keys.
const SCROLL_KEY: &str = "pageScroll";

/// htmx's mark on an entry it restores in place, written with the scroll:
/// htmx 4.0.1 replaces a state without it, the scroll with it
/// (docs/decisions.md, 2026-11-03).
const HTMX_KEY: &str = "htmx";

/// How long the page rests after a scroll before its entry keeps it: one
/// write when the scrolling stops, not one a frame (Safari refuses more
/// than a hundred history writes in thirty seconds).
const SCROLL_REST: Duration = Duration::from_millis(150);

thread_local! {
    /// Where the page Back or Forward is bringing back was scrolled, until
    /// htmx has swapped it in, or its request failed.
    static RESTORING: RefCell<Option<f64>> = const { RefCell::new(None) };
    /// The page's region as the last look found it: a different element
    /// after a swap is a move between pages.
    static REGION: RefCell<Option<web_sys::Element>> = const { RefCell::new(None) };
    /// The build this page loaded ([`BUILD_META`]): the pages move in place,
    /// so it is the build of the whole visit's code.
    static BUILD: RefCell<Option<String>> = const { RefCell::new(None) };
    /// [`Navigation`]'s scheduled jobs, made when it starts in the browser
    /// and dropped with it.
    static JOBS: RefCell<Option<Jobs>> = const { RefCell::new(None) };
}

/// Something the page schedules again and again: its callback made once,
/// and the call still queued, which a new one replaces, so the last one
/// runs. Scheduled through the browser's own `setTimeout` and
/// `requestAnimationFrame`, never Leptos's helpers (the module's doc).
/// Dropped, it cancels what is queued, then frees its callback.
struct Job {
    run: Closure<dyn Fn()>,
    queued: std::cell::Cell<Option<Queued>>,
}

/// A [`Job`]'s call the browser holds: a timeout's id or a frame's.
#[derive(Clone, Copy)]
enum Queued {
    Timeout(i32),
    Frame(i32),
}

impl Job {
    fn new(run: fn()) -> Self {
        Self {
            run: Closure::new(run),
            queued: std::cell::Cell::new(None),
        }
    }

    /// Runs once `delay` has passed, the call still queued replaced.
    fn after(&self, delay: Duration) {
        self.cancel();
        let ms = i32::try_from(delay.as_millis()).unwrap_or(i32::MAX);
        if let Ok(id) = window().set_timeout_with_callback_and_timeout_and_arguments_0(
            self.run.as_ref().unchecked_ref(),
            ms,
        ) {
            self.queued.set(Some(Queued::Timeout(id)));
        }
    }

    /// Runs before the next repaint, the call still queued replaced.
    fn next_frame(&self) {
        self.cancel();
        if let Ok(id) = window().request_animation_frame(self.run.as_ref().unchecked_ref()) {
            self.queued.set(Some(Queued::Frame(id)));
        }
    }

    /// The call still queued, if any, cancelled; one that ran already is
    /// no call the browser holds, so cancelling it does nothing.
    fn cancel(&self) {
        match self.queued.take() {
            Some(Queued::Timeout(id)) => window().clear_timeout_with_handle(id),
            Some(Queued::Frame(id)) => {
                let _ = window().cancel_animation_frame(id);
            }
            None => {}
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// [`Navigation`]'s jobs.
struct Jobs {
    /// The page's scroll kept in its history entry once scrolling rests.
    rest: Job,
    /// A move's new page asked to its top again a frame later.
    top: Job,
    /// The new page's title read out a tick after a move.
    announce: Job,
}

/// `schedule` given [`Navigation`]'s jobs, where they exist: in the browser,
/// once it has started.
fn with_jobs(schedule: impl FnOnce(&Jobs)) {
    JOBS.with_borrow(|jobs| {
        if let Some(jobs) = jobs {
            schedule(jobs);
        }
    });
}

/// What a move between pages in place needs that htmx doesn't do, on every
/// page, outside the region a move replaces (views/layout.rs). Ported from
/// the `Chores` island of the project this template shares its stack with,
/// where each part was found and proven by its browser tests:
///
/// - **The focus and the screen reader.** A swap is no page load to a
///   screen reader. Nothing keeps the focus into the swap, so htmx brings
///   none back; once the new region is in, the focus goes to `<main>` (or
///   stays on a field the new page focuses, `autofocus`), and the new title
///   is read out by a live region ([`PAGE_ANNOUNCE`]); the page Back or
///   Forward brings too.
/// - **The scroll on Back and Forward.** htmx 4 swaps the earlier page back
///   with no scroll of its own, and the browser's restoration ran on the
///   page still showing. So the browser's is off, each history entry keeps
///   its page's scroll, and the page Back brings is scrolled there.
/// - **A deploy while a tab is open.** A move whose answer names a newer
///   build ([`BUILD_HEADER`]) loads plainly instead of swapping, so the next
///   page runs the new build's htmx, Alpine and islands, never an old wasm
///   against a new page's islands.
/// - **An answer the app never sends as a page** (a gateway's 502, the 429)
///   loads plainly, so the browser shows what came instead of a click doing
///   nothing; so does any error answering Back or Forward, whose request
///   htmx sends from `<body>`, out of the region's `hx-status`.
///
/// The listeners are removed with the island's owner, and its jobs dropped.
#[island]
pub fn Navigation() -> impl IntoView {
    // Effects run in the browser only, after hydration.
    Effect::new(move |_| {
        take_scroll_over();
        REGION.set(document().get_element_by_id(PAGE_REGION));
        BUILD.set(
            document()
                .query_selector(&format!("meta[name={BUILD_META}]"))
                .ok()
                .flatten()
                .and_then(|meta| meta.get_attribute("content")),
        );
        JOBS.set(Some(Jobs {
            rest: Job::new(keep_scroll),
            top: Job::new(to_the_top),
            announce: Job::new(announce_title),
        }));
    });
    let fresher = window_event_listener_untyped("htmx:before:response", |ev| {
        if let Some(page) = newer_build(&ev) {
            ev.prevent_default();
            let _ = window().location().assign(&page);
        }
    });
    let swapped = window_event_listener_untyped("htmx:after:swap", |_| {
        if moved() {
            with_jobs(|jobs| jobs.top.next_frame());
            arrive();
        }
    });
    // Back's swap keeps the region's element, so it is no move to
    // `moved`: the scroll is put back once htmx has settled the swap.
    let settled = window_event_listener_untyped("htmx:after:settle", |_| restored());
    // The entry Back or Forward brings back is the current one by now:
    // its scroll is read before htmx asks for its page.
    let restoring = window_event_listener_untyped("htmx:before:history:restore", |_| {
        RESTORING.set(Some(kept_scroll().unwrap_or(0.0)));
    });
    // The entry a move leaves keeps where its page stood, before the
    // move pushes the next one; not while a page is being brought back.
    let leaving = window_event_listener_untyped("htmx:before:request", |_| {
        if RESTORING.with_borrow(Option::is_none) {
            keep_scroll();
        }
    });
    let scrolled = window_event_listener_untyped("scroll", |_| {
        with_jobs(|jobs| jobs.rest.after(SCROLL_REST));
    });
    // htmx gives the focus back to the element with the same `id` in
    // the new page, and `arrive` then moves it to `<main>`: a field
    // styled on its focus (a search that opens) would open and close
    // over the new page. Nothing is focused before a move's swap; a
    // field the new page marks `autofocus` still gets the focus.
    let unfocused = window_event_listener_untyped("htmx:before:swap", |ev| {
        if into_region(&ev) {
            blur_focused();
        }
    });
    let refused = window_event_listener_untyped("htmx:response:error", load_plainly);
    // No answer at all to Back's request: the page Back went to, loaded
    // plainly, says why.
    let failed = window_event_listener_untyped("htmx:error", |_| {
        restore_failed();
    });
    // A picture chosen in a field marked `data-shrink` shrunk before
    // its form goes (shrink.rs): on every page, as this island is.
    let chosen = window_event_listener_untyped("change", |ev| {
        if let Some(field) = ev
            .target()
            .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
            .filter(|field| field.has_attribute("data-shrink"))
        {
            shrink::chosen(field);
        }
    });
    on_cleanup(move || {
        chosen.remove();
        fresher.remove();
        swapped.remove();
        settled.remove();
        restoring.remove();
        leaving.remove();
        scrolled.remove();
        unfocused.remove();
        refused.remove();
        failed.remove();
        // Their queued calls cancelled, their callbacks freed.
        JOBS.take();
    });
}

/// Whether a swap brought a new page's region: the region is another
/// element than the last look found. A part swapped inside the region
/// leaves it the same element.
fn moved() -> bool {
    let region = document().get_element_by_id(PAGE_REGION);
    let same = REGION.with_borrow(|last| match (last, &region) {
        (Some(last), Some(region)) => last.is_same_node(Some(region.as_ref())),
        _ => false,
    });
    REGION.set(region.clone());
    !same && region.is_some()
}

/// The page's scroll the page's own: the browser's restoration off, so
/// Back never scrolls the page still showing; and a page loaded whole on an
/// entry that kept its scroll (a reload) scrolled there, unless the visitor
/// has scrolled already.
fn take_scroll_over() {
    if let Ok(history) = window().history() {
        let _ = history.set_scroll_restoration(ScrollRestoration::Manual);
    }
    if let Some(top) = kept_scroll()
        && window().scroll_y().is_ok_and(|y| y == 0.0)
    {
        scroll_to(top);
    }
}

/// Where the current history entry's page was scrolled, if it kept it.
fn kept_scroll() -> Option<f64> {
    let state = window().history().ok()?.state().ok()?;
    Reflect::get(&state, &JsValue::from_str(SCROLL_KEY))
        .ok()?
        .as_f64()
}

/// The current entry keeps the page's scroll in its state, htmx's own keys
/// and its mark ([`HTMX_KEY`]) kept beside it (Back restores only the
/// entries htmx marked). Best effort: a browser that refuses the write
/// keeps the entry as it was.
fn keep_scroll() {
    let (Ok(history), Ok(y)) = (window().history(), window().scroll_y()) else {
        return;
    };
    let kept = Object::new();
    if let Some(state) = history
        .state()
        .ok()
        .and_then(|state| state.dyn_into::<Object>().ok())
    {
        Object::assign(&kept, &state);
    }
    if Reflect::set(&kept, &JsValue::from_str(SCROLL_KEY), &JsValue::from_f64(y)).is_ok()
        && Reflect::set(&kept, &JsValue::from_str(HTMX_KEY), &JsValue::TRUE).is_ok()
    {
        let _ = history.replace_state(&kept, "");
    }
}

/// After a move, the page Back or Forward brought scrolled where its entry
/// kept it, then arrived as a link's page does: the focus on `<main>`, its
/// title read out. Back's swap keeps the region's element, so `moved` never
/// sees it, and the focus went with the page it replaced. A move by a link
/// starts at the top, as its swap put it.
fn restored() {
    if let Some(top) = RESTORING.take() {
        scroll_to(top);
        arrive();
    }
}

/// Back or Forward whose page didn't come: an error's status, which htmx
/// leaves unswapped there (its request comes from `<body>`, above the
/// region's `hx-status`), or no answer at all. The address is the entry's
/// already, so the page is loaded plainly there and the browser shows what
/// answered; the scroll kept for it is let go. Whether it was such a move.
fn restore_failed() -> bool {
    if RESTORING.take().is_none() {
        return false;
    }
    let _ = window().location().reload();
    true
}

/// A move's new page, which htmx's swap puts at its top (`show:top`, the
/// shell's `hx-boost`), asked there again a frame later where it isn't
/// ([`Jobs::top`]): iOS's WebKit drops a scroll right after the swap now
/// and then, and the new page showed at its foot; the second always takes
/// (an islands app on this stack, a probe on the iPhone, 2026-11-03). An
/// address with a `#` is left where it went.
fn to_the_top() {
    let anchored = window()
        .location()
        .hash()
        .is_ok_and(|hash| !hash.is_empty());
    if !anchored && window().scroll_y().is_ok_and(|y| y != 0.0) {
        scroll_to(0.0);
    }
}

/// The page scrolled to `top` at once, never a smooth glide from wherever
/// the swap left it.
fn scroll_to(top: f64) {
    let options = ScrollToOptions::new();
    options.set_top(top);
    options.set_behavior(ScrollBehavior::Instant);
    window().scroll_to_with_scroll_to_options(&options);
}

/// The focus and the screen reader's line after a move: on a field the new
/// page focuses (`autofocus`, which htmx honours), else on `<main>`,
/// without scrolling; then the new title into the live region, a tick
/// later: htmx 4 fires `htmx:after:swap` before it sets the new page's
/// title, so read at once it would announce the page just left (found by
/// the browser check, 2026-10-18).
fn arrive() {
    let on_autofocus = document()
        .active_element()
        .is_some_and(|active| active.has_attribute("autofocus"));
    if !on_autofocus
        && let Some(main) = document()
            .get_element_by_id("content")
            .and_then(|main| main.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = main.focus_with_options(&options);
    }
    with_jobs(|jobs| jobs.announce.after(Duration::ZERO));
}

/// The page's title into the live region ([`PAGE_ANNOUNCE`]), a tick after
/// a move ([`arrive`], [`Jobs::announce`]).
fn announce_title() {
    if let Some(region) = document().get_element_by_id(PAGE_ANNOUNCE) {
        region.set_text_content(Some(&document().title()));
    }
}

/// A link's answer that htmx left unswapped, a status not among
/// [`PAGE_STATUSES`], loaded plainly: the link's own address, so the
/// browser shows whatever answered. A link alone (a GET): a form's post is
/// never sent twice, and an element with its own request keeps its own
/// handling. Back's answer with any error loads plainly too
/// ([`restore_failed`]).
fn load_plainly(ev: web_sys::Event) {
    if restore_failed() {
        return;
    }
    let detail = Reflect::get(&ev, &JsValue::from_str("detail")).unwrap_or(JsValue::UNDEFINED);
    let ctx = Reflect::get(&detail, &JsValue::from_str("ctx")).unwrap_or(JsValue::UNDEFINED);
    let status = Reflect::get(&ctx, &JsValue::from_str("response"))
        .and_then(|response| Reflect::get(&response, &JsValue::from_str("status")))
        .ok()
        .and_then(|status| status.as_f64());
    let link = Reflect::get(&ctx, &JsValue::from_str("sourceElement"))
        .ok()
        .and_then(|source| source.dyn_into::<web_sys::HtmlAnchorElement>().ok())
        .filter(|link| !link.has_attribute("hx-get"));
    let shown =
        status.is_some_and(|status| PAGE_STATUSES.iter().any(|&page| f64::from(page) == status));
    if let Some(link) = link
        && !shown
    {
        let _ = window().location().assign(&link.href());
    }
}

/// The page a move leads to, where its answer comes from a newer build than
/// this page loaded: a deploy since. Asked before the answer is read
/// (`htmx:before:response`), so the caller cancels the swap and loads that
/// page plainly. A move alone: an answer going into the page's region, a
/// link's or Back's (a GET) or a form's whose post led on to a page (a
/// redirect followed, its GET safe to load again); never a post answered in
/// place, which would be sent twice. Nothing where the answer names no
/// build or this page has none.
fn newer_build(ev: &web_sys::Event) -> Option<String> {
    let field = |of: &JsValue, name: &str| Reflect::get(of, &JsValue::from_str(name)).ok();
    let ctx = field(&field(ev.as_ref(), "detail")?, "ctx")?;
    let response = field(&ctx, "response")?;
    let headers = field(&response, "headers")?;
    let get: Function = field(&headers, "get")?.dyn_into().ok()?;
    let theirs = get
        .call1(&headers, &JsValue::from_str(BUILD_HEADER))
        .ok()?
        .as_string()?;
    let ours = BUILD.with_borrow(Clone::clone)?;
    if theirs == ours {
        return None;
    }
    let raw: web_sys::Response = field(&response, "raw")?.dyn_into().ok()?;
    let method = field(&field(&ctx, "request")?, "method")?
        .as_string()
        .unwrap_or_default();
    (into_region(ev) && (method.eq_ignore_ascii_case("get") || raw.redirected())).then(|| raw.url())
}

/// Whether htmx's event `ev` is about a swap into the page's region: its
/// `detail.ctx.target`, the element or its `#id` selector.
fn into_region(ev: &web_sys::Event) -> bool {
    let field = |of: &JsValue, name: &str| Reflect::get(of, &JsValue::from_str(name)).ok();
    let Some(target) = field(ev.as_ref(), "detail")
        .and_then(|detail| field(&detail, "ctx"))
        .and_then(|ctx| field(&ctx, "target"))
    else {
        return false;
    };
    target
        .dyn_ref::<web_sys::Element>()
        .is_some_and(|target| target.id() == PAGE_REGION)
        || target
            .as_string()
            .is_some_and(|target| target == format!("#{PAGE_REGION}"))
}

/// Nothing focused: whatever has the focus loses it.
fn blur_focused() {
    if let Some(focused) = document()
        .active_element()
        .and_then(|focused| focused.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = focused.blur();
    }
}

/// Shows whether the browser half runs: the server renders "not running",
/// and the island switches to "running" once its chunk has loaded and
/// hydrated. Lazy, so it also proves the split build works. Home page only.
#[island(lazy)]
pub fn IslandsCheck() -> impl IntoView {
    let (running, set_running) = signal(false);
    // Effects run in the browser only, after hydration.
    Effect::new(move |_| set_running.set(true));
    view! {
        <p class="mt-8 text-sm text-ink-muted">
            {move || {
                if running.get() {
                    "Islands: running. This line is a lazy island, loaded on its own."
                } else {
                    "Islands: not running."
                }
            }}
        </p>
    }
}

/// Reads the source, so native only: the browser tests build for wasm32.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::{fs, path::Path};

    /// The islands module's files: `src/islands.rs` and every file under
    /// `src/islands/`, where an island would compile in both halves too.
    fn sources() -> Vec<std::path::PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = vec![root.join("islands.rs")];
        let mut folders = vec![root.join("islands")];
        while let Some(folder) = folders.pop() {
            let Ok(entries) = fs::read_dir(&folder) else {
                continue;
            };
            for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
                if path.is_dir() {
                    folders.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    found.push(path);
                }
            }
        }
        found
    }

    /// Leptos's timer helpers, called bare as the prelude brings them.
    const HELPERS: [&str; 4] = [
        "set_timeout(",
        "set_interval(",
        "request_animation_frame(",
        "request_idle_callback(",
    ];

    /// The helper a line of code calls bare, if any: the browser's own
    /// methods (`window().request_animation_frame(…)`, a [`super::Job`]'s)
    /// are no call of it, a `.` or a longer name before the helper's.
    /// Comments aside.
    fn bare_helper(line: &str) -> Option<&'static str> {
        let code = line.split("//").next().unwrap_or_default();
        HELPERS.into_iter().find(|helper| {
            code.match_indices(helper).any(|(start, _)| {
                !code[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c == '.' || c == '_' || c.is_alphanumeric())
            })
        })
    }

    /// No island schedules through Leptos's timer helpers: each call's
    /// callback would stay in the tab's memory for the whole visit (the
    /// module's doc), which no other test sees. Read in every file of the
    /// module, `src/islands/` too, up to its tests, which name the helpers.
    #[test]
    fn no_island_schedules_through_leptos_timer_helpers() {
        for path in sources() {
            let source =
                fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let code = source
                .lines()
                .take_while(|line| !line.contains("mod tests"));
            for (at, line) in code.enumerate() {
                if let Some(helper) = bare_helper(line) {
                    panic!(
                        "{}:{}: `{helper}` is Leptos's helper; schedule through a `Job`",
                        path.display(),
                        at + 1
                    );
                }
            }
        }
    }

    /// The check finds a bare call and leaves the browser's own methods.
    #[test]
    fn the_timer_check_finds_a_bare_call() {
        assert_eq!(
            bare_helper("    set_timeout(|| {}, Duration::ZERO);"),
            Some("set_timeout(")
        );
        assert!(bare_helper("let _ = request_animation_frame(f);").is_some());
        assert!(bare_helper("window().request_animation_frame(f)").is_none());
        assert!(
            bare_helper("window().set_timeout_with_callback_and_timeout_and_arguments_0(f, 0)")
                .is_none()
        );
        assert!(bare_helper("// set_timeout( in a comment").is_none());
        assert!(bare_helper("jobs.rest.after(SCROLL_REST)").is_none());
    }
}
