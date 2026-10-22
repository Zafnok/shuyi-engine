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
fn the_tools_that_run_the_game_take_a_project() {
    let takes: Vec<&str> = Op::ALL
        .into_iter()
        .filter(|op| takes_project(*op, false))
        .map(Op::name)
        .collect();
    assert_eq!(
        takes,
        [
            "check", "run", "scene", "replay", "bots", "playtest", "import", "bake"
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
fn mcp_with_a_word_it_doesnt_know_prints_its_usage() {
    assert_eq!(main(&["serve".to_owned()], Path::new(".")), 2);
    assert!(usage("shuyi").contains("mcp install [--dry-run]"));
}
