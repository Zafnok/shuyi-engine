---
id: "3058"
title: "Parked: Screen reader output on the web: the accessibility tree mirrored into ARIA elements under the canvas, and announcements in live regions"
type: feature
milestone: "Engine: accessibility"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2322", "3035"]
nick_input: none
completed:
---

# 3058 — Screen reader output on the web

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`, as 3035's note quotes): a follow-up of the parked screen reader output (3035) and accessibility tree (2322), and a follow-up of parked work is parked, so it starts after both are unparked and done. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2322 builds an `AccessTree` from the engine's views and hands an
`AccessUpdate` (the tree when it changed, and the frame's
announcements) to `Platform::access`. 3035 gives it to the desktop's
screen readers through AccessKit, and turns their requests into UI
actions. It left the web to this ticket.

AccessKit has no adapter for the browser. In a page, a screen reader
reads the DOM, so the tree has to become elements there: each node an
element with an ARIA role, a name and its state, the focus named on the
canvas, and announcements put into ARIA live regions. The canvas's
pixels tell a screen reader nothing.

Today the web build runs on macroquad's loader, with no `wasm-bindgen`
(game ADR-0034, `AGENTS.md` *Environment*). Rust reaches the page only
through a miniquad JS plugin of our own, as `crates/cli/web/gamepad.js`
does for controllers (its Rust side is `crates/app/src/pads/web.rs`).
3034's research, done before this ticket since 3035 needs it, plans a
move of the web build to `wasm-bindgen` in a ticket it writes. This
ticket uses whichever the web build runs on the day (*Scope*).

Three facts about the page shape the design:

- macroquad's loader reads keys in `canvas.onkeydown`
  (`mq_js_bundle.js`), so the game gets keys only while the canvas has
  the DOM focus. The elements must never take it. The focus is named
  with `aria-activedescendant` on the canvas instead: the ARIA way for a
  widget that keeps the focus and points at its active item.
- A `<canvas>` element's children are its fallback content: the browser
  draws none of them and shows all of them to screen readers. So the
  tree lives inside the canvas with no CSS to hide it, and every element
  is a descendant of the canvas, as `aria-activedescendant` requires.
- `role="application"` on the canvas tells a screen reader to pass every
  key to the page (its focus mode), so the player's controls reach the
  game. A `document` or a plain `group` would leave the arrow keys to
  the screen reader.

Only platform backends touch the page (`AGENTS.md` rule 5): the tree is
plain data until here. Keys stay the player's (rule 8): a screen
reader's request becomes a UI action, never a key.

## Nick input

None.

## Scope

**In:**
- **The web build on the day.** If it still runs on macroquad's loader,
  the page side is `crates/cli/web/access.js`, a miniquad plugin of the
  engine's own like `gamepad.js`, and Rust hands it the changes as text
  (below). If the web build has moved to `wasm-bindgen` (the ticket
  3034's research writes for that), there is no plugin: the backend the
  web build runs on then makes the same DOM changes through `web-sys`,
  with the same `Mirror`, and what this scope says about `access.js`,
  its text and its `SHELL` entry doesn't apply. Say which in the
  Completion notes. If the move comes later, it takes
  `access.js` along with the other plugins (3034's plan moves every
  plugin `crates/cli/web` holds by then).
- **The mirror**, `crates/app/src/access.rs` (`pub(crate)`), plain Rust
  compiled on every target so native tests cover it, as `pads.rs` sits
  beside `pads/web.rs`:
  - **The role table**, `aria_role(Role) -> Option<&'static str>`, for
    every `Role` 2322 defines:

    | `Role` | ARIA role |
    | --- | --- |
    | `Window`, `Group` | `group` |
    | `Dialog` | `dialog` |
    | `Menu`, `MenuItem` | `menu`, `menuitem` |
    | `List`, `ListItem` | `listbox`, `option` |
    | `CheckBox` | `checkbox` |
    | `Slider` | `slider` |
    | `TextInput` | `textbox` |
    | `Button` | `button` |
    | `Text` | none: its name is the element's text |

    `List` and `ListItem` are `listbox` and `option`, not `list` and
    `listitem`: only the first pair can hold the focus through
    `aria-activedescendant`. `Role` is `#[non_exhaustive]`, so a role
    added later and missing here is a `group`, and the table's test
    lists every role there is on the day.
  - **A node's attributes:** `id` is `shuyi-a-` and its `NodeId` in 16
    hex digits; `aria-label` is its name (every role but `Text`), with
    its value after a comma ("Text speed, Fast"); a `Slider` has
    `aria-valuenow`, `aria-valuemin` and `aria-valuemax`, and its value
    as `aria-valuetext` in place of the comma; a `TextInput`'s value is
    its element's text. Then `aria-disabled="true"`, `aria-checked` on
    a check box, and `aria-selected="true"` on a selected item. An
    attribute with nothing to say is left off.
  - **`Mirror { last: Option<AccessTree> }`** and
    `Mirror::update(&mut self, &AccessUpdate) -> Vec<DomOp>`. It
    compares the new tree with the last one by `NodeId`, and gives only
    the changes:
    - `Create { id, role }` for a new node, followed by its attributes;
    - `Attr { id, name, value: Option<String> }` for an attribute set,
      changed, or removed (`None`);
    - `Text { id, text }` for a `Text` node's or a `TextInput`'s text;
    - `Order { parent: Option<NodeId>, children: Vec<NodeId> }` when a
      node's children are new or in a new order (`None` is the canvas);
    - `Remove { id }` for a node that's gone;
    - `Focus(Option<NodeId>)` when the focus moved;
    - `Announce { assertive: bool, text: String }` for each
      announcement, in order.

    `tree: None` (nothing changed, 2322) with no announcements gives no
    ops, and the page isn't touched. Moving the menu cursor gives one
    `Focus` and nothing else, as 2322 changes nothing else.
  - **The text** `access.js` reads, `encode(&[DomOp]) -> String`: one
    op a line, its fields split by a tab, a node id in 16 hex digits,
    `-` for none, and in a text `\`, tab and line feed written `\\`,
    `\t` and `\n`. Simple enough for one short function in JS.
- **`crates/app/src/access/web.rs`** (wasm32 only):
  - declares `access.js`'s functions, as `pads/web.rs` declares
    `gamepad.js`'s: `shuyi_access_apply(ptr, len)` as an `unsafe fn`,
    wrapped in a safe `apply(&str)` with a `SAFETY` comment (the plugin
    copies the bytes out of `wasm_memory` before it returns and keeps no
    pointer), and the clicks' functions (below), which take and give
    numbers only and are `safe fn`;
  - exports `shuyi_access_crate_version()`, which the loader compares
    with `access.js`'s `version`, as for `gamepad.js`;
  - `#[allow(unsafe_code)]` on those items only, as in `pads/web.rs`.
