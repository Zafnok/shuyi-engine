//! `cargo xtask site-data --site <dir>`: the home page's feature cards'
//! statuses, made from the tickets they name (ticket 2815).
//!
//! A card of `<dir>/data/features.toml` with `tickets` is **Ready** when
//! every one is in `tickets/done/`, **In progress** when some are and
//! **Planned** when none is; with no `link_text`, its link text names them.
//! The command writes each such card's status and link text, by its title,
//! to `<dir>/data/tickets.toml`, which is never committed: the home page's
//! template reads it with `load_data` and takes it over the card's own.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::tickets::read_ticket_files;

/// The file the cards are read from, under the site's folder.
const FEATURES: &str = "data/features.toml";
/// The file the made statuses are written to, under the site's folder.
const MADE: &str = "data/tickets.toml";

/// `cargo xtask site-data`'s options.
#[derive(Debug, PartialEq, Eq)]
pub struct Options {
    /// `--site <dir>`: the Zola project, `site`.
    pub site: PathBuf,
}

impl Options {
    /// The options in `args`, or `None` on anything else.
    pub fn parse(args: &[String]) -> Option<Self> {
        match args {
            [flag, dir] if flag == "--site" => Some(Self {
                site: PathBuf::from(dir),
            }),
            _ => None,
        }
    }
}

/// `features.toml`: its cards.
#[derive(Debug, Deserialize)]
struct Features {
    #[serde(default)]
    feature: Vec<Card>,
}

/// One card, as far as its status goes.
#[derive(Debug, Deserialize)]
struct Card {
    title: String,
    status: Option<String>,
    link_text: Option<String>,
    tickets: Option<Vec<String>>,
}

/// A card's status, made from its tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ready,
    InProgress,
    Planned,
}

impl Status {
    /// The words the card shows.
    pub fn word(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::InProgress => "In progress",
            Self::Planned => "Planned",
        }
    }
}

/// What the command makes for one card with `tickets`.
#[derive(Debug, PartialEq, Eq)]
pub struct Made {
    pub title: String,
    pub status: Status,
    pub link_text: String,
}

/// The status of a card naming `tickets`, by those in `done`.
pub fn status(tickets: &[String], done: &HashSet<String>) -> Status {
    let finished = tickets.iter().filter(|id| done.contains(*id)).count();
    if finished == tickets.len() {
        Status::Ready
    } else if finished > 0 {
        Status::InProgress
    } else {
        Status::Planned
    }
}

/// The link text that names `tickets`: "Ticket 2904", "Tickets 2904 and
/// 2107", "Tickets 2904, 2107 and 2108".
pub fn link_text(tickets: &[String]) -> String {
    match tickets {
        [] => String::new(),
        [one] => format!("Ticket {one}"),
        [rest @ .., last] => format!("Tickets {} and {last}", rest.join(", ")),
    }
}

/// Each card of `features` with `tickets`, made from the ids in `open` and
/// `done`. The errors name the card: one with both `tickets` and `status`,
/// with an empty `tickets`, or naming a ticket in neither folder.
pub fn make(
    features: &str,
    open: &HashSet<String>,
    done: &HashSet<String>,
) -> Result<Vec<Made>, Vec<String>> {
    let features: Features =
        toml::from_str(features).map_err(|e| vec![format!("{FEATURES}: {e}")])?;
    let mut made = Vec::new();
    let mut errors = Vec::new();
    for card in features.feature {
        let Some(tickets) = card.tickets else {
            continue;
        };
        let title = &card.title;
        if card.status.is_some() {
            errors.push(format!(
                "card `{title}` has both `tickets` and `status`: its status is made from \
                 its tickets, so drop `status`"
            ));
        }
        if tickets.is_empty() {
            errors.push(format!(
                "card `{title}` has an empty `tickets`: name its tickets, or drop it and \
                 give the card a `status`"
            ));
        }
        for id in tickets
            .iter()
            .filter(|id| !open.contains(*id) && !done.contains(*id))
        {
            errors.push(format!(
                "card `{title}` names ticket {id}, which is in neither tickets/open/ nor \
                 tickets/done/"
            ));
        }
        made.push(Made {
            status: status(&tickets, done),
            link_text: card.link_text.unwrap_or_else(|| link_text(&tickets)),
            title: card.title,
        });
    }
    if errors.is_empty() {
        Ok(made)
    } else {
        Err(errors)
    }
}

