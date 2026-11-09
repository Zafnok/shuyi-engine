// A card game's card text: "Deal 2 damage for each ally, then heal 1 if
// you have fewer cards than your opponent."
fn play(table) {
    let effects = [];
    let allies = table.mine.filter(|c| c.ally).len();
    if allies > 0 { effects.push(#{ Damage: #{ amount: 2 * allies, target: "Opponent" } }); }
    if table.hand < table.opponent_hand { effects.push(#{ Heal: 1 }); }
    effects
}
