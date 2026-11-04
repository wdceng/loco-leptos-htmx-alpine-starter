// For htmx 4 only (htmx 2 names its events differently: htmx:afterSwap).
// Keeps Leptos islands alive in HTML htmx swaps in. The one hand-written
// script in the template: plumbing, no page logic (CLAUDE.md, "Islands,
// htmx and Alpine").

// 1. Chrome 150 to 156 drop the comments Leptos hydrates by in
//    parseHTMLUnsafe (https://github.com/bigskysoftware/htmx/issues/4147);
//    without it htmx falls back to DOMParser, which keeps them, in every
//    browser, as an islands app on this stack does it. Passing options
//    instead, the htmx maintainer's way, throws in a Chrome without the
//    options overload (145 on Android), and every swap failed. Stays past
//    htmx 4.0.0 too: 4.0.1's fix for #4147 calls it with that second
//    argument, which Chrome 124 to 145 refuse, every swap then failing
//    (htmx #4182); gone, htmx never calls it. Remove only once Chrome 156
//    is below the low bar and an htmx release has #4182's fix.
delete Document.parseHTMLUnsafe;

// In blocks, so these consts stay out of the page's global scope.
{
  // 2. Wake islands htmx swaps in, as Leptos's own router does. The
  //    islands the page loads with are Leptos's first pass: this script
  //    runs in <head>, before any is parsed, so the observer below marks
  //    them as the parser adds them, the wasm not yet there to wake them.
  const wake = () => {
    const hydrate = window.__hydrateIsland;
    for (const el of document.querySelectorAll("leptos-island")) {
      if (el.$$hydrated) continue;
      el.$$hydrated = true;
      // A failure while the wasm still starts: the first pass takes it.
      if (hydrate) hydrate(el, el.dataset.component).catch(() => {});
    }
  };
  document.addEventListener("htmx:after:swap", wake);

  // 3. htmx 4 doesn't watch the page: process hx-* added later, and wake
  //    islands added later (the swap event may fire on a removed element).
  const HX =
    "[hx-get],[hx-post],[hx-put],[hx-patch],[hx-delete],[hx-action],[hx-trigger]";
  new MutationObserver((records) => {
    let islands = false;
    for (const record of records) {
      for (const node of record.addedNodes) {
        if (node.nodeType !== 1) continue;
        // Not where htmx was blocked: the page works as plain links then.
        if (window.htmx && (node.matches(HX) || node.querySelector(HX)))
          window.htmx.process(node);
        islands ||=
          node.matches("leptos-island") ||
          !!node.querySelector("leptos-island");
      }
    }
    if (islands) wake();
  }).observe(document.documentElement, { childList: true, subtree: true });
}
