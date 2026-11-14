# Before the first public footage: Moon Garden

Tick these before the reveal. This file is a working copy: `shuyi reveal verify` doesn't check it, and everything else in the folder is sealed.

- [ ] Open the store page, <https://store.example.com/app/1/Moon_Garden/>: it is live, its name is "Moon Garden", and the media it shows are the approved captures in `EVIDENCE.md`. A capture it shows that isn't there: add it to `shuyi.ron`'s `reveal`, then `shuyi reveal prepare` again.
- [ ] Show only the approved captures: the reveal's trailer, pictures and clips are files of this kit, as they are here.
- [ ] `shuyi reveal verify` passes, and the kit's digest, `c04c26cfc00a907a607c67ef3d4072a02d505371ba74f989239d8ed0c7c1be80`, is published somewhere that dates it: a commit or a tag pushed to the game's repository, or a release's notes. Then `shuyi reveal verify --digest <digest>` checks a copy against it.
- [ ] The salts file of the commitments is kept privately, and backed up: never in a public repository, nor in a folder the build embeds. Without it no commitment opens.
- [ ] Copyright registration of the game's art, music and text, before the first public footage. In the US the standard fee is about US$65, and statutory damages and attorney's fees need the registration before the infringement, or within three months of first publication (17 U.S.C. 412). Whether to register, where, and paying the fee, are the game owner's decisions: the kit only reminds.
- [ ] Keep the kit's folder whole and unchanged: copy it or zip it, never edit it. A capture that changes makes a new kit, with a new digest.

This is information, not legal advice. The kit records what it can check and judges nothing: whether a clip or a game copies this one, and what to do about it, is for the game's owner and their lawyer.
