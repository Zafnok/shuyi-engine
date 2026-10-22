---
id: "2601"
title: "Parked: HTTP requests as a platform capability: an ADR, Platform::http, a request handshake through Ctx that never blocks a frame, and scripted replies, with the ureq and fetch backends as two follow-up tickets"
type: feature
milestone: "Engine: networking"
model: opus-5.5
effort: high
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2601 — HTTP requests as a platform capability

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`). It needs nothing from 1140, so it may be unparked on its own, with its first consumer: sending crash reports (3047), high scores on a web service (a `Leaderboards` backend under 3054), a mod-hosting service, or 1148's client if that talks to its services over HTTP. Its two follow-ups (the native and web backends, written in step 2) are unparked with it, and a consumer that sends from a real build waits for them too. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and Defold. Each of them lets a game call a web API:

- Godot has the `HTTPRequest` node, with CA certificates bundled
  (<https://docs.godotengine.org/en/stable/classes/class_httprequest.html>);
- Unity has `UnityWebRequest`, and Unreal its HTTP module;
- Defold has `http.request` (<https://defold.com/ref/stable/http/>);
- Bevy is narrower: it loads assets from `http://` and `https://`
  sources (its `http` and `https` features), with no request API.

What Shuyi has today (checked 2026-10-20 and 2026-10-21):

- **No HTTP anywhere.** `Cargo.lock` has no HTTP or TLS crate (no
  `ureq`, `reqwest`, `hyper`, `rustls` or `ring`), and nothing in
  `crates/` sends a request.
- **The `Platform` trait** (`crates/platform/src/platform.rs`) has
  `display`, `present`, `poll_input`, `system_events`, `storage`,
  `audio` and `text_input`. None reaches a network.
- **ADR-0006 §1** plans peer transports (in memory, a WebSocket to our
  relay, UDP on a LAN, Steam, later WebRTC or WebTransport) and services
  on our own server. A request to someone else's web API is neither.
  1141 (a native WebSocket) and 1142 (`net.js`, a WebSocket in the
  browser) carry bytes between peers and wait for 1140. 1148's client
  API names no transport.
- **The web build can't use an HTTP crate.** macroquad's loader doesn't
  use `wasm-bindgen` (game ADR-0034), so `reqwest`, `ehttp` and `gloo`
  don't run there. On native a game could call `ureq` itself; on the web
  only the engine can give it HTTP, through a miniquad JS plugin like
  `crates/cli/web/gamepad.js`, whose Rust side is
  `crates/app/src/pads/web.rs` (the only file in `shuyi-app` with
  `#[allow(unsafe_code)]`; its imports are `safe fn`s that take and
  return plain numbers). `cargo shuyi web` writes the files listed in
  `SHELL` (`crates/cli/src/web.rs:25`), and `shuyi-cli` tests keep the
  plugin's functions and version equal on both sides
  (`gamepad_js_provides_exactly_the_functions_the_app_imports` and the
  two tests after it).
- **A handshake for work the platform does between frames exists:** the
  system keyboard. A screen asks through `Ctx::system_keyboard`
  (`crates/ui/src/system_keyboard.rs`); `App`'s `step`
  (`crates/platform/src/app.rs`) hands the ask to
  `Platform::text_input` after the update, and the answer back before a
  later frame. `HeadlessPlatform` scripts it with `ScriptedKeyboard`
  (`crates/platform/src/headless.rs`).
- **A worker for slow work exists on native:** `Pending` in
  `crates/app/src/audio/native_music.rs` runs a closure on a thread of
  its own and is polled each frame; dropping it detaches the thread.
- **TLS has a rule and two traps.**
  - 1141 sets the rule: rustls with ring, never aws-lc-rs, which needs
    NASM on the GNU toolchain. rustls 0.23's default features turn
    aws-lc-rs on.
  - ureq 3's default TLS features pull `webpki-roots`, and
    `rustls-platform-verifier` pulls `webpki-root-certs` on wasm32.
    Both are CDLA-Permissive-2.0 (their manifests, `webpki-roots` 1.0.0
    and `webpki-root-certs` 1.0.6), which is not on `deny.toml`'s allow
    list. ureq's default roots (`RootCerts::WebPki`) panic when
    `webpki-roots` is off, so the roots must be set.
  - `deny.toml`'s `[graph]` has `all-features = false`, and CI's
    `deny`, `clippy` and `test` jobs (`.github/workflows/ci.yml`) build
    default features only. A crate behind an off-by-default feature is
    not checked today.

