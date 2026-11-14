// The playground's colors match the book's (ticket 2832): for each sample,
// static/ron-paint.js must give every character the scopes Zola gives it
// in a `ron` or `dlg` code block. Plain Node, no packages; CI's `site` job
// runs it with `zola` on PATH (or ZOLA naming it):
//
//     node site/tests/ron-paint.mjs [<a built site>]
//
// The samples are Hello, Grid's `.ron`, `.map` and `.dlg` files, the book's
// project's, the skill pack's dialogue, and the cases below, built by Zola
// into a small site of their own with the website's grammars and settings.
// With a built site, every `ron` and `dlg` block of its pages is checked
// too.
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { cpSync, mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const site = join(here, "..");
const repo = join(site, "..");
const ronPaint = createRequire(import.meta.url)(join(site, "static", "ron-paint.js"));

// What a file the editors hold may have, that no sample file has, by
// language.
const RON_CASES = [
    "/* a block /* nested */ still the block */ (x: 1)\n",
    "/* over\n   two lines */ Some(\"a\\\"b\\n\")\n",
    "(text: \"over\nlines\", flag: true, none: None, n: -12, h: 0xFF_ff, o: 0o17, b: 0b1010, f: 1.5e-3)\n",
    "{ \"key\": 'c', \"k\\\"ey\" : '\\n', \"a\", \"b\": false }\n",
    "// a comment with \"quotes\" and /* a block */\nName(field_1: Other, _x : [1, 2])\n",
    "\"never closed\n(on: 1)\n",
    "(legend: { '#': Wall })\n---\n#####\n#@ \"x\" // #\n",
    "tags: [Tag, true, \"x\"], <&> é: 1\n",
    "(a:1,b:\"\\u{1F600}\",c:'\\'')\n"
];
const DLG_CASES = [
    "# a comment: with a colon\n@scene intro\n@left guide calm\n> Narration: not a speaker.\n@end\n",
    "@scene s\nguide[smile]: Hi.\nguide (softly): Hi.\nguide[smile] (softly): Hi.\n> (aside): Narration.\n@end\n",
    "@scene s\n@choice\n* earnest: Then we go now.\n  guide: Good. Keep close.\n  @if guide\n  guide: x[1]: y\n  @endif\n* wry: Then.\n@endchoice\n@end\n",
    "@scene s\nguide: A line\n  that goes on: here.\nno colon here\n  # indented comment\n*bad\n<&> é: \"x\"\n@end\n"
];

// Each language, by the name its code blocks are fenced with: its files'
// endings, the folders its sample files are in, and its cases.
const LANGUAGES = {
    ron: {
        ends: [".ron", ".map"],
        dirs: [join(repo, "examples", "hello-grid", "files"), join(site, "tutorial")],
        cases: RON_CASES
    },
    dlg: {
        ends: [".dlg"],
        dirs: [join(repo, "examples", "hello-grid", "files"), join(site, "tutorial"), join(repo, "skills")],
        cases: DLG_CASES
    }
};

// The files under `dir` whose names `keep`, builds' folders left out.
function files(dir, keep) {
    let found = [];
    for (const name of readdirSync(dir)) {
        const path = join(dir, name);
        if (statSync(path).isDirectory()) {
            if (name === "target" || name === "dist") {
                continue;
            }
            found = found.concat(files(path, keep));
        } else if (keep(name)) {
            found.push(path);
        }
    }
    return found.sort();
}

function samples(lang) {
    const { ends, dirs, cases } = LANGUAGES[lang];
    const keep = (name) => ends.some((end) => name.endsWith(end));
    const paths = dirs.flatMap((dir) => files(dir, keep));
    return paths.map((path) => ({ name: relative(repo, path), text: readFileSync(path, "utf8").replace(/\r\n/g, "\n") }))
        .concat(cases.map((text, i) => ({ name: lang + " case " + (i + 1), text })));
}

// Zola's HTML of each sample, in order: a site of its own with the
// website's grammars and highlighting, one page holding them as `lang`
// blocks.
function zola(list, lang) {
    const dir = mkdtempSync(join(tmpdir(), "ron-paint-"));
    try {
        cpSync(join(site, "syntaxes"), join(dir, "syntaxes"), { recursive: true });
        writeFileSync(join(dir, "config.toml"), [
            'base_url = "http://127.0.0.1"',
            "[markdown]",
            "highlight_code = true",
            'highlight_theme = "css"',
            'extra_syntaxes_and_themes = ["syntaxes"]',
            ""
        ].join("\n"));
        mkdirSync(join(dir, "templates"));
        writeFileSync(join(dir, "templates", "index.html"), "{{ section.content | safe }}\n");
        mkdirSync(join(dir, "content"));
        const fence = "``````";
        writeFileSync(join(dir, "content", "_index.md"), "+++\n+++\n" + list.map((s) => fence + lang + "\n" + s.text + fence + "\n").join("\n"));
        execFileSync(process.env.ZOLA || "zola", ["build", "--output-dir", "public", "--force"], { cwd: dir, stdio: ["ignore", "ignore", "inherit"] });
        return blocks(readFileSync(join(dir, "public", "index.html"), "utf8"), lang);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
}

// Every `lang` block's inner HTML in a page.
function blocks(html, lang) {
    const found = [];
    const block = new RegExp('<pre data-lang="' + lang + '"[^>]*><code[^>]*>([\\s\\S]*?)</code></pre>', "g");
    let m;
    while ((m = block.exec(html)) !== null) {
        found.push(m[1]);
    }
    return found;
}

function unescape(text) {
    return text.replace(/&(#x[0-9a-fA-F]+|#[0-9]+|quot|amp|lt|gt|apos);/g, (all, name) => {
        if (name[0] === "#") {
            return String.fromCodePoint(name[1] === "x" ? parseInt(name.slice(2), 16) : parseInt(name.slice(1), 10));
        }
        return { quot: '"', amp: "&", lt: "<", gt: ">", apos: "'" }[name];
    });
}

// Each character of Zola's block with the classes over it, outermost
// first, `|` between spans. Zola wraps each line in `z-source z-<lang>`,
// which colors nothing: it is left out.
function zolaChars(html, lang) {
    const chars = [];
    const stack = [];
    const part = /<span class="([^"]*)">|<\/span>|([^<]+)/g;
    let m;
    while ((m = part.exec(html)) !== null) {
        if (m[1] !== undefined) {
            stack.push(m[1]);
        } else if (m[2] !== undefined) {
            const over = stack.filter((c) => c !== "z-source z-" + lang).join("|");
            for (const ch of unescape(m[2])) {
                chars.push({ ch, over });
            }
        } else {
            stack.pop();
        }
    }
    return chars;
}

