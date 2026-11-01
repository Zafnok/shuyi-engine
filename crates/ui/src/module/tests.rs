//! Installing modules on the toy game: what each declares, and the two
//! mistakes.

use super::*;
use crate::screen::Transition;
use crate::toy::{Toy, ctx, game};

/// How many frames a module counted.
#[derive(Debug, Default, PartialEq)]
struct Frames(u32);

/// A module that declares a hook and a state.
struct Counter;

impl Module<Toy> for Counter {
    fn name(&self) -> &'static str {
        "counter"
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        install.hook(Hook::BeforeUpdate);
        install.hook(Hook::BeforeUpdate);
        install.state(Frames::default());
    }
}

/// A module that puts a [`Frames`] on the context too.
struct Other;

impl Module<Toy> for Other {
    fn name(&self) -> &'static str {
        "other"
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        install.state(7_u8);
        install.state(Frames(1));
    }
}

/// A module with two tools on the debug menu, the second named from the
/// context.
struct Tools;

impl Module<Toy> for Tools {
    fn name(&self) -> &'static str {
        "tools"
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        install.debug_entry(|| DebugEntry::new("Reload", |_| Transition::None));
        install.debug_entry(|| {
            DebugEntry::named(
                |ctx| format!("Tools {}", ctx.debug_tools),
                |_| Transition::None,
            )
        });
    }
}

/// A module that declares nothing.
struct Quiet;

impl Module<Toy> for Quiet {
    fn name(&self) -> &'static str {
        "quiet"
    }
}

#[test]
fn install_records_the_hooks_and_states_a_module_declares() {
    let game = game()
        .with_module(Counter)
        .unwrap()
        .with_module(Quiet)
        .unwrap();
    let mut counter = InstalledModule::new("counter");
    counter.hooks = vec![Hook::BeforeUpdate];
    counter.states = vec![type_name::<Frames>()];
    assert_eq!(game.modules(), [counter, InstalledModule::new("quiet")]);
    assert_eq!(game.ctx().module_state::<Frames>(), Some(&Frames(0)));
    let shown = format!("{:?}", game.ctx());
    assert!(shown.contains(type_name::<Frames>()), "{shown}");
}

#[test]
fn debug_entries_are_recorded_by_label_in_the_order_declared() {
    let game = game()
        .with_module(Quiet)
        .unwrap()
        .with_module(Tools)
        .unwrap();
    let mut tools = InstalledModule::new("tools");
    tools.debug_entries = vec!["Reload".into(), "Tools true".into()];
    assert_eq!(game.modules(), [InstalledModule::new("quiet"), tools]);
    let made: Vec<String> = game
        .ctx()
        .modules
        .debug_entries()
        .map(|e| e.label(game.ctx()))
        .collect();
    assert_eq!(made, ["Reload", "Tools true"]);
}

#[test]
fn a_name_installed_twice_is_refused() {
    let error = game()
        .with_module(Counter)
        .and_then(|g| g.with_module(Counter))
        .err();
    assert_eq!(error, Some(ModuleError::SameName { name: "counter" }));
    assert_eq!(
        error.unwrap().to_string(),
        "the module counter is installed twice"
    );
}

#[test]
fn a_state_two_modules_put_on_the_context_is_refused_naming_both() {
    let modules: [Box<dyn Module<Toy>>; 2] = [Box::new(Counter), Box::new(Other)];
    let error = game().with_modules(modules).err().unwrap();
    let state = type_name::<Frames>();
    assert_eq!(
        error,
        ModuleError::SameState {
            state,
            module: "other",
            other: "counter",
        }
    );
    assert_eq!(
        error.to_string(),
        format!("the modules counter and other both put a {state} on the context")
    );
}

#[test]
fn a_hook_is_named_after_its_method() {
    assert_eq!(Hook::BeforeUpdate.name(), "before_update");
    assert_eq!(Hook::DrawOver.name(), "draw_over");
    assert_eq!(Hook::Reloaded.name(), "reloaded");
}

#[test]
fn a_modules_name_is_snake_case_ascii() {
    for name in ["counter", "net_2", "a"] {
        assert!(is_snake_case(name), "{name}");
    }
    for name in ["", "Counter", "2d", "_x", "net-2", "café"] {
        assert!(!is_snake_case(name), "{name}");
    }
    let mut c = ctx();
    let install = Install::new(&mut c, "counter");
    assert!(format!("{install:?}").starts_with("Install"));
}