- **`Platform::access` on the macroquad backend**
  (`crates/app/src/backend.rs`): on wasm32 it runs the `Mirror` and
  hands `encode`'s text to `apply` when there are ops. On native it
  keeps 2322's default: the desktop is 3035's.
- **`access.js`**, with a header saying it is the engine's own code and
  that its Rust side is `crates/app/src/access/web.rs`:
  - when the plugin registers, it sets `role="application"` and an
    `aria-label` of `document.title` on `#glcanvas`, and adds two live
    regions after it in `<body>`, `aria-live="polite"` and
    `aria-live="assertive"`. They are visually hidden (one pixel,
    clipped), never `display: none` or `aria-hidden`, which a screen
    reader skips. They exist from the page's start, since screen readers
    watch only the regions that were there before they changed;
  - `shuyi_access_apply` decodes the text and runs each op: a `<div>`
    created inside the canvas, attributes set or removed, a text kept
    as the element's first child before its children's elements,
    children put in order with `appendChild`, an element removed, the
    focus written to the canvas's `aria-activedescendant` (removed for
    none). An announcement is a new `<div>` in its region, so the same
    words twice are said twice, and only the last few are kept;
  - no element gets a `tabindex`, and nothing calls `focus()`: the DOM
    focus stays on the canvas, and the game keeps its keys;
  - a `click` on an element (what a screen reader sends when the player
    activates an item it reads) queues the node's id.
    `shuyi_access_clicks()` moves the queue into a list Rust reads and
    returns its length, and `shuyi_access_click_hi(i)` and
    `shuyi_access_click_lo(i)` give an id's two 32-bit halves.
- **Requests to actions:** each frame the backend reads the clicks and
  gives each one to the game the way 3035 gives AccessKit's requests
  (its Completion notes name the path), through 3035's mapping. A click
  on a node is a `Focus` on it, then a `Click`, so a click on a menu
  item moves the cursor there and confirms it through
  `UiAction::Confirm`. A click on a node the last tree doesn't hold is
  dropped. If 3035 left the mapping in the winit backend's crate, first
  move the part that doesn't touch AccessKit beside the tree in
  `shuyi-ui`, with its tests, so both backends share one copy.
