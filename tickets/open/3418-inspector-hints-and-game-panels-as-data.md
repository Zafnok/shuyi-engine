---
id: "3418"
title: Games add inspector hints to their types and their own tool panels as data, through the contract
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3417"]
nick_input: none
completed:
---

# 3418 — Games add inspector hints and their own panels as data

## Context

Found by the blind-spot audit of 2026-11-23 ("Games can't extend the
editor", medium, no ticket). Godot has `@export_range` and
`EditorPlugin`, Unity has `PropertyDrawer` and `EditorWindow`, Defold
has editor scripts, and Bevy has reflect attributes. Shuyi has none of
these.

What Shuyi has today:

- One description of a game's types (ADR-0084): serde for values,
  `schemars` for shapes in the tools, paths as JSON Pointer. A game
  derives `JsonSchema` under its tools feature
  (`examples/hello-grid/src/rules.rs` line 51, `Tuning`).
- `scene --shape` answers the scene data's schema (2581, done;
  `Scene::with_data` in `crates/cli/src/contract/session.rs` line 92).
- 2515 (open) plans `shuyi kinds`: each file kind's text schema.
- A schema says only a field's type and its doc comment. The tools use
  one standard bound today, `#[schemars(range(min = 1, max = 3600))]`
  (`crates/cli/src/contract/attach.rs` line 81), and the
  `schemars` 1.2.3 in `Cargo.lock` also takes
  `#[schemars(extend("key" = value))]`. No convention says what an
  inspector reads from a schema: a step, a unit, a colour, a file of a
  given kind, a group, an advanced field. `grep -rn "x-shuyi"` over the
  repository finds nothing.
- A game can't add a tool view of its own (a balance table, a tuning
  grid). `GameTools` (`crates/cli/src/game.rs` line 470) has no such
  method, and the contract's operations (`Op::ALL`,
  `crates/cli/src/contract.rs` line 137) have no panel.

The editor (2506, 2507, 2508) is parked, and it runs in its own
process. It must not load a game's code, so a game reaches it only as
data. This ticket builds that data now, for agents and the CLI, so the
editor finds it ready:

- **hints** are schema keywords a game writes with `schemars`
  attributes, and `check` checks them;
- **panels** are a contract operation run in the game's tool program
  (as `bake` is, ADR-0101). It answers generic panel data (a table or a
  property grid). A panel's edit is a `patch` (3417) on the file it
  shows, so a panel never writes a file itself, and an agent can do whatever a
  panel does.

Builds on ADR-0005 (one contract, three surfaces), ADR-0084, ADR-0028
(problem codes), 2581 (done), 1129 (done) and 1133 (done; MCP tools come
from `Op::ALL`).

## Nick input

None.

## Scope

**In:**
- **An ADR** (`write-adr`; the next free number): the hint vocabulary
  and the panel data, as below. It extends ADR-0084 and ADR-0005.
- **Hints.** A field's hints are its JSON Schema keywords. Use the
  standard keyword where one exists. A game writes it with `schemars`'
  own attributes:
  - `description` (the doc comment), `title`, `default`, `deprecated`
    and `readOnly`;
  - `minimum` and `maximum` (`#[schemars(range(min = …, max = …))]`).

  The ones JSON Schema lacks are `x-shuyi-*` keywords, written with
  `#[schemars(extend("x-shuyi-step" = 5))]`:

  | Keyword | Value | Means |
  | ------- | ----- | ----- |
  | `x-shuyi-step` | integer above 0 | a number moves by this much |
  | `x-shuyi-unit` | a word, written in full (`percent`, `ticks`, `pixels`) | shown after the value |
  | `x-shuyi-widget` | `slider`, `color`, `multiline`, `asset` or `action` | how to edit the field |
  | `x-shuyi-asset-kind` | a kind's name (`Importers::kinds`) | with `asset`: which files it may name |
  | `x-shuyi-group` | a heading | fields shown together |
  | `x-shuyi-advanced` | `true` | folded away by default |
  | `x-shuyi-hidden` | `true` | not shown. It changes no hash and hides nothing from `scene`: it is a hint, never a view (ADR-0084 §4) |

  `action` names one of the game's actions, never a key (`AGENTS.md`
  rule 8).
