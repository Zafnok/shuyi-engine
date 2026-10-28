//! Chapter 10, tools for agents: the tool contract's operations, which
//! `cargo shuyi <op>` and the MCP server run, called as library functions
//! on the game's tools, as a game's own tests call them.

#[cfg(test)]
mod tests {
    use shuyi_cli::BasicTools;
    use shuyi_cli::contract::check::{CheckInput, check};
    use shuyi_cli::contract::run::{RunInput, keys, run};
    use shuyi_cli::contract::session::Event;

    use crate::game;

    // ANCHOR: contract
    #[test]
    fn the_contract_checks_the_files_and_plays_a_script() {
        let tools = BasicTools::new(game());
        let answer = check(&tools, &CheckInput::default());
        assert!(answer.ok, "{answer:?}");

        let mut input = RunInput::default();
        input.script = keys("Enter Right Enter Backspace Enter Backspace");
        let answer = run(&tools, &input);
        let out = answer.result.as_ref().unwrap();
        assert_eq!(out.screens, ["title"]);
        assert!(out.events.iter().any(|e| matches!(e, Event::Quit { .. })));
    }
    // ANCHOR_END: contract
}
