//! Moving between pages in place (docs/architecture.md, "Moves in place"):
//! a link or a form in the shell's `#page` brings the next page's region in
//! with no page load, its lazy island woken, its title, the focus on
//! `<main>` and the title read out; Back brings the page back where it was
//! scrolled; a missing page shows in the region; a newer build and a
//! gateway's error load the page plainly instead (`Navigation`,
//! src/islands.rs). A move fetches no WASM again, starts the new page at
//! its top at once, and a desktop's is asked for at the mouse's press,
//! never at a hover. An answer is changed in the page itself
//! ([`Page::answer_with`]), so every browser runs those.

use std::time::Duration;

use app_e2e::{HOME_LINK, Page, RESET, WebDriverResult, browse};
use serde_json::{Value, json};

/// The home page's title, from a full load.
async fn home_title(page: &Page) -> WebDriverResult<String> {
    page.open("/").await?;
    page.driver.title().await
}

/// From the reset page, the header's link home with htmx's answer made by
/// `answer` ([`Page::answer_with`]): the home page must load whole, not
/// swap in. The browser asks the link's address itself then, which the
/// real server answers.
async fn loads_plainly(page: &Page, answer: &str) -> WebDriverResult<()> {
    page.open(RESET).await?;
    page.mark().await?;
    page.answer_with("/", answer).await?;
    page.press(HOME_LINK).await?;
    page.until(
        "the home page loaded whole",
        "return window.__e2eMark !== true && location.pathname === '/'
             && document.readyState === 'complete'",
    )
    .await?;
    page.ready().await
}

/// The lazy island wakes, and htmx parses with DOMParser in every browser:
/// public/js/htmx-islands.js removes `parseHTMLUnsafe` (htmx #4147), whose
/// options-passing workaround threw on every swap in Chrome 145.
#[tokio::test]
async fn a_full_load_wakes_the_lazy_island() {
    browse(|page| async move {
        page.open("/").await?;
        assert!(page.says("Islands: running").await?, "the lazy island runs");
        assert_eq!(
            page.eval("return typeof Document.parseHTMLUnsafe").await?,
            Value::from("undefined"),
            "parseHTMLUnsafe removed, so htmx uses DOMParser"
        );
        let title = page.driver.title().await?;
        assert!(
            !title.contains("<!>"),
            "no hydration marker in the tab: {title}"
        );
        Ok(())
    })
    .await;
}

#[tokio::test]
async fn a_link_moves_in_place_with_its_title_the_focus_and_the_announcement() {
    browse(|page| async move {
        let title = home_title(&page).await?;
        page.open(RESET).await?;
        page.mark().await?;
        page.move_to(HOME_LINK, "/").await?;
        assert!(page.in_place().await?, "no full load");
        assert!(
            page.says("Islands: running").await?,
            "the lazy island swapped in wakes"
        );
        assert_eq!(page.driver.title().await?, title, "the new page's title");
        assert_eq!(
            page.eval("return document.activeElement?.id").await?,
            Value::from("content"),
            "the focus on <main>"
        );
        // Read out a tick after the swap: htmx sets the title after it.
        page.until(
            "the title read out",
            &format!("return document.getElementById('page-announce').textContent === {title:?}"),
        )
        .await?;
        Ok(())
    })
    .await;
}

/// htmx gives the focus back to an element with the same `id` in the new
/// page; `Navigation` blurs before a move's swap, so no field takes the
/// focus on the way to `<main>`, where a field styled on its focus would
/// open and close over the new page. The link keeps the field's focus when
/// pressed, as a search's suggestion does; a plain link takes the focus
/// itself.
#[tokio::test]
async fn a_move_focuses_no_field_on_its_way() {
    browse(|page| async move {
        page.open(RESET).await?;
        page.mark().await?;
        let again = format!("{RESET}&again=1");
        page.add_link(&again).await?;
        page.eval(
            "window.__focused = [];
             addEventListener('focusin', e => __focused.push(e.target.id || e.target.tagName), true);
             document.getElementById('e2e-link').addEventListener('mousedown', e => e.preventDefault());
             document.getElementById('password').focus()",
        )
        .await?;
        page.move_to("#e2e-link", &again).await?;
        assert!(page.in_place().await?, "no full load");
        assert_eq!(
            page.eval("return __focused").await?,
            json!(["password", "content"]),
            "the field focused by the test, then <main> alone"
        );
        Ok(())
    })
    .await;
}

