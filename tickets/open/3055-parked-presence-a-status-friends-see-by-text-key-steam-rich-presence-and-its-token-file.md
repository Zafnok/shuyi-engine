---
id: "3055"
title: "Parked: Presence: a status friends see, set by text key through a service with no store in it, Steam rich presence behind the steam feature, and Steam's token file written from the language packs"
type: feature
milestone: "Engine: store services"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["3036", "3037", "3040"]
nick_input: setup
completed:
---

# 3055 — Presence: a status friends see

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`), and after 3036 and 3037. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8.

**The benchmarks:**

- Unreal's online subsystem has Presence, Friends and an external UI
  with an invite dialog. EOS and Unity's UGS Friends have presence too.
- Steam's Enhanced Rich Presence: the game sets keys, and
  `steam_display` names a token from a localization file uploaded to
  Steamworks, so each friend reads the status in their own Steam
  language. `steam_player_group` and its size group friends who play
  together, and `connect` puts "Join Game" on the friends list
  (<https://partner.steamgames.com/doc/features/enhancedrichpresence>).
- Godot and Bevy have none of this in core. They reach it through
  GodotSteam and `bevy_steamworks`, as GameMaker does through its
  Steamworks extension. So this is parity with the services of Unreal
  and Unity, not a gap in the core.

**Shuyi today:**

- Nothing sets a status. No type and no other ticket mentions presence.
- `Platform` (`crates/platform/src/platform.rs`) hands over storage,
  audio and text input. `Setup` (`crates/platform/src/app.rs`) carries
  the display, storage, a seed, the voice manifest, the keyboard and the
  key prompt, and `Setup::ctx` puts storage in the context with
  `Ctx::with_storage`.
- 3036 sets the shape of every store service, in an ADR of its own: a
  trait in `shuyi_ui` with `available()`, gathered in a `StoreServices`
  set (`crates/ui/src/store_services.rs`) that
  `Platform::store_services()` hands in, every member `None` by
  default; calls that never block; outputs only; text by key; and a
  memory backend for tests, replays and bots. Achievements are its
  first member.
- 3037 adds `steamworks` (MIT OR Apache-2.0, both in `deny.toml`'s
  allow list) behind a `steam` feature in `shuyi-app`, its GNU link
  check, a fake of the Steam calls, and the fallback when Steam isn't
  running. The one Steam client is a `steam` module in `shuyi-app`,
  written by 3037 or 1144, whichever is done first. Every Steam service
  takes its handle and none starts Steam. This ticket adds no
  dependency.
- 3040 gives `Ctx::log(level, category, text)`, with `LogLevel::Warn`
  and `LogCategory::Platform` among its values.
- A view's text is a `Label` (`crates/ui/src/text.rs`): a key with its
  placeholders' values, or text as it is (ADR-0020). `Ctx::label` puts
  one in the player's language (`crates/ui/src/ctx.rs`).
- A game's languages are a `Lang` (`crates/assets/src/lang.rs`):
  English's screen text by key, and packs over it. `Lang::has` and
  `Lang::text(code, key)` look up one key, but nothing lists the keys.
  `GameTools::lang` (`crates/cli/src/game.rs`) hands a game's languages
  to `lang-status` (`crates/cli/src/lang_status.rs`).
- Invites and joins are not this ticket's. They are small additions to
  the session tickets: the Steam overlay's invite dialog, Steam's join
  callbacks and `+connect_lobby` (1144); a join token and join requests
  (1145); a join link in the browser (1142). This ticket shows a status,
  and carries a join token's text when a session gives one.

**Steam's rules** (its `ISteamFriends` documentation, *SetRichPresence*
and *Rich Presence Localization*):

- at most 20 keys per player, a key up to 64 bytes, a value up to 256
  (`k_cchMaxRichPresenceKeys`, `k_cchMaxRichPresenceKeyLength`,
  `k_cchMaxRichPresenceValueLength`). `SetRichPresence` returns false
  past any of them;
- a token starts with `#` and holds letters, digits and underscores. In
  a token's text, `%key%` is the value of the key `key` (letters,
  digits, underscores and colons);
- a token missing in a friend's language falls back to English. With no
  English, or a `%key%` whose key isn't set, Steam shows nothing;
- the token file is KeyValues text: a `"lang"` block holding one block
  per language, by Steam's API language name (`english`, `japanese`,
  `koreana`, `schinese`, `brazilian`, `latam`), each with its
  `"tokens"`. One upload may hold every language, and only the
  languages in it are replaced. It is uploaded on the Steamworks site
  under *Edit Steamworks Settings*, *Community*, *Rich Presence*;
- `status` is a plain line, shown in the friends list's "view game
  info" dialog;
