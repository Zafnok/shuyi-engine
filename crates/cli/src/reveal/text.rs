//! The kit's two texts: `EVIDENCE.md`, the index a report of a public clip
//! copies, sealed through the manifest, and `CHECKLIST.md`, the steps
//! before the reveal, a working copy.

use std::fmt::Write as _;

use super::kit::{Kept, MANIFEST, Manifest, SEAL};

/// The line every text of the kit ends with.
const NOT_ADVICE: &str = "This is information, not legal advice. The kit records what it can \
check and judges nothing: whether a clip or a game copies this one, and what to do about it, \
is for the game's owner and their lawyer.";

/// `text` fit for one cell of a Markdown table.
fn cell(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        "-".to_owned()
    } else {
        text.replace('|', "\\|")
    }
}

/// The source build, in a line.
fn build_line(manifest: &Manifest) -> String {
    let build = &manifest.build;
    let version = build.version.as_deref().unwrap_or("not set");
    match &build.commit {
        Some(commit) if build.dirty => {
            format!("version {version}, commit `{commit}` with changes not committed")
        }
        Some(commit) => format!("version {version}, commit `{commit}`"),
        None => format!("version {version}, not in git"),
    }
}

/// A row of a table of kept files.
fn kept_row(text: &mut String, kept: &Kept, first: &str) {
    // Writing to a `String` can't fail.
    let _ = writeln!(
        text,
        "| {first}`{}` | `{}` | {} |",
        cell(&kept.path),
        kept.sha256,
        kept.bytes
    );
}

/// `EVIDENCE.md`: what the kit holds, every hash, and the part of a report
/// a person fills in for each public clip.
pub fn evidence(manifest: &Manifest) -> String {
    let mut text = String::new();
    // Writing to a `String` can't fail.
    let _ = writeln!(text, "# Evidence index: {}\n", manifest.game);
    let _ = writeln!(
        text,
        "Written by `shuyi reveal prepare` on {} (this machine's clock: the date is \
         a claim; a receipt, or the place the kit's digest was published, dates it). \
         The kit's digest is the SHA-256 of `{MANIFEST}`, written in `{SEAL}`. Check \
         the kit with `shuyi reveal verify`, or by hand: `sha256sum -c {SEAL}`, then \
         each file's SHA-256 below.\n",
        manifest.prepared
    );
    let _ = writeln!(text, "- Game: {}", manifest.game);
    let _ = writeln!(text, "- Store page: <{}>", manifest.store_url);
    let keywords = if manifest.keywords.is_empty() {
        "none".to_owned()
    } else {
        manifest.keywords.join(", ")
    };
    let _ = writeln!(text, "- Keywords: {keywords}");
    let _ = writeln!(text, "- Source build: {}\n", build_line(manifest));
    captures(&mut text, manifest);
    held(&mut text, manifest);
    text.push_str(
        "## A public clip\n\n\
         Copy this section into a report for each clip, and fill it in.\n\n\
         - The clip's address:\n\
         - Where and when it was first seen:\n\
         - The approved capture it shows or matches (its file above):\n\
         - That capture's SHA-256 (above), and the kit's digest (`reveal.ron.sha256`):\n\
         - The source build that made the capture (above):\n\
         - What else ties the clip to the build (a detail only the build shows, a file \
         comparison):\n\n",
    );
    text.push_str(NOT_ADVICE);
    text.push('\n');
    text
}

/// The index's section of the approved captures.
fn captures(text: &mut String, manifest: &Manifest) {
    text.push_str("## Approved captures\n\n");
    if manifest.captures.is_empty() {
        text.push_str("None.\n\n");
    } else {
        text.push_str("| File | SHA-256 | Bytes |\n| --- | --- | --- |\n");
        for capture in &manifest.captures {
            kept_row(text, &capture.original, "");
            for encode in &capture.encodes {
                kept_row(text, encode, "re-encode: ");
            }
        }
        text.push('\n');
        for capture in &manifest.captures {
            let _ = write!(
                text,
                "- `{}`: {}",
                capture.original.path,
                cell(&capture.what)
            );
            if !capture.args.is_empty() {
                let _ = write!(
                    text,
                    "; made by `shuyi frame-png <out> {}` on the source build",
                    capture.args.join(" ")
                );
            }
            text.push('\n');
        }
        text.push('\n');
    }
}

