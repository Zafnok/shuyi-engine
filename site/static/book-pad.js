// A step's page served from the book's Codespace (ticket 2840,
// `cargo xtask book-pad`, crates/xtask/src/book_pad.rs): the page follows
// the step's builds there. Plain JavaScript with no build step, loading
// nothing from another host.
//
// It asks `<base>pad/status` once. The public site has no such file, so
// there the answer is a 404 and nothing changes. In the Codespace it shows
// the line saying so in place of the Codespace's button, and polls
// `<base>pad/build/<step>` each second while the page is shown: *Building…*
// beside **Play this step** while a build runs, the playground's frame
// loaded again on each new good build, and a failed build's output where
// the playground shows its Problems (under the line, on a step with no
// playground), the frame keeping the last good build.
(function () {
    "use strict";

    var EVERY = 1000;
    var FAILED = "This step's last build failed, so its game is the build before it. Cargo said:\n\n";

    function start() {
        var holder = document.querySelector("[data-pad]");
        if (holder === null || typeof window.fetch !== "function") {
            return;
        }
        // The pad's address at the site's base, `…/pad`, with its last `/`.
        var base = holder.dataset.pad.replace(/\/?$/, "/");
        var step = holder.dataset.step;
        window.fetch(base + "status", { cache: "no-store" }).then(function (response) {
            return response.ok ? response.json() : null;
        }).then(function (status) {
            if (status !== null && typeof status === "object" && status.pad === true) {
                follow(base, step, holder);
            }
        }, function () {
            // Not the Codespace: nothing to follow.
        });
    }

    function follow(base, step, holder) {
        var line = holder.querySelector(".pad-state");
        var output = document.querySelector(".pad-output");
        Array.prototype.slice.call(document.querySelectorAll(".pad-away")).forEach(function (away) {
            away.hidden = true;
        });
        Array.prototype.slice.call(document.querySelectorAll(".pad-here")).forEach(function (here) {
            here.hidden = false;
        });

        // The good build the frame plays, once read; and the last answer
        // shown, so an unchanged one changes nothing.
        var seen = null;
        var shown = "";

        function problems() {
            return document.querySelector(".playground-problems");
        }

        // The frame, loaded again, if it has loaded at all: a folded
        // playground's loads the newest build once it is opened.
        function reload() {
            var frame = document.querySelector(".playground iframe");
            if (frame === null) {
                return;
            }
            try {
                if (frame.contentWindow.location.href === "about:blank") {
                    return;
                }
                frame.contentWindow.location.reload();
            } catch (e) {
                frame.src = frame.getAttribute("src");
            }
        }

        // `text` in the playground's Problems, or in the line's own box.
        function tell(text) {
            var box = problems();
            if (box !== null) {
                box.textContent = text;
                if (output !== null) {
                    output.hidden = true;
                }
            } else if (output !== null) {
                output.textContent = text;
                output.hidden = text === "";
            }
        }

        // Empties what `tell` said of a failed build, and nothing else.
        function untell() {
            var box = problems();
            if (box !== null && box.textContent.indexOf(FAILED) === 0) {
                box.textContent = "";
            }
            if (output !== null) {
                output.textContent = "";
                output.hidden = true;
            }
        }

        function show(build) {
            var key = build.state + ":" + build.number + ":" + build.good;
            if (key === shown) {
                return;
            }
            shown = key;
            if (build.state === "building") {
                line.textContent = "Building…";
            } else if (build.state === "failed") {
                line.textContent = problems() !== null ?
                    "The last build failed: Problems, in the playground below, says why." :
                    "The last build failed: what cargo said is below.";
                tell(FAILED + build.output);
            } else if (build.state === "none") {
                line.textContent = "Not built yet: the Codespace is still building the steps.";
            } else {
                line.textContent = "";
                untell();
            }
            if (seen === null) {
                seen = build.good;
            } else if (build.good > seen && build.state !== "failed") {
                seen = build.good;
                reload();
            }
        }

        function poll() {
            if (document.hidden) {
                window.setTimeout(poll, EVERY);
                return;
            }
            window.fetch(base + "build/" + encodeURIComponent(step), { cache: "no-store" }).then(function (response) {
                return response.ok ? response.json() : null;
            }).then(function (build) {
                if (build !== null && typeof build === "object" && typeof build.state === "string") {
                    show(build);
                }
            }, function () {
                // The server stopped, for now: ask again.
            }).then(function () {
                window.setTimeout(poll, EVERY);
            });
        }
        poll();
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", start);
    } else {
        start();
    }
})();