Why it matters:

- **The game first.** Visions of Shuyi doesn't ask for HTTP: its online
  question is its ticket 0053, after launch. Nothing here changes it.
- **Other 2D games** reach web APIs for their small online parts:
  sending crash reports (3047), high scores on a web service, mods from
  a mod-hosting service, a daily level, news on a title screen. Each one
  stalls on the web build without this, and on native each game would
  write its own thread code.

## Nick input

None to build it. Unparking is Nick's (the note above). The engine
sends nothing by itself; a game that sends something about a player
takes on the privacy duties for it.

## Scope

**This ticket is the platform side.** The whole capability is three
PRs: this one (the ADR, the types and their rules, the handshake, `App`
and the scripted backend: plain Rust, no new crate), then the native
backend, then the web backend. Step 2 writes the last two as tickets,
from the two sections after **Out**.

**In:**

- **A new ADR** (the next free number, `write-adr` skill), "HTTP
  requests are a platform capability". It extends ADR-0006 §1 and §6,
  ADR-0024 §2 and ADR-0002 §8; accepted ADRs are not edited. It says:
  1. a request to a web API is a platform capability beside ADR-0006's
     four layers, not a `Transport`, and it needs nothing from 1140;
  2. a reply is platform input. It reaches a turn game's rules only as
     a command a screen issues, and a tick game's only as an input for a
     step, so a record replays without the network, and `shuyi replay`,
     bots and the tool contract never fetch;
  3. HTTPS only (`http://` to loopback only, for tests), the limits
     below, and the same refused headers on every backend, so a request
     that works on native works in the browser;
  4. TLS is rustls on ring with the system's root certificates; never
     `aws-lc-rs`, never `webpki-roots` or `webpki-root-certs`;
  5. in the browser, our own fetch plugin; CORS is the server's to
     allow;
  6. it is opt-in: the `http` feature of `shuyi-app`, off by default, so
     a game without it pays nothing in crates or WASM size;
  7. the engine never sends anything by itself. Each request carries a
     `purpose`, which the log shows and a game's privacy notice lists.
