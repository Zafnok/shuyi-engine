// The book's contents and the page's headings beside a page of the book
// (ticket 2834).
//
// - The book's contents (`nav.book-side`) is a column on the left from
//   960 px up; its button (`.book-fold` for `#book-side`) folds it, and the
//   choice stays in `localStorage` under `shuyi-book-side` ("shut" or
//   "open"). Below 960 px it is folded, and the button opens it over the
//   text: Close, Escape or a click beside it closes it, and the focus goes
//   back to the button.
// - The page's headings (`nav.book-toc`) are a column on the right from
//   1240 px up, folded by their own button, kept under `shuyi-book-toc`.
//   Below that they are a `<details>` above the text. The heading being
//   read is marked `aria-current="location"` in both, as the reader
//   scrolls.
//
// The templates (`book-macros.html`, `book_open` and `book_close`) hold
// the lists and the buttons; this file holds no text. It is loaded in the
// `<head>`: it marks `<html>` with the folds before the page is drawn, and
// `style.css` lays the page out by those marks. With no JavaScript the
// buttons stay hidden and both lists show.
(function () {
  "use strict";

  const root = document.documentElement;
  // The widths `style.css` lays the page out at.
  const narrow = window.matchMedia("(width < 960px)");
  const KEYS = { side: "shuyi-book-side", toc: "shuyi-book-toc" };

  function read(key) {
    try {
      return window.localStorage.getItem(key);
    } catch (e) {
      return null;
    }
  }

  function write(key, value) {
    try {
      window.localStorage.setItem(key, value);
    } catch (e) {
      // Storage is blocked: the fold holds for this page.
    }
  }

  // `<html data-book-side-shut>` folds the contents' column, and
  // `data-book-toc-shut` the headings'; `data-book-nav` shows the buttons.
  function shut(name) {
    return root.hasAttribute("data-book-" + name + "-shut");
  }

  function setShut(name, value) {
    root.toggleAttribute("data-book-" + name + "-shut", value);
  }

  root.setAttribute("data-book-nav", "");
  setShut("side", read(KEYS.side) === "shut");
  setShut("toc", read(KEYS.toc) === "shut");

  let side = null;
  let sideButton = null;
  let tocButton = null;
  let scrim = null;
  // What the open drawer makes inert, to give it back.
  let inert = [];

  function drawerOpen() {
    return side !== null && side.classList.contains("is-open");
  }

  function sync() {
    if (sideButton) {
      const open = narrow.matches ? drawerOpen() : !shut("side");
      sideButton.setAttribute("aria-expanded", String(open));
    }
    if (tocButton) tocButton.setAttribute("aria-expanded", String(!shut("toc")));
  }

  function openDrawer() {
    side.classList.add("is-open");
    scrim.hidden = false;
    inert = Array.from(
      document.querySelectorAll(".site-header, .skip-link, .book-main, .book-toc, .site-footer")
    ).filter((node) => !node.inert);
    for (const node of inert) node.inert = true;
    sync();
    const here = side.querySelector('[aria-current="page"]') || side.querySelector("a");
    if (here) {
      here.focus({ preventScroll: true });
      here.scrollIntoView({ block: "center" });
    }
  }

  function closeDrawer(focus) {
    side.classList.remove("is-open");
    scrim.hidden = true;
    for (const node of inert) node.inert = false;
    inert = [];
    sync();
    if (focus) sideButton.focus();
  }

  // The heading being read: the last one whose top is above a quarter of
  // the window, or the last of all once the end of the text shows. None
  // before the first.
  function marker(links) {
    const ids = [];
    for (const a of links) {
      const id = decodeURIComponent(a.hash.slice(1));
      if (!ids.includes(id)) ids.push(id);
    }
    const headings = ids.map((id) => document.getElementById(id)).filter(Boolean);
    const end = document.querySelector("nav.pager") || document.querySelector("#book-text");
    let current = "";

    function mark() {
      const line = window.innerHeight / 4;
      // A heading in a passage the reader's filter left out has no box.
      const shown = headings.filter((h) => h.getClientRects().length > 0);
      let at = null;
      for (const h of shown) {
        if (h.getBoundingClientRect().top <= line) at = h;
        else break;
      }
      if (end && shown.length > 0 && end.getBoundingClientRect().bottom <= window.innerHeight) {
        at = shown[shown.length - 1];
      }
      const id = at ? at.id : "";
      if (id === current) return;
      current = id;
      for (const a of links) {
        if (id !== "" && decodeURIComponent(a.hash.slice(1)) === id) {
          a.setAttribute("aria-current", "location");
        } else {
          a.removeAttribute("aria-current");
        }
      }
      // Keep the marked heading in sight in a column that scrolls.
      const column = document.getElementById("book-toc");
      const marked = column && column.querySelector('[aria-current="location"]');
      if (marked) {
        const box = column.getBoundingClientRect();
        const its = marked.getBoundingClientRect();
        if (its.top < box.top) column.scrollTop -= box.top - its.top;
        else if (its.bottom > box.bottom) column.scrollTop += its.bottom - box.bottom;
      }
    }

    return { headings, end, mark };
  }

  function start() {
    side = document.getElementById("book-side");
    sideButton = document.querySelector('.book-fold[aria-controls="book-side"]');
    tocButton = document.querySelector('.book-fold[aria-controls="book-toc"]');
    if (!side || !sideButton) return;

    scrim = document.createElement("div");
    scrim.className = "book-scrim";
    scrim.hidden = true;
    scrim.addEventListener("click", () => closeDrawer(true));
    document.body.appendChild(scrim);

    sideButton.addEventListener("click", () => {
      if (narrow.matches) {
        if (drawerOpen()) closeDrawer(true);
        else openDrawer();
        return;
      }
      setShut("side", !shut("side"));
      write(KEYS.side, shut("side") ? "shut" : "open");
      sync();
    });
    const close = side.querySelector(".book-side-close");
    if (close) close.addEventListener("click", () => closeDrawer(true));
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && drawerOpen()) {
        event.preventDefault();
        closeDrawer(true);
      }
    });
    // Widened past the drawer's width: the contents is a column again.
    narrow.addEventListener("change", () => {
      if (!narrow.matches && drawerOpen()) closeDrawer(false);
      sync();
    });

    if (tocButton) {
      tocButton.addEventListener("click", () => {
        setShut("toc", !shut("toc"));
        write(KEYS.toc, shut("toc") ? "shut" : "open");
        sync();
      });
    }
    sync();

    const links = Array.from(document.querySelectorAll(".book-toc a, .book-toc-fold a"));
    if (links.length > 0 && "IntersectionObserver" in window) {
      const reading = marker(links);
      // Called as a heading crosses the line a quarter down the window, or
      // the end of the text comes into sight or leaves it.
      const observer = new IntersectionObserver(reading.mark, {
        rootMargin: "0px 0px -75% 0px",
      });
      for (const h of reading.headings) observer.observe(h);
      if (reading.end) {
        new IntersectionObserver(reading.mark, { threshold: [0, 1] }).observe(reading.end);
      }
      // A jump that carries no heading across the line (a link, the End
      // key) is seen once the scroll ends.
      window.addEventListener("scrollend", reading.mark);
      reading.mark();
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start);
  } else {
    start();
  }
})();
