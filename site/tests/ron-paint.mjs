// The playground's RON colors match the book's (ticket 2832): for each
// sample, static/ron-paint.js must give every character the scopes Zola
// gives it in a `ron` code block. Plain Node, no packages; CI's `site` job
// runs it with `zola` on PATH (or ZOLA naming it):
//
//     node site/tests/ron-paint.mjs [<a built site>]
//
// The samples are Hello, Grid's `.ron` files, the book's project's, and the
// cases below, built by Zola into a small site of their own with the
// website's grammars and settings. With a built site, every `ron` block of
// its pages is checked too.
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

// What a file the editors hold may have, that no sample file has.
const CASES = [
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

function samples() {
    const ron = (name) => name.endsWith(".ron");
    const paths = files(join(repo, "examples", "hello-grid", "files"), ron)
        .concat(files(join(site, "tutorial"), ron));
    return paths.map((path) => ({ name: relative(repo, path), text: readFileSync(path, "utf8").replace(/\r\n/g, "\n") }))
        .concat(CASES.map((text, i) => ({ name: "case " + (i + 1), text })));
}

// Zola's HTML of each sample, in order: a site of its own with the
// website's grammars and highlighting, one page holding them as `ron`
// blocks.
function zola(list) {
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
        writeFileSync(join(dir, "content", "_index.md"), "+++\n+++\n" + list.map((s) => fence + "ron\n" + s.text + fence + "\n").join("\n"));
        execFileSync(process.env.ZOLA || "zola", ["build", "--output-dir", "public", "--force"], { cwd: dir, stdio: ["ignore", "ignore", "inherit"] });
        return blocks(readFileSync(join(dir, "public", "index.html"), "utf8"));
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
}

// Every `ron` block's inner HTML in a page.
function blocks(html) {
    const found = [];
    const block = /<pre data-lang="ron"[^>]*><code[^>]*>([\s\S]*?)<\/code><\/pre>/g;
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
// first, `|` between spans. Zola wraps each line in `z-source z-ron`,
// which colors nothing: it is left out.
function zolaChars(html) {
    const chars = [];
    const stack = [];
    const part = /<span class="([^"]*)">|<\/span>|([^<]+)/g;
    let m;
    while ((m = part.exec(html)) !== null) {
        if (m[1] !== undefined) {
            stack.push(m[1]);
        } else if (m[2] !== undefined) {
            const over = stack.filter((c) => c !== "z-source z-ron").join("|");
            for (const ch of unescape(m[2])) {
                chars.push({ ch, over });
            }
        } else {
            stack.pop();
        }
    }
    return chars;
}

function ourChars(text) {
    const chars = [];
    for (const run of ronPaint.tokenize(text)) {
        const over = run.scopes.map(ronPaint.classes).join("|");
        for (const ch of run.text) {
            chars.push({ ch, over });
        }
    }
    return chars;
}

// The first difference between the two, or null. Newlines are left out:
// they color nothing, and Zola closes its spans at each line's end.
function compare(name, text, html) {
    const theirs = zolaChars(html);
    const ours = ourChars(text);
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

const list = samples();
const built = zola(list);
if (built.length !== list.length) {
    failures.push("Zola made " + built.length + " ron blocks of " + list.length + " samples");
} else {
    list.forEach((s, i) => {
        const problem = compare(s.name, s.text, built[i]);
        if (problem !== null) {
            failures.push(problem);
        }
        checked += 1;
    });
}

// A built site's `ron` blocks: each one's own text.
const out = process.argv[2];
if (out !== undefined) {
    for (const page of files(out, (name) => name.endsWith(".html"))) {
        blocks(readFileSync(page, "utf8")).forEach((html, i) => {
            const text = zolaChars(html).map((c) => c.ch).join("");
            const problem = compare(relative(out, page) + " block " + (i + 1), text, html);
            if (problem !== null) {
                failures.push(problem);
            }
            checked += 1;
        });
    }
}

if (failures.length > 0) {
    for (const f of failures) {
        console.error("ron-paint: " + f);
    }
    process.exit(1);
}
console.log("ron-paint: " + checked + " ron blocks colored as Zola colors them");