- **`shuyi_ui::http`** (`crates/ui/src/http.rs`), `no_std` + `alloc`
  (through `crate::prelude`), the screens' side, in the style of
  `system_keyboard.rs`:
  - `Method`: `Get`, `Post`, `Put`, `Patch`, `Delete`;
  - `HttpRequest { method, url: String, headers: Vec<(String, String)>,
    body: Vec<u8>, timeout_ms: u32, max_response_bytes: u32, purpose:
    &'static str }`, with `HttpRequest::get(url, purpose)`,
    `HttpRequest::post(url, purpose, content_type, body)`,
    `with_header`, `with_timeout_ms` and `with_max_response_bytes`;
  - `RequestId(NonZeroU32)`;
  - `HttpResponse { status: u16, content_type: Option<String>, body:
    Vec<u8> }`. A 404 or a 500 is a response, not an error;
  - `HttpReply`: `Response(HttpResponse)` or `Failed(HttpError)`;
  - `HttpError` (`thiserror`, as `shuyi-ui` already uses it):
    `Unsupported` (no HTTP on this platform), `Busy` (every slot taken),
    `BadUrl`, `BadRequest` (a refused header, a body too big), `Timeout`,
    `TooLarge` (the response passed its limit), and `Network(String)`
    (no connection, DNS, TLS, or a refusal by the browser, CORS
    included, which the browser doesn't tell apart);
  - the limits, as constants: 8 requests at once (`MAX_REQUESTS`); a
    timeout of 30 s by default and 120 s at most; a response of 1 MiB by
    default and 16 MiB at most; a request body of 1 MiB at most; a URL of
    2048 bytes at most; 16 headers at most. Larger timeouts and response
    limits are clamped; a body, URL or header list past its limit is
    `BadRequest` or `BadUrl`;
  - **the URL rule**, checked without a URL crate: `https://` and a
    host, or `http://` to `localhost`, `127.0.0.1` or `[::1]` with any
    port; no user or password before the host (an `@` in the
    authority); no spaces or control characters;
  - **the header rule:** names are tokens, values have no CR or LF, and
    the fetch standard's forbidden request headers
    (<https://fetch.spec.whatwg.org/#forbidden-request-header>) are
    refused, the `Proxy-` and `Sec-` prefixes included;
  - **`HttpQueue`**, the handshake:
    - the screens' side: `is_present()`; `send(request) ->
      Result<RequestId, HttpError>`, which checks the rules and the
      slots and is `Unsupported` without HTTP; `cancel(id)`;
      `take_reply(id) -> Option<HttpReply>`. A request holds its slot
      from `send` until its reply is taken or it is cancelled, so the
      docs on `send` say a screen that leaves cancels what it sent;
    - the platform's side: `set_present(bool)` before each frame (HTTP
      that goes away fails what waits with `Unsupported`, as the
      keyboard cancels); `take_sent()`; `take_cancelled()`;
      `answer(id, reply)`, which drops a reply to a cancelled or unknown
      id;
  - `Ctx` gains `pub http: HttpQueue`, absent until the platform says
    otherwise;
  - the new enums and structs that may grow are `#[non_exhaustive]`.
    **Either order with 2013:** if it is done, follow its rules for
    public types; if not, 2013 covers these with the rest.
- **`shuyi_platform::http`** (`crates/platform/src/http.rs`),
  re-exporting the types above:
  - `trait Http { fn send(&mut self, id: RequestId, request:
    &HttpRequest); fn cancel(&mut self, id: RequestId); fn poll(&mut
    self, out: &mut Vec<(RequestId, HttpReply)>); }`. `send` never
    blocks; a backend that can't start a request answers it with
    `Failed` at a later poll;
  - `Platform::http(&mut self) -> Option<&mut dyn Http>`, with a default
    body that returns `None`, so every backend that exists, a private
    console backend included, builds unchanged;
  - `App`'s `step`: after `text_input`, it sets `Ctx::http`'s presence
    and moves the replies from `poll` into it; after the update and the
    keyboard's ask, it sends what the screens queued and cancels what
    they dropped. While the game is suspended `run` returns before
    `step`, so nothing is polled or sent, and replies wait in the
    backend. A game that quits drops what is in flight;
  - `ScriptedHttp` in `headless.rs`: `ScriptedHttp::new()`,
    `.reply(url, HttpReply)` (per URL, in order, each used once),
    `.after_polls(n)` (1 by default), and `sent()`, every request in
    order. A URL with no scripted reply fails with `Network`.
    `HeadlessPlatform::with_http(ScriptedHttp)` and
    `HeadlessPlatform::scripted_http() -> Option<&ScriptedHttp>` (as
    `with_keyboard` and `keyboard` are). Without it `http()` is `None`,
    so the Harness, `shuyi run`, `shuyi replay` and the MCP tools never
    fetch;
  - `docs/platform-backends.md`: the two new steps in "What one frame
    does", a `### http(&mut self)` section, and a line in the checklist;
  - **the log (3040, done):** `App` logs each request at `Debug`,
    category `Platform`: the method, the host and the purpose, then the
    status, the bytes and the time. Never the query, a header or a
    body;
  - **either order with 3053:** if it is done, its platform facts say
    whether HTTP is present; if not, `HttpQueue::is_present` is enough,
    and 3053 needs nothing from here.
- **What changes for the game:** nothing at its next pin bump. The API
  is additive: `Platform::http` has a default, and `Ctx` gains a field
  but has private fields, so no game builds it with a struct literal.
- **Size:** if the diff, fixtures and snapshots aside, passes about 600
  changed lines when step 5 starts, the wiring in `App`, `ScriptedHttp`
  and their tests become one more parked follow-up ticket, blocked by
  this one, and the two backend tickets wait for it too.

**Out (do not do):**
- the native backend and the web backend: the two follow-up tickets
  below, written in step 2;
- caching, retries, ETags and resumable downloads;
- cookies, client certificates, proxies a game sets, and choosing
  HTTP/2;
- handing a screen a response while it arrives, and bodies past the
  limits above;
- remote content: fetched files as a bundle layer (2207) or a game's
  remote settings. That is a later ticket and Nick's call, since content
  that changes after release changes what a record replays;
- the consumers: sending crash reports (3047), high scores on a web
  service (a backend under 3054's `Leaderboards`), a mod-hosting
  service's client, and 1148's services. They use this;
- WebSockets and peer transports (1141, 1142), and WebRTC or
  WebTransport (1147);
- a web portal's JavaScript SDK: its own plugin, not HTTP;
- the Playdate: its SDK has networking since OS 2.7 (from memory; check
  it then). A follow-up after 3010.

### Follow-up 1: the native backend

Step 2 writes this as a parked ticket with the next free 26xx id, this
ticket's Parked note, the milestone "Engine: networking", and
`blocked_by: ["2601", "2015"]` (2015 makes `tools/layers.ron`). Its
scope:

- **Native, in `shuyi-app`** (`crates/app/src/http.rs` and
  `http/native.rs`, as `pads.rs` is split), behind a new cargo feature
  `http`, off by default:
  - `ureq` 3 (MIT OR Apache-2.0) with `default-features = false` and
    `rustls-no-provider`; `rustls` 0.23 (Apache-2.0 OR ISC OR MIT) with
    `default-features = false` and `ring`, `std` and `tls12`, its ring
    provider handed to ureq through `TlsConfig`'s
    `unversioned_rustls_crypto_provider`; `rustls-native-certs`
    (Apache-2.0 OR ISC OR MIT) for the roots, handed to ureq as
    `RootCerts::Specific`. `ring` 0.17 is Apache-2.0 AND ISC, and
    `rustls-webpki` and `untrusted` are ISC: `deny.toml` allows every one
    of these, and nothing is added to its list. All are native-only
    optional dependencies the feature turns on. Use the versions current
    when this starts, and list every new crate in the tree with its
    license in the PR;
  - each request runs on a worker thread through `Pending`, moved from
    `audio/native_music.rs` to `crates/app/src/worker.rs` since two
    modules use it now. `poll` takes what is done; `cancel` drops the
    `Pending`, and the thread's result with it;
  - ureq's config per request (`RequestBuilder::config`):
    `http_status_as_error(false)`; `timeout_global` from the request;
    `https_only(true)` for every URL that isn't loopback, so a redirect
    can't step down to `http://`; the body read with
    `with_config().limit(max)`. ureq's `Error::Timeout` becomes
    `Timeout`, `Error::BodyExceedsLimit` becomes `TooLarge`, and the
    rest `Network` with ureq's text;
  - the system's roots load once, at the first request. If none load,
    every request fails with `Network("no root certificates")`;
  - `Macroquad::http` (`crates/app/src/backend.rs`) returns the native
    backend with the feature on, and `None` without it or on wasm32.
- **The gates see the feature:**
  - `deny.toml`'s `[graph]` gains `features = ["shuyi-app/http"]`, or
    CI's `deny` job and the `run-gates` skill run `cargo deny` with that
    feature: either way `cargo deny check` checks the TLS crates. Shown
    once in the PR, not committed: with ureq's default features on,
    `cargo deny check` fails on `webpki-roots`' license;
  - CI's `clippy` job also runs `cargo clippy -p shuyi-app
    --all-targets --features http --locked -- -D warnings`; the `test`
    job runs `cargo test -p shuyi-app --features http --locked` on its
    three systems; the `run-gates` skill lists the same commands;
  - the `devices` list in `tools/layers.ron` (2015) gains `ureq`,
    `rustls`, `ring` and `rustls-native-certs`, so no crate marked
    `devices: false` (`shuyi-platform` among them) reaches a network
    crate; any outside crate added to a pure crate joins that crate's
    `outside` list.
- **Either order with 2003:** if it is done, the `shuyi` crate gains an
  `http` feature that turns on `shuyi-app/http`, and the catalog
  (`docs/modules.md`) lists it with the platform backends; if not, a
  game turns on `shuyi-app/http` itself, and 2003 adds the feature.
- **Notices:** nothing to add. 3039 (web builds) and 3004 (packages)
  list the crates a build compiles in from `cargo metadata` with the
  game's features, ring's and rustls's license files included.
- **What changes for the game:** nothing; the `http` feature is off.

Its steps: read `Pending`, `backend.rs`, `deny.toml`, the `clippy`,
`test` and `deny` jobs and `tools/layers.ron`; move `Pending`; write the
gates first and show the failing `webpki-roots` case; write the backend
and its tests; build it on the Windows GNU toolchain.

Its acceptance criteria:
- [ ] Tests in `http/native.rs` (the backend is crate-private) with the
      `http` feature, against a server on `127.0.0.1` that the test
      starts: a RON palette (a new fixture,
      `crates/app/tests/fixtures/palette.ron`, read with
      `include_str!`) fetched and decoded with
      `Formats::standard().decode::<PaletteDef>(&AssetPath::new("palette.ron"),
      &body, &DecodeCx::new())`; a 404 as a response; a body past its
      limit is `TooLarge`; a slow answer with a 200 ms timeout is
      `Timeout`; a cancelled request is never answered; `poll` returns in
      under 5 ms while a request is slow.
- [ ] `cargo deny check` passes with the `http` feature in its graph,
      and fails on `webpki-roots` with ureq's default features (shown in
      the PR).
- [ ] `cargo build -p shuyi-app --features http` passes on the Windows
      GNU toolchain (ring builds with MSYS2's gcc; no NASM, no cmake).
- [ ] By hand on native: a GET to a public `https://` URL succeeds with
      the system's roots. Recorded in the Completion notes.
- [ ] The `layers` and `no-std` jobs pass.
- [ ] All gates in the `run-gates` skill pass.

### Follow-up 2: the web backend

Step 2 writes this as a parked ticket with the id after follow-up 1's,
the same note and milestone, and `blocked_by` this ticket and follow-up
1 (which adds the `http` feature). Its scope:

- **`crates/cli/web/http.js`**, our own miniquad plugin `shuyi_http`
  with a `version`, in the style of `gamepad.js`:
  - `shuyi_http_send(id, method, url_ptr, url_len, headers_ptr,
    headers_len, body_ptr, body_len, timeout_ms, max_bytes)` copies the
    URL, the headers (one block of `name\nvalue\n` pairs) and the body
    out of `wasm_memory` before it returns, and starts a `fetch` with an
    `AbortController` that the timeout aborts. No cookies
    (`credentials: "omit"`), no cache (`cache: "no-store"`), redirects
    followed;
  - it reads the body with a stream reader and aborts once the body
    passes `max_bytes`;
  - finished requests wait in a queue: `shuyi_http_poll()` gives the
    next finished id, or 0; then `shuyi_http_status()`,
    `shuyi_http_error()` (0, or a code for `Timeout`, `TooLarge`,
    `Network` or `Busy`), `shuyi_http_len(part)` and
    `shuyi_http_read(part, ptr, len)` for its body, content type and
    error text. `shuyi_http_cancel(id)` aborts and forgets it;
  - one request past `MAX_REQUESTS` in flight fails at once with
    `Busy`.
- **`crates/app/src/http/web.rs`:** the `extern` block and
  `shuyi_http_crate_version`, under the comment rules of `pads/web.rs`:
  - the imports that take only numbers are `safe fn`, as the pads' are.
    `shuyi_http_send` and `shuyi_http_read` take pointers, so they are
    not: a safe function could be handed a dangling pointer. Each is
    called from one small safe function that takes a slice, in an
    `unsafe` block under `#[allow(unsafe_code)]`, whose `// SAFETY:`
    comment says why: the JS reads or writes exactly `len` bytes at
    `ptr` during the call, and Rust passes a live slice;
  - the version export compiles on wasm32 with or without the feature,
    so the loader never logs that the plugin is unused; the imports and
    the backend need the feature;
  - `Macroquad::http` returns the web backend on wasm32 with the
    feature on.
- **`cargo shuyi web`:** `SHELL` gains `http.js`; `index.html` loads it
  after `mq_js_bundle.js` and before `load(...)`. `shuyi-cli` tests,
  copied from the gamepad ones: the functions in `http.js` match those
  declared in `http/web.rs`'s `extern` block (read the block, since not
  all are `safe fn`, and leave out the version export), the versions
  match, `index.html` loads it in that order, and
  `package_copies_wasm_and_writes_the_shell` lists the new file.
- **`crates/cli/web/README.md`:** a section like `gamepad.js`'s, with
  CORS (the server must allow the page's origin, and the browser
  reports a refusal only as a network error) and mixed content (a page
  served over HTTPS can't call `http://`).
- **The gates:** CI's `clippy` job and the `run-gates` skill also run
  `cargo clippy -p shuyi-app --lib --features http --target
  wasm32-unknown-unknown --locked -- -D warnings` (`--lib`: the native
  tests don't build for wasm32).
- **`unsafe` named where it is:** `crates/app/Cargo.toml`'s comment on
  `unsafe_code` calls `src/pads/web.rs` "the only
  `#[allow(unsafe_code)]`". It names every plugin `extern` block
  instead: `pads/web.rs`, `http/web.rs`, and `lifecycle/web.rs` if 3043
  is done. Whichever of this ticket and 3043 lands second makes that
  comment and `lib.rs`'s docs name all three.
- **Either order with 1142:** if it is done, `http.js` follows
  `net.js`'s queue and its limit on queued bytes; if not, 1142 follows
  this.
- **Either order with 3039:** if it is done, `http.js` joins its table
  of the web shell's scripts as the engine's own, as `gamepad.js` is;
  if not, 3039 lists it with the rest.
- **Out:** moving the web backend to a crate if 3034 (parked) moves the
  web build to `wasm-bindgen` (3034's tickets decide); automating the
  browser check (a follow-up for 2019's checker, written in that PR if
  2019 is done).

Its steps: read `pads/web.rs`, `gamepad.js`, `crates/cli/src/web.rs`
and its tests, and check whether 1142, 3039 and 3043 are done; write
`http.js` and `http/web.rs`; wire `SHELL` and `index.html`; write the
`shuyi-cli` tests and the README section; run the browser check and the
gates, the web build included.

Its acceptance criteria:
- [ ] The `shuyi-cli` tests for `http.js` pass: functions, version,
      load order, package.
- [ ] In a browser (the browser pane, by hand): a throwaway build of
      hello-grid with the feature on, not committed, fetches a file from
      its own localhost server, gets a 404 as a response, gets
      `TooLarge` past the limit, and gets `Network` from a server that
      doesn't allow its origin. The console has no error. Recorded in
      the Completion notes.
- [ ] hello-grid's web build without the feature loads with no new
      console line from the loader.
- [ ] All gates in the `run-gates` skill pass.

## Implementation steps

1. Read ADR-0006, ADR-0024, `system_keyboard.rs`, `App`'s `run` and
   `step`, and `headless.rs`. Check whether 2013 and 3053 are
   done.
2. Unless `tickets/open` already holds them, write the two follow-up
   tickets with the `write-ticket` skill, from the two sections above:
   each with the context it needs from this ticket, its scope, steps,
   acceptance criteria, tests and a Completion notes placeholder. In
   this ticket, replace each section with one line naming its ticket.
3. Write the ADR and its row in `docs/adr/README.md`.
4. `shuyi_ui::http` and its unit and property tests; `Ctx::http`.
5. `shuyi_platform::http`, `Platform::http`, the wiring in `App`,
   `ScriptedHttp`, and the `App` and Harness tests.
6. `docs/platform-backends.md`, then the gates, the `no-std` build
   included.

## Acceptance criteria

- [ ] The ADR is in `docs/adr/` and listed in `docs/adr/README.md`.
- [ ] The two follow-up tickets are in `tickets/open/`, parked, with the
      `blocked_by` lists above, and the Completion notes name them.
- [ ] `shuyi_ui::http` tests: the URL rule (an `https://` URL, loopback
      `http://` with a port, `http://example.com` refused, a user and
      password refused, a 2049-byte URL refused); the header rule
      (`Cookie`, `Host` and `Sec-Fetch-Mode` refused, a CR or LF in a
      value refused); timeouts and response limits clamped; the ninth
      `send` is `Busy`, and taking a reply frees a slot; a reply to a
      cancelled id is dropped; HTTP going away fails what waits with
      `Unsupported`.
- [ ] `App` tests on `HeadlessPlatform` with `ScriptedHttp`: a request
      sent in frame N is answered in frame N + 1 at the earliest;
      nothing is sent or polled while suspended; without
      `ScriptedHttp`, `send` is `Unsupported`.
- [ ] `docs/platform-backends.md` has the two new steps, the
      `### http(&mut self)` section and the checklist line.
- [ ] `Cargo.lock` is unchanged: this ticket adds no crate.
- [ ] The `no-std` job passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the URL and header rules, the limits, the slots and the
  handshake (`shuyi_ui::http`); `ScriptedHttp`.
- Property: any URL `send` accepts starts with `https://`, or is
  `http://` to a loopback host, and has no `@` in its authority.
- Integration: `App` and the Harness with `ScriptedHttp`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