/// The index's sections of what else the kit holds or names: the builds,
/// the receipts and the commitments.
fn held(text: &mut String, manifest: &Manifest) {
    text.push_str("## Builds\n\n");
    if manifest.artifacts.is_empty() {
        text.push_str("None: the reveal is a store page, with no build released.\n\n");
    } else {
        text.push_str("| File | SHA-256 | Bytes |\n| --- | --- | --- |\n");
        for artifact in &manifest.artifacts {
            let bytes = artifact
                .bytes
                .map_or_else(|| "-".to_owned(), |bytes| bytes.to_string());
            let _ = writeln!(
                text,
                "| `{}` | `{}` | {bytes} |",
                cell(&artifact.name),
                artifact.sha256
            );
        }
        text.push('\n');
    }

    text.push_str("## Receipts\n\n");
    if manifest.receipts.is_empty() {
        text.push_str("None.\n\n");
    } else {
        text.push_str("| File | SHA-256 | Bytes |\n| --- | --- | --- |\n");
        for receipt in &manifest.receipts {
            kept_row(text, receipt, "");
        }
        text.push('\n');
    }

    text.push_str("## Commitments\n\n");
    if manifest.commitments.is_empty() {
        text.push_str("None.\n\n");
    } else {
        text.push_str(
            "Each opens with its file's bytes and the salt kept privately \
             (`shuyi reveal open`).\n\n| Label | SHA-256 |\n| --- | --- |\n",
        );
        for commitment in &manifest.commitments {
            let _ = writeln!(
                text,
                "| {} | `{}` |",
                cell(&commitment.label),
                commitment.sha256
            );
        }
        text.push('\n');
    }
}

/// `CHECKLIST.md`: the steps before the first public footage. It is a
/// working copy to tick: the kit doesn't seal it.
pub fn checklist(manifest: &Manifest, digest: &str) -> String {
    let mut text = String::new();
    // Writing to a `String` can't fail.
    let _ = writeln!(
        text,
        "# Before the first public footage: {}\n\n\
         Tick these before the reveal. This file is a working copy: `shuyi reveal \
         verify` doesn't check it, and everything else in the folder is sealed.\n",
        manifest.game
    );
    let _ = writeln!(
        text,
        "- [ ] Open the store page, <{}>: it is live, its name is \"{}\", and the media \
         it shows are the approved captures in `EVIDENCE.md`. A capture it shows that \
         isn't there: add it to `shuyi.ron`'s `reveal`, then `shuyi reveal prepare` \
         again.",
        manifest.store_url, manifest.game
    );
    let _ = writeln!(
        text,
        "- [ ] Show only the approved captures: the reveal's trailer, pictures and clips \
         are files of this kit, as they are here."
    );
    let _ = writeln!(
        text,
        "- [ ] `shuyi reveal verify` passes, and the kit's digest, `{digest}`, is \
         published somewhere that dates it: a commit or a tag pushed to the game's \
         repository, or a release's notes. Then `shuyi reveal verify --digest \
         <digest>` checks a copy against it."
    );
    if !manifest.commitments.is_empty() {
        let _ = writeln!(
            text,
            "- [ ] The salts file of the commitments is kept privately, and backed up: \
             never in a public repository, nor in a folder the build embeds. Without it \
             no commitment opens."
        );
    }
    let _ = writeln!(
        text,
        "- [ ] Copyright registration of the game's art, music and text, before the \
         first public footage. In the US the standard fee is about US$65, and \
         statutory damages and attorney's fees need the registration before the \
         infringement, or within three months of first publication (17 U.S.C. 412). \
         Whether to register, where, and paying the fee, are the game owner's \
         decisions: the kit only reminds."
    );
    let _ = writeln!(
        text,
        "- [ ] Keep the kit's folder whole and unchanged: copy it or zip it, never edit \
         it. A capture that changes makes a new kit, with a new digest.\n"
    );
    text.push_str(NOT_ADVICE);
    text.push('\n');
    text
}
