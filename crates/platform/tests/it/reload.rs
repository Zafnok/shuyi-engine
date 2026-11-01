//! A changed file reaching a game's modules through the frame: on the next
//! frame, once, before the update, and only with debug tools on.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use shuyi_assets::bundle::{Bundle, Layer};
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::module::{Hook, Install, Module};
use shuyi_ui::toy::{self, Toy, ToyCtx};

/// What ran, in order.
type Log = Rc<RefCell<Vec<String>>>;

/// A module that notes its hooks' calls.
struct Noter(Log);

impl Module<Toy> for Noter {
    fn name(&self) -> &'static str {
        "noter"
    }

    fn install(&mut self, install: &mut Install<'_, Toy>) {
        install.hook(Hook::Reloaded);
        install.hook(Hook::BeforeUpdate);
    }

    fn before_update(&mut self, _: &mut ToyCtx, _: f32) {
        self.0.borrow_mut().push("update".to_owned());
    }

    fn reloaded(&mut self, _: &mut ToyCtx, changed: &[String]) {
        self.0.borrow_mut().push(changed.join(","));
    }
}

/// An empty layer, as a folder on disk gives one.
fn disk() -> Layer {
    Layer::owned("disk".to_owned(), BTreeMap::new()).unwrap_or_else(|e| panic!("{e:?}"))
}

#[test]
fn a_changed_file_is_told_on_the_next_frame_once_before_the_update() {
    let log = Log::default();
    let game = toy::game().with_module(Noter(Rc::clone(&log))).unwrap();
    let mut h = Harness::new(game);
    let layer = disk();
    h.change_file(layer, "a.ron", b"new".to_vec());
    assert!(log.borrow().is_empty(), "not before the frame");
    h.wait(FRAME_DT);
    assert_eq!(*log.borrow(), ["a.ron", "update"]);
    assert_eq!(Bundle::new(vec![layer]).file("a.ron"), Some("new"));
    h.wait(FRAME_DT);
    assert_eq!(*log.borrow(), ["a.ron", "update", "update"]);
    assert_eq!(h.platform().changed_asks(), 2);
}

#[test]
fn with_debug_tools_off_the_platform_is_never_asked_and_no_file_changes() {
    let log = Log::default();
    let game = toy::game()
        .with_debug_screens(false)
        .with_module(Noter(Rc::clone(&log)))
        .unwrap();
    let mut h = Harness::new(game);
    let layer = disk();
    h.change_file(layer, "a.ron", b"new".to_vec());
    for _ in 0..3 {
        h.wait(FRAME_DT);
    }
    assert_eq!(h.platform().changed_asks(), 0);
    assert_eq!(*log.borrow(), ["update", "update", "update"]);
    assert_eq!(Bundle::new(vec![layer]).file("a.ron"), None);
}

#[test]
fn a_reload_error_reaches_the_frame_until_it_clears() {
    use shuyi_assets::ContentError;
    use shuyi_assets::hot::Reload;

    /// A module that fails, then mends, as its file changes.
    #[derive(Default)]
    struct Flaky(bool);

    impl Module<Toy> for Flaky {
        fn name(&self) -> &'static str {
            "flaky"
        }

        fn install(&mut self, install: &mut Install<'_, Toy>) {
            install.hook(Hook::Reloaded);
        }

        fn reloaded(&mut self, ctx: &mut ToyCtx, _: &[String]) {
            self.0 = !self.0;
            let reload = if self.0 {
                Reload::Failed(vec![ContentError::new("a.ron", "broken").at(3, None)])
            } else {
                Reload::Reloaded
            };
            ctx.note_reload("a.ron", &reload);
        }
    }

    let mut h = Harness::new(toy::game().with_module(Flaky::default()).unwrap());
    let layer = disk();
    h.change_file(layer, "a.ron", b"x".to_vec());
    h.wait(FRAME_DT);
    assert_eq!(h.platform().reload_errors(), ["a.ron:3: broken"]);
    h.wait(FRAME_DT);
    assert_eq!(h.platform().reload_errors(), ["a.ron:3: broken"]);
    h.change_file(layer, "a.ron", b"y".to_vec());
    h.wait(FRAME_DT);
    assert_eq!(h.platform().reload_errors(), [] as [String; 0]);
}
