# The look

Tailwind's defaults, the colour roles in `style/tailwind.css`, and the
parts in `src/views/parts.rs`. Read before adding anything that floats
or covers.

## What floats over what

One scale, lowest first. Every `z-` in the code is one of these.

| Step | What stands there |
|---|---|
| none | the page itself |
| `z-20` | the skip link while it has focus (`src/views/layout.rs`) |
| the top layer | a dialog (`Dialog`, a native popover), over everything, with no `z-` at all |

- **A new part that floats takes a step here first.** Between two steps,
  leave room: the next is `z-10` under the skip link.
- **Popovers need no `z-`:** the browser puts them in the top layer.

## The parts

| Part | Component | Notes |
|---|---|---|
| A dialog over the page | `Dialog`, `DialogCancel` | A native popover: Escape, a tap outside and Cancel close it, with no script of the site's |
| The main button of a step that can't be undone | `DANGER_BUTTON` | `BUTTON`'s shape in the danger colour |
| A button that leaves without doing the step | `QUIET_BUTTON` | A dialog's Cancel |
| A part folded under its heading | `Fold` | The native `<details>` |
| A count beside what it counts | `Badge` | The number to the eye, the number and its words to a screen reader |

All of them are on `/e2e/parts`, which the browser tests alone can open:
their health checks run there (tap targets, no sideways scroll, the
accessibility audit).
