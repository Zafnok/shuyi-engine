---
id: "3051"
title: "Parked: Signed packages that cost money: a Developer ID signature with notarization on macOS, and Windows exes signed through a signing service, with credentials only from CI secrets"
type: feature
milestone: "Parked: export targets"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3004", "3049"]
nick_input: decision
completed:
---

# 3051 — Signed packages: Developer ID and notarization on macOS, and Windows signing

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`). Like the other export work that costs money (3014–3016, 2026-10-15), it also waits for Nick's decision below. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8.

**The benchmarks:**

- Godot signs on export. On macOS it signs with Xcode's `codesign`,
  with `rcodesign` from Linux or Windows, or ad hoc when there is no
  certificate. It notarizes with `notarytool` or `rcodesign`, with an
  Apple ID or an App Store Connect API key
  (<https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_macos.html>).
  On Windows it signs with `signtool`, or `osslsigncode` on other
  systems.
- Unity and Unreal leave signing to steps the developer runs, and Bevy
  has no export step. So Godot is the one to match.

**What players see without it** (checked 2026-10-20):

- macOS 15 removed the Control-click way to open an app that isn't
  notarized. The player has to go to System Settings, Privacy &
  Security, and allow the app there.
- Windows SmartScreen blocks an unsigned download hard. A signed one
  can still warn until its publisher builds a reputation, and since
  2024 an EV certificate no longer skips that
  (<https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options>).

**What it costs** (checked 2026-10-20 on Apple's and Microsoft's pages;
re-check when unparked):

- **macOS:** the Apple Developer Program, 99 USD a year. Only its
  Developer ID certificates can be notarized.
- **Windows:**
  - Azure Artifact Signing (formerly Trusted Signing): about 9.99 USD a
    month. Individuals can join only in the US and Canada; organizations
    also in the EU and the UK. Its certificates last 3 days, so every
    signature needs a timestamp.
  - An OV certificate from a certificate authority: 150 to 300 USD a
    year. Since June 2023 its private key must live on a hardware token
    or a cloud HSM, so it can't sit in a CI secret as a `.pfx` file.
  - SignPath Foundation signs qualifying open-source projects for free.

**Shuyi today:**

- 3004 (`tickets/open/3004-shuyi-export-one-command-per-target.md`)
  plans `shuyi export`, the `Exporter` trait and the `macos` exporter,
  which builds on macOS runners only. Its text says "A `.app` folder,
  unsigned; signing is a later ticket and needs Nick's Apple account",
  and its Out list names signing and notarising. No ticket did it; this
  is that ticket. An edit to 3004 moves the free part there:
  - an ad-hoc signature on the `.app` (nested code deepest first, then
    the bundle, never `--deep`);
  - `signing` in `export-report.json`: `"none"` for `windows`, `linux`
    and `web`, `"ad-hoc"` for `macos`;
  - `HOW-TO-OPEN.txt` in every `windows` and `macos` package, telling a
    player how to open an unsigned build.
- 3048 adds the `app` part of `shuyi.ron` (`AppPart`: the reverse-DNS
  id, the version). 3049 writes it into the packages: `Info.plist` and
  the `.icns` in the `.app`, and the exe's version resource.
- 3050 releases a game from a tag. Each template gets a reusable
  `.github/workflows/export.yml` (one target on its runner) and a
  `release.yml` that calls it over a matrix. The engine keeps a
  byte-equal copy of `export.yml`. 3050 leaves signing to this ticket
  and names `SHUYI_SIGN` and `docs/signing.md`.
- `shuyi doctor` (`crates/cli/src/contract/doctor.rs`) checks Rust,
  cargo, rustup's targets, MSYS2's GCC on a Windows GNU host and the
  wasm target. It reads the host from `rustc -vV` and looks at the
  machine through the `Probe` trait (`output` and `exists`), which
  tests fake. Its input, `DoctorInput`, is empty, and it passes only
  when every check does. It knows no signing tool.
- The contract's version is `VERSION` in `crates/cli/src/contract.rs`
  (4 today). A change of shape regenerates `crates/cli/schemas/` with
  `cargo shuyi schema --out crates/cli/schemas`, raises `VERSION` and
  adds a line to `crates/cli/schemas/versions.json`
  (`crates/cli/tests/it/schemas.rs` checks all three).
- CI uses no signing secret: only `SONAR_TOKEN` and `GITHUB_TOKEN`
  (`.github/workflows/ci.yml`). Its `test` job runs on `macos-latest`
  as well as Windows and Linux.
- 3015 (parked) covers signing for the Android and iOS store builds.
  ADR-0023's signed packs are about a pack's integrity, which is a
  different thing.

**Why it matters:**

- Visions of Shuyi sells through Steam (game ticket 0903,
  `docs/decisions.md` 2026-10-15), so it doesn't need this. That is why
  it is parked.
- Any other game sold as a download (itch.io, its own site, a GitHub
  release) loses players at these warnings, and Godot spares its
  developers that.
- The end goal (2026-10-19) is a game with many ports in under an
  hour. A desktop build that players can open is part of a port.

## Nick input

**Decision** (the `ask-nick` skill), when unparked. He can answer
later: the free part (the ad-hoc signature and the player note) is in
3004 and doesn't wait for him.

- **Should the engine support paid signing now,** for Shuyi games sold
  outside Steam, or stay parked until a game sells a direct download?
  The engine never buys anything for a game: each game brings its own
  accounts.
- **Who checks the real path once?** The tests here use fakes. The
  first real run needs a paid Apple membership and a Windows signing
  service (the costs above). Options:
  - the first game that brings its own accounts checks it and reports
    (recommended: it costs Nick nothing);
  - Nick pays for a year to check it himself.
- **The engine's own example packages** (on the engine's GitHub
  Release, through 3050's job in 2005's tag workflow):
  - stay ad-hoc signed, with the player note (recommended);
  - or, on Windows, are signed for free through SignPath Foundation, if
    the engine qualifies (a follow-up ticket);
  - or are signed with Nick's paid accounts (a follow-up ticket with
    `nick_input: setup`); each secret the engine's own workflow then
    holds gets a row in `docs/continuity.md` (2016).

If he picks paying, the session gives him the exact setup steps then
(the Apple membership, a Developer ID Application certificate, an App
Store Connect API key; an Azure subscription, an Artifact Signing
account, its identity check and a certificate profile).

## Scope

**In:**
- **`shuyi export <target> --sign`** (3004's command; its contract
  input, `ExportInput`, gains `sign: bool`, default `false`):
  - `macos`: a Developer ID signature with the hardened runtime,
    notarized and stapled, or the export fails;
  - `windows`: the exe signed through Azure Artifact Signing, or the
    export fails;
  - any other target: a warning in the report and nothing else, so a
    release matrix can pass `--sign` to every target;
  - without `--sign` nothing changes: 3004's ad-hoc signature, and the
    player note.
- **Credentials come only from environment variables, with fixed
  names.** No credential and no name of one goes in `shuyi.ron`. The
  names are fixed because a workflow maps its own secrets onto them
  (`env: SHUYI_MACOS_CERTIFICATE: ${{ secrets.ANY_NAME }}`), so a game
  picks its secrets' names there. One table in the code (a `const`
  array in `crates/cli/src/sign.rs`) and in the docs:

  | Variable | Holds | Secret |
  | -------- | ----- | ------ |
  | `SHUYI_MACOS_CERTIFICATE` | the Developer ID Application certificate with its private key, a `.p12` file in base64 | yes |
  | `SHUYI_MACOS_CERTIFICATE_PASSWORD` | the `.p12` file's password | yes |
  | `SHUYI_NOTARY_KEY` | an App Store Connect API key (`.p8`), in base64 | yes |
  | `SHUYI_NOTARY_KEY_ID` | its key id | no |
  | `SHUYI_NOTARY_ISSUER` | its issuer id | no |
  | `SHUYI_AZURE_SIGNING_ENDPOINT` | the Artifact Signing account's regional endpoint | no |
  | `SHUYI_AZURE_SIGNING_ACCOUNT` | the account's name | no |
  | `SHUYI_AZURE_SIGNING_PROFILE` | the certificate profile's name | no |
  | `AZURE_TENANT_ID`, `AZURE_CLIENT_ID` | the service principal the signing client signs in with (the Azure SDK reads these names itself) | no |
  | `AZURE_CLIENT_SECRET` | that service principal's secret | yes |

  - They are read through a trait, `Env { fn var(&self, name: &str) ->
    Option<String>; }`, with `ProcessEnv` (`std::env::var`) for real
    use and a map in tests: `Credentials::from_env(&dyn Env, target)`.
  - An empty value counts as missing, because GitHub gives an empty
    string for a secret that isn't set.
  - A missing variable gives one error that lists every missing name
    for the target, and no value.
- **Secrets are never shown.**
  - A `Redactor` holds every value marked secret above, the temporary
    keychain's password, the base64 text of each, and the decoded text
    of a base64 value when it is UTF-8 (the `.p8` key). It also matches
    base64 text with a line break every 64 or 76 characters, as
    `openssl` and `base64` wrap it. Each match becomes `***`.
  - It works on a tool's whole captured stdout and stderr, not line by
    line, so a value split across two reads is still found. All a child
    tool prints passes through it before it reaches stdout, an error,
    `export-report.json` or the contract's answer.
  - A command line the exporter prints shows `***` for any argument
    that holds a secret.
  - The `.p12` and `.p8` files go in a fresh folder in the system's
    temp folder, made with mode `0o700` on Unix
    (`std::os::unix::fs::DirBuilderExt`). A guard deletes it on drop,
    so on every way out, errors and panics included (the workspace
    doesn't set `panic = "abort"`).
  - Decoding base64: the `base64` crate, `base64 = "0.22"` (MIT OR
    Apache-2.0, both in `deny.toml`'s allow list; its default features
    are `std` only), in `shuyi-cli` only. A game's shipped binary never
    links `shuyi-cli` (the templates take it only under the `tools`
    feature).
- **Either order with 3015** (mobile): if 3015's `ios` exporter is
  done, it already wrote a temporary keychain step and a `Redactor`:
  use them here (moved into `crates/cli/src/sign.rs` and
  `crates/cli/src/sign/macos.rs` if they live elsewhere), adding the
  desktop's secrets to the `Redactor`, instead of writing new ones; if
  not, write them here, and the `ios` exporter reuses them (its scope
  says so).
- **One seam for the tools.** The signers run every outside tool
  through a trait, `ToolRunner { fn run(&self, program: &str, args:
  &[String], env: &[(String, String)]) -> io::Result<ToolOutput>; }`,
  where `ToolOutput` has `success: bool`, `stdout` and `stderr`. If
  3004 already has such a seam, use it instead. `SystemRunner` runs
  `std::process::Command` with `env` added to the inherited
  environment. Tests give a fake that records each call and answers
  with canned output.
- **macOS, on a macOS runner** (3004 builds macOS there only). It runs
  after every file inside the `.app` is written (3049's `Info.plist`
  and `.icns`, the licenses), because any later change breaks the
  signature. A file beside the `.app` in the zip doesn't matter; if
  3004 put `HOW-TO-OPEN.txt` inside the `.app`, it is written first
  too. In order:
  1. A temporary keychain in the temp folder, with a password made from
     32 bytes of `/dev/urandom` in hex:
     - `security list-keychains -d user` (the old list, kept);
     - `security create-keychain -p <pw> <kc>`,
       `security set-keychain-settings -lut 21600 <kc>` and
       `security unlock-keychain -p <pw> <kc>`;
     - `security import <p12> -k <kc> -P <p12 password> -f pkcs12 -T
       /usr/bin/codesign`;
     - `security set-key-partition-list -S apple-tool:,apple:,codesign:
       -s -k <pw> <kc>`, so `codesign` can use the key without a
       prompt;
     - `security list-keychains -d user -s <kc> <old list>`.

     A guard runs `security list-keychains -d user -s <old list>` and
     `security delete-keychain <kc>` on drop.
  2. The identity: the one `Developer ID Application` identity that
     `security find-identity -v -p codesigning <kc>` lists. None, or
     more than one, is an error that names what it found.
  3. `codesign --force --options runtime --timestamp --keychain <kc>
     --sign <identity>` on each piece of nested code, deepest first,
     then on the bundle. Not `--deep`, which Apple advises against for
     signing. If 3004's exporter joins architectures with `lipo`,
     signing comes after it, because `lipo` drops signatures.
  4. No entitlements. The backend (macroquad, its audio, gilrs) needs
     no JIT, no unsigned memory and no library loaded from outside the
     system. If notarization or a run shows one is needed, add only
     that one, in a file under `crates/cli/`, and say why in the PR.
  5. `codesign --verify --strict --verbose=2` on the bundle.
  6. `ditto -c -k --keepParent <app> <zip>` for upload, then:
     - `xcrun notarytool submit <zip> --key <p8> --key-id <id> --issuer
       <id> --output-format json`, which uploads and answers the
       submission's `id` at once;
     - `xcrun notarytool wait <id> --timeout 30m --key … --output-format
       json`;
     - `Accepted` goes on;
     - any other status (`Invalid`, `Rejected`) runs `xcrun notarytool
       log <id> --key …` and puts its `issues` in the error;
     - a timeout gives an error with the submission id and the `xcrun
       notarytool info <id>` command that checks it later.
  7. `xcrun stapler staple <app>`, then `xcrun stapler validate <app>`,
     then `spctl --assess --type execute --verbose=4 <app>`, read from
     stdout and stderr, which must say `source=Notarized Developer ID`.
     If it says `override=security disabled` (Gatekeeper is off on that
     runner), that is a warning in the report, since `stapler validate`
     already passed.
  8. The package's zip is made again from the stapled `.app` with
     `ditto -c -k --keepParent`. A zip writer that drops file modes or
     links breaks the signature.
- **Windows, on a Windows runner.** In order:
  1. Find `signtool.exe`: `SHUYI_SIGNTOOL` if set, else the newest
     version folder in `C:/Program Files (x86)/Windows Kits/10/bin/`
     that has `x64/signtool.exe`. Find the signing client's dlib:
     `SHUYI_ARTIFACT_SIGNING_DLIB` if set, else where the client tools
     install it (`winget install -e --id
     Microsoft.Azure.ArtifactSigningClientTools`; check the id, the
     path and the minimum `signtool` version on the day). The dlib's
     architecture must match `signtool`'s (x64). These two are paths,
     not secrets. One function, `find_signing_tools(&dyn Probe, &dyn
     Env)`, finds both, for `export` and for `doctor`.
  2. A `metadata.json` with `Endpoint`, `CodeSigningAccountName` and
     `CertificateProfileName`, in the temp folder.
  3. `signtool sign /v /fd SHA256 /tr http://timestamp.acs.microsoft.com
     /td SHA256 /dlib <dlib> /dmdf <metadata.json> <exe>`. The signer
     passes `AZURE_TENANT_ID`, `AZURE_CLIENT_ID` and
     `AZURE_CLIENT_SECRET` in the call's `env`, read through `Env`,
     never as arguments.
  4. `signtool verify /pa /v <exe>`.
  5. Signing comes after the exe is linked with 3049's version resource,
     and before the zip.
  - The signer is behind a trait, `WindowsSigner`, with
    `ArtifactSigning` its one implementation, so a token or another
    cloud service can be added later.
  - This runs in CI. Nick's machine has no Windows SDK and never needs
    one (`AGENTS.md`, *Environment*).
- **The report** (3004's `export-report.json`): `signing` gains
  `"developer-id-notarized"` and `"authenticode"`, beside 3004's
  `"none"` and `"ad-hoc"`. It also records:
  - `signer`: the certificate's subject, which is public. On macOS the
    first `Authority=` line of `codesign -d --verbose=2`; on Windows
    the `Issued to:` line of `signtool verify /v`;
  - `notary_id`, for macOS: the submission's `id`.
- **The player note** (3004's `HOW-TO-OPEN.txt`): a notarized `macos`
  package leaves out the macOS lines, and the file if nothing is left.
  A signed `windows` package keeps the SmartScreen lines, reworded: a
  new release may still be warned about for a while, and More info,
  then Run anyway, opens it.
- **`shuyi doctor --signing`.** `DoctorInput` gains `signing: bool`,
  and the usage line in `crates/cli/src/contract/cli.rs` becomes
  `doctor [--signing] [--json]`.
  - On a host whose triple ends in `apple-darwin`: `/usr/bin/codesign`
    exists, and `xcrun --find notarytool` and `xcrun --find stapler`
    answer.
  - On a host whose triple holds `windows`: `signtool.exe` and the
    dlib, through `find_signing_tools`. `Probe` gains `fn list(&self,
    dir: &Path) -> Vec<String>` (the entries' names; empty when the
    folder can't be read), for the Windows Kits lookup.
  - On any other host: one failing check that says signing runs on
    macOS and Windows runners.
  - Plain `doctor` is unchanged, so Nick's machine still passes it.
  - This changes `shuyi-cli`'s public API: `Probe` gains a required
    method (nothing outside `shuyi-cli`'s own tests implements it, so a
    game builds unchanged), and `DoctorInput` gains a field. The PR
    says so, and if 2010's job reports a break, the completion notes
    carry its `Breaking:` line.
- **The contract:** `VERSION` goes up, the schemas are regenerated with
  `cargo shuyi schema --out crates/cli/schemas`, `versions.json` gets
  the new line, and `docs/contract.md`'s `export` and `doctor` rows name
  `--sign` and `--signing`.
- **A page for a game's owner:** `docs/signing.md`. The templates'
  "Releasing" sections link it (the part on 3050 below says who adds
  the link). It says:
  - what each option costs, checked that day;
  - the steps to get each credential;
  - the variables' table;
  - a workflow step that passes `--sign` on tags only (pull requests
    from forks get no secrets);
  - what players see with and without signing.

  **Either order with 2802:** if 2802 is done, the book's chapter 9
  ("Targets and export") gets a short "Signing" section that links
  it; if not, 2802 writes that section (its chapter 9 says so).
- **Either order with 3050:**
  - If 3050 is done, each template's `release.yml` passes `export.yml`
    an input `sign`, true only on a tag push when the repository
    variable `SHUYI_SIGN` is `true` (never on a dry run). It passes the
    signing secrets by name, not with `secrets: inherit`, which zizmor
    flags. `export.yml` declares them as optional secrets, maps them
    onto the variables above in the export step's `env:` and passes
    `--sign` when `sign` is true. The engine's own
    `.github/workflows/export.yml` stays a byte-equal copy. The
    workflow names Actions secrets with the variables' own names, and
    `docs/signing.md` says so. Each template's `README.md` and
    `AGENTS.md` "Releasing" sections say signing is opt-in through
    `SHUYI_SIGN` and link the engine's `docs/signing.md`, and their
    macOS line says the `.app` has no Developer ID signature unless it
    is on.
  - If not, `docs/signing.md` carries the step, and 3050 takes it in
    and links the page from the "Releasing" sections (its scope says
    so).
- **A real run without an account, if it works:** an integration test,
  `crates/cli/tests/sign_macos.rs`, marked `#[ignore]`, and a
  `sign-macos` job in `.github/workflows/ci.yml` on `macos-latest` that
  runs it with `cargo test -p shuyi-cli --test sign_macos --
  --ignored`.
  - The test makes `Fixture.app` in a temp folder: a minimal
    `Contents/Info.plist`, and `Contents/MacOS/fixture`, a copy of the
    test's own executable (`std::env::current_exe()`).
  - It makes a self-signed certificate with `openssl`: a key, then a
    certificate with `keyUsage=digitalSignature` and
    `extendedKeyUsage=codeSigning`, then a `.p12` (with OpenSSL 3,
    `pkcs12 -export -legacy`, so `security import` reads it; the
    LibreSSL in `/usr/bin/openssl` writes the old format by default).
  - It runs the real keychain, `codesign` and verify steps through
    `SystemRunner`, stopping before notarization, and checks that the
    keychain is gone and the search list is back.
  - The identity lookup is a parameter for this test only: in real use
    it is `Developer ID Application` with `-v`; here it is the
    certificate's common name without `-v`, because `-v` lists only
    trusted identities.
  - If `codesign` refuses an untrusted self-signed identity, drop the
    test and the job and say so in the PR.
