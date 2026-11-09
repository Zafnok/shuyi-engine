// A farming game's modded crop: it grows faster in rain, and slower
// beside its own kind.
fn grows(crop) {
    let rate = crop.base;
    if crop.weather == "Rain" { rate *= 2; }
    let kind = crop.kind;
    let crowded = crop.neighbours.filter(|n| n == kind).len();
    let height = crop.height + rate - crowded;
    #{ height: height.max(crop.height), ripe: height >= crop.ripe_at }
}