function ourChars(text, lang) {
    const chars = [];
    for (const run of ronPaint.tokenize(text, lang)) {
        const over = run.scopes.map(ronPaint.classes).join("|");
        for (const ch of run.text) {
            chars.push({ ch, over });
        }
    }
    return chars;
}

// The first difference between the two, or null. Newlines are left out:
// they color nothing, and Zola closes its spans at each line's end.
function compare(name, text, html, lang) {
    const theirs = zolaChars(html, lang);
    const ours = ourChars(text, lang);
    const got = theirs.map((c) => c.ch).join("");
    if (got !== text) {
        return name + ": Zola's block doesn't hold the sample's text (" + JSON.stringify(got.slice(0, 80)) + ")";
    }
    let line = 1;
    let column = 1;
    for (let i = 0; i < ours.length; i++) {
        if (ours[i].ch === "\n") {
            line += 1;
            column = 1;
            continue;
        }
        if (ours[i].over !== theirs[i].over) {
            return name + ":" + line + ":" + column + ": " + JSON.stringify(ours[i].ch) +
                " is [" + ours[i].over + "] here, [" + theirs[i].over + "] in Zola's block";
        }
        column += 1;
    }
    return null;
}

const failures = [];
let checked = 0;

for (const lang of Object.keys(LANGUAGES)) {
    const list = samples(lang);
    const built = zola(list, lang);
    if (built.length !== list.length) {
        failures.push("Zola made " + built.length + " " + lang + " blocks of " + list.length + " samples");
        continue;
    }
    list.forEach((s, i) => {
        const problem = compare(s.name, s.text, built[i], lang);
        if (problem !== null) {
            failures.push(problem);
        }
        checked += 1;
    });
}

// Every language the painter colors has its samples here.
for (const lang of ronPaint.languages) {
    if (!Object.hasOwn(LANGUAGES, lang)) {
        failures.push("ron-paint.js colors " + lang + ", which this test has no samples of");
    }
}

// A built site's `ron` and `dlg` blocks: each one's own text.
const out = process.argv[2];
if (out !== undefined) {
    for (const page of files(out, (name) => name.endsWith(".html"))) {
        const html = readFileSync(page, "utf8");
        for (const lang of Object.keys(LANGUAGES)) {
            blocks(html, lang).forEach((block, i) => {
                const text = zolaChars(block, lang).map((c) => c.ch).join("");
                const problem = compare(relative(out, page) + " " + lang + " block " + (i + 1), text, block, lang);
                if (problem !== null) {
                    failures.push(problem);
                }
                checked += 1;
            });
        }
    }
}

if (failures.length > 0) {
    for (const f of failures) {
        console.error("ron-paint: " + f);
    }
    process.exit(1);
}
console.log("ron-paint: " + checked + " blocks colored as Zola colors them");
