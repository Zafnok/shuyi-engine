# Party RPG

- **Covers:** `kind/party-rpg`
- **Status:** planned
- **Tickets:** 2745, 2746, 2747, 2748, 2749, 2750, 2751, 2752

A party of characters who grow, carry items and fight in turns, with
towns, shops and encounters between the fights. A module, `shuyi-rpg`
(ticket 2745), will hold it: items as data (ticket 2746), making
characters and the roster (ticket 2747), party battles as turn rules and
their screen (tickets 2748 and 2749), abilities (ticket 2750), encounters
and drops (ticket 2751), and shops and paid services (ticket 2752). No
ticket writes this guide in full yet.

## Until then

- A battle of pieces moving on a board is [tactics](top-down-grid.md),
  which is built. A party battle, where the party and the foes face each
  other and take turns, waits for the module.
- Keep a character's numbers in data files, not code, so the module's
  formats can take them over.
