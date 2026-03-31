# Audit: Slayer of the Wicked

## Oracle (Scryfall)
- **Name:** Slayer of the Wicked
- **Cost:** {3}{W}
- **Type:** Creature -- Human Soldier
- **Oracle:** When Slayer of the Wicked enters the battlefield, you may destroy target Vampire, Werewolf, or Zombie.
- **P/T:** 3/2

## Implementation: `mtg-engine/src/cards/slayer_of_the_wicked.rs`
- **Name:** Slayer of the Wicked ✅
- **Cost:** {3}{W} ✅
- **Type:** Creature ✅
- **Subtypes:** Human, Soldier ✅
- **P/T:** 3/2 ✅
- **Triggered ability:** EntersBattlefield ✅
- **"You may":** uses present_optional_target_choice ✅
- **Target filter:** checks subtypes for "Vampire", "Werewolf", or "Zombie" ✅
- **Destroy effect:** PendingEffect::Destroy ✅
- **Target scope:** any controller (not just opponent) -- matches oracle (no "an opponent controls" restriction) ✅

## Verdict: PASS -- no issues found