/// A device's screen can't be made shorter: where the reset page fits it,
/// there is no scroll to put back, and the test is skipped.
#[tokio::test]
async fn back_comes_back_in_place_where_it_was_scrolled() {
    browse(|page| async move {
        // A short window, so the reset page scrolls.
        page.viewport(800, 300).await?;
        page.open(RESET).await?;
        page.mark().await?;
        // A link home at the end of `<main>`, scrolled to, as a person
        // scrolls to what they press (the header's link is above).
        page.add_link("/").await?;
        page.eval("document.getElementById('e2e-link').scrollIntoView({ block: 'end' })")
            .await?;
        let before = page.eval("return window.scrollY").await?;
        let before = before.as_f64().unwrap_or_default();
        if before == 0.0 && page.browser.screen().is_some() {
            page.skip("the reset page fits this device's screen: no scroll to put back");
            return Ok(());
        }
        assert!(before > 0.0, "the reset page scrolls in 300 pixels");
        page.move_to("#e2e-link", "/").await?;
        page.eval("history.back()").await?;
        page.until(
            "the reset page back",
            &format!("return location.pathname + location.search === {RESET:?}"),
        )
        .await?;
        page.settled().await?;
        assert!(page.in_place().await?, "Back in place");
        page.until(
            "the scroll put back",
            &format!("return Math.abs(window.scrollY - {before}) < 2"),
        )
        .await?;
        Ok(())
    })
    .await;
}

/// A link to nothing shows the not-found page in the region; a page
/// answered with a 500 shows in the region too (`PAGE_STATUSES`,
/// src/views/layout.rs), the 500 made in the page ([`Page::answer_with`]).
#[tokio::test]
async fn a_404_and_a_500_are_shown_in_the_region() {
    browse(|page| async move {
        // The 404 asked for, which Chrome logs.
        page.allow_failed_load(404, "/no-such-page");
        page.open("/").await?;
        page.mark().await?;
        page.add_link("/no-such-page").await?;
        page.press("#e2e-link").await?;
        page.until(
            "the not-found page in the region",
            "return document.body.innerText.includes('Page not found')",
        )
        .await?;
        assert!(page.in_place().await?, "the 404 in place");
        page.open("/").await?;
        page.mark().await?;
        page.answer_with(
            RESET,
            "const real = await original(...args);
             const html = (await real.text()).replace('</main>', '<p id=\"e2e-500\">e2e 500</p></main>');
             return new Response(html, { status: 500, headers: { 'Content-Type': 'text/html' } });",
        )
        .await?;
        page.add_link(RESET).await?;
        page.press("#e2e-link").await?;
        page.until(
            "the 500's page in the region",
            "return !!document.getElementById('e2e-500')",
        )
        .await?;
        assert!(page.in_place().await?, "the 500 in place");
        Ok(())
    })
    .await;
}

#[tokio::test]
async fn a_newer_build_loads_the_next_page_plainly() {
    browse(|page| async move {
        loads_plainly(
            &page,
            "const real = await original(...args);
             const headers = new Headers(real.headers);
             headers.set('X-Build', 'e2e-newer');
             return new Response(await real.text(), { status: real.status, headers });",
        )
        .await
    })
    .await;
}

#[tokio::test]
async fn a_refused_form_swaps_in_and_keeps_the_address() {
    browse(|page| async move {
        // The refusal's 400, which Chrome logs.
        page.allow_failed_load(400, "/reset");
        page.open(RESET).await?;
        page.mark().await?;
        // As Enter in the field sends it: the focus in the field.
        page.eval(
            "const field = document.getElementById('password');
             field.value = 'short'; field.focus();
             field.form.requestSubmit()",
        )
        .await?;
        page.until(
            "the refused form in the region",
            "return document.body.innerText.includes('must be 8 to 128')",
        )
        .await?;
        assert!(page.in_place().await?, "no full load");
        assert_eq!(page.path().await?, RESET, "the address stays the page's");
        page.settled().await?;
        assert_eq!(
            page.eval("return document.activeElement?.id").await?,
            Value::from("password"),
            "the focus in the field for the next try, not on <main>"
        );
        Ok(())
    })
    .await;
}

