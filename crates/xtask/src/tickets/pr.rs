//! The ticket a pull request is for (ticket 2051, ADR-0091): read from its
//! title first, then its branch.
//!
//! A title says what kind of PR it is:
//!
//! - `[NNNN] …`: ticket NNNN's work, which moves it to `tickets/done/`;
//! - `Ticket NNNN: …` or `Tickets NNNN, MMMM and KKKK: …` (a range
//!   `NNNN-MMMM` too): a PR that only writes or edits open tickets;
//! - `Game NNNN: …`: a game ticket's small change, which needs no ticket here
//!   (the `write-ticket` skill) and adds nothing to the public API;
//! - `sonar(…)`, `[advisory #N] …`, `engine-pin…`, or a `dependabot/` branch:
//!   the bots' and the Sonar batches' PRs, which keep their own rules.
//!
//! A `t<NNNN>-<slug>` branch must name the title's ticket. Any other branch
//! (a desktop worktree's `claude/<slug>-<hex>`) passes when the title names
//! the ticket. Without a title (a local run) the branch is all there is.

use super::{Folder, Ticket, is_four_digits, ticket_id_from_branch};

/// The paths a ticket-only PR may change.
const TICKET_ONLY_PATHS: &[&str] = &["tickets/", "docs/ROADMAP.md", "docs/decisions.md"];

/// A pull request as the checks read it: its head branch and, in CI, its
/// title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pr<'a> {
    pub branch: &'a str,
    pub title: Option<&'a str>,
}

impl Pr<'_> {
    /// What kind of PR this is, or why its title and branch don't agree.
    pub fn kind(&self) -> Result<PrKind, String> {
        pr_kind(self.title, self.branch)
    }
}

/// What a pull request is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrKind {
    /// `[NNNN] …`: this repository's ticket NNNN's work.
    Work(String),
    /// `Ticket NNNN: …`: writes or edits these open tickets, and nothing
    /// else.
    TicketsOnly(Vec<String>),
    /// `Game NNNN: …`: a game ticket's small change.
    GameSmall(String),
    /// Dependabot, an advisory or the game's pin watcher.
    Bot,
    /// A batch of Sonar findings (ADR-0017).
    Sonar,
    /// Nothing names a ticket. Only a local run with no title gets this: in
    /// CI it is an error.
    None,
}

/// The kind of PR `title` and `branch` make, or why they don't agree.
pub fn pr_kind(title: Option<&str>, branch: &str) -> Result<PrKind, String> {
    if branch.starts_with("dependabot/") {
        return Ok(PrKind::Bot);
    }
    let from_branch = ticket_id_from_branch(branch);
    let Some(title) = title else {
        return Ok(match from_branch {
            Some(id) => PrKind::Work(id.to_string()),
            None if branch.starts_with("sonar/") => PrKind::Sonar,
            None => PrKind::None,
        });
    };
    match (title_kind(title), from_branch) {
        (Some(kind), None) => Ok(kind),
        (Some(kind), Some(id)) if names(&kind, id) => Ok(kind),
        (Some(_), Some(id)) => Err(format!(
            "branch `{branch}` names ticket {id}, but the title `{title}` doesn't. Retitle \
             the PR, or name the branch after the title's ticket before the first push (a \
             rename after the PR opens closes it)."
        )),
        (None, Some(id)) => Ok(PrKind::Work(id.to_string())),
        (None, None) if branch.starts_with("sonar/") => Ok(PrKind::Sonar),
        (None, None) => Err(format!(
            "neither the title `{title}` nor the branch `{branch}` names a ticket. Title a \
             ticket's work `[NNNN] …`, a PR that only writes or edits tickets `Ticket NNNN: …` \
             (or `Tickets NNNN, MMMM: …`), and a game ticket's small change `Game NNNN: …` \
             (tickets/README.md, *Pull requests*)."
        )),
    }
}

/// Whether `kind` names this repository's ticket `id`.
fn names(kind: &PrKind, id: &str) -> bool {
    match kind {
        PrKind::Work(work) => work == id,
        PrKind::TicketsOnly(ids) => ids.iter().any(|named| named == id),
        _ => false,
    }
}

/// The kind a title names, or `None` when it names none.
fn title_kind(title: &str) -> Option<PrKind> {
    let title = title.trim();
    if title.starts_with("[advisory #") || title.starts_with("engine-pin") {
        return Some(PrKind::Bot);
    }
    if title.starts_with("sonar(") {
        return Some(PrKind::Sonar);
    }
    if let Some(rest) = title.strip_prefix('[') {
        let (id, _) = rest.split_once(']')?;
        return is_four_digits(id).then(|| PrKind::Work(id.to_string()));
    }
    if let Some(rest) = title.strip_prefix("Game ") {
        let (id, _) = rest.split_once(':')?;
        return is_four_digits(id).then(|| PrKind::GameSmall(id.to_string()));
    }
    let rest = title
        .strip_prefix("Tickets ")
        .or_else(|| title.strip_prefix("Ticket "))?;
    let (list, _) = rest.split_once(':')?;
    id_list(list).map(PrKind::TicketsOnly)
}

