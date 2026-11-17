use super::*;

use std::io::Cursor;
use std::time::UNIX_EPOCH;

/// A fixture: a built site and a project of `steps`, each built, in a
/// temporary folder named after `name`.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str, steps: &[&str]) -> Self {
        let root =
            std::env::temp_dir().join(format!("xtask-book-pad-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let fixture = Self { root };
        fixture.write("site/index.html", "the front page");
        fixture.write("site/404.html", "not here");
        fixture.write("site/style.css", "body {}");
        for step in steps {
            fixture.write(
                &format!("site{PROJECT}{step}/index.html"),
                &format!("{step}'s page"),
            );
            fixture.write(
                &format!("site{PROJECT}{step}/play/index.html"),
                "CI's build",
            );
            fixture.write(&format!("tutorial/{step}/Cargo.toml"), "[workspace]");
            fixture.write(
                &format!("tutorial/{step}/crates/game/src/main.rs"),
                "fn main() {}",
            );
            fixture.write(&format!("tutorial/{step}/{GAME_FILES}/levels/1.map"), "#.#");
            fixture.write(
                &format!("tutorial/{step}/{DIST}/index.html"),
                &format!("{step}, built here"),
            );
            fixture.write(&format!("tutorial/{step}/{DIST}/game.wasm"), "\0asm");
        }
        fixture
    }

    fn path(&self, file: &str) -> PathBuf {
        self.root.join(file)
    }

    fn write(&self, file: &str, text: &str) {
        let path = self.path(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// Sets `file`'s modification time `seconds` after the epoch.
    fn touch(&self, file: &str, seconds: u64) {
        fs::File::options()
            .write(true)
            .open(self.path(file))
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_secs(seconds))
            .unwrap();
    }

    fn server(&self, steps: &[&str]) -> Server {
        let steps: Vec<String> = steps.iter().map(|s| (*s).to_owned()).collect();
        let tutorial = self.path("tutorial");
        Server {
            site: self.path("site"),
            pad: Arc::new(Mutex::new(Pad::new(&tutorial, &steps))),
            tutorial,
            address: Address::from_env(|_| None),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn get(path: &str) -> Request {
    Request {
        method: "GET".to_owned(),
        path: path.to_owned(),
        host: Some("127.0.0.1:4000".to_owned()),
    }
}

fn body(response: &Response) -> String {
    String::from_utf8(response.body.clone()).unwrap()
}

/// Records the steps built, and does `during` as each builds.
struct Fake<'a> {
    built: Vec<String>,
    fail: bool,
    during: Box<dyn FnMut(&str) + 'a>,
}

impl<'a> Fake<'a> {
    fn new() -> Self {
        Self::with(|_| {})
    }

    fn with(during: impl FnMut(&str) + 'a) -> Self {
        Self {
            built: Vec::new(),
            fail: false,
            during: Box::new(during),
        }
    }
}

impl Builder for Fake<'_> {
    fn build(&mut self, step: &str) -> Result<(), String> {
        self.built.push(step.to_owned());
        (self.during)(step);
        if self.fail {
            Err(format!("error[E0425]: in {step}"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn the_options_are_out_and_no_build() {
    let args = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert_eq!(Options::parse(&args(&[])), Some(Options::default()));
    assert_eq!(
        Options::parse(&args(&["--out", "/tmp/site", "--no-build"])),
        Some(Options {
            out: Some(PathBuf::from("/tmp/site")),
            no_build: true
        })
    );
    assert_eq!(Options::parse(&args(&["--out"])), None);
    assert_eq!(Options::parse(&args(&["--port", "1"])), None);
}

#[test]
fn the_address_is_the_codespaces_forwarded_port_or_the_loopback() {
    let codespace = Address::from_env(|name| match name {
        "CODESPACE_NAME" => Some("ideal-robot".to_owned()),
        "GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN" => Some("app.github.dev".to_owned()),
        _ => None,
    });
    assert_eq!(codespace.base, "https://ideal-robot-4000.app.github.dev/");
    assert_eq!(
        codespace.hosts,
        [
            "127.0.0.1:4000",
            "localhost:4000",
            "ideal-robot-4000.app.github.dev"
        ]
    );
    let here = Address::from_env(|_| None);
    assert_eq!(here.base, "http://127.0.0.1:4000/");
    assert_eq!(here.hosts, ["127.0.0.1:4000", "localhost:4000"]);
}

#[test]
fn a_steps_play_is_its_own_build_not_cis() {
    let fixture = Fixture::new("play", &["02-board"]);
    let server = fixture.server(&["02-board"]);
    let page = server.answer(&get("/learn/book/box-pusher/02-board/play/"));
    assert_eq!(
        (page.status, body(&page).as_str()),
        (200, "02-board, built here")
    );
    assert_eq!(
        page.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
    let wasm = server.answer(&get("/learn/book/box-pusher/02-board/play/game.wasm"));
    assert_eq!(wasm.status, 200);
    assert_eq!(wasm.header("Content-Type"), Some("application/wasm"));
    let files = server.answer(&get("/learn/book/box-pusher/02-board/files/levels/1.map"));
    assert_eq!((files.status, body(&files).as_str()), (200, "#.#"));
    let missing = server.answer(&get("/learn/book/box-pusher/02-board/play/gone.js"));
    assert_eq!((missing.status, body(&missing).as_str()), (404, "not here"));
}

#[test]
fn the_sites_files_are_served_uncached() {
    let fixture = Fixture::new("site", &["02-board"]);
    let server = fixture.server(&["02-board"]);
    let css = server.answer(&get("/style.css"));
    assert_eq!((css.status, body(&css).as_str()), (200, "body {}"));
    assert_eq!(css.header("Content-Type"), Some("text/css; charset=utf-8"));
    let sent = String::from_utf8_lossy(&css.bytes()).into_owned();
    assert!(sent.starts_with("HTTP/1.1 200 OK\r\n"), "{sent}");
    assert!(sent.contains("Cache-Control: no-store\r\n"), "{sent}");
    assert!(sent.ends_with("\r\n\r\nbody {}"), "{sent}");
    let page = server.answer(&get("/learn/book/box-pusher/02-board/"));
    assert_eq!(body(&page), "02-board's page");
    // A folder named without its last `/`.
    let folder = server.answer(&get("/learn/book/box-pusher/02-board"));
    assert_eq!(folder.status, 301);
    assert_eq!(
        folder.header("Location"),
        Some("/learn/book/box-pusher/02-board/")
    );
    // An unknown step's `play/` is the site's.
    let unknown = server.answer(&get("/learn/book/box-pusher/99-none/play/"));
    assert_eq!(unknown.status, 404);
}

#[test]
fn the_root_goes_to_the_last_step_page_served() {
    let fixture = Fixture::new("root", &["02-board", "03-walk"]);
    let server = fixture.server(&["02-board", "03-walk"]);
    let first = server.answer(&get("/"));
    assert_eq!(first.status, 302);
    assert_eq!(
        first.header("Location"),
        Some("/learn/book/box-pusher/02-board/")
    );
    server.answer(&get("/learn/book/box-pusher/03-walk/"));
    // Its game and other files don't count.
    server.answer(&get("/learn/book/box-pusher/02-board/play/"));
    let then = server.answer(&get("/"));
    assert_eq!(
        then.header("Location"),
        Some("/learn/book/box-pusher/03-walk/")
    );
}

#[test]
fn a_path_that_leaves_its_folder_is_refused() {
    let fixture = Fixture::new("dots", &["02-board"]);
    fixture.write("secret.txt", "no");
    let server = fixture.server(&["02-board"]);
    for path in [
        "/../secret.txt",
        "/learn/book/box-pusher/02-board/play/../../../../../secret.txt",
        "/learn/book/box-pusher/02-board/play/%2e%2e/%2E%2E/x",
        "/a%2f..%2fb",
        "/a%5c..",
        "/C:/x",
        "/%zz",
        "/%ff",
        "relative",
    ] {
        assert_eq!(server.answer(&get(path)).status, 400, "{path}");
    }
}

#[test]
fn a_wrong_host_or_a_method_but_get_is_refused() {
    let fixture = Fixture::new("host", &["02-board"]);
    let server = fixture.server(&["02-board"]);
    for host in [
        None,
        Some("evil.example:4000"),
        Some("127.0.0.1"),
        Some("127.0.0.1:4001"),
    ] {
        let request = Request {
            host: host.map(str::to_owned),
            ..get("/pad/status")
        };
        assert_eq!(server.answer(&request).status, 403, "{host:?}");
    }
    let localhost = Request {
        host: Some("LOCALHOST:4000".to_owned()),
        ..get("/pad/status")
    };
    assert_eq!(server.answer(&localhost).status, 200);
    let post = Request {
        method: "POST".to_owned(),
        ..get("/pad/status")
    };
    let refused = server.answer(&post);
    assert_eq!(refused.status, 405);
    assert_eq!(refused.header("Allow"), Some("GET"));
}

#[test]
fn the_pad_answers_its_status_and_each_steps_build() {
    let fixture = Fixture::new("routes", &["02-board"]);
    fs::remove_file(fixture.path("tutorial/02-board/dist/web/index.html")).unwrap();
    let server = fixture.server(&["02-board"]);
    let status = server.answer(&get("/pad/status"));
    assert_eq!(
        (status.status, body(&status).as_str()),
        (200, r#"{"pad":true}"#)
    );
    assert_eq!(status.header("Content-Type"), Some("application/json"));
    let build = server.answer(&get("/pad/build/02-board"));
    let value: serde_json::Value = serde_json::from_slice(&build.body).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"step": "02-board", "number": 0, "good": 0, "state": "none", "output": ""})
    );
    assert_eq!(server.answer(&get("/pad/build/03-walk")).status, 404);
    assert_eq!(server.answer(&get("/pad/other")).status, 404);
}

#[test]
fn a_request_is_read_with_its_host_and_without_its_query() {
    let text = "GET /learn/?x=1 HTTP/1.1\r\nUser-Agent: a\r\nhost:  127.0.0.1:4000 \r\n\r\nrest";
    assert_eq!(
        read_request(&mut Cursor::new(text)),
        Some(Request {
            method: "GET".to_owned(),
            path: "/learn/".to_owned(),
            host: Some("127.0.0.1:4000".to_owned()),
        })
    );
    // Not a request: no version, no end of the headers, or too long.
    assert_eq!(read_request(&mut Cursor::new("GET /\r\n\r\n")), None);
    assert_eq!(
        read_request(&mut Cursor::new("GET / HTTP/1.1\r\nHost: a\r\n")),
        None
    );
    let long = format!("GET / HTTP/1.1\r\nX: {}\r\n\r\n", "a".repeat(MAX_HEAD));
    assert_eq!(read_request(&mut Cursor::new(long)), None);
}

#[test]
fn a_change_under_one_steps_crates_builds_that_step_only() {
    let steps = ["02-board", "03-walk"];
    let fixture = Fixture::new("watch", &steps);
    let server = fixture.server(&steps);
    let names: Vec<String> = steps.iter().map(|s| (*s).to_owned()).collect();
    let mut watcher = Watcher::new(&server.tutorial, &names);
    let mut fake = Fake::new();
    watcher.tick(&mut fake, &server.pad);
    assert!(fake.built.is_empty(), "{:?}", fake.built);

    fixture.touch("tutorial/03-walk/crates/game/src/main.rs", 1_000);
    watcher.tick(&mut fake, &server.pad);
    assert_eq!(fake.built, ["03-walk"]);
    let pad = lock(&server.pad);
    assert_eq!(pad.steps["03-walk"].number, 1);
    assert_eq!(pad.steps["03-walk"].good, 1);
    assert_eq!(pad.steps["03-walk"].state, State::Ok);
    assert_eq!(pad.steps["02-board"].number, 0);
    drop(pad);

    // A new file, its manifest, and a file outside crates/.
    fixture.write("tutorial/02-board/crates/game/src/new.rs", "");
    fixture.touch("tutorial/03-walk/Cargo.toml", 1_000);
    fixture.write("tutorial/03-walk/notes.txt", "");
    watcher.tick(&mut fake, &server.pad);
    watcher.tick(&mut fake, &server.pad);
    watcher.tick(&mut fake, &server.pad);
    assert_eq!(fake.built, ["03-walk", "02-board", "03-walk"]);
}

#[test]
fn a_change_during_a_build_builds_it_once_more() {
    let fixture = Fixture::new("during", &["03-walk"]);
    let server = fixture.server(&["03-walk"]);
    let mut watcher = Watcher::new(&server.tutorial, &["03-walk".to_owned()]);
    let mut saves = 0;
    let mut fake = Fake::with(|_| {
        // The reader saves twice while the first build runs.
        if saves == 0 {
            fixture.touch("tutorial/03-walk/crates/game/src/main.rs", 2_000);
            fixture.write("tutorial/03-walk/crates/game/src/more.rs", "");
        }
        saves += 1;
    });
    fixture.touch("tutorial/03-walk/crates/game/src/main.rs", 1_000);
    watcher.tick(&mut fake, &server.pad);
    assert_eq!(watcher.queued(), Vec::<&str>::new());
    watcher.tick(&mut fake, &server.pad);
    watcher.tick(&mut fake, &server.pad);
    assert_eq!(fake.built, ["03-walk", "03-walk"]);
    assert_eq!(lock(&server.pad).steps["03-walk"].number, 2);
}

#[test]
fn changes_queue_each_step_once_in_order() {
    let steps = ["02-board", "03-walk", "04-push"];
    let fixture = Fixture::new("queue", &steps);
    let server = fixture.server(&steps);
    let names: Vec<String> = steps.iter().map(|s| (*s).to_owned()).collect();
    let mut watcher = Watcher::new(&server.tutorial, &names);
    for step in ["02-board", "04-push"] {
        fixture.touch(&format!("tutorial/{step}/crates/game/src/main.rs"), 1_000);
    }
    let mut fake = Fake::new();
    watcher.tick(&mut fake, &server.pad);
    assert_eq!(fake.built, ["02-board"]);
    assert_eq!(watcher.queued(), ["04-push"]);
    // Changed again while it waits: still once.
    fixture.touch("tutorial/04-push/crates/game/src/main.rs", 2_000);
    watcher.tick(&mut fake, &server.pad);
    watcher.tick(&mut fake, &server.pad);
    assert_eq!(fake.built, ["02-board", "04-push"]);
}

#[test]
fn a_failed_build_keeps_the_last_good_one_and_says_why() {
    let fixture = Fixture::new("failed", &["03-walk"]);
    let server = fixture.server(&["03-walk"]);
    let mut watcher = Watcher::new(&server.tutorial, &["03-walk".to_owned()]);
    let mut fake = Fake::with(|_| {});
    fake.fail = true;
    fixture.touch("tutorial/03-walk/crates/game/src/main.rs", 1_000);
    watcher.tick(&mut fake, &server.pad);
    let build = server.answer(&get("/pad/build/03-walk"));
    let value: serde_json::Value = serde_json::from_slice(&build.body).unwrap();
    assert_eq!(value["state"], "failed");
    assert_eq!(value["number"], 1);
    assert_eq!(value["good"], 0);
    assert_eq!(value["output"], "error[E0425]: in 03-walk");
    // The last good build is still played.
    let play = server.answer(&get("/learn/book/box-pusher/03-walk/play/"));
    assert_eq!(body(&play), "03-walk, built here");

    fake.fail = false;
    fixture.touch("tutorial/03-walk/crates/game/src/main.rs", 2_000);
    watcher.tick(&mut fake, &server.pad);
    let build = lock(&server.pad).steps["03-walk"].clone();
    assert_eq!(
        build,
        Build {
            number: 2,
            good: 2,
            state: State::Ok,
            output: String::new()
        }
    );
}

#[test]
fn a_build_made_another_way_is_a_good_build() {
    let fixture = Fixture::new("elsewhere", &["05-undo"]);
    fs::remove_file(fixture.path("tutorial/05-undo/dist/web/index.html")).unwrap();
    let server = fixture.server(&["05-undo"]);
    let mut watcher = Watcher::new(&server.tutorial, &["05-undo".to_owned()]);
    assert_eq!(lock(&server.pad).steps["05-undo"].state, State::None);
    // The container's build.sh, or the reader's own `cargo shuyi web`.
    fixture.write("tutorial/05-undo/dist/web/index.html", "built by hand");
    let mut fake = Fake::new();
    watcher.tick(&mut fake, &server.pad);
    assert_eq!(fake.built, Vec::<String>::new());
    let build = lock(&server.pad).steps["05-undo"].clone();
    assert_eq!((build.state, build.number, build.good), (State::Ok, 1, 1));

    // Its own build isn't counted twice.
    let mut rebuild = Fake::with(|_| {
        fixture.write("tutorial/05-undo/dist/web/index.html", "built by the pad");
        fixture.touch("tutorial/05-undo/dist/web/index.html", 5_000);
    });
    fixture.touch("tutorial/05-undo/crates/game/src/main.rs", 1_000);
    watcher.tick(&mut rebuild, &server.pad);
    watcher.tick(&mut rebuild, &server.pad);
    assert_eq!(rebuild.built, ["05-undo"]);
    let build = lock(&server.pad).steps["05-undo"].clone();
    assert_eq!((build.number, build.good), (2, 2));
}

#[test]
fn the_server_answers_over_tcp() {
    let fixture = Fixture::new("tcp", &["02-board"]);
    let server = fixture.server(&["02-board"]);
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        server.serve(stream);
    });
    let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
    client
        .write_all(b"GET /pad/status HTTP/1.1\r\nHost: 127.0.0.1:4000\r\n\r\n")
        .unwrap();
    let mut answer = String::new();
    client.read_to_string(&mut answer).unwrap();
    handle.join().unwrap();
    assert!(answer.starts_with("HTTP/1.1 200 OK\r\n"), "{answer}");
    assert!(answer.ends_with(r#"{"pad":true}"#), "{answer}");
}