/// A second press while the form is on its way sends nothing: htmx 4's
/// default queues one more request (`queue first`), which would post the
/// reset again with a token the first post used up, and swap "expired"
/// over "changed". The form drops it (`hx-sync`).
#[tokio::test]
async fn a_second_press_sends_no_second_post() {
    browse(|page| async move {
        // The e2e token is no real one: the post is refused as expired.
        page.allow_failed_load(400, "/reset");
        page.open(RESET).await?;
        page.mark().await?;
        // Every POST htmx sends, counted where it leaves: htmx binds
        // `window.fetch` per request, a queued one too.
        page.eval(
            "window.__posts = 0;
             const fetchFirst = window.fetch;
             window.fetch = (...args) => {
                 const method = (args[1]?.method ?? args[0]?.method ?? 'GET').toUpperCase();
                 if (method === 'POST') window.__posts++;
                 return fetchFirst(...args);
             };
             const field = document.getElementById('password');
             field.value = 'long enough to pass';
             field.form.requestSubmit();
             field.form.requestSubmit()",
        )
        .await?;
        page.until(
            "the answer in the region",
            "return !document.querySelector('main form')",
        )
        .await?;
        page.settled().await?;
        // Long enough for a queued second post to have gone out.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert_eq!(
            page.eval("return window.__posts").await?,
            Value::from(1),
            "one press, one post"
        );
        Ok(())
    })
    .await;
}

/// While the form is on its way, slower than 200 ms, it's `aria-busy` and
/// its button takes the `busy:` look; the answer's page holds neither. The
/// answer held back a second and a half in the page
/// ([`Page::answer_with`]), so every browser runs it.
#[tokio::test]
async fn a_sent_form_looks_busy_until_its_answer() {
    browse(|page| async move {
        // The e2e token is no real one: the post is refused as expired.
        page.allow_failed_load(400, "/reset");
        page.open(RESET).await?;
        let button = "document.querySelector('main form [type=submit]')";
        let idle = page
            .text(&format!("return getComputedStyle({button}).opacity"))
            .await?;
        // A slow answer: longer than the look's delay.
        page.answer_with(
            "/reset",
            "await new Promise(done => setTimeout(done, 1500));
             return original(...args);",
        )
        .await?;
        page.eval(
            "const field = document.getElementById('password');
             field.value = 'long enough to pass';
             field.form.requestSubmit()",
        )
        .await?;
        page.until(
            "the form busy and its button's look changed",
            &format!(
                "return document.querySelector('main form')?.getAttribute('aria-busy') === 'true'
                     && getComputedStyle({button}).opacity !== {idle:?}"
            ),
        )
        .await?;
        page.until(
            "the answer in the region",
            "return !document.querySelector('main form')",
        )
        .await?;
        page.settled().await?;
        assert_eq!(
            page.eval("return !!document.querySelector('[aria-busy=true], .htmx-request')")
                .await?,
            Value::Bool(false),
            "nothing busy left after the answer"
        );
        Ok(())
    })
    .await;
}

#[tokio::test]
async fn a_gateway_error_loads_the_link_plainly() {
    browse(|page| async move {
        loads_plainly(
            &page,
            "return new Response('<!doctype html><title>502</title><h1>Bad gateway</h1>',
                 { status: 502, headers: { 'Content-Type': 'text/html' } });",
        )
        .await
    })
    .await;
}

