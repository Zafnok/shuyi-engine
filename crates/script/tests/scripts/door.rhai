// A platformer's level script: the door opens when three switches are on,
// and says which are still off.
let on = view.switches.filter(|s| s.on);
let off = view.switches.filter(|s| !s.on).map(|s| s.name);
#{
    open: on.len() >= 3,
    left: off,
    message: if off.is_empty() { "The door opens." } else { `${off.len()} to go.` },
}
