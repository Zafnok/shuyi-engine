// The reader's filter (ticket 2826, ADR-0090 §6 and ADR-0098): leaves out
// the passages of the book a reader doesn't need, by what they said they
// know.
//
// - On a first visit to any page of the book, the popup (`#book-welcome`)
//   asks three questions, one at a time, with a skip.
// - The answers stay in this browser, in `localStorage` under `shuyi-book`:
//   `{"v": 1, "knows": [...], "engine": true}`. The key being there means
//   answered or skipped; a skip writes `"knows": []`.
// - The bar at the top of each page changes them, says how many passages
//   it leaves out, and shows them all for this visit.
// - A passage the answers leave out leaves nothing behind. Every passage
//   shown has "hide", which folds it to a stub, a button that shows it.
// - A page whose page-level mark the reader knows is passed over by the
//   pager and tagged in the contents.
//
// The templates (`book-macros.html`) hold the bar, the popup and the
// book's order as JSON; this file holds no text but what it counts. It is
// loaded in the `<head>`, so it marks `<html>` before the page is drawn,
// and `style.css` folds by those marks. With no JavaScript nothing folds.
(function () {
  "use strict";

  // The four audiences a passage may be marked for (ADR-0090 §4).
  // `site/check-build.sh` fails a mark of any other.
  const AUDIENCES = ["programming", "games", "rust", "shuyi"];
  const KEY = "shuyi-book";
  const root = document.documentElement;

  // What the reader said, `{ knows, engine }`, or null until they answer or
  // skip. When storage throws it lives for this page only.
  let answers = read();
  // Show all, for this visit only.
  let showAll = false;

  function read() {
    try {
      const data = JSON.parse(window.localStorage.getItem(KEY));
      if (!data || data.v !== 1 || !Array.isArray(data.knows)) return null;
      return {
        knows: AUDIENCES.filter((a) => data.knows.includes(a)),
        engine: data.engine === true,
      };
    } catch (e) {
      return null;
    }
  }

  function write() {
    try {
      window.localStorage.setItem(
        KEY,
        JSON.stringify({ v: 1, knows: answers.knows, engine: answers.engine })
      );
    } catch (e) {
      // Storage is blocked: the answers hold for this page.
    }
  }

  function forget() {
    try {
      window.localStorage.removeItem(KEY);
    } catch (e) {
      // Storage is blocked: there was nothing kept.
    }
  }

  function knows(audience) {
    return answers !== null && answers.knows.includes(audience);
  }

  // `<html data-book-fold="rust shuyi">` folds those audiences' passages.
  function fold() {
    root.setAttribute(
      "data-book-fold",
      showAll || answers === null ? "" : answers.knows.join(" ")
    );
  }

  root.setAttribute("data-book", "");
  fold();

  // The page's path, as the book's order names it.
  function pathOf(url) {
    return new URL(url, window.location.href).pathname.replace(/index\.html$/, "");
  }

  let order = [];
  let here = -1;
  let enginePath = "";
  let dialog = null;

  // A page of the order whose page-level mark the reader knows.
  function skipped(entry) {
    return entry.newcomer !== null && knows(entry.newcomer);
  }

  function element(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text) node.textContent = text;
    return node;
  }

  function link(entry, rel) {
    const a = element("a", "", "");
    a.href = entry.url;
    if (rel) a.rel = rel;
    a.textContent = (rel === "prev" ? "← " : "") + entry.title + (rel === "next" ? " →" : "");
    return a;
  }

  function folded(aside) {
    return window.getComputedStyle(aside).display === "none";
  }

  // Each passage's stub, shown once the reader hides it: its label and
  // topic, and "show".
  function stubs() {
    let n = 0;
    for (const aside of document.querySelectorAll("aside.newcomer")) {
      if (!aside.id) aside.id = "book-note-" + ++n;
      const stub = element("button", "newcomer-stub", "");
      stub.type = "button";
      stub.dataset.newcomer = aside.dataset.newcomer;
      stub.setAttribute("aria-controls", aside.id);
      stub.setAttribute("aria-expanded", "false");
      const label = aside.querySelector(".newcomer-label");
      if (label) {
        for (const node of label.childNodes) stub.appendChild(node.cloneNode(true));
        // A button holds no link: a topic's link becomes its text.
        for (const a of stub.querySelectorAll("a")) a.replaceWith(...a.childNodes);
      }
      stub.append(" · ");
      stub.appendChild(element("span", "newcomer-stub-show", "show"));
      stub.addEventListener("click", () => open(aside, true));
      aside.before(stub);
      // And on the passage's label, "hide", which folds it to its stub.
      if (label) {
        const hide = element("button", "newcomer-hide", "hide");
        hide.type = "button";
        hide.setAttribute("aria-controls", aside.id);
        hide.addEventListener("click", () => shut(aside));
        const wrap = element("span", "newcomer-hide-wrap", " · ");
        wrap.appendChild(hide);
        label.appendChild(wrap);
      }
    }
  }

  // A passage's own state, which wins over the answers: `is-open` shows a
  // passage they leave out, `is-shut` hides one they show.
  function mark(aside, open) {
    const stub = aside.previousElementSibling;
    for (const node of [aside, stub]) {
      node.classList.toggle("is-open", open);
      node.classList.toggle("is-shut", !open);
    }
    stub.setAttribute("aria-expanded", String(open));
  }

  function unmark(aside) {
    const stub = aside.previousElementSibling;
    for (const node of [aside, stub]) node.classList.remove("is-open", "is-shut");
    stub.setAttribute("aria-expanded", "false");
  }

  function open(aside, focus) {
    mark(aside, true);
    if (focus) {
      aside.tabIndex = -1;
      aside.focus();
    }
    apply();
  }

  function shut(aside) {
    mark(aside, false);
    aside.previousElementSibling.focus();
    apply();
  }

  // A link to an anchor inside a passage left out or hidden opens the
  // passage.
  function openTarget() {
    let id = "";
    try {
      id = decodeURIComponent(window.location.hash.slice(1));
    } catch (e) {
      return;
    }
    const target = id && document.getElementById(id);
    const aside = target && target.closest("aside.newcomer");
    if (aside && folded(aside)) {
      open(aside, false);
      target.scrollIntoView();
    }
  }

  // The bar: the toggles, how many passages are hidden, and Show all.
  function bar() {
    for (const toggle of document.querySelectorAll(".book-toggle")) {
      toggle.setAttribute("aria-pressed", String(!knows(toggle.dataset.audience)));
    }
    // The passages the answers leave out, not those the reader hid.
    let hidden = 0;
    for (const aside of document.querySelectorAll("aside.newcomer")) {
      if (folded(aside) && !aside.classList.contains("is-shut")) hidden += 1;
    }
    const count = document.querySelector(".book-bar-count");
    if (count) {
      count.textContent =
        hidden === 0 ? "No notes hidden" : hidden === 1 ? "1 note hidden" : hidden + " notes hidden";
    }
    const all = document.querySelector("[data-book-show-all]");
    if (all) all.hidden = hidden === 0;
  }

  // A page the reader can skip says so at its top, with the next page they
  // need.
  function skipLine() {
    const line = document.querySelector(".book-skip");
    if (!line) return;
    line.hidden = !(here >= 0 && skipped(order[here]));
    if (line.hidden) return;
    const next = order.slice(here + 1).find((entry) => !skipped(entry));
    const span = line.querySelector("[data-book-next]");
    span.hidden = !next;
    if (next) span.querySelector("a").replaceWith(link(next, "next"));
  }

  // The pager follows the reader's path, and names the pages it passes over.
  function pager() {
    const nav = document.querySelector("nav.pager");
    if (!nav || here < 0) return;
    let prev = here - 1;
    const passed = [];
    while (prev >= 0 && skipped(order[prev])) passed.unshift(order[prev--]);
    let next = here + 1;
    while (next < order.length && skipped(order[next])) passed.push(order[next++]);
    const children = [prev >= 0 ? link(order[prev], "prev") : element("span", "", "")];
    if (next < order.length) children.push(link(order[next], "next"));
    if (passed.length > 0) {
      const line = element("p", "pager-skipped", "Skipped: ");
      passed.forEach((entry, i) => {
        if (i > 0) line.append(", ");
        const a = element("a", "", "");
        a.href = entry.url;
        a.appendChild(element("em", "", entry.title));
        line.appendChild(a);
      });
      children.push(line);
    }
    nav.replaceChildren(...children);
  }

  // The contents' tags, once the reader has answered: Start here, You can
  // skip this, and Read this first on appendix A for a reader from another
  // engine.
  function tags() {
    for (const tag of document.querySelectorAll(".book-tag")) tag.remove();
    if (answers === null) return;
    const start = order.find((entry) => !skipped(entry));
    for (const a of document.querySelectorAll(".contents a")) {
      const path = pathOf(a.href);
      const entry = order.find((e) => e.path === path);
      const names = [];
      if (entry && entry === start) names.push("Start here");
      if (entry && skipped(entry)) names.push("You can skip this");
      if (answers.engine && path === enginePath) names.push("Read this first");
      for (const name of names) a.after(element("span", "book-tag", name));
    }
  }

  function apply() {
    fold();
    bar();
    skipLine();
    pager();
    tags();
  }

  // The popup's answers so far, by question.
  let said = {};

  function ask() {
    if (!dialog || typeof dialog.showModal !== "function") return;
    said = {};
    questions();
    dialog.returnFocus = document.activeElement;
    dialog.showModal();
  }

  // Which questions show: "No" to programming passes over Rust, the game
  // question comes after either, and "in another engine" adds its line.
  function questions(asked) {
    const shows = {
      programming: true,
      rust: said.programming === "yes",
      games: said.programming === "no" || said.rust !== undefined,
      engine: said.games === "engine",
    };
    for (const group of dialog.querySelectorAll("[data-question]")) {
      const question = group.dataset.question;
      const was = group.hidden;
      group.hidden = !shows[question];
      for (const button of group.querySelectorAll("[data-answer]")) {
        button.setAttribute("aria-pressed", String(said[question] === button.dataset.answer));
      }
      if (asked && was && !group.hidden) group.querySelector("button").focus();
    }
  }

  function answer(question, value) {
    said[question] = value;
    if (question === "programming" && value === "no") delete said.rust;
    if (said.games === "no" || said.games === "shuyi" || question === "engine") {
      finish();
    } else {
      questions(true);
    }
  }

  function finish() {
    const known = [];
    if (said.programming === "yes") known.push("programming");
    if (said.rust === "yes") known.push("rust");
    if (said.games === "engine" || said.games === "shuyi") known.push("games");
    if (said.games === "shuyi") known.push("shuyi");
    answers = { knows: AUDIENCES.filter((a) => known.includes(a)), engine: said.games === "engine" };
    close();
  }

  // Skip, and Escape: the whole book, nothing hidden.
  function skip() {
    answers = { knows: [], engine: false };
    close();
  }

  // Set while a `close` event of ours is on its way: the event comes after
  // `dialog.close()` returns.
  let closing = false;

  function close() {
    showAll = false;
    write();
    if (dialog.open) {
      closing = true;
      dialog.close();
    }
    apply();
  }

  function start() {
    const data = document.getElementById("book-order");
    if (!data) return;
    const book = JSON.parse(data.textContent);
    order = book.pages.map((entry) => Object.assign({ path: pathOf(entry.url) }, entry));
    enginePath = pathOf(book.engine.url);
    const path = pathOf(window.location.href);
    here = order.findIndex((entry) => entry.path === path);

    stubs();
    for (const toggle of document.querySelectorAll(".book-toggle")) {
      toggle.addEventListener("click", () => {
        const audience = toggle.dataset.audience;
        const known = knows(audience)
          ? answers.knows.filter((a) => a !== audience)
          : (answers ? answers.knows : []).concat([audience]);
        answers = {
          knows: AUDIENCES.filter((a) => known.includes(a)),
          engine: answers !== null && answers.engine,
        };
        showAll = false;
        // New answers start every passage again from them.
        for (const aside of document.querySelectorAll("aside.newcomer")) unmark(aside);
        write();
        apply();
      });
    }
    const all = document.querySelector("[data-book-show-all]");
    if (all) {
      all.addEventListener("click", () => {
        showAll = true;
        for (const aside of document.querySelectorAll("aside.newcomer.is-shut")) mark(aside, true);
        apply();
      });
    }
    const over = document.querySelector("[data-book-start-over]");
    if (over) {
      over.addEventListener("click", () => {
        forget();
        answers = null;
        showAll = false;
        for (const aside of document.querySelectorAll("aside.newcomer")) unmark(aside);
        apply();
        ask();
      });
    }

    dialog = document.getElementById("book-welcome");
    if (dialog) {
      for (const button of dialog.querySelectorAll("[data-answer]")) {
        const question = button.closest("[data-question]").dataset.question;
        button.addEventListener("click", () => answer(question, button.dataset.answer));
      }
      dialog.querySelector("[data-book-skip]").addEventListener("click", skip);
      // Escape closes the popup without a `close()` of ours: that is a skip.
      dialog.addEventListener("close", () => {
        if (closing) closing = false;
        else skip();
        const back = dialog.returnFocus;
        if (back && back !== document.body && document.contains(back)) back.focus();
      });
    }

    apply();
    openTarget();
    window.addEventListener("hashchange", openTarget);
    if (answers === null) ask();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start);
  } else {
    start();
  }
})();