/// A move fetches no WASM again: the main file and the lazy island's come
/// once, at the first load, however many moves follow.
#[tokio::test]
async fn a_move_fetches_no_wasm_again() {
    browse(|page| async move {
        page.open("/").await?;
        page.mark().await?;
        let loaded = page.wasm_settled().await?;
        page.add_link(RESET).await?;
        page.move_to("#e2e-link", RESET).await?;
        page.move_to(HOME_LINK, "/").await?;
        assert!(page.in_place().await?, "every move in place");
        assert!(page.says("Islands: running").await?, "the lazy island back");
        assert_eq!(page.wasm_settled().await?, loaded, "no WASM fetched again");
        Ok(())
    })
    .await;
}

/// A move starts the new page at its top at once (`show:top`,
/// src/views/layout.rs), never passing below where the page stood on the
/// way: every scroll of the move recorded by the page. A device's screen
/// can't be made shorter: where the reset page fits it, skipped.
#[tokio::test]
async fn a_move_starts_at_the_top_at_once_never_below() {
    browse(|page| async move {
        page.viewport(800, 300).await?;
        page.open(RESET).await?;
        page.add_link("/").await?;
        page.eval(
            "document.documentElement.style.scrollBehavior = 'auto';
             document.getElementById('e2e-link').scrollIntoView({ block: 'end' });
             document.documentElement.style.scrollBehavior = ''",
        )
        .await?;
        let from = page.eval("return Math.round(window.scrollY)").await?;
        let from = from.as_i64().unwrap_or_default();
        if from == 0 && page.browser.screen().is_some() {
            page.skip("the reset page fits this device's screen: no scroll to start from");
            return Ok(());
        }
        assert!(from > 0, "the reset page scrolls in 300 pixels");
        // The link's own click: no scroll of the harness's on the way.
        page.eval(
            "window.__scrolls = [];
             addEventListener('scroll', () => window.__scrolls.push(Math.round(scrollY)), { passive: true });
             document.getElementById('e2e-link').click()",
        )
        .await?;
        page.until("home", "return location.pathname === '/'")
            .await?;
        page.settled().await?;
        page.until("home at its top", "return window.scrollY === 0")
            .await?;
        let scrolls = page.eval("return window.__scrolls").await?;
        let deepest = scrolls
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_i64)
            .max()
            .unwrap_or(0);
        assert!(
            deepest <= from,
            "the move passed below where the page stood ({from}): {scrolls}"
        );
        Ok(())
    })
    .await;
}

/// A desktop's move is asked for at the mouse's press, before the click
/// (`PRELOAD`, src/views/layout.rs), and a hover asks nothing: one request
/// a move, the click taking the press's. Counted where the requests leave,
/// `window.fetch` wrapped: Chrome lists a request among the page's
/// resources only once its answer is read, and the press's waits unread
/// for the click (a probe, 2026-10-29). A real mouse's: Safari's driver and
/// the devices skip it.
#[tokio::test]
async fn a_desktop_move_is_asked_for_at_the_press_and_a_hover_asks_nothing() {
    browse(|page| async move {
        if page.browser.is_safari() || page.browser.screen().is_some() {
            page.skip("no real mouse to hover and press with");
            return Ok(());
        }
        page.open("/").await?;
        page.add_link(RESET).await?;
        page.eval(&format!(
            "window.__asked = 0;
             const original = window.fetch.bind(window);
             window.fetch = (...args) => {{
                 const u = new URL(typeof args[0] === 'string' ? args[0] : args[0].url, location.href);
                 if (u.pathname + u.search === {RESET:?}) window.__asked++;
                 return original(...args);
             }}"
        ))
        .await?;
        let asked = "return window.__asked";
        page.hover("#e2e-link").await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(
            page.eval(asked).await?,
            Value::from(0),
            "a hover asks nothing"
        );
        let at = page.hold("#e2e-link").await?;
        page.until("asked at the press", "return window.__asked === 1")
            .await?;
        assert_eq!(
            page.path().await?,
            "/",
            "still home while the button is down"
        );
        page.let_go(at).await?;
        page.until(
            "the reset page",
            &format!("return location.pathname + location.search === {RESET:?}"),
        )
        .await?;
        page.settled().await?;
        assert_eq!(page.eval(asked).await?, Value::from(1), "one request a move");
        Ok(())
    })
    .await;
}