- **`shuyi_cli::hints`** (`crates/cli/src/hints.rs`):
  - `FieldHints`, read from one field's schema:
    `FieldHints::of(&Schema, pointer) -> Option<FieldHints>`. The
    pointer is a JSON Pointer into a value of the schema's type
    (ADR-0084 §5), resolved through `properties`, `items` and `$ref`
    into `$defs`.
  - `Widget` names the widgets. It and `FieldHints` are
    `#[non_exhaustive]`: the list will grow (ADR-0039).
  - `check(&Schema, kinds: &[&str]) -> Vec<Problem>` finds:
    - an `x-shuyi-*` keyword that isn't in the table;
    - a value of the wrong type;
    - a step, a unit or a `slider` on a field that isn't a number;
    - a `slider` without both `minimum` and `maximum`;
    - a `default` outside them;
    - `color` or `multiline` on a field that isn't a string;
    - `asset` without `x-shuyi-asset-kind`, or naming a kind the game
      doesn't register.

    Each problem has its pointer and one new code, `bad-hint`: the next
    free `SH21nn` code, the game's code (2579 plans `SH2105`; take
    whichever is free when this lands), with its row in
    `contract/codes.rs` and its page in `crates/cli/errors/`.
  - `check_type::<T: JsonSchema>(kinds)`, for a game's own unit test.
- **`check` checks hints** in each panel's property schema. If the game
  names scene data, it checks that schema too (`Scene::shape`).
- **Panels.**
  - `GameTools` gains `fn panels(&self) -> Vec<PanelInfo>` (default
    empty) and `fn panel(&self, id: &str) -> Result<Panel, String>`
    (default `unsupported`, `game.rs` line 690). Both are additive:
    a game that adds none builds as before.
  - The types go in `crates/cli/src/contract/panels.rs`:
    - `PanelInfo { id, title }`;
    - `Panel { id, title, body: PanelBody }`;
    - `PanelBody::Table { columns, rows }`: each cell is a value with
      an optional `PanelAction`;
    - `PanelBody::Properties { value: serde_json::Value, schema:
      Schema }`: a property grid whose fields are read with `hints`;
    - `PanelAction { label, patch }`, whose `patch` is 3417's patch.

    All are `#[non_exhaustive]` with constructors (ADR-0039), so a
    chart or a list can come later without a break.
- **`shuyi panels [<id>] [--json]`.** It is a contract operation
  (`Op::Panels`, named `panels`) that runs in the game's tool program
  (`contract::cli::runs_the_game`). Without an id it lists the panels.
  With one it answers that panel. An id the game doesn't have answers
  `SH0001` with the ids it has.
  - Its input and output schemas go in `crates/cli/schemas/`
    (`panels.input.json`, `panels.output.json`).
  - It raises `contract::VERSION` (54 today), with its line in
    `schemas/versions.json`.
  - It is an MCP tool: its arms go in `mcp::host::one_off` and
    `mcp::description`.
  - Applying a panel's action is 3417's patch operation, called with
    the action's `patch`. Nothing in `panels` writes a file.
- **Hello, Grid as the worked example** (`examples/hello-grid/src/tools.rs`):
  - `Tuning`'s `trip_percent` gets `range(min = 0, max = 100)`, the
    `percent` unit, a step of 5 and `slider`;
  - a `tuning` panel: `Properties` of the default `Tuning`, read only;
  - a `palette` panel: a `Table` of `files/palette.ron`'s names and
    colours, each colour cell `color`, with a `PanelAction` whose patch
    sets that colour (3417's patch, at the colour's pointer in
    `files/palette.ron`).
- **Docs:** `docs/contract.md` gets a `panels` row and a *Hints* section
  with the table. The skill pack's `shuyi-stage-grid` skill, which
  already names `JsonSchema`, gets how to write hints and a panel.
  `check-skills` passes. The book's chapter on tools, or a `Book:` line.

