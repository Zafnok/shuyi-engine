// The book's quick checks (ticket 2827, ADR-0090 §8): each check's answers
// become buttons, and choosing one says in the check's `role="status"`
// line whether it is right, and why.
//
// The shortcode (`templates/shortcodes/check.html`) writes the buttons
// hidden and the answers with why in a `<details>`, which is all a reader
// with no JavaScript gets. This file shows the buttons, hides the
// `<details>`, and copies why from it. Nothing is kept or sent anywhere.
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

  function start() {
    for (const check of document.querySelectorAll(".check")) {
      const choices = check.querySelector(".check-choices");
      const list = check.querySelector(".check-answers");
      const result = check.querySelector(".check-result");
      if (!choices || !list || !result) continue;
      const whys = Array.from(list.querySelectorAll("li"), (li) => li.querySelector(".check-why"));
      const buttons = Array.from(choices.querySelectorAll(".check-choice"));
      buttons.forEach((button, i) => {
        button.addEventListener("click", () => choose(buttons, button, whys[i], result));
      });
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
