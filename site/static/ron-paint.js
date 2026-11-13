// The playground's editors' colors (ticket 2832): RON cut into tokens by
// the rules of site/syntaxes/ron.sublime-syntax, the grammar Zola colors a
// `ron` code block with, each token given the classes Zola writes for its
// scope (`comment.line.ron` → `z-comment z-line z-ron`), so
// static/style.css colors an editor as it colors a code block. Plain
// JavaScript with no build step: the page loads it before playground.js,
// and site/tests/ron-paint.mjs checks it against Zola's own output.
(function (root) {
    "use strict";

    // The grammar's contexts, their rules in its order. A rule has a
    // `scope`, or `push`es a context, or `pop`s back. `meta` scopes all a
    // context holds, the match that pushes it and the one that pops it too;
    // `content` scopes only what is between them.
    var CONTEXTS = {
        main: {
            rules: [
                { re: /^---\s*$/g, scope: "punctuation.separator.ron", push: "rows" },
                { re: /\/\/.*$/g, scope: "comment.line.ron" },
                { re: /\/\*/g, push: "block-comment" },
                { re: /"(?:[^"\\]|\\.)*"(?=\s*:)/g, scope: "entity.name.tag.ron" },
                { re: /"/g, push: "string" },
                { re: /'(?:[^'\\]|\\.)[^']*'/g, scope: "string.quoted.single.ron" },
                { re: /\b(?:true|false)\b/g, scope: "constant.language.ron" },
                {
                    re: /-?\b(?:0x[0-9a-fA-F_]+|0o[0-7_]+|0b[01_]+|[0-9][0-9_]*(?:\.[0-9_]+)?(?:[eE][+-]?[0-9]+)?)\b/g,
                    scope: "constant.numeric.ron"
                },
                { re: /\b[a-z_][A-Za-z0-9_]*(?=\s*:)/g, scope: "entity.name.tag.ron" },
                { re: /\b[A-Z][A-Za-z0-9_]*\b/g, scope: "entity.name.type.ron" }
            ]
        },
        "block-comment": {
            meta: "comment.block.ron",
            rules: [
                { re: /\/\*/g, push: "block-comment" },
                { re: /\*\//g, pop: true }
            ]
        },
        string: {
            meta: "string.quoted.double.ron",
            rules: [
                { re: /\\./g, scope: "constant.character.escape.ron" },
                { re: /"/g, pop: true }
            ]
        },
        // A `.map` file's rows after its `---` line: plain to the end.
        rows: { content: "text.plain.ron", rules: [] }
    };

    // The scopes over text in the contexts `stack` (bottom first): each
    // one's meta scope, and its content scope unless `edge` (the match
    // that pushes or pops the top one).
    function scopes(stack, edge) {
        var out = [];
        stack.forEach(function (context, i) {
            if (context.meta) {
                out.push(context.meta);
            }
            if (context.content && !(edge && i === stack.length - 1)) {
                out.push(context.content);
            }
        });
        return out;
    }

    // The text cut into runs, `{ text, scopes }`, `scopes` the stack of
    // scopes over it, outermost first ([] for plain text). As the grammar
    // reads it: line by line, the earliest match of the top context's rules
    // at each point, the first rule winning a tie, and what no rule matches
    // left plain. Every character is in exactly one run.
    function tokenize(text) {
        var runs = [];
        var stack = [CONTEXTS.main];

        function emit(part, over) {
            if (part === "") {
                return;
            }
            var last = runs[runs.length - 1];
            if (last !== undefined && last.scopes.join(" ") === over.join(" ")) {
                last.text += part;
            } else {
                runs.push({ text: part, scopes: over });
            }
        }

        text.split("\n").forEach(function (line, n) {
            if (n > 0) {
                emit("\n", scopes(stack, false));
            }
            var at = 0;
            while (at <= line.length) {
                var rules = stack[stack.length - 1].rules;
                var best = null;
                var rule = null;
                for (var i = 0; i < rules.length; i++) {
                    rules[i].re.lastIndex = at;
                    var found = rules[i].re.exec(line);
                    if (found !== null && (best === null || found.index < best.index)) {
                        best = found;
                        rule = rules[i];
                    }
                }
                if (best === null) {
                    emit(line.slice(at), scopes(stack, false));
                    break;
                }
                emit(line.slice(at, best.index), scopes(stack, false));
                if (rule.pop) {
                    emit(best[0], scopes(stack, true));
                    stack = stack.slice(0, -1);
                } else if (rule.push) {
                    var under = rule.scope ? scopes(stack, false).concat([rule.scope]) : null;
                    stack = stack.concat([CONTEXTS[rule.push]]);
                    emit(best[0], under !== null ? under : scopes(stack, true));
                } else {
                    emit(best[0], scopes(stack, false).concat([rule.scope]));
                }
                // No rule matches nothing, but never loop on one that does.
                at = best.index + Math.max(best[0].length, 1);
                if (best[0].length === 0) {
                    emit(line.charAt(best.index), scopes(stack, false));
                }
            }
        });
        return runs;
    }

    function escape(text) {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }

    // A scope's classes as Zola writes them: each dotted part, `z-` first.
    function classes(scope) {
        return scope.split(".").map(function (part) {
            return "z-" + part;
        }).join(" ");
    }

    // The text as HTML: colored spans if `lang` is "ron", else only escaped.
    function paint(text, lang) {
        if (lang !== "ron") {
            return escape(text);
        }
        return tokenize(text).map(function (run) {
            var open = run.scopes.map(function (scope) {
                return '<span class="' + classes(scope) + '">';
            }).join("");
            return open + escape(run.text) + "</span>".repeat(run.scopes.length);
        }).join("");
    }

    var api = { tokenize: tokenize, paint: paint, classes: classes };
    if (typeof module === "object" && module.exports) {
        module.exports = api;
    } else {
        root.shuyiRonPaint = api;
    }
})(this);
