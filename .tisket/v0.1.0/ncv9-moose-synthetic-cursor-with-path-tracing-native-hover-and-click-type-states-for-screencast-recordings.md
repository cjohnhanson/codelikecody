---
title: "moose: synthetic cursor with path tracing, native hover, and click/type states for screencast recordings"
status: discovery
priority:
assignee:
labels: [moose, ux]
depends_on: []
created: "2026-03-26T12:56:04Z"
updated: "2026-03-26T12:56:04Z"
---

## Problem

Moose screencast recordings (`moose record`) show pages changing with no
indication of what's being interacted with. No cursor, no hover states,
no click feedback. The `highlight` and `--annotate` features flash for a
single frame — invisible in video playback. Recordings are unwatchable as
demos or documentation.

A human operating a browser naturally produces visual signals: the cursor
moves across the page, elements react with hover styles as the cursor
crosses them, clicks produce a focused/pressed state. Moose automation
produces none of this because CDP commands execute instantly with no
input simulation along the path.

## Design

Opt-in via `--cursor` flag (or `MOOSE_CURSOR` env). When enabled, every
interaction command gets a synthetic cursor overlay and path-traced
movement before acting.

### Cursor element

Injected into the page as a `position: fixed` SVG/CSS element on top of
everything. Re-injected on `Page.frameNavigated` to survive navigation.
The daemon holds current position in state.

### Visual states

- **Idle** — small dot/arrow, semi-transparent
- **Moving** — animating along the path
- **Hovering** — glow or size increase on reaching the target
- **Clicking** — pulse/shrink animation (the "press" feel)
- **Typing** — sits next to the active input

### Path tracing with native hover

Movement from current position to target is NOT a straight CSS animation.
Instead:

1. Sample points along the path (every ~10-20px)
2. For each point, dispatch `Input.dispatchMouseEvent` with
   `type: "mouseMoved"` via CDP
3. The injected cursor element tracks the position visually
4. The browser fires native `:hover` pseudo-classes, `mouseenter`/
   `mouseleave` events, cursor changes, tooltips — because it thinks
   a real mouse is moving
5. Interactive elements along the path (links, buttons — anything with
   a ref) get their real hover styles as the cursor passes over them
6. Speed scales with distance: `clamp(150, distance * 0.8, 500)` ms

### Action sequence

For each interaction command (click, fill, type, check, etc.):

1. Compute target element center via `getBoundingClientRect()`
2. Trace path from current position to target (dispatching mouseMoved)
3. Dwell on target (~150ms) — hover state visible in recording
4. Set cursor to click/type state (~100ms)
5. Execute the actual CDP action
6. Return cursor to idle state

### What this doesn't change

Without `--cursor`, moose behaves exactly as it does now. Zero overhead,
instant execution, no injected DOM elements.

## Open Questions

- Should the path be a straight line or slightly curved for a more
  natural feel? Straight is simpler; a bezier with slight arc looks
  more human.
- Should the cursor element be an SVG arrow (classic mouse pointer)
  or a simpler dot? Arrow is more recognizable but harder to render
  well at various DPIs.
- How to handle scroll-then-click? If the target is off-screen,
  moose scrolls to it first. The cursor should probably appear at
  the viewport edge and move in after the scroll completes.
- Does the cursor interact with the page's own `cursor: pointer`
  CSS? The synthetic cursor is a DOM overlay — the browser's actual
  cursor is hidden in headless mode. The overlay should probably
  change its own appearance when over a `cursor: pointer` element.

## Why It Matters

Screencast recordings are the primary way to produce demos, documentation
walkthroughs, and visual test evidence. Without cursor feedback they're
useless for those purposes. The highlight command exists but is too
transient for video. This makes moose recordings watchable.
