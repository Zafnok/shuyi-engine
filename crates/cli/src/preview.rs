//! The hosted preview: the game played by visitors as pictures from this
//! machine, never as a program they download (ADR-0141). A visitor presses
//! the game's actions and gets the frame after each one back, as a PNG with
//! the text it shows; the program, its files, its scene and its state stay
//! here. What a visitor sees they can still record and copy.
//!
//! Each visitor's game runs in a **worker** of its own ([`worker`]): the
//! game's tool program run as `preview-worker`, one process a session. It
//! starts the game's session as its arguments say, the same for every
//! visitor, takes the actions a player can rebind with the developer's
//! debug tools off
//! ([`Session::for_visitor`](crate::contract::session::Session::for_visitor)),
//! and answers each action, a JSON line, with the frame after it, and
//! nothing else: no script, no frame count, no scene, snapshot or log.

pub(crate) mod worker;