- **Split rule:** if Nick unparks one OS without the other, or the PR
  passes about 600 changed lines (fixtures and snapshots aside),
  Windows moves to a parked follow-up ticket and macOS stays here.

**Out (do not do):**
- buying, creating or setting up any account, Nick's or a game's;
- the ad-hoc signature, the `signing` field's first values and
  `HOW-TO-OPEN.txt` (3004, by its edit);
- the bundle id, `Info.plist`, the `.icns` and the exe's version
  resource (3048, 3049), and releasing from a tag (3050);
- a `.dmg`: a follow-up if a game asks. A zip of a stapled `.app`
  opens without the network;
- installers (MSI, MSIX, `.pkg`), the Mac App Store and the Microsoft
  Store;
- signing macOS apps off a Mac with `rcodesign` (MPL-2.0) or Windows
  exes off Windows with `osslsigncode` (GPL-3.0). As outside tools that
  never ship they would be allowed, but 3004 builds each on its own
  runner, so neither is needed;
- OV or EV certificates on a token or another cloud HSM, and SignPath:
  a follow-up when a game or Nick picks one;
- notarizing with an Apple ID and an app-specific password: one path,
  the API key, so there is one path to test;
- entitlements beyond none;
- Linux, the web and the Steam Deck (nothing to sign; Steam is the
  game's 0903), the Playdate, consoles and mobile (3015);
- signing packs (ADR-0023);
- signing the engine's own example packages, and any secret of the
  engine's own repository: SignPath or Nick's accounts are each a
  follow-up ticket if Nick picks them, and its secrets get rows in
  `docs/continuity.md` (2016).

## Implementation steps

1. Ask Nick (the `ask-nick` skill) the questions above, and record his
   answers in `docs/decisions.md`. Re-check the facts in *Context*:
   prices, who can join Artifact Signing, the client tools' winget id
   and dlib path, the minimum `signtool` version, `notarytool`'s flags,
   and which tools GitHub's macOS and Windows runner images carry.
2. Read 3004's exporters, `ExportInput`, the report and
   `HOW-TO-OPEN.txt`; 3049's `Info.plist`, `.icns` and version
   resource; 3050's `export.yml` and `release.yml` if it is done; and
   `Probe` in `crates/cli/src/contract/doctor.rs` as it stands (2513
   may have added `read`).
3. Add `base64 = "0.22"` to `crates/cli/Cargo.toml`, and
   `proptest.workspace = true` to its dev-dependencies. Run `cargo deny
   check licenses`.
4. Write `crates/cli/src/sign.rs`: the variables' table, `Env`,
   `ProcessEnv`, `Credentials::from_env`, the `Redactor`, the temp
   folder guard, `ToolRunner`, `ToolOutput` and `SystemRunner`, with
   their tests.
5. Write `crates/cli/src/sign/macos.rs`: the keychain guard and the
   eight steps, with fake-runner tests.
6. Write `crates/cli/src/sign/windows.rs`: `find_signing_tools`,
   `WindowsSigner` and `ArtifactSigning`, with their tests.
7. Add `--sign` to `export`, the report's fields and the changes to
   `HOW-TO-OPEN.txt`. Raise `VERSION`, regenerate the schemas, add the
   `versions.json` line and update `docs/contract.md`.
8. Add `doctor --signing` and `Probe::list`.
9. Write `crates/cli/tests/sign_macos.rs` and the `sign-macos` job. Keep
   them if `codesign` accepts the self-signed identity.
10. Write `docs/signing.md`, the book's section if 2802 is done, and
    the workflow changes and the templates' "Releasing" link if 3050
    is done.
11. Run the gates.

## Acceptance criteria

- [ ] Nick's decision is recorded in `docs/decisions.md`.
- [ ] With a fake runner, `shuyi export macos --sign` runs the steps in
      the order above: keychain made and imported, nested code then the
      bundle with `--options runtime --timestamp`, verify, submit,
      wait, staple, validate, `spctl`, keychain removed and the search
      list restored (test).
- [ ] A failure at any step leaves no temporary keychain, `.p12` or
      `.p8` behind, and restores the keychain search list (a test fails
      the fake at each step in turn).
- [ ] A missing or empty variable gives one error naming every missing
      variable, and no value (test).
- [ ] No secret value, its base64 text (plain or wrapped at 64 or 76
      columns) or the decoded `.p8` text appears in stdout, stderr, an
      error, `export-report.json` or the contract's answer. A test uses
      distinctive values and a fake tool that echoes its arguments and
      its environment.
- [ ] A notarization that comes back `Invalid` gives an error with the
      log's issues, and a timeout gives the submission id and the
      `notarytool info` command (tests, with canned output).
- [ ] With a fake runner, `shuyi export windows --sign` runs `signtool
      sign` with `/fd SHA256`, `/tr`, `/td SHA256`, `/dlib` and
      `/dmdf`, then `signtool verify /pa`. The three `AZURE_` variables
      are in its `env` and in none of its arguments (test).
- [ ] `find_signing_tools` picks the newest of several Windows Kits
      versions, and `SHUYI_SIGNTOOL` overrides it (test).
- [ ] `--sign` on `linux` and `web` adds a warning and doesn't fail
      (test).
- [ ] The report's `signing`, `signer` and `notary_id`, and
      `HOW-TO-OPEN.txt` for each case, are snapshot-tested.
- [ ] `shuyi doctor --signing` gives the right checks on fake macOS,
      Windows and Linux hosts, and plain `doctor`'s output is unchanged
      (test).
- [ ] `VERSION` is raised, the schemas are regenerated, `versions.json`
      has the new line, and `docs/contract.md` names `--sign` and
      `--signing`.
- [ ] `docs/signing.md` lists the costs as checked that day, the steps,
      the variables and the workflow step.
- [ ] Either the `sign-macos` job signs `Fixture.app` with a
      self-signed certificate through the real keychain and `codesign`
      steps and `codesign --verify --strict` passes, or the PR says why
      `codesign` refused it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: reading the variables (all there, some missing, some empty);
  the `Redactor` (a value, its base64 text, its base64 text wrapped at
  64 columns, the decoded `.p8` text, a value split across two reads);
  the order of the macOS steps and of the Windows steps; the temp
  folder and keychain guards on every failing step; reading
  `notarytool`'s JSON (`Accepted`, `Invalid`, a timeout); reading
  `spctl`'s answer (notarized, Gatekeeper off); finding `signtool`
  among several Windows Kits versions; `doctor --signing` on each fake
  host.
- Property (proptest): for any secret of 8 to 64 characters from
  `[A-Za-z0-9+/=_-]` and any tool output that holds it (alone, in
  base64, inside other text), the redacted output never holds it.
- Snapshot / integration: the report and `HOW-TO-OPEN.txt` for
  unsigned, ad-hoc, notarized and Authenticode packages; the
  regenerated schemas; on the macOS runner, `sign_macos` if it works.

## Completion notes

*(Filled in by the session that completes the ticket.)*
