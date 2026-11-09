# Reverse-engineering hardening: angles the ticket track has not covered

Research for ticket 2917, first pass of 2026-11-08; that ticket confirms
the web findings at their sources and finishes this report. Research
only; nothing here was attempted or built. Eleven research agents ran
(one read this repository, ten read the web). Web findings rest mostly on
search summaries and secondary sources, not primary reads; each section
says how firm it is. The repository findings in section 1 were checked by
hand against the code. Not legal advice anywhere.

**Tickets written from this pass:** 3099 (release-build hygiene, section
1), 3300 (a shipped pack, sections 1 and 3), 3301 (a store watch, section
11), 3302 (a per-recipient overlay, section 7), a checklist step in 2816
(section 9) and 2917 itself (the confirmation and the book's statements).

## What the track already covers

Tickets 2912–2915 measured AI-assisted cloning by threat path, tested
perturbed gameplay clips (no-go), tested a diversified bytecode VM for a
rules kernel against dynamic analysis (no delay; stopped) and studied
capture flags, per-recipient video marks and AI-use signals. 3080–3082
cover export hygiene, a disclosure manifest and demo editions. 2816 is the
reveal/evidence kit. 2602, 2603 and 3083 (hosted preview, server-held
content, marked previews) are parked with networking.

Everything below is outside those.

## 1. The engine's own release builds leak more than they need to

This is the one angle with concrete, cheap engine work. Verified in code.

| Finding | Where | What it hands an analyst |
| --- | --- | --- |
| Windows and Linux exports strip **debug info only**. The exporter builds with `strip=false` and line tables so a `.debug` companion can symbolicate crashes, then runs `objcopy --strip-debug` on the shipped copy. That keeps the full symbol table, so shipped desktop binaries carry every mangled Rust function name. macOS keeps `strip=true`. | `crates/cli/src/export/desktop.rs:236-241`, `crates/cli/src/export.rs:393-396` | Free function map for Ghidra; the single biggest orientation shortcut for an agent. |
| Desktop and web games embed the whole `files/` tree with `include_dir!`, unbaked: every RON file, original path and text sits in the binary as written. Only Playdate bakes a pack. | `templates/shuyi-default/base/crates/game/src/lib.rs:22`, `crates/cli/src/export/playdate.rs:29` | All rule tables, maps and text with `strings`. No decompilation needed. |
| Export refuses the `link` feature but not `debug-tools`. A game whose export part lists `debug-tools` ships the debug menu, `SHUYI_PERF` traces and session recording. | `crates/cli/src/export.rs:745-762`, `crates/app/src/link.rs:55-65` | A built-in headless-style oracle in a shipped build, if misconfigured. |
| Every release build writes crash reports holding a play record, backtrace, `file:line` and the log ring, and honours `SHUYI_PANIC_AT_FRAME`. | `crates/app/src/crash.rs:26-27,200-225`, `crates/app/src/lib.rs:528-532` | A free state-dump-on-demand: force a panic at frame N, read the record. |
| `wasm-opt` is skipped **silently** when not on PATH. Nothing strips the Wasm `name` section explicitly; no source maps are written. | `crates/cli/src/web.rs:578-600` | A web build from a machine without Binaryen ships names. |
| Release profile sets no `panic` (unwind), so panic message and location strings stay. `Ctx::log` is never compiled out; its text and the engine's env-var names (`SHUYI_LINK`, `SHUYI_PERF`) are in every binary. Saves, settings, profiles and key bindings are RON text with struct names. | `Cargo.toml:58-61`, `crates/ui/src/ctx.rs:767`, `crates/core/src/save.rs:17-18` | String anchors an LLM agent uses first. |

Not exposed by default: the tool contract, MCP server and problem codes
live only in `shuyi-cli`, which a shipped game does not link. The live link
needs both a feature and a runtime gate, binds to localhost and needs a
token. Good.

**Candidate ticket (cheap, engine-side):** release-build hygiene. Use
`objcopy --strip-all` on the shipped copy (the `.debug` companion keeps
symbolication); have `no_live_link` also refuse `debug-tools`; make the
crash play-record and `SHUYI_PANIC_AT_FRAME` opt-in in release; warn or
fail when `wasm-opt` is missing and strip the `name` and `producers`
sections explicitly; consider `panic = "abort"` for exports; gate
Debug/Info log lines out of release. Each is a profile or one-line change.
A second, larger candidate: bake a pack for desktop and web exports too,
replacing `include_dir!`. Section 3 says how little a pack's obscurity
buys; the point is removing paths and RON text from `strings` output, not
secrecy.

**Honest expected effect (section 2's estimate, unmeasured):** perhaps one
to three hours of an agent's orientation time lost. It does not stop
anyone. It is still the only hardening in this report that costs nothing
and has no bypass cheaper than the status quo.

## 2. Rust build metadata and stable-toolchain obfuscation

Firmness: moderate on settings, low on effect sizes. No study measures
Rust binaries against LLM agents, and none isolates string encryption or
symbol stripping as a variable.

- `--remap-path-scope` appears stabilised in rustc in early 2026 (PR
  147611); Cargo's `trim-paths` profile key was not part of it, so the
  exporter's current `--remap-path-prefix` route stays right. Remap the
  sysroot too, or `library/core/src/...` paths still leak.
- `lto = "fat"` with `codegen-units = 1` is the only free setting that
  hurts *structure* recovery rather than names, by erasing crate
  boundaries through inlining. The workspace uses `thin`.
- Panic-string removal entirely (`panic_immediate_abort`) needs nightly
  and build-std, so it is out.
- Stable, permissively licensed obfuscation crates only encrypt strings
  or add junk: `obfstr` (MIT, quiet since 2023), `cryptify` (MIT, active),
  `litcrypt2`, `goldberg` (dormant). Control-flow flattening and bogus
  flow (OLLVM/Hikari ports) need nightly `-Z llvm-plugins` and a matching
  LLVM, often a rebuilt toolchain, which Windows rustup usually cannot
  load. Out under the stable and GNU constraints.
- Evidence on LLM deobfuscation: Tkachenko et al. 2025 (arXiv 2505.19887)
  found frontier models handle bogus flow and substitution, with flattening
  in the middle tier. Archibald and Thijssen 2026 (arXiv 2606.06838) found
  an agent with direct Ghidra MCP control did *not* reliably improve
  output without a quantitative scorer. Decompile-Rust (arXiv 2507.18792)
  calls Rust hard to decompile for its types and inlining, without LLM
  data.

**No ticket beyond section 1.** String-encryption crates would add an
unmaintained dependency for an estimated zero to two hours of delay.

## 3. Pack-format protection: what other engines learned

Firmness: moderate; dated time-to-defeat figures mostly do not exist.

| Engine | Protection | Defeat | Side effects |
| --- | --- | --- | --- |
| Godot | AES PCK, key in export template | KeyDot finds the key statically in milliseconds; GDRE Tools recovers whole projects | Custom templates per platform, key/IV sync |
| Unity | None by default; IL2CPP for code | AssetStudio, Il2CppDumper | Paid vendor add-ons |
| Unreal/Fortnite | AES pak/utoc, rotated per release | FModel plus community key archives after each release | The only measurable delay: hours to days per release, for a game with a huge dataminer community |
| RPG Maker MV/MZ | XOR header, key in `System.json` | Immediate | None |
| Ren'Py | `.rpa`, compiled `.rpyc` | unrpa, unrpyc; format-tweaking forks break tools until the next release | Tooling churn |
| GameMaker | `data.win`; YYC to native | UndertaleModTool; YYC needs Ghidra | Longer builds |
| LÖVE | None (zip) | n/a | n/a |

Lessons: a key the client can use is a key the client can extract; making
it harder to find bought no documented delay; per-release keys cost the
*community* effort only where a community exists; the index is the ripper's
map, so paths and kind names in it are the leak; Mega Crit and much of
indie practice have stopped trying. For an open engine the format is public
by definition, so the pack's only honest job is tidiness (section 1), not
secrecy. **No encryption ticket.** This confirms 2603's exclusion of a
client-held key.

## 4. The web build is the cheapest extraction surface

Firmness: moderate. No sourced account of a full LLM clone from a deployed
Wasm and its assets; nearest cases are partial.

Ranked by cost to the attacker: every asset is a URL in the Network tab
(browser add-ons auto-download a game's `.wasm` and data); the Wasm's
imports, exports and data segments survive any stripping because the
loader needs them; the `name` and producers sections go only if
`wasm-opt`/`wasm-tools strip --all` runs; linear memory is readable from
DevTools or a userscript for decoded textures, levels and live state;
Spector.js pulls every WebGL texture; and the deterministic core makes a
clone's fidelity checkable against a recorded input trace from the browser.
Site-locking, signed URLs, CDN bot rules and JS obfuscation stop
hotlinking and scraping only; a person playing the game holds a valid URL.
Hosting choice (Pages, itch.io, CDN) changes nothing here.

Case to note: an Oct 2025 attempt to reverse Catan Universe's Unity Wasm
with an LLM over chrome-devtools MCP captured nothing useful and
hallucinated. Large noisy Wasm is not trivially read dynamically; small
Shuyi games will be easier.

**Candidate text, not a ticket:** the book's web-export chapter should
say plainly that a web demo is a published copy of its assets and rules.
A game that wants a secret should not ship it in a web demo. That is a
`document`-skill sentence, or part of 3082's demo-edition guidance.

## 5. Runtime extraction on desktop defeats any at-rest protection

Firmness: moderate; tool capabilities are documented, countermeasure
claims are inference.

RenderDoc, apitrace and Ninja Ripper dump every texture a miniquad GL game
uploads, in the exact bytes uploaded. WASAPI loopback captures the mix;
only DRM-flagged media streams are excluded, which a game cannot use.
Cheat Engine, ReClass and Frida recover struct layouts, and LLM agents now
drive them (Claude over a memory-dumper DLL; a Frida game-hacking MCP
server, May 2026). Saves and replays as RON hand over the schema; binary
formats remove names but not shape. Commercial anti-tamper (Denuvo,
VMProtect, Themida, Arxan) targets cracking, not ripping, is proprietary
and per-seat, so it cannot ship in an MIT/Apache engine, and 2025–2026
Denuvo titles fell in roughly 41 days to over a year. Deterministic
simulation helps the cloner: it is a free test oracle.

**No ticket.** Record in the research docs that anything decoded, drawn,
heard or simulated on the player's machine is extractable, so the engine
must not promise otherwise.

## 6. Attacker capability has moved: whole-game ports are routine

Firmness: moderate; the catalogue is self-reported.

- reversedwithai.com lists 323 AI-reversed games as of 2026-11-08 (138
  working), including 2D titles ported to the browser from shipped EXEs:
  Micro Machines, SkyRoads (about one day), One Must Fall 2097, Freedom
  Planet, Mega Man X. Claude is the model named most often.
- Reported costs: Space Cadet Pinball to Rust in about 2h25m and about
  US$140 of API (secondary, unverified). A simple web game cloned in under
  30 minutes with REA and Codex (creator's claim).
- Why native falls fastest: matching decompilation gives the agent a
  byte-exact pass/fail signal. Wasm and custom bytecode lack that answer
  key. That is the agents' inference, consistent with 2914's result that
  a VM bought nothing once the attacker had a *behavioural* oracle.
- GhidraMCP (Apache-2.0), ReVa, pyghidra-mcp and REA (MIT, July 2026)
  make the decompiler loop free; cost is model tokens only.
- Valve announced no anti-clone measures on 2026-10-27 after the wave of
  AI-decompiled browser ports (Halo CE, Vice City, Simpsons Hit and Run).
  DMCA notices against those ports targeted the *game data*, and the
  ports moved to bring-your-own-assets.
- World models (Project Genie, Jan 2026; Genie 4 preview, Oct 2026) are
  60-second 720p toys with no rules or saves. Video-to-playable is still
  not a product; the sprite-from-video tools work only on clean AI-made
  video. Art is far easier to rip from the assets than from footage.
- New benchmark not on 2912's list: CrackMeBench (arXiv 2605.10597),
  not game-specific; results not extracted.

**Implication for the track:** 2912's "behavioural replacement" track is
now the mainstream attack, not the exotic one. Any future measurement
should assume a REA-class harness and a frontier model as the floor.

## 7. Fingerprinting the build artifact to trace a leak

Firmness: low to moderate. Theory is solid; practice for a one-person
studio is thin and documented leak-tracing successes are rare.

Naive marks (an ID string, a pack trailer) fall to a two-copy diff or a
repack. Collusion-resistant codes (Tardos; Škorić's improvements) need
hundreds of independent, natural-looking positions per copy, which a small
Rust game does not obviously offer, and rustc/LLVM expose no per-copy
randomisation knob. Industry (Denuvo TraceMark, GDC 2024) marks *rendered
output*, which 2915 and 3083 already cover. Confirmed cases of a press
build leaker identified by a binary watermark: none found; Half-Life 2's
ATI trace is lore. Honeytokens that phone home on launch collect personal
data (IP, agent), so they need a privacy notice and Nick's decision.
Reproducible builds conflict with per-copy diversity unless the recipient
mark is a small overlay applied after a reproducible build.

**Candidate (small, after 2816):** a per-recipient *overlay* for closed
builds: a data-segment ID plus a few redundant marks (pack entry order,
one asset pixel mark), recorded in a signed ledger of 10–20 recipients,
paired with per-recipient Steam keys. Described as a deterrent and
circumstantial evidence, never proof. No honeytoken without a product
decision.

## 8. What Steam and itch.io already give for free

Firmness: good; mostly from Valve's own partner pages.

| Mechanism | Gives | Lacks |
| --- | --- | --- |
| Steam Playtest (own app ID) | Limited or open signup, keys, friend invites, a dashboard of who has access | No NDA, no confidentiality; "Not Playable" does not revoke keys; the only revoke is a full irreversible reset |
| Password beta branch | A per-build gate | Shared password, so untraceable; per-account download logging unconfirmed |
| Release override keys | Per-account access, revocable with the key-ban tool | You run signup and NDAs yourself |
| Steam DRM wrapper | Ownership check | Valve: stops "extremely casual piracy", "easily removed by a motivated attacker"; untested on a macroquad binary |
| Separate demo app | Cut-down depots; deactivatable; one wishlist notification | Deactivation does not stop launching outside Steam |
| itch.io Restricted page, download keys | Hidden from search; revocable keys | One password at a time; HTML5 behaviour murky |

Valve recommends announcing first and using a codename for unannounced
games. **This largely substitutes for the parked hosted-preview tickets**
(2602, 3083): a Hidden Playtest with keys, or override keys plus a
password branch, is a controlled preview at zero cost. The gaps, no NDA
and weak revocation and no tracing, are what 2915/3083's marks and
section 7's overlay would fill. Hosted previews stay unnecessary unless
expiry or tracing that Steam cannot give is wanted.

## 9. Law and model-provider policy (research, not advice)

Firmness: moderate on statute and well-known cases, low on 2025–2026
case law; the agent read snippets, not opinions.

- Rules and mechanics are unprotected (Tetris v. Xio, Spry Fox v.
  LOLApps settled, DaVinci v. Ziko). Art, music, text and specific
  expressive sequences are protected. No court decision on an AI-made
  clone was found.
- DMCA 1201 needs a measure that "effectively controls access". A
  per-recipient *mark* controls nothing; a trivial pack lock is a closer
  call and unlitigated for a small studio. 2024 triennial exemptions
  cover preservation and interoperability; next cycle 2027.
- EU Software Directive art. 6 and 8 make no-decompilation clauses void
  for interoperability; SAS v. WPL protects studying a program's ideas.
  US clickwrap generally binds; MDY v. Blizzard limits covenants. EULA
  clauses against AI extraction have no case law.
- Steam removes on a facially valid DMCA notice within about a day and
  restores on counter-notice if no suit follows in about 10 business days
  (Allumeria, Feb 2026, removed and restored in 24 hours after Microsoft
  withdrew; No Players Online lost about three weeks to a bogus claim).
  Art-copy notices work; mechanics-only notices fail and risk 512(f).
- US copyright registration (about US$65 standard) before infringement,
  or within three months of first publication, is the prerequisite for
  statutory damages and fees (17 U.S.C. 412). Register at reveal.
- Anthropic's and OpenAI's policies forbid infringing third-party rights
  in generic terms; neither names DRM or game cloning; open-weight models
  bypass policy entirely. Treat provider policy as light friction at
  most, consistent with 2912 counting refusals as a failure mode, not a
  defence.

**Candidate (goes with 2816, product decision for Nick):** the reveal kit
should include a "register the copyright before the first public
footage" step and a per-platform notice template. Trademark: USPTO US$350
per class since 2025-01-18; EUIPO about EUR 850.

## 10. Art style cloning and asset-side protection

Firmness: low; nothing tests any of this on small palette-limited sprites.

Few-shot LoRAs on 16–50 images reproduce a pixel-art style; Retro
Diffusion and PixelLab sell reference-guided sprite and animation
generation. Glaze, Nightshade, Mist, PhotoGuard and Anti-DreamBooth are
untested on 32×32 sprites, where a palette-quantised image leaves no room
for a perturbation; published breaks (robust mimicry, LightShed at 99.98%
detection, "Fragile by Design" 2025) stand, and the tools have no CLI or
API for a build step. Robust image watermarks lose to diffusion
regeneration; a redraw carries no mark by construction. C2PA in PNG is a
signed authorship claim that any re-encode strips. The only item that
works today is detection: perceptual hashes (pHash, DINOv2 embeddings)
find ripped or upscaled reuse, not redraws.

**No protection ticket.** The honest engine offering is a build-time
manifest of asset hashes and perceptual hashes (an input to section 11),
and a sentence in the docs that perturbation tools are unproven on
sprites.

## 11. Detect-to-takedown speed is the angle with the most leverage

Firmness: moderate on incidents, low on timings and false-positive rates.

| Case | Clone after | Gone after | Channel |
| --- | --- | --- | --- |
| Wordle, Jan 2022 | 0 days | about 2 days | Public backlash, then Apple's own rule |
| Needle In A Haystack Simulator, 2026 | days after a viral reveal; 8 copycat titles before release | the code-copy clone within about 9 days of appearing | Developer's copyright notice to Valve with a file comparison; the original added "original" to its key art |
| Allumeria (reverse case), Feb 2026 | n/a | removed and restored in 24 hours | DMCA, withdrawn |
| Vampire Survivors mobile clones | months | not found | Poncle shipped its own mobile port |
| Palworld | n/a | unresolved since Sept 2024 | Patent suit, not a takedown |

Valve's stated advice to developers facing AI clones: do not rely on a
single copyable mechanic, and report clone-extortion threats to Valve and
to law enforcement. Impact evidence is anecdotal and mixed; one fishing
game's bad clones drove about 15k wishlists to the original.

**Candidate ticket (after 2816):** a `watch` step in the ship workflow.
Poll `IStoreService/GetAppList` and itch.io daily, fuzzy-match new titles
against the game's name and keywords, perceptual-hash new capsules and
screenshots against the reveal kit's hashes, and emit a dated evidence
bundle plus pre-filled notice templates. Advisory only: false-positive
rates are unmeasured. With a daily watch, detection within a day and a
well-evidenced Steam takedown within days is plausible; that is a smaller
demo-to-clone window than any technical measure in this track has bought.

## Ranked candidates, cheapest and surest first

1. **Release-build hygiene** (section 1): `--strip-all` on the shipped
   copy, refuse `debug-tools` in exports, opt-in crash play-record, loud
   `wasm-opt` absence plus explicit name-section strip, `panic = "abort"`
   for exports, Debug/Info logs out of release. Hours of work; hours of
   attacker delay; no bypass cheaper than today.
2. **Bake a pack for desktop and web** (sections 1, 3): removes raw RON
   and paths from `strings`. Days of work; tidiness, not secrecy.
3. **Store watch and notice templates** in the ship workflow (section
   11), fed by 2816's hashes. Days of work; shortens the clone's life,
   the only lever with case evidence.
4. **Reveal-kit additions** (section 9): a register-before-reveal step
   and per-platform notice templates. Documentation; Nick decides whether
   registration is a product step.
5. **Per-recipient overlay for closed builds** (section 7), after 2816 and
   only with Steam keys per recipient. Deterrent, not proof.
6. **Documentation**: the web demo is a published copy; runtime
   extraction cannot be prevented; perturbation tools are unproven on
   sprites; Steam Playtest substitutes for a hosted preview (section 8).

Researched and dropped: string-encryption crates (section 2), pack
encryption (section 3), anti-tamper and injection detection (section 5),
art perturbation and image watermarks (section 10), launch-time
honeytokens without a product decision (section 7), model-provider policy
as a defence (section 9).

## Not researched, on purpose

Prompt-injection notices aimed at agents (excluded by 2915 and by
provider specs giving such text no authority); anything requiring hosted
networking (parked); console and mobile store test tracks (irrelevant to
Playdate, GBA and NDS); CrackMeBench's figures (PDF unreadable to the
agent); Google, Meta and xAI usage policies; itch.io's DMCA
responsiveness beyond one anecdote.

## Method and limits

Eleven agents, one on the repository (all tools, read-only) and ten on
the web, each capped near 900 words, run on a smaller model to limit
usage. About 5–17 tool calls each. Most web agents read search summaries
rather than full pages and said so; every effect-size number above is
their judgment unless a source is named. The repository findings were
re-read in the code by the coordinating session. No ADR changed. The
tickets named at the top were written from this pass; 2917 finishes the
report.
