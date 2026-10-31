# Tickets

The backlog lives here as Markdown files. One ticket = one branch = one
PR; the PR moves the ticket from `open/` to `done/` (the `work-ticket`
skill).

```
tickets/
  README.md     ← you are here
  TEMPLATE.md   ← copy for new tickets (see the write-ticket skill)
  open/         ← todo / in-progress / blocked / parked
  done/         ← archived; moved here by the PR that completes the ticket
```

## Numbering

`NNNN-short-kebab-title.md`. Numbers are never reused.

| Ids | What |
| --- | ---- |
| `1119`–`1150` | Moved here from the game repository on 2026-10-14, keeping their numbers so links from the game still make sense |
| `20xx` | Repository and foundation: setup, CI, gates, releases |
| `21xx` | Core and simulation |
| `22xx` | Assets and formats |
| `23xx` | Frame, screens and skins |
| `24xx` | Input |
| `25xx` | Tools and AI support (CLI, MCP, skills) |
| `26xx` | Networking (new tickets; the first ones are 1140–1150) |
| `27xx` | Game styles and examples (new tickets; the first ones are 1119–1128) |
| `28xx` | Docs and community |
| `29xx` | Research |
| `30xx` | Export targets: platforms, devices, stores and performance budgets (ADR-0008, ADR-0012) |
| `31xx` | 3D and HD-2D: the fixed-point 3D space, meshes, the 3D view, cameras, the diorama skin, and their examples (ADR-0011) |

Any other four-digit id in a ticket's text is the **game repository's**
ticket (`github.com/Zafnok/visions-of-shuyi/tree/main/tickets`): for
example 0804 (Nick's Chapter 1 playtest), 1102–1118 (the carve) and 1131
(the code moving here).

## Model and effort guidance

Each ticket's `model` line names both recommended alternatives, Claude first
and OpenAI second. Choose the model for your current harness; do not run both.

| Recommendation (`model` value) | Use for |
| ------------------------------ | ------- |
| `haiku-4.5 / gpt-6-luna` | Mechanical renames, file moves, small data edits |
| `sonnet-5 / gpt-6.1-terra` | Specified changes, existing patterns, clear bug fixes |
| `opus-5.5 / gpt-6.1-sol` | Architecture, tricky invariants, work across crates, design facilitation |
| `fable-5.1 / gpt-6-astra` | Story, dialogue, character voice, art direction |

These are Nick's paired routing preferences, not benchmark equivalence or
guaranteed availability. They are recommendations, not a requirement to switch
providers. Use the available member of the pair, or an explicitly chosen override.
The `model` field is a human-readable pair, not a single API model id.

`effort` is separate: high effort on an Opus/Sol ticket still recommends
Opus/Sol. Complexity alone never changes that recommendation to Fable/Astra.
Use the closest supported effort setting in the current harness. In particular,
game ticket 1008 recommends `opus-5.5 / gpt-6.1-sol`, effort `high`.

The mapping applies to open, parked and done tickets. Scope, blockers, status,
effort and completion dates stay unchanged. Historical model comparisons,
quotations, URLs and real harness configuration paths retain their meaning.
Shared `AGENTS.md` instructions and skills remain usable by either harness.

## Frontmatter

```yaml
---
id: "2201"
title: Short imperative title
type: feature        # feature | infra | decision | bug | research
milestone: MVP       # MVP | After MVP | Parked: styles | Parked: networking | Parked: 3D | …
model: opus-5.5 / gpt-6.1-sol # choose one complete pair above
effort: high         # low | medium | high | xhigh | max
status: todo         # todo | in-progress | blocked | done
blocked_by: ["2002", "game#1131"]  # this repo's ids, or game#NNNN for the game's
nick_input: none     # none | decision | setup | sign-off
completed:           # YYYY-MM-DD when done
---
```

## Breaking changes

A ticket that breaks the public API, a file format or a golden value
(ADR-0038) says so in its `## Completion notes`, and `cargo xtask changelog`
(ticket 2804) lists it at the top of its release's notes. Two things go
there, in the PR that breaks:

1. A `Breaking:` line, one for each break:

   ```text
   Breaking: <crate>[, <crate>…]: <item>: <what a game changes>
   ```

2. Under it, a fenced block whose info string is `migration`, in YAML. It
   becomes the release's *Migration guide*, and a game's agent reads it.

   ````text
   ```migration
   since: 0.2.0
   what: A timer takes its step as a Step, not a number.
   old: Timer::new(u32)
   new: Timer::new(Step)
   before: |
     let t = Timer::new(60);
   after: |
     let t = Timer::new(Step::from_ticks(60));
   find: ["Timer::new(60)"]
   ```
   ````

   | Field | What |
   | ----- | ---- |
   | `since` | The engine version that first has the change, `X.Y.Z`: the workspace `version` after the PR |
   | `what` | One sentence |
   | `old`, `new` | The old and new path or signature |
   | `before`, `after` | A game's code before and after |
   | `find` | Optional. Literal strings that occur in `before` and not in `after`, so each marks a call site in a game's code |

   A ticket may have several blocks. A block that doesn't parse fails the
   changelog with its file and line.

## Parked tickets

A ticket whose title starts with `Parked:` has `status: blocked` and a
**Parked** note under its heading saying when it may start. Nobody starts
it until Nick unparks it (`docs/ROADMAP.md`, *Parked*). Unparking sets
`status: todo` and removes `Parked:` from the title, in the PR that
starts the work or in a small PR of its own.

## Picking the next ticket

A ticket is **ready** when `status: todo` and every id in `blocked_by` is
done: this repository's in `done/`, a `game#NNNN` in the game
repository's `tickets/done/`. Prefer the order in `docs/ROADMAP.md`.

## Nick input legend

- `none`: no Nick involvement.
- `decision`: the ticket *is* a question for Nick (the `ask-nick` skill).
- `setup`: Nick must create an account, a key or a repository; the
  ticket lists the exact steps.
- `sign-off`: Nick looks at the result and approves or comments.
