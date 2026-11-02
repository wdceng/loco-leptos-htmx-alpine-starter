// Which side of a turned phone keeps the safe-area inset, on <html> as
// data-safe-area ("left" or "right", the side of the phone's camera, an
// iPhone's Dynamic Island; "none"; nothing upright), so the stylesheet
// keeps that side alone clear of it (--safe-area-left/right,
// style/tailwind.css): iOS reports the inset on both sides of an iPhone on
// its side. The side by the screen's angle, which Safari and Chrome agree
// on (WebKit bug 254863: the direction of `type` they do not), and the
// camera being at a phone's top, the same for any phone. "none" where the
// page doesn't reach the glass: such a browser keeps its view clear of the
// camera itself and still reports the inset, which the page would add a
// second time (Chrome on an iPhone, 724px of 852, Chromium issue
// 465922618; measured in an islands app on this stack, 2026-10-27).
// Before the first paint, so nothing moves on load, again at every turn and
// every resize (Chrome changes its width as the page scrolls), and on a
// page the browser brings back from its cache. A helper with no page
// logic, as the islands script is (CLAUDE.md).

// In a block, so these consts stay out of the page's global scope.
{
  // Which angle puts the camera on which side, seen on an iPhone in
  // Safari, Chrome, Firefox and Edge (an islands app on this stack,
  // 2026-10-28).
  const SIDE = { 90: "left", 270: "right" };
  const mark = () => {
    const turned = SIDE[screen.orientation?.angle];
    // The page reaches the glass only where it is as wide as the screen's
    // long side, give or take a rounding.
    const glass = innerWidth >= Math.max(screen.width, screen.height) - 2;
    // On its side by the window's own shape, which turns with the page,
    // where the angle can come later: "none" needs no angle, so a browser
    // that keeps clear of the camera itself gets it at the turn's first
    // resize, never the inset for a moment (an islands app on this stack,
    // 2026-10-28: Chrome showed the inset, then took it back).
    const sideways = innerWidth > innerHeight;
    const side = sideways && (glass ? turned : "none");
    if (side) document.documentElement.dataset.safeArea = side;
    else delete document.documentElement.dataset.safeArea;
  };
  mark();
  screen.orientation?.addEventListener("change", mark);
  addEventListener("resize", mark);
  addEventListener("pageshow", mark);
}
