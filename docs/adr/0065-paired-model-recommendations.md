# ADR-0065: Paired Claude and OpenAI model recommendations

- **Status:** Accepted
- **Date:** 2026-10-31
- **Related tickets:** 2038
- **Amends:** ADR-0064's model routing only; shared instructions and skill discovery still apply.

## Context

Nick corrected the merged portability migration. Its categories hid the
actual model choice and incorrectly suggested Opus/Astra. He explicitly
requested Fable/Astra, Opus/Sol, Sonnet/Terra and Haiku/Luna, with both
specific models on each ticket's model line. Game ticket 1008 is an example
of Opus work that should recommend Sol, not Astra.

## Decision

The four complete pairs and their uses live in tickets/README.md. The model
field stays a scalar: the Claude model id, a spaced slash, and the OpenAI
model id. It recommends alternatives, not a two-model run. The harness uses
its member of the pair; effort is independent. High effort on an Opus
ticket still recommends Sol. These are Nick's routing preferences, not
claims of benchmark equivalence.

Migrate all open, parked and done tickets, the template, active skills and
generated guidance. Preserve all other metadata, scope, historical research,
shared instructions and skill loaders. The lint accepts exactly the four
approved pairs. Tests reject categories, single ids and incorrect pairings.
Future model-version changes update the guide, validator and tickets together.

## Consequences

Tickets show the models directly, usable by either provider. Paired model
versions require a coordinated update. No engine pin or API changes.

## Alternatives considered

- Keep categories: Nick rejected the indirection and incorrect mapping.
- Separate provider fields: duplicates routing without the requested single line.
- Recommend Astra for every hard task: confuses Opus/Sol with Fable/Astra.
