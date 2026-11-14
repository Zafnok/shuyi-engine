//! A ticket's size (ticket 2053): the `**Estimated size:**` an open ticket
//! plans, the `**Split:**` point a large one names, and the `Size:` line of
//! the done ticket of a PR over the limit (`cargo xtask pr-size`).
//!
//! - An open `feature` or `infra` ticket of effort `high` or above, not
//!   parked, has an `**Estimated size:** about N lines` line. The ids in
//!   [`UNESTIMATED`] were written before the rule and are let off until
//!   their session estimates them.
//! - An estimate over [`SPLIT_OVER`] lines comes with a `**Split:**` line
//!   that says at what point (a number of lines) and what to split off.
//! - A `Size:` line in a done ticket's completion notes is `Size: N counted
//!   lines, <why it wasn't split>`.

use std::collections::HashSet;

use super::{Folder, Ticket, notes_lines, without_inline_code};

/// The mark of an open ticket's estimate.
pub const ESTIMATE: &str = "**Estimated size:**";
/// The mark of where a large ticket splits.
pub const SPLIT: &str = "**Split:**";
/// The mark of a done ticket's measured size, in its completion notes.
pub const SIZE: &str = "Size:";
/// An estimate over this many lines names its split point.
pub const SPLIT_OVER: usize = 1_000;
/// The open tickets written before the rule, one id a line (`#` comments).
pub const UNESTIMATED: &str = "tools/unestimated-tickets.txt";

const ESTIMATED_TYPES: &[&str] = &["feature", "infra"];
const ESTIMATED_EFFORTS: &[&str] = &["high", "xhigh", "max"];

/// A count at the start of `text`, after an optional `about` or `~`: digits,
/// with commas between thousands. Returns it and the text after it.
pub fn leading_count(text: &str) -> Option<(usize, &str)> {
    let text = text.trim_start();
    let text = text
        .strip_prefix("about ")
        .or_else(|| text.strip_prefix('~'))
        .unwrap_or(text)
        .trim_start();
    let end = text
        .find(|c: char| !(c.is_ascii_digit() || c == ','))
        .unwrap_or(text.len());
    let digits = text[..end].trim_end_matches(',');
    if digits.is_empty() || digits.starts_with(',') {
        return None;
    }
    let count = digits.replace(',', "").parse().ok()?;
    Some((count, &text[digits.len()..]))
}

/// The number a `Size:` line of `body`'s completion notes measured, or
/// `None` without one, or why the line is wrong: it needs the number and,
/// after it, why the PR wasn't split.
pub fn size_line(body: &str) -> Result<Option<usize>, String> {
    let Some(line) = notes_lines(body, SIZE).into_iter().next() else {
        return Ok(None);
    };
    let rest = &line[SIZE.len()..];
    let Some((count, rest)) = leading_count(rest) else {
        return Err(format!(
            "`{line}`: a `Size:` line starts with the measured number, as `Size: 1,820 \
             counted lines, <why it wasn't split>`"
        ));
    };
    let rest = rest.trim_start();
    let rest = rest
        .strip_prefix("counted lines")
        .or_else(|| rest.strip_prefix("lines"))
        .unwrap_or(rest);
    let why = rest.trim_start_matches([',', ';', ':', '.', '—', '-', '(', ' ']);
    if why.split_whitespace().count() < 3 {
        return Err(format!(
            "`{line}`: a `Size:` line says why the PR wasn't split, after the number"
        ));
    }
    Ok(Some(count))
}

/// Whether `ticket` must carry an estimate: open, not parked, a `feature` or
/// `infra` of effort `high` or above.
fn needs_estimate(ticket: &Ticket) -> bool {
    let fields = &ticket.fields;
    ticket.folder == Folder::Open
        && !fields.parked
        && fields.status.as_deref() != Some("done")
        && fields
            .kind
            .as_deref()
            .is_some_and(|k| ESTIMATED_TYPES.contains(&k))
        && fields
            .effort
            .as_deref()
            .is_some_and(|e| ESTIMATED_EFFORTS.contains(&e))
}

/// The text after `mark` on the first line of `body` that holds it, outside
/// fenced code blocks and inline code (a ticket quoting the rule).
fn marked(body: &str, mark: &str) -> Option<String> {
    let mut fenced = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        let line = without_inline_code(line);
        if !fenced && let Some(at) = line.find(mark) {
            return Some(line[at + mark.len()..].to_owned());
        }
    }
    None
}