**Either order with 2515:** if 2515 is done, `check` also checks the
hints in every kind's text schema (`shuyi kinds`). The palette format's
colour strings get `x-shuyi-widget = "color"`. `x-shuyi-asset-kind` is
checked against its kinds. If not, 2515 writes them: when it derives a
format's text schema, it adds the hints of this ticket's table that fit
(`color` on the palette's colour strings), and its `kinds` op checks
them with `hints::check`.

**Out (do not do):**
- drawing hints or panels in a window: the editor (2507, parked) reads
  them;
- the patch operation, its undo and its check against a file changed on
  disk: 3417;
- picking and dragging in the live view: 3416;
- a panel that runs code in the editor's process, or loads a game's
  library: never (the editor stays a client of the contract);
- chart or list bodies, and a panel that updates live over the link
  (2521): a later ticket, when the editor needs one;
- panels or hints in the game template: a game adds them when it wants
  them.

**Estimated size:** about 1,400 lines (code 650, tests 450, docs 300).

**Split:** past 900 lines, split off the panels (the panel types,
`shuyi panels`, the MCP tool and Hello, Grid's panels) into a new
ticket, and keep the hints, the `check` part and the ADR here.

## Implementation steps

1. Read ADR-0084, ADR-0005, ADR-0028, ADR-0039 and ADR-0101. Read 3417's
   patch type, `contract/session.rs` (`Scene::shape`), `game.rs`
   (`GameTools`), `contract.rs` (`Op`, `pair`, `VERSION`),
   `contract/check.rs` and `contract/cli.rs` (`runs_the_game`).
2. Write the ADR: the table, the panel data, and why panels are data
   from the tool program.
3. Add `hints.rs`: `FieldHints::of`, `Widget`, `check` and `check_type`.
   Add the new code with its row and page.
4. Add the panel types and `GameTools::panels` and `panel`, then
   `Op::Panels`, its CLI form, the schemas, `VERSION` and the MCP arms.
5. Make `check` check the panels' and the scene's schemas.
6. Add Hello, Grid's hints and its two panels.
7. Write the docs and the skill section, then run the gates.

## Acceptance criteria

- [ ] `shuyi scene --shape` on Hello, Grid with a tuning value shows
      `minimum` 0, `maximum` 100, `x-shuyi-unit` `percent`,
      `x-shuyi-step` 5 and `x-shuyi-widget` `slider` on `trip_percent`.
      Or, if its scene doesn't hold `Tuning`, `shuyi panels tuning
      --json` shows them in the panel's schema.
- [ ] `FieldHints::of(schema, "/trip_percent")` gives that range, step,
      unit and widget. A pointer into a nested struct through `$ref`
      resolves. A pointer that names nothing gives `None`.
- [ ] `hints::check` gives one `bad-hint` problem, with its pointer,
      for each case in the Scope's list, and none for Hello, Grid.
- [ ] `shuyi panels` in Hello, Grid lists `palette` and `tuning`.
      `shuyi panels palette --json` answers a table with one row per
      colour of `files/palette.ron`, each with an action.
- [ ] Applying one row's action with 3417's patch operation changes
      that colour in `files/palette.ron`, and `git diff` shows one
      line.
- [ ] `shuyi panels nosuch` answers `SH0001` naming `palette` and
      `tuning`.
- [ ] The MCP server lists a `panels` tool, and calling it answers as
      the CLI does.
- [ ] `contract::VERSION` is raised, and `versions.json` and the
      checked-in schemas match (the contract's schema test passes).
- [ ] A game that adds no panel builds and answers `panels` with an
      empty list.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `FieldHints::of` over a hand-written schema (properties,
  items, `$ref`); each `check` case once; `check_type` on a derived
  type with `extend` attributes.
- Property: for any `Panel` built from Hello, Grid's palette, its JSON
  answer read back gives the same panel.
- Integration: `panels` driven through `contract::cli` and through MCP
  on Hello, Grid; a panel action applied through 3417's patch
  operation on a copy of the files.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
