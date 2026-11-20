// The globals the helpers in public/js/ read, for `tsc` alone
// (jsconfig.json); no browser loads this file.

interface HTMLElementTagNameMap {
  // Leptos's island element, so `querySelectorAll` finds `HTMLElement`s.
  "leptos-island": HTMLElement;
}

interface Element {
  // Leptos 0.9's: frees an island's state, its owner's `on_cleanup` run.
  $$dispose?: () => void;
  // htmx-islands.js's mark: the island was handed to Leptos to wake.
  $$hydrated?: boolean;
}

interface Window {
  // Leptos's, once the wasm has loaded (island_script.js): wakes one island.
  __hydrateIsland?: (island: HTMLElement, component: string | undefined) => Promise<void>;
  // htmx 4, where it loaded: the one call the helpers make.
  htmx?: { process(element: Element): void };
}