/// What is wrong with an open ticket's estimate: a mark with no number, or
/// an estimate over [`SPLIT_OVER`] with no `**Split:**` line naming a point.
/// `None` also when the ticket has no estimate: [`check_estimates`] says
/// whether it needs one.
fn estimate_error(ticket: &Ticket) -> Option<String> {
    let body = &ticket.fields.body;
    let estimate = marked(body, ESTIMATE)?;
    let Some((lines, _)) = leading_count(&estimate) else {
        return Some(format!(
            "`{ESTIMATE}` starts with the lines the PR will change, as `{ESTIMATE} about \
             800 lines` (write-ticket step 5)"
        ));
    };
    if lines <= SPLIT_OVER {
        return None;
    }
    let names_a_point = marked(body, SPLIT).is_some_and(|split| {
        split
            .split_whitespace()
            .any(|word| leading_count(word).is_some())
    });
    (!names_a_point).then(|| {
        format!(
            "the estimate of {lines} lines is over {SPLIT_OVER}: add a `{SPLIT}` line saying \
             at what point (`past 900 lines`) and what to split off into a new ticket \
             (write-ticket step 5)"
        )
    })
}

/// The ids listed in [`UNESTIMATED`]'s `text`.
pub fn unestimated_ids(text: &str) -> HashSet<String> {
    text.lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The estimate rules over every ticket: errors for a ticket that needs an
/// estimate and has none (unless `unestimated` lets it off) or has a wrong
/// one, and warnings for an id of `unestimated` that no longer needs to be
/// there.
pub fn check_estimates(
    tickets: &[Ticket],
    unestimated: &HashSet<String>,
) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut let_off = HashSet::new();
    for ticket in tickets.iter().filter(|t| t.folder == Folder::Open) {
        if let Some(error) = estimate_error(ticket) {
            errors.push(format!("{}: {error}", ticket.display_path));
            continue;
        }
        if !needs_estimate(ticket) || marked(&ticket.fields.body, ESTIMATE).is_some() {
            continue;
        }
        match ticket.fields.id.as_deref() {
            Some(id) if unestimated.contains(id) => {
                let_off.insert(id);
            }
            _ => errors.push(format!(
                "{}: an open `feature` or `infra` ticket of effort `high` or above has an \
                 `{ESTIMATE} about N lines` line, and past {SPLIT_OVER} lines a `{SPLIT}` line \
                 (write-ticket step 5)",
                ticket.display_path
            )),
        }
    }
    let mut stale: Vec<&String> = unestimated
        .iter()
        .filter(|id| !let_off.contains(id.as_str()))
        .collect();
    stale.sort();
    let warnings = stale
        .into_iter()
        .map(|id| {
            format!(
                "{UNESTIMATED}: ticket {id} is no longer an open ticket without an estimate \
                 that needs one; take it off the list"
            )
        })
        .collect();
    (errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::super::tests::ticket;
    use super::*;

    fn open(id: &str, kind: &str, effort: &str, body: &str) -> Ticket {
        let mut t = ticket(id, Folder::Open, "todo", &[]);
        t.fields.kind = Some(kind.to_owned());
        t.fields.effort = Some(effort.to_owned());
        t.fields.body = body.to_owned();
        t
    }

    fn check(tickets: &[Ticket], unestimated: &[&str]) -> (Vec<String>, Vec<String>) {
        let unestimated = unestimated.iter().map(|s| (*s).to_owned()).collect();
        check_estimates(tickets, &unestimated)
    }

    #[test]
    fn a_count_reads_commas_and_about() {
        assert_eq!(leading_count("1,820 counted"), Some((1_820, " counted")));
        assert_eq!(leading_count(" about 650 lines"), Some((650, " lines")));
        assert_eq!(leading_count("~1,200."), Some((1_200, ".")));
        assert_eq!(leading_count("900, then"), Some((900, ", then")));
        assert_eq!(leading_count("lines"), None);
        assert_eq!(leading_count(",5"), None);
        assert_eq!(leading_count(""), None);
    }

    #[test]
    fn a_high_feature_ticket_with_no_estimate_fails() {
        let (errors, _) = check(&[open("2901", "feature", "high", "# body\n")], &[]);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("Estimated size"), "{errors:?}");
        for effort in ["xhigh", "max"] {
            let (errors, _) = check(&[open("2901", "infra", effort, "")], &[]);
            assert_eq!(errors.len(), 1, "{effort}");
        }
    }

    #[test]
    fn only_open_unparked_high_features_and_infra_need_one() {
        let mut parked = open("2902", "feature", "high", "");
        parked.fields.parked = true;
        let mut done = open("2903", "feature", "high", "");
        done.folder = Folder::Done;
        let tickets = [
            open("2901", "feature", "medium", ""),
            open("2904", "bug", "high", ""),
            open("2905", "research", "max", ""),
            parked,
            done,
        ];
        assert_eq!(check(&tickets, &[]), (vec![], vec![]));
    }

    #[test]
    fn an_estimate_under_the_split_point_passes() {
        let body = "## Scope\n\n**Estimated size:** about 800 lines.\n";
        assert_eq!(
            check(&[open("2901", "feature", "high", body)], &[]),
            (vec![], vec![])
        );
    }

    #[test]
    fn an_estimate_needs_a_number() {
        let body = "**Estimated size:** large\n";
        let (errors, _) = check(&[open("2901", "feature", "high", body)], &[]);
        assert!(errors[0].contains("starts with the lines"), "{errors:?}");
    }

    #[test]
    fn a_large_estimate_names_where_to_split() {
        let large = "**Estimated size:** about 1,400 lines.\n";
        let (errors, _) = check(&[open("2901", "feature", "high", large)], &[]);
        assert!(errors[0].contains("1400 lines is over 1000"), "{errors:?}");

        let vague = format!("{large}**Split:** split off the cache if it grows.\n");
        let (errors, _) = check(&[open("2901", "feature", "high", &vague)], &[]);
        assert_eq!(errors.len(), 1, "{errors:?}");

        let split = format!("{large}**Split:** past 900 lines, split off the cache.\n");
        assert_eq!(
            check(&[open("2901", "feature", "high", &split)], &[]),
            (vec![], vec![])
        );
        // Any open ticket's estimate follows the rule, whatever its effort.
        let (errors, _) = check(&[open("2901", "bug", "low", large)], &[]);
        assert_eq!(errors.len(), 1, "{errors:?}");
    }

    #[test]
    fn a_mark_in_a_fenced_block_is_not_an_estimate() {
        let body = "```text\n**Estimated size:** about 800 lines\n```\nIt has an \
                    `**Estimated size:**` line.\n";
        let (errors, _) = check(&[open("2901", "feature", "high", body)], &[]);
        assert_eq!(errors.len(), 1, "{errors:?}");
    }

    #[test]
    fn a_listed_ticket_is_let_off_and_a_stale_entry_warns() {
        let tickets = [
            open("2901", "feature", "high", ""),
            open(
                "2902",
                "feature",
                "high",
                "**Estimated size:** about 500 lines\n",
            ),
        ];
        let (errors, warnings) = check(&tickets, &["2901", "2902", "2999"]);
        assert_eq!(errors, Vec::<String>::new());
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].contains("ticket 2902"), "{warnings:?}");
        assert!(warnings[1].contains("ticket 2999"), "{warnings:?}");
        // A wrong estimate fails even on the list.
        let wrong = [open("2901", "feature", "high", "**Estimated size:** big\n")];
        assert_eq!(check(&wrong, &["2901"]).0.len(), 1);
    }

    #[test]
    fn the_list_reads_ids_and_skips_comments() {
        let ids = unestimated_ids("# written before 2053\n2901\n\n2902  # big\n");
        assert_eq!(ids, ["2901", "2902"].map(str::to_owned).into());
    }

    #[test]
    fn a_size_line_has_the_number_and_why() {
        let notes = |line: &str| format!("## Scope\n\nSize: 9\n\n## Completion notes\n\n{line}\n");
        assert_eq!(size_line(&notes("Done.")), Ok(None));
        assert_eq!(
            size_line(&notes(
                "Size: 1,820 counted lines, the parser and its fixtures are one change."
            )),
            Ok(Some(1_820))
        );
        assert_eq!(
            size_line(&notes(
                "- Size: about 2,100 lines; the tests can't land apart."
            )),
            Ok(Some(2_100))
        );
        assert!(
            size_line(&notes("Size: big, it is one change."))
                .unwrap_err()
                .contains("starts with the measured number")
        );
        assert!(
            size_line(&notes("Size: 1,820 counted lines."))
                .unwrap_err()
                .contains("says why")
        );
    }
}
