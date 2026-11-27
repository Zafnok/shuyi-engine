+++
title = "How to post"
description = "Writing a post for News: in the browser, or by asking the agent."
+++

A post on [News](@/news/_index.md) is one Markdown file in the
repository's
[`site/content/news/`](https://github.com/Zafnok/shuyi-engine/tree/main/site/content/news)
folder. It goes live when its pull request is merged into `main`, a few
minutes later. There are two ways to write one.

## In the browser

No tools to install: GitHub's web editor is enough.

1. Open
   [`site/content/news/_template.md`](https://github.com/Zafnok/shuyi-engine/blob/main/site/content/news/_template.md)
   and copy all of its text (the "Copy raw file" button, top right of the
   file).
2. Open the
   [`site/content/news/`](https://github.com/Zafnok/shuyi-engine/tree/main/site/content/news)
   folder and choose **Add file**, then **Create new file**.
3. Name the file with the date and a short name, such as
   `2026-12-01-snake-runs-in-real-time.md`, and paste the template.
4. Write the post. Fill in each line at the top: the `title`, the `date`
   (the same as the file's name), the one-line `description`. Set
   `draft = false`. Write the post itself below the second `+++` line.
5. Choose **Commit changes…**. As the **Commit message**, write `Post: `
   and the post's title, such as `Post: Snake runs in real time`: the
   checks read it to know this is a post. Choose **Create a new branch
   for this commit and start a pull request**, then **Propose changes**.
6. On the next page the pull request's title is that same message. Choose
   **Create pull request**. Once its checks are green, choose **Squash
   and merge**.

A post with pictures is a folder instead: name the file
`2026-12-01-snake-runs-in-real-time/index.md`, and in that pull request's
branch use **Add file**, then **Upload files**, to put the pictures in the
same folder. Name one in `image` to show it at the top.

## With the agent

In a Claude Code session in this repository, say "write a blog post about
…" and what you want it to say. The agent drafts it in plain words, using
only facts from the repository and from you, and opens a pull request. It
never merges it: read the post in the pull request's **Files changed**, ask
for changes, and merge it when you are happy with it.

## Before it goes live

`draft = true` keeps a post off the live site, even once it is merged.
To see drafts on your own machine, run `zola serve --drafts` in `site/`
([the site's README](https://github.com/Zafnok/shuyi-engine/blob/main/site/README.md)).
