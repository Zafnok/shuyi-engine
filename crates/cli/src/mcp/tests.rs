//! The tools the server lists, and base64.

use super::*;

#[test]
fn every_operation_and_session_tool_is_a_tool_once() {
    let listed = tools();
    let names: Vec<&str> = listed.iter().filter_map(|t| t["name"].as_str()).collect();
    for op in Op::ALL.into_iter().chain(Op::SESSION) {
        assert_eq!(
            names.iter().filter(|name| **name == op.name()).count(),
            1,
            "{}",
            op.name()
        );
    }
    assert_eq!(names.len(), Op::ALL.len() - 1 + Op::SESSION.len());
    for tool in &listed {
        let name = tool["name"].as_str().unwrap();
        assert!(!tool["description"].as_str().unwrap().is_empty(), "{name}");
        assert_eq!(tool["inputSchema"]["type"], "object", "{name}");
        assert_eq!(tool["outputSchema"]["type"], "object", "{name}");
        let (op, session) = super::tool(name).unwrap();
        assert_eq!(op.name(), name);
        assert_eq!(
            tool["inputSchema"]["properties"]["project"].is_object(),
            takes_project(op, session) || op == Op::Start,
            "{name}"
        );
    }
    assert_eq!(super::tool("scene"), Some((Op::Scene, true)));
    assert_eq!(super::tool("bots"), Some((Op::Bots, false)));
    assert_eq!(super::tool("fly"), None);
}

#[test]
fn each_tool_says_what_it_does_in_its_own_words() {
    let listed = tools();
    let mut descriptions: Vec<&str> = listed
        .iter()
        .filter_map(|tool| tool["description"].as_str())
        .collect();
    assert!(descriptions.iter().all(|d| d.ends_with('.')));
    descriptions.sort_unstable();
    descriptions.dedup();
    assert_eq!(descriptions.len(), listed.len());
    assert_ne!(description(Op::Scene, false), description(Op::Scene, true));
}

#[test]
fn the_tools_that_run_the_game_or_read_its_files_take_a_project() {
    let takes: Vec<&str> = Op::ALL
        .into_iter()
        .filter(|op| takes_project(*op, false))
        .map(Op::name)
        .collect();
    assert_eq!(
        takes,
        [
            "check",
            "lint",
            "modules",
            "add",
            "run",
            "scene",
            "replay",
            "bots",
            "playtest",
            "autobalance",
            "import",
            "bake"
        ]
    );
    assert!(!takes_project(Op::Scene, true));
}

#[test]
fn base64_is_the_standard_alphabet_with_padding() {
    for (bytes, text) in [
        (&b""[..], ""),
        (b"a", "YQ=="),
        (b"ab", "YWI="),
        (b"abc", "YWJj"),
        (b"abcdef", "YWJjZGVm"),
        (&[0xfb, 0xff, 0xbf][..], "+/+/"),
    ] {
        assert_eq!(base64(bytes), text);
    }
}

#[test]
fn the_server_answers_each_line_with_a_line() {
    let game = crate::contract::fake::Fake::default();
    let mut server = server::Server::new(".".into(), Box::new(host::InProcess::new(&game)));
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n\
                 {\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n\
                 {\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n";
    let mut output = Vec::new();
    serve(&mut server, input.as_bytes(), &mut output).unwrap();
    let text = String::from_utf8(output).unwrap();
    let ids: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap()["id"].clone())
        .collect();
    assert_eq!(ids, [json!(1), json!(2)]);
    assert!(text.ends_with('\n'));
}

#[test]
fn mcp_install_installs() {
    let dir = std::env::temp_dir().join(format!("shuyi-mcp-main-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let args = ["install".to_owned(), "--dry-run".to_owned()];
    assert_eq!(main(&args, &dir), 0);
    assert!(!dir.join(".mcp.json").exists(), "a dry run writes nothing");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mcp_with_a_word_it_doesnt_know_prints_its_usage() {
    assert_eq!(main(&["serve".to_owned()], Path::new(".")), 2);
    assert!(usage("shuyi").contains("mcp install [--dry-run]"));
}
