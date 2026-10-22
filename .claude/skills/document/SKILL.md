---
name: document
description: Write what a change did in the one place each fact lives, as the docs map (docs/README.md) says, for people making games and for engine developers. Use for any change that adds or changes a public item, a contract operation or field, a problem code, a format, a rule, a skill or a template, and any time you are about to write a new Markdown file. Not for tickets (write-ticket), ADRs (write-adr) or Nick's decisions (ask-nick).
---

# Write the docs

The map is [`docs/README.md`](../../../docs/README.md): a row per kind of
fact, its one home, and when to update it. This skill holds no copy of
it; read the map.

## Steps

1. **List what the change touches:** public items, contract operations
   and fields, problem codes, formats, rules, skills, templates,
   examples, crates, new Markdown files.
2. **Find each one's row** in the map's three tables. The reader of the
   row decides the words: people making games read nothing of this
   repository's process (tickets, Nick, the game's pin).
3. **Update each home.** Then look the fact up in the map's *Pairs*: if
   it is one, change the other half in the same PR.
4. **Put nothing in a second place.** Anywhere else links to the home, or
   says it in one line with the link.
5. **A new home gets a row.** A new file under `docs/`, a new folder of
   pages, or any new Markdown file the map doesn't cover: add its row to
   the map, in the table of its reader.
6. **Run the check:**
   ```bash
   cargo xtask check-docs   # every *.md has a row in docs/README.md
   cargo xtask check-links  # and every link resolves
   ```

If the change needs no docs, say why in the ticket's completion notes.
