# Reverse-engineering hardening: angles the ticket track had not covered

Research for ticket 2917, first pass **2026-11-08**, confirmed at its
sources **2026-11-14**. Research only: nothing here was attempted, built or
measured, and nothing is from *Visions of Shuyi*. Not legal advice anywhere.

**Only the engine's own leaks were worth engine work, and three tickets have
removed them (3099, 3303, 3300).** Every other angle either has no defence
that survives a player's machine or belongs to the reveal kit and the store
(3301, 3302). The book now says plainly what a web demo, a running game and
a store's own preview tools give away
([*Targets and export*](../../site/content/learn/book/targets-and-export.md)).
The confirmation pass corrected many details and struck a few, and changed
no decision: see [What changed on confirmation](#what-changed-on-confirmation).

## Decision

| Angle | Section | Became |
| --- | --- | --- |
| Release-build hygiene: symbols, debug features, crash play-record on demand, silent `wasm-opt` skip | 1 | 3099 (done, [ADR-0094](../adr/0094-release-exports-ship-no-symbols-no-debug-features-and-no-state-on-demand.md)), 3303 (done) |
| A baked pack for desktop and web, not the files folder as text | 1, 3 | 3300 (done, [ADR-0110](../adr/0110-desktop-and-web-exports-ship-a-baked-pack-beside-the-program.md)) |
| A store watch matched by name and picture, with notice templates | 11 | 3301 |
| A per-recipient overlay for closed builds, a deterrent never proof | 7 | 3302 |
| Register before the reveal; notice templates | 9 | a step of the reveal kit's checklist, 2816 (done) |
| A web demo is a published copy; runtime extraction can't be stopped; art perturbation is unproven on sprites; Steam's own tools are a controlled preview; attacker capability moved | 4, 5, 6, 8, 10 | the book's export chapter (this ticket) |

Dropped, with the reason:

- **String-encryption and obfuscation crates** (section 2): a dependency
  for no measured delay, and one of the four candidates is GPL-3.0.
  Control-flow flattening needs nightly.
- **Pack encryption** (section 3): a key the client uses is a key the
  client gives away; every engine's scheme in the table has a public tool.
- **Anti-tamper and injection detection** (section 5): proprietary and
  commercially licensed, so it can't ship in an MIT/Apache engine, and it
  targets cracking, not ripping.
- **Art perturbation and image watermarks** (section 10): no evaluation on
  small palette-limited sprites, published breaks, and no build-step tool.
- **Launch-time honeytokens** (section 7): personal data, so a product
  decision Nick hasn't taken.
- **Model-provider policy as a defence** (section 9): generic wording and
  open-weight models; 2912 already counts a refusal as a failure mode.
- **A hosted preview** (section 8): Steam's own tools give a controlled
  preview at no cost; the parked 2602 and 3083 stay parked.

## What the track already covers

Tickets 2912–2915 measured AI-assisted cloning by threat path, tested
perturbed gameplay clips (no-go), tested a diversified bytecode VM for a
rules kernel against dynamic analysis (no delay; stopped) and studied
capture flags, per-recipient video marks and AI-use signals. 3080–3082
cover export hygiene, a disclosure manifest and demo editions. 2816 is the
reveal kit. 2602, 2603 and 3083 (hosted preview, server-held content,
marked previews) are parked with networking.

Everything below is outside those.

## 1. The engine's own release builds leaked more than they needed to

Verified in the code on 2026-11-08; the paths are as they were then.
3099, 3303 and 3300 have since changed each row, as the Decision says.

| Finding (2026-11-08) | Where | What it handed an analyst |
| --- | --- | --- |
| Windows and Linux exports stripped **debug info only** (`objcopy --strip-debug`), keeping the full symbol table. | `crates/cli/src/export/desktop.rs`, `crates/cli/src/export.rs` | A free function map for Ghidra. |
| Desktop and web games embedded the whole `files/` tree with `include_dir!`, unbaked. Only the Playdate baked a pack. | `templates/shuyi-default/base/crates/game/src/lib.rs`, `crates/cli/src/export/playdate.rs` | Every rule table, map and line of text with `strings`. |
| Export refused the `link` feature but not `debug-tools`. | `crates/cli/src/export.rs`, `crates/app/src/link.rs` | A built-in oracle in a shipped build, if misconfigured. |
| Every release build wrote a play record into crash reports and honoured `SHUYI_PANIC_AT_FRAME`. | `crates/app/src/crash.rs`, `crates/app/src/lib.rs` | A state dump on demand. |
| `wasm-opt` was skipped **silently** when not on PATH. | `crates/cli/src/web.rs` | A web build that ships names. |
| Panic strings, `Ctx::log` text and the engine's env-var names stayed in every binary; saves and settings are RON text with struct names. | `Cargo.toml`, `crates/ui/src/ctx.rs`, `crates/core/src/save.rs` | String anchors an agent uses first. |

Not exposed by default: the tool contract, MCP server and problem codes
live only in `shuyi-cli`, which a shipped game does not link. The live link
needs a feature and a runtime gate, binds to localhost and needs a token.

Expected effect (section 2's estimate, unmeasured): perhaps one to three
hours of an agent's orientation time. It stops no one; it was the only
hardening here that cost nothing and had no cheaper bypass than the status
quo.

## 2. Rust build metadata and stable-toolchain obfuscation

Firmness: good on settings and papers (read at their sources), none on
effect sizes: no study measures Rust binaries against LLM agents, and none
isolates string encryption or symbol stripping.

- rustc's `--remap-path-scope` was stabilised by
  [rust-lang/rust#147611](https://github.com/rust-lang/rust/pull/147611),
  merged in January 2026 for 1.95; the PR says none of Cargo's side was
  stabilised, so `trim-paths` stayed unstable and the exporter's
  `--remap-path-prefix` route stays right. Remap the sysroot too, or
  `library/core/src/...` paths still leak.
- `lto = "fat"` with `codegen-units = 1` is the only free setting that
  hurts *structure* recovery rather than names, by erasing crate
  boundaries through inlining. The workspace uses `thin`.
- Removing panic strings entirely (`-Cpanic=immediate-abort`, formerly
  the `panic_immediate_abort` std feature) is nightly-only and rebuilds
  the standard library
  ([rust-lang/rust#147286](https://github.com/rust-lang/rust/issues/147286)),
  so it is out.
- Obfuscation crates on stable only encrypt strings or add junk:
  [`obfstr`](https://crates.io/crates/obfstr) (MIT, last release July
  2026), [`cryptify`](https://crates.io/crates/cryptify) (MIT, January
  2026), [`litcrypt2`](https://crates.io/crates/litcrypt2) (Apache-2.0,
  September 2024) and [`goldberg`](https://crates.io/crates/goldberg)
  (GPL-3.0, so denied here; dormant since 2022). Control-flow flattening
  ports go through rustc's unstable, internal
  [`-Z llvm-plugins`](https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/llvm-plugins.html),
  so nightly; out under the stable and GNU constraints.
- Evidence on LLM deobfuscation: Tkachenko, Suskevic and Adolphi 2025
  ([arXiv 2505.19887](https://arxiv.org/abs/2505.19887)) found bogus
  control flow the least resistant, flattening in the middle, and
  instruction substitution and combined techniques the most resistant.
  Archibald and Thijssen 2026
  ([arXiv 2606.06838](https://arxiv.org/abs/2606.06838)) found an agent
  driving Ghidra over MCP gave incomplete coverage and inconsistent
  improvements until a quantitative readability score guided it.
  Zhou 2025 ([arXiv 2507.18792](https://arxiv.org/abs/2507.18792)) calls
  Rust hard to decompile for its types, abstractions and aggressive
  optimisation, with no LLM data.

**No ticket beyond section 1.** A string-encryption crate adds a
dependency for an estimated zero to two hours of delay.

## 3. Pack-format protection: what other engines learned

Firmness: good on the tools (their repositories read), none on dated
time-to-defeat figures, which mostly do not exist.

| Engine | Protection | Defeat | Side effects |
| --- | --- | --- | --- |
| Godot | AES PCK, key in the exported binary | [KeyDot](https://github.com/Titoot/KeyDot) and GodotPCKExplorer find the key statically; [GDRE Tools](https://github.com/GDRETools/gdsdecomp) recovers the project given the key | Custom templates per platform |
| Unity | None by default; IL2CPP for code | AssetStudio, Il2CppDumper | Paid vendor add-ons |
| Unreal/Fortnite | AES pak/utoc, new dynamic keys each version | Community key services and archives ([fortnite-api.com](https://fortnite-api.com) `/v2/aes`, [fortnite-aes-archive](https://github.com/dippyshere/fortnite-aes-archive)) | The only measurable delay, for a game with a huge dataminer community |
| RPG Maker MV/MZ | First 16 bytes XORed with the `encryptionKey` from `System.json` | Immediate ([rmvdec](https://github.com/Bioruebe/rmvdec)) | None |
| Ren'Py | `.rpa` archives, compiled `.rpyc` | [unrpa](https://github.com/Lattyware/unrpa) opens `.rpa`; [unrpyc](https://github.com/CensoredUsername/unrpyc) decompiles `.rpyc` | Tooling churn |
| GameMaker | `data.win`; YYC | [UndertaleModTool](https://github.com/UnderminersTeam/UndertaleModTool) opens `data.win`; its code tools don't support YYC | Longer builds |
| LÖVE | None (zip) | n/a | n/a |

Lessons: a key the client can use is a key the client can extract; making
it harder to find bought no documented delay; per-release keys cost the
*community* effort only where a community exists; the index is the
ripper's map, so paths and kind names in it are the leak. For an open
engine the format is public by definition, so the pack's only honest job
is tidiness (section 1), not secrecy. **No encryption ticket**, which
confirms 2603's exclusion of a client-held key.

## 4. The web build is the cheapest extraction surface

Firmness: moderate. No sourced account of a full LLM clone from a deployed
Wasm and its assets; the nearest cases are partial.

Ranked by cost to the attacker: every asset is a URL in the Network tab;
the Wasm's imports, exports and data segments are core sections the loader
needs, so no strip removes them (inference, not a documented guarantee);
the `name` and `producers` custom sections go only if
`wasm-opt --strip-debug --strip-producers` or `wasm-tools strip --all`
runs ([Binaryen's passes](https://github.com/WebAssembly/binaryen/blob/main/src/passes/pass.cpp),
[wasm-tools `strip`](https://github.com/bytecodealliance/wasm-tools/blob/main/src/bin/wasm-tools/strip.rs));
linear memory is readable from DevTools or a userscript;
[Spector.js](https://github.com/BabylonJS/Spector.js) records a frame's
WebGL commands and state, texture inputs included; and the deterministic
core makes a clone's fidelity checkable against a recorded input trace.
Site-locking, signed URLs, CDN bot rules and JS obfuscation stop
hotlinking and scraping only; a person playing the game holds a valid URL.

Case to note: an October 2025 attempt to read Catan Universe's Unity Wasm
client with GLM-4.6 over the chrome-devtools MCP
([engineered.at](https://engineered.at/articles/reverse-engineering-catan-universe-gameplay-with-large-language-models))
captured nothing and mistook a particle system for a dice hook. Large
noisy Wasm is not trivially read dynamically; small Shuyi games will be
easier.

**The book's statement:** a web demo is a published copy of every file and
rule it was built with ([*A public demo*](../../site/content/learn/book/targets-and-export.md#a-public-demo)).

## 5. Runtime extraction on desktop defeats any at-rest protection

Firmness: moderate; tool capabilities are documented, countermeasure
claims are inference.

Graphics-API capture tools such as RenderDoc and
[apitrace](https://github.com/apitrace/apitrace) record an OpenGL game's
calls, its texture uploads among them. WASAPI loopback records the
shared-mode system mix; only exclusive-mode and DRM-protected streams
escape it ([Microsoft](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)),
and a game can use neither to hide its sound. Cheat Engine, ReClass and
Frida recover struct layouts, and community projects wire LLMs to them
([frida-game-hacking-mcp](https://github.com/0xhackerfren/frida-game-hacking-mcp);
a February 2026 showcase of Claude driving a memory-dumper DLL, read only
as a summary). Saves and replays as RON hand over the schema; binary
formats remove names but not shape. Commercial anti-tamper (Denuvo,
VMProtect, Themida, Arxan) is proprietary and commercially licensed, so it
can't ship in an MIT/Apache engine, and it targets cracking, not ripping.
Denuvo titles of 2025–2026 fell in days (hypervisor bypasses), about six
weeks (*Resident Evil Requiem*, [tbreak](https://www.tbreak.com/resident-evil-requiem-denuvo-cracked/))
or over a year (*Assassin's Creed Shadows*). Deterministic simulation
helps the cloner: it is a free test oracle.

**The book's statement:** anything decoded, drawn, heard or simulated on
the player's machine can be extracted, and no engine setting changes that
([*What a package gives away*](../../site/content/learn/book/targets-and-export.md#what-a-package-gives-away)).

## 6. Attacker capability has moved: whole-game ports are routine

Firmness: moderate; the catalogue is self-reported and the costs are
unsourced.

- [reversedwithai.com](https://reversedwithai.com) lists 594 projects on
  2026-11-14, 308 marked working, 367 of them games and mods; Claude
  models are named most often in the sample read.
- Kotaku reported working browser ports of *Halo: Combat Evolved*,
  *GTA: Vice City* and *The Simpsons: Hit & Run* made with AI decompilation
  on 2026-10-09; one moved to bring-your-own-files after a complaint
  (search summary only).
- Why native falls fastest: matching decompilation gives the agent a
  byte-exact pass/fail signal. Wasm and custom bytecode lack that answer
  key. That is the agents' inference, consistent with 2914's result that
  a VM bought nothing once the attacker had a *behavioural* oracle. The
  oracle, not 2914's timing, is the reason to believe it; see
  [What changed on confirmation](#what-changed-on-confirmation).
- [GhidraMCP](https://github.com/LaurieWired/GhidraMCP) (Apache-2.0), ReVa
  and [REA](https://github.com/morluto/rea) (MIT, 2026) make the
  decompiler loop free; the cost is model tokens.
- Valve's Tom Giardino and Marc Wilkinson said at Busan Indie Connect,
  reported 2026-10-28, that Valve won't be a gatekeeper against AI clones
  on Steam ([Game Developer](https://www.gamedeveloper.com/pc/valve-addresses-steam-ai-cloning-problem)).
- World models: Genie 4 (a research preview for trusted testers,
  2026-10-27, [Google](https://blog.google/innovation-and-ai/models-and-research/google-deepmind/introducing-genie-4/))
  keeps a world coherent for about two minutes by secondary reports.
  Video-to-playable is still not a product. Art is far easier to rip from
  the assets than from footage.
- [CrackMeBench](https://arxiv.org/abs/2605.10597) (David and Gervais,
  May 2026), not game-specific: on 12 generated C, Rust and Go tasks with
  executable oracles, pass@3 in five minutes was 11/12 for GPT-5.5, 7/12
  for Claude Opus 4.7 and 5/12 for Kimi K2 (as summarised from the paper's
  page).

**Implication for the track:** 2912's "behavioural replacement" track is
now the mainstream attack, not the exotic one. Any later measurement in
the track assumes a REA-class harness and a frontier model as its floor.

## 7. Fingerprinting the build artifact to trace a leak

Firmness: low to moderate. Theory is solid; practice for a one-person
studio is thin and documented leak-tracing successes are rare.

Naive marks (an ID string, a pack trailer) fall to a two-copy diff or a
repack. Collusion-resistant codes (Tardos; Škorić, Katzenbeisser and
Celik's symmetric version, about ten times shorter,
[ePrint 2007/041](https://eprint.iacr.org/2007/041)) need code length of
order c² log(n/ε): hundreds of independent, natural-looking positions per
copy, which a small Rust game does not obviously offer, and rustc/LLVM
expose no per-copy randomisation knob. Industry marks *content and
rendered output*: Denuvo TraceMark (GDC 2024,
[Irdeto](https://irdeto.com/news/denuvo-by-irdeto-unveils-tracemark-for-gaming-at-gdc))
marks screenshots and footage, which 2915 and 3083 already cover.
Confirmed cases of a press-build leaker identified by a binary watermark:
none found. Honeytokens that phone home on launch collect personal data,
so they need a privacy notice and Nick's decision. Reproducible builds
conflict with per-copy diversity unless the recipient mark is a small
overlay applied after a reproducible build.

**Became 3302:** a per-recipient overlay for closed builds in a signed
ledger, paired with per-recipient Steam keys; a deterrent and
circumstantial evidence, never proof.

## 8. What Steam and itch.io already give for free

Firmness: good; read on Valve's partner pages and itch.io's docs.

| Mechanism | Gives | Lacks |
| --- | --- | --- |
| [Steam Playtest](https://partner.steamgames.com/doc/features/playtest) (its own app ID) | Limited or open signup, Standard Release keys, friend invites | Testers are under no NDA; "Not Playable" stops launching but revokes no issued key; the documented way to remove everyone is an irreversible Reset |
| [Password beta branch](https://partner.steamgames.com/doc/store/application/branches) | A per-branch gate | One password per branch, so untraceable |
| [Release override keys](https://partner.steamgames.com/doc/features/keys) | Per-account access before release, revocable by banning the keys | You run signup and NDAs yourself |
| [Steam DRM wrapper](https://partner.steamgames.com/doc/features/drm) | Ownership check | Valve: protects against "extremely casual piracy", "easily removed by a motivated attacker" |
| [Separate demo app](https://partner.steamgames.com/doc/store/application/demos) | Its own depots; deactivatable; a one-time notification to wishlisters | A deactivated demo may still launch outside Steam |
| [itch.io Restricted page, download keys](https://itch.io/docs/creators/access-control) | Hidden from browse, search and profiles; a password option; revocable keys | No tracing |

Valve's Playtest page suggests a nondescript codename for an unannounced
game, and announcing before external playtests; a confidential test is
only as private as its least confidential tester. **This substitutes for
the parked hosted-preview tickets** (2602, 3083): a limited Playtest with
keys, or override keys plus a password branch, is a controlled preview at
no cost. The gaps, no NDA, weak revocation and no tracing, are what
2915/3083's marks and 3302's overlay would fill.

**The book's statement:** in [*Releasing*](../../site/content/learn/book/targets-and-export.md#releasing).

## 9. Law and model-provider policy (research, not advice)

Firmness: moderate on statute (read at its source) and well-known cases
(read in secondary accounts, not the opinions).

- Rules and mechanics are unprotected; specific expression is. *Tetris
  Holding v. Xio* (D.N.J. 2012) protected Tetris's specific look, not its
  rules; *Spry Fox v. LOLApps* survived a motion to dismiss and settled
  (2012); *DaVinci Editrice v. Ziko Games* (S.D. Tex. 2016) found the
  mechanics of *Bang!* unprotected. No court decision on an AI-made clone
  was found.
- [17 U.S.C. 1201](https://www.law.cornell.edu/uscode/text/17/1201)
  needs a measure that "effectively controls access". A per-recipient
  *mark* controls nothing; a trivial pack lock is a closer call and
  unlitigated for a small studio. The [2024 rulemaking](https://copyright.gov/1201/2024/)
  kept the game-preservation exemption, and its exemptions lapse on
  2027-10-28.
- EU Software Directive 2009/24/EC: articles 5(3) and 6 allow studying a
  program and decompiling it for interoperability, and article 9(1)
  voids contract terms against them (from memory of the text; EUR-Lex
  was unreachable). *SAS v. World Programming* (C-406/10) leaves
  functionality and languages unprotected. *MDY v. Blizzard* (9th Cir.
  2010, amended 2011) makes a breached covenant a contract claim, not
  infringement. EULA clauses against AI extraction have no case law.
- Steam removed *Allumeria* on a Microsoft DMCA notice on 2026-02-10 and
  restored it the next day when the notice was withdrawn; *No Players
  Online* was off sale about three weeks in late 2025 over an unfounded
  claim by a former collaborator (both in secondary reports). The statute
  restores counter-noticed content after 10 to 14 business days unless a
  suit is filed ([17 U.S.C. 512(g)](https://www.law.cornell.edu/uscode/text/17/512)).
- US copyright registration before infringement, or within three months
  of first publication, is the prerequisite for statutory damages and
  fees ([17 U.S.C. 412](https://www.law.cornell.edu/uscode/text/17/412));
  the [fee](https://www.copyright.gov/about/fees.html) is US$65 standard,
  US$45 single author and work. Trademark: USPTO US$350 per class since
  2025-01-18 (more for a free-text description); EUIPO about EUR 850 for
  one class (both from secondary pages).
- [Anthropic's usage policy](https://www.anthropic.com/legal/aup) forbids
  infringing third parties' intellectual property in generic terms and
  names neither DRM nor game cloning; OpenAI's page was unreachable.
  Open-weight models bypass policy entirely. Treat provider policy as
  light friction at most, consistent with 2912 counting refusals as a
  failure mode, not a defence.

**Became a step in 2816's checklist** (register before the reveal), which
the book's *Before the reveal* says.

## 10. Art style cloning and asset-side protection

Firmness: low; we found no test of any of this on small palette-limited
sprites.

Few-shot LoRAs reproduce a pixel-art style;
[Retro Diffusion](https://replicate.com/retro-diffusion/rd-animation)
sells reference-guided sprite and animation generation, and PixelLab
sells pose-based sprite animation. We found no evaluation of Glaze,
Nightshade, Mist, PhotoGuard or Anti-DreamBooth on 32×32 sprites (not
checked paper by paper), where a palette-quantised image leaves little
room for a perturbation. Published breaks stand: Hönig, Rando, Carlini
and Tramèr's robust mimicry
([arXiv 2406.12027](https://arxiv.org/abs/2406.12027), ICLR 2025);
LightShed (USENIX Security 2025, 99.98% detection of Nightshade-protected
images); and "Fragile by Design"
([arXiv 2511.10382](https://arxiv.org/abs/2511.10382)), on identity
protections. We found no documented CLI or API for these tools to run in
a build step. Pixel-level invisible watermarks are removed by
regeneration with a diffusion model
([arXiv 2306.01953](https://arxiv.org/abs/2306.01953)); a redraw carries
no mark by construction. Content credentials (C2PA) are metadata that a
strip loses, which is why durable credentials add watermarks. The only
item that works today is detection: perceptual hashes find ripped or
upscaled reuse, not redraws.

**The book's statement:** perturbation tools are unproven on small
palette-limited sprites ([*Before the reveal*](../../site/content/learn/book/targets-and-export.md#before-the-reveal)).
Perceptual hashes feed 3301.

## 11. Detect-to-takedown speed is the angle with the most leverage

Firmness: moderate on incidents, low on timings and false-positive rates.

| Case | Clone after | Gone after | Channel |
| --- | --- | --- | --- |
| Wordle, Jan 2022 | days | removed around 11–12 January 2022, days into the news cycle ([Game Developer](https://www.gamedeveloper.com/game-platforms/-i-wordle-i-clones-nuked-from-ios-app-store)) | Public backlash, then Apple's own rule |
| Needle In A Haystack Simulator, 2026 | days after a viral reveal; 8 copycat titles before release | none of the original's clones is reported removed; one clone *of a clone* left Steam around 2026-10-21, reason undisclosed | The other cloner's code-copy accusation, not a notice from the original's developer; the original added "original" to its key art |
| Allumeria (reverse case), Feb 2026 | n/a | removed and restored in about 24 hours | DMCA, withdrawn |
| Vampire Survivors mobile clones | months | not found | Poncle shipped its own free mobile port on 2022-12-08 ([Game Developer](https://gamedeveloper.com/business/-i-vampire-survivors-i-dev-credits-clones-for-surprise-mobile-release)) |
| Palworld | n/a | pending (latest reports found, mid-2026) | Patent suit in Tokyo, September 2024, not a takedown |

Valve's people, at Busan Indie Connect (reported 2026-10-28), suggested
making games harder to copy and building a community that prefers the
original, and pointed extortion cases to law enforcement
([Game Developer](https://www.gamedeveloper.com/pc/valve-addresses-steam-ai-cloning-problem)).
Impact evidence is anecdotal and mixed.

**Became 3301:** a store watch. Steam's
[`IStoreService/GetAppList`](https://partner.steamgames.com/doc/webapi/IStoreService)
takes a key and an `if_modified_since`, so a daily poll is cheap. Advisory
only: false-positive rates are unmeasured. A well-evidenced Steam takedown
within days rests on the Allumeria and Wordle rows only; no case in this
table shows an original developer's notice removing an AI clone. So
"shortens the clone's life" is an inference, not a measured case.
Detection is still a smaller demo-to-clone window than any technical
measure in this track has bought.

## What changed on confirmation

Entries 1 and 2 are ticket 2918's (2026-11-09); entry 3 is this ticket's
pass of 2026-11-14, claim by claim.

1. **Section 11, Needle In A Haystack Simulator: corrected.** The draft
   said a code-copy clone was gone within about nine days after the
   original developer's copyright notice with a file comparison. The
   sources say the removed game was a clone *of a clone*: NoGlyph's
   *Needle In A Haystack* accused Little Field Studios' *A Game About
   Needle In A Haystack* of copying its code, including its error
   messages, and that second clone left Steam around 2026-10-21 with no
   reason disclosed. No removal of any clone of the original is reported,
   and the original's developer is not the party that complained. The
   accusation itself is unverified. Sources: GamesRadar, "dev who copied
   a popular game on Steam complains another copycat stole our code"
   (2026-11); Kotaku, "devs behind Needle In A Haystack clone complain
   their clone has been cloned" (2026-11); GamesRadar on the eight
   copycats and the "original" key art (2026-09).
2. **Section 6 and the track summary, 2914: the reason, not the result,
   is restated.** 2914's no-go stands, but its measured delay (−6 min 22 s
   against the ordinary arm, +44 s against the layout control) is one
   analyst per arm with unequal shell stalls, and the report says so. The
   stronger ground is in the fixture: the protected kernel decides whether
   gathers plus a hidden phase is divisible by three, the phase is one of
   three values, and the trace emits a `BonusOre` event on every gather.
   An analyst learns the phase from the first three gathers without
   reading the bytecode, so no code-level defense could have scored in
   that experiment. The lesson this report draws from 2914 is therefore
   that a shipped deterministic game is its own oracle, which 2912's
   ordinary baseline (both fixtures replaced in about 15 minutes with
   about 900 queries) shows directly. 2914's attempt ledgers are not in
   the repository (2913's and 2915's are), so its protocol and code were
   checked, not its attempts.
3. **The 2026-11-14 pass.** K is kept, C corrected in the section above,
   S struck. The source is the one linked in the section unless named.

   | § | Claim | Verdict and reason |
   | --- | --- | --- |
   | 2 | `--remap-path-scope` stabilised early 2026, `trim-paths` not | K, dated: merged January 2026 for 1.95 |
   | 2 | `panic_immediate_abort` needs nightly and build-std | C: now `-Cpanic=immediate-abort`, still nightly and a rebuilt std |
   | 2 | `obfstr` quiet since 2023 | C: releases in June and July 2026 |
   | 2 | `litcrypt2` and `goldberg` licences (unstated) | C: Apache-2.0; GPL-3.0, denied by ADR-0002 §8 |
   | 2 | Flattening needs nightly `-Z llvm-plugins` and a matching LLVM | C: the flag is unstable and internal; "matching LLVM" unconfirmed, dropped |
   | 2 | Tkachenko et al.: models handle bogus flow *and substitution* | C: substitution is the most resistant technique |
   | 2 | Archibald and Thijssen: MCP agent did not reliably improve output | K, reworded to the paper's "incomplete coverage, inconsistent improvements" |
   | 2 | Decompile-Rust names inlining | C: the abstract says aggressive optimisation |
   | 3 | Godot: key in the export template; GDRE recovers projects | C: key in the binary; GDRE takes the key, KeyDot finds it |
   | 3 | RPG Maker XORs the header | C: the first 16 bytes after a fixed header |
   | 3 | FModel and community key archives, rotated per release | C: key services and archives confirmed; FModel's role unconfirmed, dropped; "new dynamic keys each version" |
   | 3 | unrpa and unrpyc open `.rpa`/`.rpyc` | C: one tool each |
   | 3 | YYC compiles to native, needs Ghidra | C: only "UndertaleModTool's code tools don't support YYC" confirmed |
   | 4 | `wasm-opt` strips `name` and `producers` | C: only with `--strip-debug --strip-producers`; `wasm-tools strip --all` removes all custom sections |
   | 4 | Imports, exports and data survive any strip | K as inference: core sections, no tool documents it |
   | 4 | Spector.js pulls every WebGL texture | C: a frame's commands and state, texture inputs included |
   | 4 | Catan Universe LLM attempt, Oct 2025 | K; the model was GLM-4.6 |
   | 5 | RenderDoc, apitrace, Ninja Ripper dump textures | C: apitrace confirmed; RenderDoc kept as common knowledge; Ninja Ripper unread, dropped |
   | 5 | WASAPI loopback excludes only DRM streams | C: exclusive-mode streams too |
   | 5 | Frida game-hacking MCP server, May 2026; Claude over a memory-dumper DLL | C: the server exists, date unconfirmed; the DLL showcase (Feb 2026) read only as a summary |
   | 5 | Anti-tamper is per-seat | C: "commercially licensed"; per-seat unsourced |
   | 5 | Denuvo titles fell in 41 days to over a year | C: days (hypervisor bypasses) to about six weeks to over a year |
   | 6 | reversedwithai.com: 323 games, 138 working, five named titles | C: 594 projects, 308 working, 367 games and mods on 2026-11-14; the five titles not seen, dropped |
   | 6 | Space Cadet Pinball to Rust in 2h25m for US$140 | S: no source found |
   | 6 | A web game cloned in under 30 minutes with REA and Codex | S: no source found |
   | 6 | GhidraMCP Apache-2.0; REA MIT, July 2026; ReVa; pyghidra-mcp | C: licences confirmed, REA's date not; pyghidra-mcp not found, dropped |
   | 6 | Valve announced no anti-clone measures on 2026-10-27 after browser ports; DMCA targeted game data | C: Valve spoke on AI clones on Steam, reported 2026-10-28, not on the ports; the ports are a separate Kotaku report; the DMCA detail unconfirmed, dropped |
   | 6 | Project Genie Jan 2026, Genie 4 preview Oct 2026: 60-second 720p | C: Genie 4 dated 2026-10-27; about two minutes per secondary reports; the January date and 720p unconfirmed, dropped |
   | 6 | CrackMeBench results not extracted | C: results added |
   | 7 | Tardos and Škorić's improvements | K, with the constant |
   | 7 | TraceMark marks rendered output | K: content, screenshots and footage, not the binary |
   | 7 | Half-Life 2 ATI trace is lore | S: not checked, and not needed |
   | 8 | Playtest: "the only revoke is a full reset" | C: Reset is the documented way to remove everyone |
   | 8 | Password branch, per-account download logging unconfirmed | C: one password per branch; logging dropped |
   | 8 | Demo app sends one wishlist notification | C: the developer may send it once, within two weeks of release |
   | 8 | Valve *recommends* a codename | C: suggests |
   | 8 | itch.io: one password at a time; HTML5 behaviour murky | C: the docs describe one password setting; HTML5 dropped |
   | 9 | Tetris v. Xio | C: a copyright ruling on specific expression |
   | 9 | 2024 exemptions; next cycle 2027 | C: preservation exemption kept; exemptions lapse 2027-10-28 |
   | 9 | EU Directive articles 6 and 8 | C: articles 5(3), 6 and 9(1) (from memory; EUR-Lex unreachable) |
   | 9 | Steam restores after about 10 business days | C: the statute says 10 to 14 |
   | 9 | No Players Online, a bogus claim | C: an unfounded claim by a former collaborator |
   | 9 | Copyright fee about US$65; USPTO US$350; EUIPO EUR 850 | K, secondary for the trademark fees |
   | 9 | Providers' policies are generic | K for Anthropic; OpenAI's page unreachable |
   | 10 | Retro Diffusion and PixelLab sell reference-guided generation | C: PixelLab's reference input unconfirmed; "pose-based" |
   | 10 | Perturbation tools untested on 32×32 sprites | C: "we found no evaluation", not checked paper by paper |
   | 10 | Robust mimicry, LightShed 99.98%, "Fragile by Design" | C: 99.98% is on Nightshade; "Fragile by Design" is about identity protection |
   | 10 | No CLI or API | C: none documented that we found |
   | 10 | C2PA in PNG stripped by any re-encode | C: metadata lost when stripped; the PNG wording unconfirmed |
   | 11 | Wordle clones gone in about two days | C: removed around 11–12 January 2022 |
   | 11 | Valve: don't rely on one mechanic; report extortion to Valve | C: make games harder to copy, build community, extortion to law enforcement |
   | 11 | Rotris clip (50 s), store clones in days, one in about a day | C: Freya Holmér's prototype (March 2026) was followed by a vibe-coded knockoff within five hours ([PC Gamer](https://www.pcgamer.com/gaming-industry/after-this-game-dev-shared-her-prototype-online-it-took-less-than-5-hours-for-someone-to-post-their-vibecoded-knockoff)); the name, clip length and store clones unconfirmed, dropped |
   | 11 | Palworld unresolved | K, as of mid-2026 reports |

   No confirmed source overturns a dropped angle, so no ticket follows.
   The corrections that matter for a decision: `goldberg` is GPL-3.0,
   which strengthens dropping the crates; Valve's 2026-10-28 remarks
   stand on their own, not as an answer to the browser ports; and the
   Holmér case is faster than the draft said (hours, not a day), which is
   2913's description bypass in the wild: the viewer is the decoder, and
   no pixel treatment reaches them.

## Not researched, on purpose

Prompt-injection notices aimed at agents (excluded by 2915 and by
provider specs giving such text no authority); anything requiring hosted
networking (parked); console and mobile store test tracks (irrelevant to
Playdate, GBA and NDS); Google, Meta and xAI usage policies; itch.io's
DMCA responsiveness beyond one anecdote.

## Method and limits

The first pass (2026-11-08) ran eleven agents on a smaller model, one on
the repository and ten on the web, each capped near 900 words and 5–17
tool calls. Most web agents read search summaries rather than pages, and
every effect-size number was their judgment. The repository findings were
re-read in the code by the coordinating session.

The confirmation pass (2026-11-14) ran six agents on a smaller model, one
per group of sections, each told to read the primary source behind each
claim and return keep, correct or strike with the URL; about 15–26 tool
calls each. Some primary pages refused automated reads (EUR-Lex, the Glaze
site, the USPTO and EUIPO fee pages, OpenAI's policy page, several news
sites); the table says where a verdict rests on a secondary page or
memory. Court opinions were not read. No ADR changed.
