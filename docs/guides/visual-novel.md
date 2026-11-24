# Visual novel

- **Covers:** `kind/visual-novel`
- **Status:** planned
- **Tickets:** 2721, 2726

A story told in scenes, with characters, choices and the paths they
open. A module, `shuyi-vn`, will hold the story as turn rules, so a save,
a replay and a bot that reads every path work as in any turn game
(ticket 2721). An example and template, a short visual novel (ticket
2726), writes this guide in full. Another kind of game can tell its
story this way too: a tactics game between its battles.

## Until then

- The default engine already has dialogue: `.dlg` scenes in the text
  box, with the `shuyi-dialogue` skill. A game that needs a few scenes
  uses them. A whole story of branching paths waits for the module.
- Don't keep the story's state in the screen: what the player chose is
  the rules', so a save and a replay hold it.
