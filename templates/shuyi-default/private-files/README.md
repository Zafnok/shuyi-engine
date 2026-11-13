## The private files

The game's writing, art and sounds are in a private repository,
{{private_url}}, and never in this one. With access to it:

```
cargo shuyi private-assets       # check it out into private/, at the commit private.rev names
cargo shuyi private-assets --pin # after a change there is merged into its main: move private.rev
```

The build lays `private/files/` over `crates/game/files/` whenever the
checkout is there; without it, the game builds and runs on its public
files. A release ships the private files inside the program, or in its
pack.
