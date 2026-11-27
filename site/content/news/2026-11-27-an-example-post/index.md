+++
title = "An example post"
date = 2026-11-27
# The example of every field (ticket 2805). It stays a draft: the live
# site never shows it, and CI checks that it doesn't. `zola serve --drafts`
# shows it.
draft = true
description = "What a post looks like, with every field of the template used."

[taxonomies]
tags = ["devlog"]

[extra]
author = "Nick"
image = "cover.png"
image_alt = "The Snake example: a run starts, and the snake turns down toward the bottom wall."
+++

This post is an example, and never goes live. It shows every field of
`site/content/news/_template.md`: a title, a date, a one-line
description, the `devlog` tag, an author, and a cover picture from the
post's folder, here the [Snake example](https://github.com/Zafnok/shuyi-engine/tree/main/examples/snake)'s
preview.

A post is plain Markdown: paragraphs, **bold**, lists, and links.

- To write one by hand, or with the agent, read
  [How to post](@/community/how-to-post.md).
