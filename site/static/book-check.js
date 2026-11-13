// The book's quick checks (ticket 2827, ADR-0090 §8): each check's answers
// become buttons, and choosing one says in the check's `role="status"`
// line whether it is right, and why.
//
// The shortcode (`templates/shortcodes/check.html`) writes the buttons
// hidden and the answers with why in a `<details>`, which is all a reader
// with no JavaScript gets. This file shows the buttons, hides the
// `<details>`, and copies why from it. Nothing is kept or sent anywhere.
//
// The buttons are shuffled at each load (ticket 2833), so the right one's
// place changes from reader to reader; each keeps its own why. The
// `<details>` list keeps the written order, which `cargo xtask check-book`
// keeps spread.
(function () {
  "use strict";

  function choose(buttons, button, why, result) {
    const right = button.dataset.right === "true";
    for (const other of buttons) {
      other.setAttribute("aria-pressed", String(other === button));
      other.classList.remove("is-right", "is-wrong");
    }
    button.classList.add(right ? "is-right" : "is-wrong");
    const verdict = document.createElement("strong");
    verdict.textContent = right ? "Right." : "Not quite.";
    const said = [verdict, " "];
    if (why) {
      // Why, without the dash that comes before it in the list.
      const copy = why.cloneNode(true);
      const first = copy.firstChild;
      if (first && first.nodeType === Node.TEXT_NODE) {
        first.textContent = first.textContent.replace(/^—\s*/, "");
      }
      said.push(...copy.childNodes);
    }
    result.classList.toggle("is-right", right);
    result.classList.toggle("is-wrong", !right);
    result.replaceChildren(...said);
  }

  // Fisher–Yates, in place. `Math.random` is enough: this orders a page's
  // buttons, not a simulation.
  function shuffle(items) {
    for (let i = items.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [items[i], items[j]] = [items[j], items[i]];
    }
  }

  function start() {
    for (const check of document.querySelectorAll(".check")) {
      const choices = check.querySelector(".check-choices");
      const list = check.querySelector(".check-answers");
      const result = check.querySelector(".check-result");
      if (!choices || !list || !result) continue;
      const whys = Array.from(list.querySelectorAll("li"), (li) => li.querySelector(".check-why"));
      const pairs = Array.from(choices.querySelectorAll(".check-choice"), (button, i) => ({
        button,
        why: whys[i],
      }));
      shuffle(pairs);
      for (const pair of pairs) choices.insertBefore(pair.button, result);
      const buttons = pairs.map((pair) => pair.button);
      for (const { button, why } of pairs) {
        button.addEventListener("click", () => choose(buttons, button, why, result));
      }
      choices.hidden = false;
      list.hidden = true;
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start);
  } else {
    start();
  }
})();
