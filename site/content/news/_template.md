+++
# Copy this file to write a post (ticket 2805): name the copy
# YYYY-MM-DD-a-short-name.md, in this folder, with the date it goes up.
# A post with pictures is a folder of that name instead, with this file
# in it as index.md and the pictures beside it.
# Lines starting with # are notes: leave them or delete them.

# The post's title, as News lists it.
title = "A title"
# The day it goes up: the same date as the file's name.
date = 2026-01-31
# true while you write it: the live site leaves it out. Set it to false
# when it should go live.
draft = true
# One line about the post, for News's list and the feed.
description = "One line about the post."

[taxonomies]
# What kind of post: "devlog" for an update. Each tag has its page and
# its feed. ("release" is for the release posts, which are written for you.)
tags = ["devlog"]

[extra]
# Who wrote it: a name in site/data/authors.toml.
author = "Nick"
# Optional, for a post that is a folder: a picture in the folder, shown at
# the top, and a few words saying what it shows. Delete both lines if
# there is none.
image = "cover.png"
image_alt = "What the picture shows"
+++

Write the post here, in Markdown. A blank line starts a new paragraph.

A link: [the roadmap](https://github.com/Zafnok/shuyi-engine/blob/main/docs/ROADMAP.md).
A picture in the post's folder goes in the text as
`![what it shows](its-name.png)`, without the backticks.