- **The tree from the first frame.** A browser doesn't tell a page
  whether a screen reader runs. If 3035 builds the tree only once a
  screen reader asks (AccessKit's activation), the web backend asks
  from the first frame. Measure the frame time of Hello, Grid's options
  screen in the web build with the tree and without it, and write both
  in the Completion notes.
- **The shell:** `access.js` joins `SHELL` (`crates/cli/src/web.rs`),
  and `index.html` loads it after the bundle and before the game's
  `load(...)`, as it loads `gamepad.js`. `crates/cli/web/README.md`
  lists it under *Files*.
- **From 3039 (done)** (the web build's notices): `access.js` (when
  this ticket writes one) gets a row in `SHELL_NOTICES`
  (`crates/cli/src/web.rs`), the table of `SHELL`'s files, as the
  engine's own, beside `gamepad.js` (its test fails until it does).
- **Licenses:** no new crate, and `access.js` is all ours, so
  `THIRD_PARTY_ASSETS.md` doesn't change. If the work borrows from an
  upstream file after all, that file gets a license header and a line
  there.
- **A check in the browser pane:** Hello, Grid's web build (`cargo shuyi
  web` in `examples/hello-grid`) served locally. The pane's reading of
  the page (its accessibility tree) shows the title menu, then the
  options screen, with their items, values and the focused item, and
  the focus follows the menu cursor. The PR shows what it read.
- **A manual check with NVDA** on Windows, with Chrome or Firefox: NVDA
  reads Hello, Grid's title menu and options and their values, and
  follows the cursor as the player moves it with the game's own
  controls; activating an item from NVDA's browse mode confirms it, if
  browse mode reaches the items. Do it if a Windows machine with NVDA
  is at hand, and say in the Completion notes whether it was done, with
  which browsers.
- **Docs:** a section "The web" in `docs/accessibility.md` (2322's
  guide, beside the desktop's from 3035): what the page holds, why the
  focus stays on the canvas, what a click does, and which checks were
  done.
- **What changes for the game:** nothing to write. Its web build gets
  `access.js` from `shuyi web` at its next pin bump. The new items are
  `pub(crate)`; if a public item changes anyway, the Completion notes
  get its `Breaking:` line (ADR-0038).

**Out (do not do):**
- the desktop's screen readers (3035);
- the tree, its nodes and what each screen describes (2322);
- speech of our own: speech is the screen reader's job;
- requests other than a click (a slider's increase and decrease,
  typing into a `TextInput`): the player changes values with the game's
  own actions, as on the desktop;
- placing elements over the canvas at their `bounds`, for touch
  exploration or a magnifier: not planned;
- moving the web build to `wasm-bindgen` (3034's plan).

## Implementation steps

1. Read 2322's `access` module, `AccessUpdate` and guide; 3035's
   adapter, its request mapping and its Completion notes;
   `pads/web.rs`, `gamepad.js` and the `gamepad_js_*` tests in
   `crates/cli/src/web.rs`. Check whether the web build still runs on
   macroquad's loader.
2. If 3035's request mapping is in the winit backend's crate, move it
   into `shuyi-ui`.
3. Write `access.rs`: the role table, the attributes, `Mirror` and
   `encode`, with their tests.
4. Write `access/web.rs` and `access.js`, and wire `Platform::access`
   and the clicks in `backend.rs`.
5. Add `access.js` to `SHELL` and `index.html`, with their tests, and
   update `crates/cli/web/README.md`.
6. Build Hello, Grid for the web, do the browser pane check, and NVDA's
   if a machine is at hand. Measure the frame times.
7. Write the guide's section, and run the gates (the examples' web build
   among them).

## Acceptance criteria

- [ ] Every `Role` maps to an ARIA role or to text, tested.
- [ ] A tree with only the focus moved gives one `Focus` op and nothing
      else; `tree: None` with no announcements gives no op, and the
      plugin isn't called.
- [ ] A changed value gives one `Attr` op on its node, a removed node
      its `Remove`, and a reordered list one `Order`.
- [ ] Two equal announcements give two ops, each to its region.
- [ ] A click on a menu item that isn't focused moves the cursor there
      and confirms it through `UiAction::Confirm` (a test with the click
      as the plugin hands it).
- [ ] `access.js` provides exactly the functions `access/web.rs`
      imports, and its `version` matches (tests in
      `crates/cli/src/web.rs`).
- [ ] `index.html` loads `access.js` after the bundle and before the
      game (test).
- [ ] The browser pane reads Hello, Grid's options screen with its
      values and its focused item, and the PR shows it.
- [ ] The Completion notes say which path the web build took, the frame
      times, and whether the NVDA check was done.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/app/src/access.rs`, native): the role table; each
  attribute (a slider's values, a value after the name, disabled,
  checked, selected); `Mirror` from no tree, after a focus move, a value
  change, a removal, a reorder, and with nothing changed; announcements,
  polite and assertive, and the same one twice; `encode`'s escapes and
  ids; a click's two halves made into a `NodeId`, and a click on a node
  that's gone dropped.
- Unit: a click as 3035's `Focus` then `Click`, giving
  `UiAction::Confirm` on a menu.
- Integration (`crates/cli/src/web.rs`): `access.js`'s functions and
  `version` against `access/web.rs`; its place in `index.html`; `SHELL`
  holds it.
- Manual: the browser pane on Hello, Grid's web build; NVDA if a machine
  is at hand.

## Completion notes

*(Filled in by the session that completes the ticket.)*