- the `steamworks` crate (0.13) has `Friends::set_rich_presence(&self,
  key: &str, value: Option<&str>) -> bool` and
  `Friends::clear_rich_presence(&self)`.

**Why it matters:**

- Visions of Shuyi doesn't ask for it: its Steam readiness ticket
  (game#0903) covers the overlay, Steam Cloud and achievements, not
  presence. If the game's 0053 picks online play on Steam, presence
  carries the "Join Game" line that fills a small game's lobby.
- For other 2D games, a status on the Steam friends list ("Level 3, two
  lives left") is free advertising, single-player games included, and
  the join line is how friends find each other in a multiplayer one.

**Rules it keeps:**

- **Only platform backends talk to a store** (`AGENTS.md` rule 5). The
  service is a trait the platform hands in.
- **Determinism.** Presence is an output, like an unlock (3036). The
  rules never read it, and `Ctx` has no getter for it. Replays, bots and
  `frame-png` run with the memory backend, so they send nothing
  anywhere.
- **Text by key** (ADR-0002 §6). Steam shows each friend the status in
  the friend's own language, from a token, so a status is always a text
  key and never plain text.

## Nick input

**Setup** (when unparked, for the manual test only; the code is tested
with a fake):

1. A Steam account logged in on the machine that runs the manual test.
   Valve's test app 480 (Spacewar) needs no Steamworks setup to show the
   keys on Steam's test page.
2. The status as friends read it needs the game's own app: on the
   Steamworks partner site, *Edit Steamworks Settings*, then
   *Community*, then *Rich Presence*: upload the file `shuyi
   steam-presence` wrote, and publish. That belongs with the game's
   Steam setup (game#0903), so this ticket's manual test may stop at
   step 1.

## Scope

**In:**
- **The service**, in `shuyi_ui::presence` (`crates/ui/src/presence.rs`),
  `no_std` with `alloc` like the rest of `shuyi-ui`:
  - `PresenceStatus { key: String, args: Vec<(String, String)>, text:
    String, group: Option<Group>, join: Option<String> }`, with `Debug`,
    `Clone`, `PartialEq` and `Eq`: the text key and its values; the
    status in the player's language, for a store's plain line (Steam's
    `status`); the group; and a join token's text form for friends to
    join with (Steam's `connect`);
  - `Group { id: String, size: u32 }`: friends playing together;
  - `trait Presence: fmt::Debug { fn available(&self) -> bool; fn
    set(&mut self, status: &PresenceStatus); fn clear(&mut self); }`,
    `Debug` as `Storage` is and `available` as 3036's ADR asks of every
    service. No call blocks;
  - `NoPresence`, which does nothing and says `available()` false:
    builds with no store (DRM-free, itch.io, the web);
  - `MemoryPresence`, for tests, the Harness, replays and bots. It says
    `available()` true and keeps every status sent in a record it shares
    with a `PresenceLog` reader (`Rc<RefCell<…>>` from `alloc` and
    `core`), so a test reads what was sent while the game owns the
    backend.
- **The game's calls on `Ctx`:**
  - `Ctx::set_presence(label: Label)` takes a `Label::Key` and fills
    `text` with `Ctx::label`. It refuses, sending nothing: a
    `Label::Text`; a key not under `presence.`; a key `Lang::has`
    doesn't know; and a `{name}` placeholder in the key's English text
    (`Lang::english`) that the label's args don't fill. A refusal is a
    `debug_assert!` in a debug build, as `Ctx::text` treats a key
    English lacks, and in release a line through `Ctx::log` at
    `LogLevel::Warn` in `LogCategory::Platform` (3040);
  - `Ctx::set_presence_group(Option<Group>)` and
    `Ctx::set_presence_join(Option<String>)` change the status last set
    and send it again. With no status set yet they are kept, and go
    with the next `set_presence`;
  - `Ctx::clear_presence()` forgets the status, its group and its join
    token, and calls `Presence::clear`;
  - a status equal to the last one sent (its text included) is not sent
    again, so a game may set it every frame, and a language change
    resends it the next time the game sets it.
- **Handing it in**, the way 3036's ADR says every store service is
  handed in. If 3036 settled on another shape, follow it and say so in
  the PR:
  - a `presence: Option<Box<dyn Presence>>` member of 3036's
    `StoreServices`, `None` in `StoreServices::default()`;
  - `Setup::ctx` gives the context the platform's backend, or
    `NoPresence` when the member is `None`, so the macroquad backend
    builds unchanged without the `steam` feature;
  - the headless backend (`crates/platform/src/headless.rs`) hands in a
    `MemoryPresence` and keeps its `PresenceLog`;
    `Harness::presence()` returns the last status sent
    (`crates/platform/src/harness.rs`), and `Harness::presence_log()`
    every status sent, in order.
- **`SteamPresence`**, in `shuyi-app` behind 3037's `steam` feature,
  over `steamworks::Friends` from the `steam` module's handle. The
  macroquad backend's `store_services()` hands it in when Steam
  started:
  - `set` writes `steam_display` as the key's token (`presence.in_town`
    gives `#presence_in_town`: the key with `.` and `-` as `_`), one key
    per argument (its placeholder's name and its value), `status` as
    `text`, `steam_player_group` and `steam_player_group_size` when
    there is a group, and `connect` as `--join <token>` when there is a
    join token (the argument 1145 reads at launch);
  - each value is cut to 255 bytes at a character boundary, so it fits
    whether Steam's 256 counts the closing zero byte or not. A `connect`
    that long is left out with a warning, since a cut token can't join;
  - keys the last status set and this one doesn't are cleared
    (`set_rich_presence(key, None)`), so a stale value never shows;
  - a status that needs more than Steam's 20 keys sends nothing, and a
    warning says why (the tool's check stops this before a release);
  - `clear` calls `clear_rich_presence`;
  - it is tested through 3037's fake of the Steam calls, extended with
    the two rich-presence calls;
  - without Steam running, the build gets `NoPresence` and a warning, as
    3037 falls back.
- **`shuyi steam-presence [--out <file>]`**, in
  `crates/cli/src/steam_presence.rs`, wired in `crates/cli/src/lib.rs`
  like `lang-status` (it needs the game's languages, `GameTools::lang`,
  so without the game's tools it runs in the game's tool program), and
  listed in `usage` under *commands* and in the module's doc comment:
  - it takes every screen text key under `presence.`, listed by a new
    `Lang::screen_keys()` (English's screen text keys, in order);
  - it writes Steam's token file (by default `steam/rich_presence.vdf`
    in the project): one block per language, each key's text as
    `Lang::text(code, key)` gives it, with `{name}` written as `%name%`,
    and `"` and `\` escaped;
  - the languages are the source language and each pack, by Steam's API
    language name from a table in the module: Steam's language list,
    keyed by its Web API code (`en` gives `english`, `ja` `japanese`,
    `ko` `koreana`, `pt-BR` `brazilian`, `es-419` `latam`), plus
    `zh-Hans` for `schinese` and `zh-Hant` for `tchinese`. A pack with
    no Steam language is left out with a warning (Steam shows those
    players English). The test pack (`lang_status::TEST_PACK`) is left
    out without one;
  - **either order with 2226:** if 2226 is done, the packs' codes are
    BCP 47 tags, the tagged rows match, and the source block is
    `Lang::source_lang().code`'s; if not, only the rows with no region
    match and the source is English;
  - it fails, listing each problem with its key, on: two keys with one
    token; a key with a character outside lowercase letters, digits,
    `_`, `.` and `-`; a placeholder outside lowercase letters, digits
    and `_`, or longer than 63 bytes (so `{Confirm}` and the other key
    names, which a friend's Steam can't fill, are refused); a
    placeholder named like one of Steam's own keys (`status`,
    `connect`, `steam_display`, `steam_player_group`,
    `steam_player_group_size`); a status with more than 15
    placeholders, the most that fit beside those five under Steam's 20;
    a `%` in the text; and no English block (the source isn't English
    and there is no `en` pack, so friends in other languages would see
    nothing);
  - the file is a tool's output for upload, not an asset kind the engine
    loads, so it needs no `Format` (ADR-0007, ADR-0019).
- **Either order with 1145:** if 1145 is done, its lobby screen
  (`shuyi_ui::screens::lobby`) sets the presence group (the session's
  id and its player count) and the join token's text while the session
  takes players, and clears both when it closes; if not, 1145 does this
  when it is done.
- **The toy game** (`crates/ui/src/toy.rs`, its text in
  `crates/ui/src/toy/en.ron`) gets two statuses: one set on `Home`, and
  one with a value set on `grid::Field` (`crates/ui/src/toy/grid.rs`).
  Its tests read them through `Harness::presence()`.
- **The docs:** a section in `docs/targets/steam.md` (3037's): how a game
  names its statuses, runs the tool, uploads the file, and checks it on
  <https://steamcommunity.com/dev/testrichpresence>.

**Out (do not do):**
- invites and joins: the overlay's invite dialog, Steam's join callbacks
  and `+connect_lobby` (1144); `JoinToken`, join requests and a join at
  launch (1145); the join link on the web (1142);
- the `StoreServices` set, its ADR and achievements (3036); the Steam
  client, its feature, its fake and its fallback (3037); leaderboards,
  another member of the same set (3054);
- Discord. It needs Nick's go-ahead: Discord's Social SDK is a
  proprietary SDK under Discord's terms, and ADR-0006 §5 allows only
  Steam's. The older local IPC route has permissively licensed crates.
  Ask him (the `ask-nick` skill) when a game ships online on Steam and
  wants Discord too;
- other stores (EOS, GOG Galaxy, consoles): each a backend under this
  trait, a ticket of its own when a game ships there and Nick has said
  yes to its SDK;
- a friends list, reading friends' statuses, and voice;
- an argument that is itself a text key (Steam's nested `{#token_%key%}`),
  for a translated value inside a status: a follow-up if a game needs it;
- running the tool's checks inside `shuyi check`: a follow-up if a game
  asks;
- uploading the file, which is done by hand on the Steamworks site.

## Implementation steps

1. Read 3036's ADR and code (`StoreServices`, how `Setup::ctx` hands
   achievements to the context, `MemoryAchievements`), 3037's `steam`
   module, fake and fallback, 3040's `Ctx::log`,
   `crates/ui/src/text.rs`, `crates/ui/src/ctx.rs`,
   `crates/assets/src/lang.rs`, `crates/cli/src/lang_status.rs` and
   `crates/platform/src/headless.rs`.
2. Write `shuyi_ui::presence` (`pub mod presence;` in
   `crates/ui/src/lib.rs`): the types, `NoPresence`, `MemoryPresence`
   and `PresenceLog`, with unit tests.
3. Add the `presence` member to `StoreServices`, the fallback to
   `NoPresence` in `Setup::ctx`, the `Ctx` calls, the headless backend's
   `MemoryPresence`, `Harness::presence()` and
   `Harness::presence_log()`.
4. Add the toy game's two statuses and their Harness test.
5. Write `SteamPresence` over 3037's fake, and hand it in from the
   macroquad backend's `store_services()` when Steam started.
6. Count the changed lines so far. If the PR would pass about 600
   changed lines (fixtures and snapshots aside) with step 7, stop here:
   write the tool, `Lang::screen_keys()` and the docs section as a new
   parked ticket (`write-ticket` skill) blocked by this one, carrying
   their scope, criteria and tests from this ticket, and say so in the
   Completion notes.
7. Add `Lang::screen_keys()`, then write `shuyi steam-presence` with its
   checks, wire it in `crates/cli/src/lib.rs`, and add it to `usage`.
8. Write the docs section.
9. Run the gates, with and without the `steam` feature.
10. Run the manual test with app 480 if a Steam account is at hand. Say
    in the Completion notes whether it was done.

## Acceptance criteria

- [ ] Setting the same status twice sends it once; a changed value,
      group, join token or language sends it again.
- [ ] A `Label::Text`, a key outside `presence.`, a key English lacks
      and an unfilled placeholder are each refused and send nothing; in
      release each logs one `Warn` line.
- [ ] `StoreServices::default()` has no presence, and a context built
      from it holds `NoPresence`; the headless backend's context holds a
      `MemoryPresence` that `Harness::presence()` reads.
- [ ] A replay of the toy game sends nothing outside its
      `MemoryPresence`, and gives the same hashes with `NoPresence`.
- [ ] With the fake, `SteamPresence` writes `steam_display`, each
      argument's key, `status`, the group's keys and `connect`, and
      clears the keys the last status used and this one doesn't.
- [ ] Without Steam running, the toy game starts with `NoPresence` and
      a warning in the log.
- [ ] `shuyi steam-presence` on the toy game (`ToyTools`, in
      `crates/cli/tests/it/toy.rs`) writes a token file equal to one
      committed beside the test, with its debug language (`DEBUG_LANG`,
      `dbg`) left out and named in a warning. If step 6 split the tool
      out, this and the next criterion move to the new ticket.
- [ ] Each of the tool's failures has a test with a small `Lang` that
      triggers it, and the message names the key and the reason.
- [ ] `docs/targets/steam.md` says how to name, write, upload and check
      statuses.
- [ ] The gates pass with the `steam` feature too (on CI's MSVC runners
      if 3037 found that the GNU toolchain can't link it).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the `Ctx` calls (sent once, each refusal, group and join set
  before and after a status, cleared, a language change);
  `available()` of each backend; token names; `{name}` to `%name%`;
  escaping; the language table; `SteamPresence` over the fake (keys
  written, stale keys cleared, the 20-key cap, the 255-byte cut at a
  character boundary, a `connect` too long left out).
- Property: any key of lowercase letters, digits, `_`, `.` and `-` under
  `presence.` gives a token Steam accepts (`#`, then letters, digits and
  `_`); for any set of keys, the tool either reports a clash or gives
  every key its own token.
- Snapshot / integration: the toy game's token file; a Harness run of
  the toy game reading its two statuses.
- Manual: the toy game with `--features steam` and app 480, its keys
  checked on Steam's test page.

## Completion notes

*(Filled in by the session that completes the ticket.)*