/// The ids of `2051-2053, 2256 and 2257`: four-digit ids or ranges, joined
/// by `,` or `and`. `None` if any part is something else.
fn id_list(list: &str) -> Option<Vec<String>> {
    let mut ids = Vec::new();
    for part in list.split(',').flat_map(|part| part.split(" and ")) {
        let part = part.trim();
        if is_four_digits(part) {
            ids.push(part.to_string());
            continue;
        }
        let (from, to) = part.split_once('-')?;
        if !is_four_digits(from) || !is_four_digits(to) {
            return None;
        }
        let (from, to): (u16, u16) = (from.parse().ok()?, to.parse().ok()?);
        if from > to {
            return None;
        }
        ids.extend((from..=to).map(|id| format!("{id:04}")));
    }
    (!ids.is_empty()).then_some(ids)
}

/// The errors of a PR of `kind`, given every ticket and the files it
/// `changed` (read only for a ticket-only PR).
pub(super) fn check_pr(tickets: &[Ticket], kind: &PrKind, changed: &[String]) -> Vec<String> {
    let find = |id: &str| tickets.iter().find(|t| t.fields.id.as_deref() == Some(id));
    match kind {
        PrKind::Work(id) => match find(id) {
            None => vec![format!(
                "the PR is for ticket `{id}`, but no ticket with that id exists"
            )],
            Some(t) if t.folder != Folder::Done => vec![format!(
                "the PR is for ticket `{id}`, which is not in tickets/done/ (a ticket's \
                 work PR must move it to done/; a PR that only writes or edits tickets is \
                 titled `Ticket {id}: …`)"
            )],
            Some(_) => Vec::new(),
        },
        PrKind::TicketsOnly(ids) => {
            let mut errors: Vec<String> = changed
                .iter()
                .find(|file| !TICKET_ONLY_PATHS.iter().any(|path| file.starts_with(path)))
                .map(|file| {
                    format!(
                        "a ticket-only PR changes only {}, but this one changes `{file}`. \
                         Title a ticket's work `[NNNN] …`.",
                        TICKET_ONLY_PATHS.join(", ")
                    )
                })
                .into_iter()
                .collect();
            for id in ids {
                match find(id) {
                    None => errors.push(format!(
                        "the title names ticket `{id}`, but no ticket with that id exists"
                    )),
                    Some(t) if t.folder != Folder::Open => errors.push(format!(
                        "the title names ticket `{id}` as only written or edited, but it is \
                         in tickets/done/. Title its work PR `[{id}] …`."
                    )),
                    Some(_) => {}
                }
            }
            errors
        }
        PrKind::GameSmall(_) | PrKind::Bot | PrKind::Sonar | PrKind::None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::ticket;
    use super::*;

    fn ids(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_string()).collect()
    }

    fn work(id: &str) -> PrKind {
        PrKind::Work(id.to_string())
    }

    // -- pr_kind --

    #[test]
    fn a_work_title_names_its_ticket_on_any_branch() {
        for branch in ["claude/x-1a2b3c", "t2051-x", "main"] {
            assert_eq!(pr_kind(Some("[2051] A thing"), branch), Ok(work("2051")));
        }
    }

    #[test]
    fn ticket_only_titles_name_their_tickets() {
        assert_eq!(
            pr_kind(Some("Ticket 3212: The options panel"), "claude/x-1a2b"),
            Ok(PrKind::TicketsOnly(ids(&["3212"])))
        );
        assert_eq!(
            pr_kind(Some("Tickets 2005 and 2823: their blockers"), "x"),
            Ok(PrKind::TicketsOnly(ids(&["2005", "2823"])))
        );
        assert_eq!(
            pr_kind(Some("Tickets 2051-2053, 2256 and 2257: guardrails"), "x"),
            Ok(PrKind::TicketsOnly(ids(&[
                "2051", "2052", "2053", "2256", "2257"
            ])))
        );
    }

    #[test]
    fn a_game_title_is_a_small_change() {
        assert_eq!(
            pr_kind(Some("Game 0859: a hook the game needs"), "claude/y-9f"),
            Ok(PrKind::GameSmall("0859".to_string()))
        );
    }

    #[test]
    fn bots_and_sonar_keep_their_kinds() {
        assert_eq!(
            pr_kind(
                Some("Bump serde from 1.0.1 to 1.0.2"),
                "dependabot/cargo/serde-1.0.2"
            ),
            Ok(PrKind::Bot)
        );
        assert_eq!(
            pr_kind(Some("[advisory #12] rustls 0.23"), "advisory/RUSTSEC-1"),
            Ok(PrKind::Bot)
        );
        assert_eq!(pr_kind(Some("engine-pin: move"), "x"), Ok(PrKind::Bot));
        assert_eq!(
            pr_kind(Some("sonar(#312): lifetimes"), "sonar/312-lifetimes"),
            Ok(PrKind::Sonar)
        );
        assert_eq!(
            pr_kind(Some("Lifetimes"), "sonar/312-lifetimes"),
            Ok(PrKind::Sonar)
        );
    }

    #[test]
    fn a_ticket_branch_must_name_the_titles_ticket() {
        let why = pr_kind(Some("[2051] A thing"), "t2052-x").unwrap_err();
        assert!(why.contains("names ticket 2052"), "{why}");
        assert!(why.contains("closes it"), "{why}");
        assert!(pr_kind(Some("Game 0859: x"), "t2051-x").is_err());
        assert_eq!(
            pr_kind(Some("Tickets 2050 and 2051: x"), "t2051-x"),
            Ok(PrKind::TicketsOnly(ids(&["2050", "2051"])))
        );
    }

    #[test]
    fn a_title_naming_nothing_falls_back_to_a_ticket_branch() {
        assert_eq!(pr_kind(Some("A thing"), "t2051-x"), Ok(work("2051")));
    }

    #[test]
    fn a_title_and_branch_naming_nothing_fail() {
        for (title, branch) in [
            ("A thing", "claude/x-1a2b3c"),
            ("Ticket one book: x", "main"),
            ("[20511] x", "claude/x"),
            ("Tickets 2051-2049: x", "claude/x"),
        ] {
            let why = pr_kind(Some(title), branch).unwrap_err();
            assert!(why.contains("names a ticket"), "{title}: {why}");
            assert!(why.contains("`Ticket NNNN: …`"), "{why}");
        }
    }

    #[test]
    fn without_a_title_the_branch_is_read() {
        assert_eq!(pr_kind(None, "t2304-pathfinding"), Ok(work("2304")));
        assert_eq!(pr_kind(None, "sonar/1-x"), Ok(PrKind::Sonar));
        assert_eq!(pr_kind(None, "dependabot/cargo/x"), Ok(PrKind::Bot));
        for branch in ["main", "claude/some-work", "game-1203-x"] {
            assert_eq!(pr_kind(None, branch), Ok(PrKind::None), "{branch}");
        }
    }

    // -- check_pr --

    #[test]
    fn work_passes_once_its_ticket_is_done() {
        let open = vec![ticket("2051", Folder::Open, "todo", &[])];
        let errors = check_pr(&open, &work("2051"), &[]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not in tickets/done/"), "{errors:?}");
        assert!(errors[0].contains("`Ticket 2051: …`"), "{errors:?}");
        let done = vec![ticket("2051", Folder::Done, "done", &[])];
        assert_eq!(check_pr(&done, &work("2051"), &[]), Vec::<String>::new());
    }

    #[test]
    fn work_on_an_unknown_ticket_fails() {
        let errors = check_pr(&[], &work("9999"), &[]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("no ticket with that id exists"));
    }

    #[test]
    fn a_ticket_only_pr_adding_an_open_ticket_passes() {
        let tickets = vec![ticket("2051", Folder::Open, "todo", &[])];
        let changed = ids(&[
            "tickets/open/2051-x.md",
            "docs/ROADMAP.md",
            "docs/decisions.md",
        ]);
        let kind = PrKind::TicketsOnly(ids(&["2051"]));
        let left = check_pr(&tickets, &kind, &changed);
        assert!(left.is_empty(), "{left:?}");
    }

    #[test]
    fn a_ticket_only_pr_changing_code_fails_naming_the_first_file() {
        let tickets = vec![ticket("2051", Folder::Open, "todo", &[])];
        let changed = ids(&[
            "tickets/open/2051-x.md",
            "crates/xtask/src/main.rs",
            "crates/core/src/lib.rs",
        ]);
        let kind = PrKind::TicketsOnly(ids(&["2051"]));
        let errors = check_pr(&tickets, &kind, &changed);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("`crates/xtask/src/main.rs`"),
            "{errors:?}"
        );
    }

    #[test]
    fn a_ticket_only_pr_naming_a_done_or_missing_ticket_fails() {
        let tickets = vec![ticket("2051", Folder::Done, "done", &[])];
        let kind = PrKind::TicketsOnly(ids(&["2051", "2052"]));
        let errors = check_pr(&tickets, &kind, &[]);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].contains("`[2051] …`"), "{errors:?}");
        assert!(
            errors[1].contains("no ticket with that id exists"),
            "{errors:?}"
        );
    }

    #[test]
    fn other_kinds_ask_nothing_of_the_tickets() {
        let changed = ids(&["crates/core/src/lib.rs"]);
        for kind in [
            PrKind::GameSmall("0859".to_string()),
            PrKind::Bot,
            PrKind::Sonar,
            PrKind::None,
        ] {
            assert!(check_pr(&[], &kind, &changed).is_empty(), "{kind:?}");
        }
    }
}
