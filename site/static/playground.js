// The playground (tickets 2814 and 2820, site/templates/playground-macros.html):
// an editor for each of a game's files beside the game in a frame, and the
// buttons of the tries (`try_it`) that make their edit in it. Plain
// JavaScript with no build step, loading nothing from another host. One
// playground a page.
//
// The game's web build reads each `sessionStorage` key
// `shuyi-page-file:files/<name>` as a file laid over its own, and sends its
// problems to this page with `postMessage({ kind: "shuyi-report", text })`,
// "" when it loaded (crates/cli/web/page-files.js, ticket 3069). The frame
// is this page's origin, so both share the tab's `sessionStorage`.
//
// Every page of the site is one origin, and so shares those keys with every
// other playground, and with a game played on its own page in the same tab.
// So each page keeps its files under keys of its own, by its path, hands
// them to the game's keys as it loads, before its frame does, and takes
// them back as it goes: the game's keys hold this page's files only while
// this page is open.
(function () {
    "use strict";

    // The game's key of a file; this page's key of a file it ran; and its
    // key of what the reader has typed in its editor, run or not, so a
    // reload of the page keeps it.
    var GAME = "shuyi-page-file:";
    var GAME_FILES = GAME + "files/";
    var PAGE = window.location.pathname;
    var RUN = "shuyi-playground-run:" + PAGE + ":files/";
    var DRAFT = "shuyi-playground-draft:" + PAGE + ":files/";

    var LOADED = "Loaded with your files.";

    function storage() {
        try {
            return window.sessionStorage;
        } catch (e) {
            return null;
        }
    }

    function get(key) {
        var s = storage();
        try {
            return s === null ? null : s.getItem(key);
        } catch (e) {
            return null;
        }
    }

    // Writes or (with `null`) removes `key`; whether it worked. Storage can
    // be turned off, or full.
    function put(key, value) {
        var s = storage();
        if (s === null) {
            return false;
        }
        try {
            if (value === null) {
                s.removeItem(key);
            } else {
                s.setItem(key, value);
            }
            return true;
        } catch (e) {
            return false;
        }
    }

    // Every key of the tab's storage starting with `prefix`.
    function keys(prefix) {
        var s = storage();
        var found = [];
        if (s === null) {
            return found;
        }
        try {
            for (var i = 0; i < s.length; i++) {
                var key = s.key(i);
                if (key !== null && key.indexOf(prefix) === 0) {
                    found.push(key);
                }
            }
        } catch (e) {
            return found;
        }
        return found;
    }

    // Empties the game's keys, then, unless `away`, fills them with the
    // files this page ran.
    function hand_over(away) {
        keys(GAME).forEach(function (key) {
            put(key, null);
        });
        if (away) {
            return;
        }
        keys(RUN).forEach(function (key) {
            put(GAME_FILES + key.slice(RUN.length), get(key));
        });
    }

    // Now, in the page's head, before its frame is there; again when the
    // page comes back from the browser's cache; and away as it goes, so a
    // game played on its own page plays its own files.
    hand_over(false);
    window.addEventListener("pageshow", function (event) {
        if (event.persisted) {
            hand_over(false);
        }
    });
    window.addEventListener("pagehide", function () {
        hand_over(true);
    });

    // The game's problems: only from the frame, at this page's origin. The
    // listener is there before the frame loads, since a game in the cache
    // can report before the page is ready; the last report waits for it.
    var show = null;
    var waiting = null;

    window.addEventListener("message", function (event) {
        var frame = document.querySelector(".playground iframe");
        if (event.origin !== window.location.origin || frame === null || event.source !== frame.contentWindow) {
            return;
        }
        var data = event.data;
        if (data === null || typeof data !== "object" || data.kind !== "shuyi-report" || typeof data.text !== "string") {
            return;
        }
        var text = data.text === "" ? LOADED : data.text;
        if (show === null) {
            waiting = text;
        } else {
            show(text);
        }
    });

    function start() {
        var root = document.querySelector(".playground");
        if (root === null) {
            return;
        }
        var frame = root.querySelector("iframe");
        var editors = root.querySelector(".playground-editors");
        var tabs = Array.prototype.slice.call(root.querySelectorAll('[role="tab"]'));
        var areas = Array.prototype.slice.call(root.querySelectorAll("textarea[data-file]"));
        var problems = root.querySelector(".playground-problems");
        var buttons = Array.prototype.slice.call(root.querySelectorAll(".playground-actions button"));
        // The game's own text of each file, by name, once fetched.
        var own = {};

        editors.hidden = false;

        function say(text) {
            problems.textContent = text;
        }
        show = say;
        if (waiting !== null) {
            say(waiting);
        }

        // The game takes the focus as it starts: give it back, to the page as
        // it loads and to the button pressed after a run, so the frame has
        // it only when the reader clicks it.
        var back = document.body;
        function give_back() {
            if (back !== null && document.activeElement === frame) {
                if (back === document.body) {
                    frame.blur();
                } else {
                    back.focus();
                }
            }
        }
        give_back();
        frame.addEventListener("load", function () {
            give_back();
            back = null;
        });

        // A folded playground (a step's) loads its game only once it is
        // opened: from then on, whether or not the browser sees the frame.
        var fold = root.closest("details");
        function unfold() {
            fold.open = true;
            frame.loading = "eager";
        }
        if (fold !== null) {
            fold.addEventListener("toggle", function () {
                if (fold.open) {
                    unfold();
                }
            });
            if (fold.open) {
                unfold();
            }
        }

        // Whether the frame holds its game yet, not the blank page a frame
        // starts on.
        function started() {
            try {
                return frame.contentWindow.location.href !== "about:blank";
            } catch (e) {
                return true;
            }
        }

        // The tabs: a click or the arrow keys, Home and End select one.
        function select(index, focus) {
            tabs.forEach(function (tab, i) {
                var on = i === index;
                tab.setAttribute("aria-selected", on ? "true" : "false");
                tab.tabIndex = on ? 0 : -1;
                document.getElementById(tab.getAttribute("aria-controls")).hidden = !on;
            });
            if (focus) {
                tabs[index].focus();
            }
        }

        function selected() {
            for (var i = 0; i < tabs.length; i++) {
                if (tabs[i].getAttribute("aria-selected") === "true") {
                    return i;
                }
            }
            return 0;
        }

        tabs.forEach(function (tab, i) {
            tab.addEventListener("click", function () {
                select(i, false);
            });
            tab.addEventListener("keydown", function (event) {
                var to = null;
                if (event.key === "ArrowRight" || event.key === "ArrowDown") {
                    to = (i + 1) % tabs.length;
                } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
                    to = (i + tabs.length - 1) % tabs.length;
                } else if (event.key === "Home") {
                    to = 0;
                } else if (event.key === "End") {
                    to = tabs.length - 1;
                }
                if (to !== null) {
                    event.preventDefault();
                    select(to, true);
                }
            });
        });

        // Run: each file whose text differs from the game's goes to the
        // game's key and this page's, the rest are removed, and the frame
        // loads again.
        function run() {
            back = document.activeElement;
            var kept = true;
            areas.forEach(function (area) {
                var name = area.dataset.file;
                var changed = area.value !== own[name];
                kept = put(GAME_FILES + name, changed ? area.value : null) && kept;
                put(RUN + name, changed ? area.value : null);
                put(DRAFT + name, changed ? area.value : null);
            });
            say(kept ? "" : "Your browser wouldn't keep the files (session storage is off or full), so the game plays its own.");
            if (!started()) {
                // Not loaded yet: it reads the files as it does.
                return;
            }
            try {
                frame.contentWindow.location.reload();
            } catch (e) {
                // The frame went to another origin: load the game again.
                frame.src = frame.getAttribute("src");
            }
        }

        function reset(area) {
            area.value = own[area.dataset.file];
        }

        buttons.forEach(function (button) {
            button.disabled = true;
        });
        root.querySelector(".playground-run").addEventListener("click", run);
        root.querySelector(".playground-reset").addEventListener("click", function () {
            reset(areas[selected()]);
            run();
        });
        root.querySelector(".playground-reset-all").addEventListener("click", function () {
            areas.forEach(reset);
            run();
        });

        // Keep what the reader types, so a reload of the page shows it.
        areas.forEach(function (area) {
            area.addEventListener("input", function () {
                var name = area.dataset.file;
                put(DRAFT + name, area.value === own[name] ? null : area.value);
            });
        });

        // The game's own text, then each editor's: the reader's if kept.
        var base = root.dataset.files;
        Promise.all(areas.map(function (area) {
            var name = area.dataset.file;
            var path = name.split("/").map(encodeURIComponent).join("/");
            return fetch(base + path, { cache: "no-cache" }).then(function (response) {
                if (!response.ok) {
                    throw new Error(name + ": " + response.status);
                }
                return response.text();
            }).then(function (text) {
                own[name] = text;
                var draft = get(DRAFT + name);
                var ran = get(RUN + name);
                area.value = draft !== null ? draft : ran !== null ? ran : text;
            });
        })).then(function () {
            buttons.forEach(function (button) {
                button.disabled = false;
            });
            tries();
        }, function (error) {
            say("The game's files didn't load (" + error.message + "), so there is nothing to edit. " +
                "On a local build, run copy-playground.sh first (site/README.md).");
        });

        // A try's button (the `try_it` shortcode, ticket 2820): each of its
        // edits made to the game's own text of its file, the playground
        // shown on the first file it edits, and run. Shown once the game's
        // text is here, and only when the playground holds every file the
        // try edits.
        function tries() {
            Array.prototype.slice.call(document.querySelectorAll(".try-here")).forEach(function (holder) {
                var button = holder.querySelector("button");
                var edits;
                try {
                    edits = JSON.parse(button.dataset.edits);
                } catch (e) {
                    return;
                }
                // Each edit's file in its step's project, named in the
                // playground by its path after the first `files/`.
                var made = edits.map(function (edit) {
                    var at = edit.file.indexOf("files/");
                    var name = at < 0 ? null : edit.file.slice(at + "files/".length);
                    var area = areas.filter(function (a) {
                        return a.dataset.file === name;
                    })[0];
                    return { area: area, find: edit.find, replace: edit.replace };
                });
                var held = made.every(function (m) {
                    return m.area !== undefined;
                });
                if (made.length === 0 || !held) {
                    return;
                }
                holder.hidden = false;
                button.addEventListener("click", function () {
                    var missed = [];
                    made.forEach(function (m) {
                        m.area.value = own[m.area.dataset.file];
                    });
                    made.forEach(function (m) {
                        var text = m.area.value;
                        var at = text.indexOf(m.find);
                        if (at < 0) {
                            missed.push("files/" + m.area.dataset.file);
                            return;
                        }
                        m.area.value = text.slice(0, at) + m.replace + text.slice(at + m.find.length);
                    });
                    select(areas.indexOf(made[0].area), false);
                    run();
                    if (fold !== null) {
                        unfold();
                    }
                    if (missed.length > 0) {
                        say("This try's change isn't in " + missed.join(", ") + " as this page has it, so it wasn't made there.");
                    }
                    root.scrollIntoView({ block: "start" });
                    made[0].area.focus({ preventScroll: true });
                    // The game takes the focus as it starts: it goes back to
                    // the editor, not to the button far above.
                    back = made[0].area;
                });
            });
        }
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", start);
    } else {
        start();
    }
})();
