// The playground (ticket 2814, site/templates/playground.html): an editor
// for each of Hello, Grid's files beside the game in a frame. Plain
// JavaScript with no build step, loading nothing from another host.
//
// The game's web build reads each `sessionStorage` key
// `shuyi-page-file:files/<name>` as a file laid over its own, and sends its
// problems to this page with `postMessage({ kind: "shuyi-report", text })`,
// "" when it loaded (crates/cli/web/page-files.js, ticket 3069). The frame
// is this page's origin, so both share the tab's `sessionStorage`.
(function () {
    "use strict";

    // The game's key of a file, and this page's key of what the reader has
    // typed in its editor, run or not, so a reload of the page keeps it.
    var GAME = "shuyi-page-file:files/";
    var DRAFT = "shuyi-playground-draft:files/";

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
        // game's key, the rest are removed, and the frame loads again.
        function run() {
            back = document.activeElement;
            var kept = true;
            areas.forEach(function (area) {
                var name = area.dataset.file;
                var changed = area.value !== own[name];
                kept = put(GAME + name, changed ? area.value : null) && kept;
                put(DRAFT + name, changed ? area.value : null);
            });
            say(kept ? "" : "Your browser wouldn't keep the files (session storage is off or full), so the game plays its own.");
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
            return fetch(base + encodeURIComponent(name), { cache: "no-cache" }).then(function (response) {
                if (!response.ok) {
                    throw new Error(name + ": " + response.status);
                }
                return response.text();
            }).then(function (text) {
                own[name] = text;
                var draft = get(DRAFT + name);
                var ran = get(GAME + name);
                area.value = draft !== null ? draft : ran !== null ? ran : text;
            });
        })).then(function () {
            buttons.forEach(function (button) {
                button.disabled = false;
            });
        }, function (error) {
            say("The game's files didn't load (" + error.message + "), so there is nothing to edit. " +
                "On a local build, run copy-playground.sh first (site/README.md).");
        });
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", start);
    } else {
        start();
    }
})();