/// `made` as `tickets.toml`: a table per card, keyed by its title.
pub fn render(made: &[Made]) -> String {
    let mut out = String::from(
        "# Written by `cargo xtask site-data` from the tickets the cards of\n\
         # features.toml name (ticket 2815). Never committed.\n",
    );
    for card in made {
        let _ = write!(
            out,
            "\n[{}]\nstatus = {}\nlink_text = {}\n",
            quoted(&card.title),
            quoted(card.status.word()),
            quoted(&card.link_text)
        );
    }
    out
}

/// `text` as a TOML basic string.
fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The ids of the tickets in `dir`, from their file names (`NNNN-slug.md`).
fn ticket_ids(dir: &Path) -> Result<HashSet<String>, String> {
    let files = read_ticket_files(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(files
        .into_iter()
        .filter_map(|(name, _)| name.split_once('-').map(|(id, _)| id.to_string()))
        .collect())
}

/// Reads the cards of the site in `options` (a path from the current
/// folder, as `changelog --site` takes) and the tickets of the repository
/// in `root`, and writes `tickets.toml`.
pub fn run(root: &Path, options: &Options) -> Result<(), Vec<String>> {
    let site = &options.site;
    let features_path = site.join(FEATURES);
    let features = fs::read_to_string(&features_path)
        .map_err(|e| vec![format!("{}: {e}", features_path.display())])?;
    let open = ticket_ids(&root.join("tickets/open")).map_err(|e| vec![e])?;
    let done = ticket_ids(&root.join("tickets/done")).map_err(|e| vec![e])?;
    let made = make(&features, &open, &done)?;
    let out = site.join(MADE);
    fs::write(&out, render(&made)).map_err(|e| vec![format!("{}: {e}", out.display())])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> HashSet<String> {
        list.iter().map(|id| (*id).to_string()).collect()
    }

    fn list(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_string()).collect()
    }

    #[test]
    fn options_read_the_site() {
        let args = list(&["--site", "site"]);
        assert_eq!(
            Options::parse(&args),
            Some(Options {
                site: PathBuf::from("site")
            })
        );
        assert_eq!(Options::parse(&list(&["--site"])), None);
        assert_eq!(Options::parse(&list(&["--other", "site"])), None);
        assert_eq!(Options::parse(&[]), None);
    }

    #[test]
    fn every_ticket_done_is_ready() {
        let done = ids(&["2001", "2002"]);
        assert_eq!(status(&list(&["2001", "2002"]), &done), Status::Ready);
    }

    #[test]
    fn some_done_is_in_progress() {
        let done = ids(&["2001"]);
        assert_eq!(status(&list(&["2001", "2002"]), &done), Status::InProgress);
    }

    #[test]
    fn none_done_is_planned() {
        let done = ids(&["2003"]);
        assert_eq!(status(&list(&["2001", "2002"]), &done), Status::Planned);
    }

    #[test]
    fn each_status_has_its_words() {
        assert_eq!(
            [Status::Ready, Status::InProgress, Status::Planned].map(Status::word),
            ["Ready", "In progress", "Planned"]
        );
    }

    #[test]
    fn the_link_text_names_one_ticket() {
        assert_eq!(link_text(&list(&["2904"])), "Ticket 2904");
    }

    #[test]
    fn the_link_text_names_several_tickets() {
        assert_eq!(link_text(&list(&["2904", "2107"])), "Tickets 2904 and 2107");
        assert_eq!(
            link_text(&list(&["2904", "2107", "2108"])),
            "Tickets 2904, 2107 and 2108"
        );
    }

    const FEATURES_TOML: &str = r#"
[[feature]]
title = "Done"
text = "x"
tickets = ["2001", "2002"]
link = "x"

[[feature]]
title = "Half"
text = "x"
tickets = ["2001", "2101"]
link = "x"
link_text = "Its own words"

[[feature]]
title = "None"
text = "x"
tickets = ["2101"]
link = "x"

[[feature]]
title = "By hand"
text = "x"
status = "Ready"
link = "x"
link_text = "The licenses"
"#;

    #[test]
    fn cards_with_tickets_are_made_and_the_others_left() {
        let made = make(FEATURES_TOML, &ids(&["2101"]), &ids(&["2001", "2002"])).unwrap();
        assert_eq!(
            made,
            vec![
                Made {
                    title: "Done".to_string(),
                    status: Status::Ready,
                    link_text: "Tickets 2001 and 2002".to_string(),
                },
                Made {
                    title: "Half".to_string(),
                    status: Status::InProgress,
                    link_text: "Its own words".to_string(),
                },
                Made {
                    title: "None".to_string(),
                    status: Status::Planned,
                    link_text: "Ticket 2101".to_string(),
                },
            ]
        );
    }

    #[test]
    fn a_card_with_tickets_and_a_status_fails_naming_it() {
        let features = "[[feature]]\ntitle = \"Both\"\nstatus = \"Ready\"\ntickets = [\"2001\"]\n";
        let errors = make(features, &HashSet::new(), &ids(&["2001"])).unwrap_err();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with("card `Both` has both `tickets` and `status`"),
            "{errors:?}"
        );
    }

    #[test]
    fn a_card_naming_an_unknown_ticket_fails_naming_it() {
        let features = "[[feature]]\ntitle = \"Lost\"\ntickets = [\"2001\", \"9999\"]\n";
        let errors = make(features, &ids(&["2001"]), &HashSet::new()).unwrap_err();
        assert_eq!(
            errors,
            vec![
                "card `Lost` names ticket 9999, which is in neither tickets/open/ nor \
                 tickets/done/"
                    .to_string()
            ]
        );
    }

    #[test]
    fn a_card_with_no_tickets_listed_fails_naming_it() {
        let features = "[[feature]]\ntitle = \"Empty\"\ntickets = []\n";
        let errors = make(features, &HashSet::new(), &HashSet::new()).unwrap_err();
        assert!(
            errors[0].starts_with("card `Empty` has an empty"),
            "{errors:?}"
        );
    }

    #[test]
    fn a_file_that_does_not_parse_fails() {
        let errors = make("[[feature]\n", &HashSet::new(), &HashSet::new()).unwrap_err();
        assert!(errors[0].starts_with("data/features.toml: "), "{errors:?}");
    }

    #[test]
    fn the_file_is_a_table_per_card_by_title() {
        let made = [Made {
            title: "Say \"hi\"".to_string(),
            status: Status::InProgress,
            link_text: "Ticket 2001".to_string(),
        }];
        let text = render(&made);
        assert!(
            text.ends_with(
                "\n[\"Say \\\"hi\\\"\"]\nstatus = \"In progress\"\nlink_text = \"Ticket 2001\"\n"
            ),
            "{text}"
        );
        let table: toml::Table = text.parse().unwrap();
        assert_eq!(table["Say \"hi\""]["status"].as_str(), Some("In progress"));
    }

    #[test]
    fn quoted_escapes_what_toml_needs() {
        assert_eq!(quoted("a\\b\n\u{1}"), "\"a\\\\b\\n\\u0001\"");
    }

    /// A repository of its own under the temp folder: two tickets open,
    /// one done, and a site whose cards name them.
    fn fixture(name: &str, features: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("xtask-site-data-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in ["tickets/open", "tickets/done", "site/data"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        for (dir, file) in [
            ("open", "2101-later.md"),
            ("open", "2102-later-still.md"),
            ("open", "README.md"),
            ("done", "2001-shipped.md"),
        ] {
            fs::write(root.join("tickets").join(dir).join(file), "---\n---\n").unwrap();
        }
        fs::write(root.join("site/data/features.toml"), features).unwrap();
        root
    }

    #[test]
    fn the_command_writes_the_made_statuses() {
        let features = "[[feature]]\ntitle = \"A\"\ntickets = [\"2001\", \"2101\"]\n\n\
                        [[feature]]\ntitle = \"B\"\ntickets = [\"2101\", \"2102\"]\n\n\
                        [[feature]]\ntitle = \"C\"\ntickets = [\"2001\"]\n";
        let root = fixture("writes", features);
        let options = Options {
            site: root.join("site"),
        };
        assert_eq!(run(&root, &options), Ok(()));
        let table: toml::Table = fs::read_to_string(root.join("site/data/tickets.toml"))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(table["A"]["status"].as_str(), Some("In progress"));
        assert_eq!(table["B"]["status"].as_str(), Some("Planned"));
        assert_eq!(
            table["B"]["link_text"].as_str(),
            Some("Tickets 2101 and 2102")
        );
        assert_eq!(table["C"]["status"].as_str(), Some("Ready"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_command_fails_on_an_unknown_ticket_and_writes_nothing() {
        let root = fixture(
            "unknown",
            "[[feature]]\ntitle = \"A\"\ntickets = [\"2999\"]\n",
        );
        let options = Options {
            site: root.join("site"),
        };
        let errors = run(&root, &options).unwrap_err();
        assert!(
            errors[0].contains("card `A` names ticket 2999"),
            "{errors:?}"
        );
        assert!(!root.join("site/data/tickets.toml").exists());
        let _ = fs::remove_dir_all(&root);
    }
}
