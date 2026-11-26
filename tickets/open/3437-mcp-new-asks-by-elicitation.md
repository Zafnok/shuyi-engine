---
id: "3437"
title: "The MCP new tool asks the person the wizard's questions by elicitation, when the client can show a form"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3435", "3426"]
nick_input: none
completed:
---

# 3437 — The MCP `new` tool asks the wizard's questions by elicitation

## Context

Anthropic's article *Skills over MCP, tasks and more* (2026-11-24) made
MCP **elicitation** generally available in Claude and Claude Code: a
server asks the person, through the client, for a few typed answers in a
form the client shows. On protocol 2026-07-28 (3435) it is no longer a
request of the server's own: the `tools/call` is answered
`resultType: "input_required"` with an `inputRequests` entry
`{ "method": "elicitation/create", "params": { "mode": "form",
"message", "requestedSchema" } }`, and the client sends the call again
with `inputResponses` holding `{ "action": "accept" | "decline" |
"cancel", "content" }` (the multi-round-trip pattern, SEP-2322; spec at
`modelcontextprotocol.io/specification/2026-07-28/client/elicitation`,
read 2026-11-26). A form's `requestedSchema` is a flat object whose
properties are strings, numbers, booleans or enums, with one array
allowed: a multi-select enum. The client SHOULD fill each field from
its `default`. Forms MUST NOT ask for passwords, keys or tokens. A
client may decline at any time. Claude Code declares
`elicitation: { form: {}, url: {} }` (`code.claude.com/docs/en/mcp`,
*Respond to MCP elicitation requests*).

`shuyi new`'s wizard asks its person questions in a terminal (3423).
3426 gives the MCP `new` tool a `questions` field that lists them
instead (`questions::listed`, the `new.questions.json` shape: each
`Question` has `id`, `text`, `takes` (`text`, `one` or `many`),
`answers` (each `Offered` with `id`, `text`, `status`), `default`,
`flag`, `agent` and `skipped`), and the agent asks its person itself.
Elicitation lets the server ask the person directly, with the same
questions and the same defaults, so the answers reach `new` as the
person gave them, not as the agent retold them. That matters most for
the questions an agent must never answer for its person
(`Question::agent`).

The questions depend on the answers before them (`listed` answers "for
the answers given so far"), so they are asked in rounds: each round's
form holds the questions not yet answered and not `skipped`.

## Nick input

None.

## Scope

**In:**
- **When it asks**: a `new` call on a 2026-07-28 request whose
  `clientCapabilities` declare `elicitation` (with `form`, or empty,
  which means form), without `questions: true` and without an `answers`
  file, is answered `input_required` with one `elicitation/create` form:
  the questions `questions::listed` lists at the answers given so far
  that aren't answered and aren't `skipped`. Its retry with `accept`
  takes the content as answers (as `answers` would) and either asks the
  next round or makes the game. Any other call answers as today.
- **The form**: one property per question, named by its `id`, titled by
  its `text`: `takes: text` a string; `one` an enum of its `ready`
  answers' ids, each titled by its `text` (`oneOf` with `const` and
  `title`, as the spec writes a titled enum); `many` an array of that
  enum. `default` from the question's. A question with no `ready`
  answer is left out, and its default is used. The `message` says what
  is being made and where (`dir`).
- **`decline` or `cancel`**: `new` makes nothing and answers an error
  result with a new problem code (the next free `SH` of `new`'s block;
  its page for `explain`) saying the person declined.
- **The state between rounds**: carried in the retry, as SEP-2322 asks
  (its request state), never in the server: the answers so far are
  re-sent with each round.
- **Docs**: the book's *Tools for agents* (one passage: a client that
  shows forms gets the questions from the server); the
  `shuyi-new-game` skill (an agent whose client shows the forms lets
  the server ask); `docs/contract.md`'s `new` row.
- No public type changes shape (`NewInput` gains nothing: the answers
  come in `inputResponses`, not in the input).

**Out (do not do):**
- Listing the questions (`questions: true`): 3426.
- The 2026-07-28 protocol and its `input_required` plumbing: 3435.
- URL mode, and elicitation on the old protocol (a server-initiated
  `elicitation/create` request).
- Elicitation in any other tool.
- The GitHub questions, which MCP's `new` skips (it makes no
  repository).

## Implementation steps

1. Read 3426's `questions::listed` path, `contract/new.rs`, the
   elicitation page and SEP-2322 (the request state's field name), and
   3435's `input_required` helper in `crates/cli/src/mcp/server.rs`.
2. A `form(questions) -> Value` in `crates/cli/src/mcp/host.rs` (or
   beside `new`'s arm), tests first: each `takes`, the defaults, a
   question with no ready answer.
3. `new`'s arm: the rounds, the retry's answers, decline and cancel.
4. The problem code and its page.
5. Docs, then the gates.
6. **Check by hand** in Claude Code on 2026-07-28: ask it to make a game
   with the MCP `new`; the forms appear, and the game is made with the
   answers picked. Write what you saw in the completion notes.

## Acceptance criteria

- [ ] A `new` call with `elicitation` declared answers `input_required`
      with a form whose properties are the first round's questions,
      each with its default.
- [ ] A `one` question's enum holds exactly its `ready` answers' ids.
- [ ] A retry that accepts the last round makes the game with those
      answers in its `shuyi.ron`, as the same answers in an `answers`
      file would.
- [ ] `decline` makes nothing and answers the new code.
- [ ] Without `elicitation`, with `questions: true`, with `answers`, or
      on 2025-11-25, `new` answers as it does after 3426.
- [ ] Checked by hand in Claude Code (step 6).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the form of each `takes`, the defaults, the left-out question.
- `server/tests.rs`: two rounds then a game made, through `handle`; a
  decline; the four cases that don't ask.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
