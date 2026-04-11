# MTG Game Engine Bug Fix List

You are fixing bugs in an MTG game engine that uses LLM players (Gemini). Fix each bug one at a time, test it in a real game after fixing, then move to the next. You can test with `cargo run --release -p mtg-runner` or write focused integration tests. If a full game is impractical for a particular fix, a synthetic test scenario is fine — but prefer real game testing when possible.

The relevant files are:
- `mtg-player/src/llm.rs` — LLM player logic, prompt building, response parsing
- `mtg-engine/src/view.rs` — Game view structs sent to the player
- `mtg-engine/src/engine.rs` — Core game engine, action generation, action formatting

Run `cargo check` after every change to catch compiler errors. Run `cargo test` too if there are relevant tests.

---

## Bug 1: Blocking is 100% broken (CRITICAL)

**File:** `mtg-player/src/llm.rs` lines 1455-1470

**Problem:** The blocker prompt asks the AI for `blocker:attacker` pairs (e.g. `0:0`), but the Gemini model always responds with a bare number (e.g. `0`, `1`, `2`). The parser splits on `:`, gets 1 part instead of 2, and silently declares no blockers.

**Evidence from logs:** 107/107 block decisions produced zero actual blocks. The AI's thinking says things like "I will block the Diregraf Ghoul with my Festerhide Boar" but responds `0`. The parser requires `parts.len() == 2` which fails on `"0"`.

**Current code:**
```rust
for pair in answer.split_whitespace() {
    let parts: Vec<&str> = if pair.contains("->") {
        pair.split("->").collect()
    } else {
        pair.split(':').collect()
    };
    if parts.len() == 2 {
        if let (Ok(b), Ok(a)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
            if b < eligible_blockers.len() && a < attackers.len() {
                assignments.push((eligible_blockers[b], attackers[a]));
            }
        }
    }
}
```

**Fix:** When a bare number is given (no `:` or `->`), treat it as a blocker index blocking attacker 0 (when there's one attacker). When there are multiple attackers, a bare number should be treated as "blocker N blocks attacker 0" since the AI is most likely trying to block the first (or only meaningful) attacker. Also consider changing the prompt to use the numbered-action system instead of the free-form format, since that's what the Gemini structured output is trained on.

**How to verify the fix:**
- Run a game and grep the log for `declared blockers:`. Before the fix, this NEVER appears (only `declared no blockers`). After the fix, you should see lines like `p0 declared blockers: Festerhide Boar blocks Diregraf Ghoul`.
- Count: `grep -c "declared blockers:" game.log` should be > 0.
- Also check `grep -c "declared no blockers" game.log` — this should still appear sometimes (when the AI genuinely doesn't want to block) but not 100% of the time.
- Check that combat damage is dealt correctly after a block — e.g. `took 0 combat damage from` a blocked creature, and the blocker/attacker die if their toughness is exceeded.

**How to test:**
- Write a unit test that constructs a `CombatPrompt::ChooseBlockers` with 1 attacker and 2 eligible blockers. Mock the AI response as `"0"` (bare number). Verify the resulting `Action::DeclareBlockers` has `assignments: [(blocker_0, attacker_0)]`.
- Write another test with 2 attackers, AI response `"0 1"` (two bare numbers). Verify blocker 0 blocks attacker 0, blocker 1 blocks attacker 1.
- Write a test with response `"0:0"` (current format) still works.
- Write a test with response `"none"` still produces empty assignments.
- Run a real game with decks that have ground creatures on both sides. Verify blocks happen in the log.

---

## Bug 2: Attack-all silently fails (CRITICAL)

**File:** `mtg-player/src/llm.rs` lines 1411-1416

**Problem:** When the AI wants to attack with all creatures, it responds with `999`, concatenated digits like `1234560` or `43210`, or just `0` (meaning "first creature only"). The parser does `split_whitespace()` then `parse::<usize>()`, treating `999` as a single number that exceeds the eligible count, resulting in zero attackers.

**Evidence from logs:** In a 50-turn game, the AI had 22 creatures and lethal on board but responded `999` six times in a row. Each time, zero attackers were declared. The AI's thinking said "attacking with all creatures is lethal" every time. Other failed responses included `1234560`, `43210`, `3031`, `234`, `20`, `31`.

**Current code:**
```rust
let attackers = answer.split_whitespace()
    .filter_map(|s| s.parse::<usize>().ok())
    .filter(|&i| i < eligible.len())
    .collect();
```

**Fix:** Before the whitespace parsing, add a heuristic: if the response is a single token that parses to a number >= eligible count, treat it as "all". Also try splitting concatenated single digits (e.g. `1234560` → `[1,2,3,4,5,6,0]`) when the number exceeds eligible count. Add a log warning when this heuristic kicks in.

**How to verify the fix:**
- Grep for `No attackers declared` in the log. Before the fix in the 50-turn game, this appeared dozens of times when the AI had 13+ creatures. After the fix, it should only appear when the AI genuinely chose "none".
- Grep for attack declarations: `grep "declared attackers:" game.log | head -20` — verify that when the AI has multiple creatures, multiple creatures are declared as attackers (not just one).
- Check the 50-turn game scenario: if you replay a similar game state (many creatures, opponent at low life), the game should end quickly.

**How to test:**
- Write unit tests for the attacker parsing:
  - Response `"all"` → all eligible creatures attack. 
  - Response `"0 1 2"` → creatures 0, 1, 2 attack.
  - Response `"999"` with 5 eligible → treat as all, 5 creatures attack.
  - Response `"43210"` with 5 eligible → split into [4,3,2,1,0], all 5 attack.
  - Response `"20"` with 3 eligible → split into [2,0], creatures 2 and 0 attack.
  - Response `"none"` → zero attackers.
  - Response `"0"` → only creature 0 attacks (this is a valid single-creature choice).
- Run a real game and verify that when one side has overwhelming force, they actually attack with everything.

---

## Bug 3: Concede confirmation broken (HIGH)

**File:** `mtg-player/src/llm.rs` lines 1181-1196

**Problem:** When the AI chooses Concede, the code sends a confirmation message: "Reply ONLY 'yes' or 'no'." The Gemini model, being in structured output mode, responds with a number (the concede action index from the previous prompt, e.g. `6`) instead of the word "yes". The code checks `if !last.contains("yes")` and cancels every time.

**Evidence from logs:** 81 concede attempts, 0 confirmed through this flow. The AI's thinking always says "yes" but its output is a number. This caused a 50-turn game where the losing player was trapped.

**Current code:**
```rust
if matches!(actions.get(idx), Some(Action::Concede)) {
    let confirm = self.send_message(
        "You chose to CONCEDE the game. Are you sure? Reply ONLY 'yes' or 'no'."
    );
    let last = confirm.lines().rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if !last.contains("yes") {
        self.log("CONCEDE-CHECK", "Concede cancelled, passing instead");
        return 0;
    }
}
```

**Fix:** The simplest fix is to remove the confirmation entirely — if the AI chose Concede, trust it. Alternatively, check if the response contains any affirmative signal (the number itself, "yes", "1", "true", etc.) or just check if the structured JSON output has `"action": 0` (yes) vs `"action": 1` (no). But removing confirmation is safest since the current implementation can never work with structured output.

**How to verify the fix:**
- Grep for `CONCEDE-CHECK` in the log. Before the fix, every concede attempt shows `Concede cancelled, passing instead`. After the fix, concedes should either succeed immediately (no CONCEDE-CHECK) or show `Concede confirmed`.
- Check that games end in reasonable turn counts. The 50-turn game should not recur — if a player is losing badly, they should be able to concede.
- Count `p[01] concedes` in the log to verify concessions actually happen.

**How to test:**
- Set up a game where one player has an overwhelming advantage (e.g. opponent at 1 life, massive board). The losing AI should concede, and the game should end.
- Verify the game result shows the conceding player lost.

---

## Bug 4: No keyword abilities shown on battlefield (HIGH)

**File:** `mtg-engine/src/view.rs` lines 50-65, `mtg-player/src/llm.rs` lines 870-882

**Problem:** The `PermanentView` struct has no field for keyword abilities. The board display shows creatures as `Abbey Griffin 2/2` with no indication of flying, vigilance, deathtouch, lifelink, hexproof, etc. The AI can't make informed combat decisions.

**Current `PermanentView` struct:**
```rust
pub struct PermanentView {
    pub object_id: ObjectId,
    pub card_id: CardId,
    pub name: String,
    pub card_types: Vec<CardType>,
    pub controller: PlayerId,
    pub owner: PlayerId,
    pub tapped: bool,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub effective_power: Option<i32>,
    pub effective_toughness: Option<i32>,
    pub damage_marked: u32,
    pub summoning_sick: bool,
    pub attached_to: Option<ObjectId>,
}
```

**Fix:** Add a `keywords: Vec<Keyword>` field to `PermanentView`. Populate it from the permanent's current effective keywords (accounting for granted/lost abilities). In `llm.rs` line 870-882, include keywords in the display string: `Abbey Griffin 2/2 flying, vigilance`. Tokens should also show their keywords: `Spirit 1/1 flying`.

You'll need to check how the engine tracks effective keywords on permanents — look at how combat checks flying/reach/deathtouch to find where keywords are stored at runtime. Check `mtg-engine/src/combat.rs` for how keywords like flying, first strike, deathtouch are accessed.

**How to verify the fix:**
- Grep for `flying` in the board state lines: `grep "Your board\|Opp board" game.log | grep flying`. Before the fix, zero matches. After the fix, creatures with flying should show it.
- Check specific creatures: `Abbey Griffin` should show as `Abbey Griffin 2/2 flying, vigilance`, `Markov Patrician` should show `Markov Patrician 3/1 lifelink`, `Ambush Viper` should show `Ambush Viper 2/1 flash, deathtouch`.
- Token check: Spirit tokens from Midnight Haunting should show `Spirit 1/1 flying`.

**Scenarios where this matters:**

1. **Flying blocks:** The opponent attacks with a 3/2 flyer (Moon Heron). You have a 3/3 ground creature (Festerhide Boar) and a 2/1 flyer (Voiceless Spirit). Without keyword info, the AI might try to block with the Boar (illegal — can't block flying). With keywords visible, the AI knows to block with its own flyer or take the damage. **Verify:** In the GEMINI_THOUGHT, the AI should reference flying when making block decisions — "I can't block Moon Heron because it has flying and my Festerhide Boar doesn't have flying or reach."

2. **Deathtouch evaluation:** The opponent attacks with Ambush Viper (2/1 deathtouch). You have a 5/5 creature. Without deathtouch info, the AI might happily block thinking it survives. With keywords, the AI knows a block trades its 5/5 for a 2/1, which changes the math completely. **Verify:** The AI's thought should mention deathtouch risk — "Ambush Viper has deathtouch, so blocking with my 5/5 would trade it away."

3. **Lifelink racing:** Both players are at low life. Opponent has Markov Patrician (3/1 lifelink). The AI needs to know about lifelink to evaluate whether to race or block — attacking lets the opponent gain 3 life on their turn. **Verify:** The AI should mention lifelink in life total calculations — "If I let Markov Patrician through, opponent gains 3 life from lifelink."

4. **Hexproof targeting:** Geist of Saint Traft has hexproof. Without this info, the AI might waste removal targeting it. **Verify:** The AI should NOT attempt to target hexproof creatures with removal spells.

**How to test:**
- Write a unit test that constructs a `PermanentView` with keywords and verifies the display format includes them.
- Run a real game with creatures that have flying. Check the log to confirm keywords appear in the board state.
- Run a game with deathtouch creatures (Ambush Viper, Typhoid Rats). In the GEMINI_THOUGHT for blocking decisions, verify the AI references deathtouch.
- Run a game with Markov Patrician. Verify the AI mentions lifelink in its combat analysis.

---

## Bug 5: Attacker list uses base P/T, not effective (HIGH)

**File:** `mtg-player/src/llm.rs` line 1380

**Problem:** The attacker selection prompt uses `p.power` and `p.toughness` (base values) instead of `p.effective_power`/`p.effective_toughness`. A creature with +1/+1 counters shows as its original P/T in the attacker list even though the board display shows the correct effective values.

**Current code (line 1380):**
```rust
let name = p.map(|p| format!("{} {}/{}", p.name, p.power.unwrap_or(0), p.toughness.unwrap_or(0)))
```

**Fix:** Use `p.effective_power.or(p.power).unwrap_or(0)` and same for toughness, matching the board display code at line 871. Apply the same fix to the attacker display in the blocker prompt (around line 1370).

**How to verify the fix:**
- Look for creatures with auras or counters in the log. In the attacker prompt, their P/T should match the board display.
- Grep for `Choose attackers:` lines and compare creature stats to the board state above them. They should match.

**Scenarios where this matters:**

1. **Aura-boosted creatures:** You have Abbey Griffin (base 2/2) enchanted with Skeletal Grimace (+1/+1). The board shows `Abbey Griffin 3/3` but the attacker list says `Abbey Griffin 2/2`. The AI might think it can only deal 2 damage when it can actually deal 3. When deciding whether to attack into a 3/3 blocker, the AI might incorrectly think the trade is unfavorable. **Verify:** The attacker list P/T matches the board display.

2. **Counter-boosted creatures:** Diregraf Ghoul with 2 +1/+1 counters from Curse of Stalked Prey. Board shows 4/4 but attacker list shows 2/2. The AI might not attack thinking the opponent's 3/3 kills it. **Verify:** Compare `Choose attackers:` P/T values to `Your board:` P/T values for the same creature.

**How to test:**
- Write a unit test with a creature that has effective_power=4, power=2. Verify the attacker prompt shows 4/X, not 2/X.
- Run a game where a creature gets a +1/+1 counter (e.g. from Curse of Stalked Prey or Elder Cathar). Check the attacker list in the log.
- Verify the AI's attack reasoning references the correct (effective) P/T.

---

## Bug 6: Blocker list shows no P/T (HIGH)

**File:** `mtg-player/src/llm.rs` lines 1424-1433

**Problem:** The blocker selection prompt shows creatures as `0:Voiceless Spirit (your)` with no power/toughness, while the attacker list shows `0:Festerhide Boar 3/3`. The AI can't evaluate trades without knowing stats.

**Fix:** Add P/T to the blocker display, using effective values. Also add P/T to the attacker display in the blocker prompt (the `Attackers:` line). After fixing Bug 4, include keywords here too (especially flying, deathtouch, first strike which are critical for blocking decisions).

**How to verify the fix:**
- Grep for `Your blockers:` in the log. Before the fix: `0:Voiceless Spirit (your)`. After the fix: `0:Voiceless Spirit 2/1 flying, first strike (your)`.
- Grep for `Attackers:` in the blocker prompt section. Should show P/T and keywords for attacking creatures.

**Scenarios where this matters:**

1. **Trade evaluation:** Opponent attacks with a 4/4. You have a 2/1 and a 3/3 as potential blockers. Without P/T in the blocker list, the AI can't tell which blocker survives the trade. It might block with the 2/1 (dies, doesn't kill the attacker) when the 3/3 would be a better chump blocker (also dies but at least deals 3 damage back). **Verify:** The AI's GEMINI_THOUGHT references specific P/T values when evaluating blocks — "My 3/3 can't survive blocking the 4/4 but deals more damage back than my 2/1."

2. **Deathtouch + P/T:** If an attacker has deathtouch (visible after Bug 4 is fixed), the AI should prefer to block with its weakest creature since all blockers die to deathtouch regardless of toughness. Without P/T and keywords in the blocker list, this evaluation is impossible. **Verify:** When blocking a deathtouch creature, the AI should choose the lowest-value blocker.

3. **First strike + toughness:** An attacker with first strike and 3 power kills anything with ≤3 toughness before it deals damage back. The AI needs to see both keywords and P/T to evaluate this. **Verify:** The AI avoids blocking first strike creatures with low-toughness creatures unless it's trading up.

**How to test:**
- Write a unit test that constructs a blocker prompt. Verify the output includes P/T and keywords for both blockers and attackers.
- Run a real game with combat. Check the blocker prompts in the log — both sides should show stats and keywords.
- Verify that the AI's blocking reasoning references specific stats: "Blocking with my 1/4 is safe since the attacker only has 2 power."

---

## Bug 7: Night Terrors shows raw object IDs (HIGH)

**File:** `mtg-player/src/llm.rs` lines 1081-1101

**Problem:** The `obj_name()` function searches the player's hand, battlefield, stack, and graveyards for an object ID. But when a card is in the **opponent's revealed hand** (e.g. Night Terrors discard choice), it falls through to `format!("{}", id)` producing `obj#6`. The AI is choosing blind.

**Current code:**
```rust
fn obj_name(view: &GameView, id: ObjectId) -> String {
    if let Some(p) = view.battlefield.iter().find(|p| p.object_id == id) { ... }
    view.your_hand.iter()
        .find(|c| c.object_id == id)
        .map(|c| c.name.clone())
        .or_else(|| view.stack.iter()...)
        .or_else(|| view.graveyards.iter()...)
        .unwrap_or_else(|| format!("{}", id))
}
```

**Fix:** Check if `GameView` has access to the opponent's revealed hand or exile zone. If not, you may need to add a `revealed_cards` field. Alternatively, the action descriptions generated by the engine in `engine.rs` (around `ChooseCardFromHand`) should include card names — check if the action text already has the name and is just not being used. As a simpler approach, the `obj_name` fallback could look up the card name from the registry using the card_id.

**How to verify the fix:**
- Search for `obj#` in the log: `grep "obj#" game.log`. Before the fix, this appears for Night Terrors choices. After the fix, it should never appear — all objects should have readable names.
- When Night Terrors resolves, the choice prompt should show card names like `0:Brimstone Volley 1:Chapel Geist 2:Midnight Haunting`.

**How to test:**
- Build a test deck with Night Terrors and ensure it gets cast. Verify the discard choice shows card names.
- Write a unit test for `obj_name` with an ID that only exists in a "revealed" zone. Verify it returns a name, not `obj#N`.

---

## Bug 8: Cloistered Youth / "may transform" cards show confusing actions (MEDIUM)

**File:** `mtg-engine/src/engine.rs` around lines 329-334 and 378-381

**Problem:** The "may transform" prompt shows actions as `0:Pay {1}  1:Don't pay (countered)` which is extremely confusing. The AI thinks "I don't want to pay" means the card will transform (since it interprets this as a tax). But `Don't pay` = decline = no transform. The description says "Cloistered Youth: choose yes or no" but the action labels say "Pay/Don't pay".

**Evidence from logs:**
```
[Cloistered Youth (#10): choose yes or no]
0:Pay {1} 1:Don't pay (countered)

GEMINI_THOUGHT: I want Cloistered Youth to transform into its stronger form, Unholy Fiend. 
Therefore, I will choose not to pay the upkeep cost.

Cloistered Youth: chose not to transform
```
The AI chose action 1 ("Don't pay") thinking it would cause the transform. 90 "chose not to transform" events in the log, zero transforms.

**Fix:** For `YesNo` choices, the actions should use the description from the card, not generic "Pay/Don't pay" labels. For Cloistered Youth, it should show:
```
[Transform Cloistered Youth into Unholy Fiend?]
0:Yes (transform)  1:No (stay as Cloistered Youth)
```

Look at how `ResolutionChoiceKind::YesNo { description, .. }` is constructed in the card implementations (e.g. `cloistered_youth.rs` line 83-86) and how the description is used in `engine.rs` line 381. The description field has good text like "Cloistered Youth: transform into Unholy Fiend?" — it just needs to be surfaced as the action labels instead of generic "Pay/Don't pay".

The root issue is likely that YesNo choices go through the same formatting as PayOrNot choices. They need distinct formatting.

**How to verify the fix:**
- Search for `Cloistered Youth` in the log. The action labels should now say `0:Yes (transform) 1:No` or similar clear text instead of `0:Pay {1} 1:Don't pay (countered)`.
- Count `chose not to transform` — this should decrease significantly (the AI should sometimes choose to transform when it wants to).
- Check that the AI's GEMINI_THOUGHT aligns with the action it takes.

**How to test:**
- Build a deck with Cloistered Youth. Run a game and check the log for the transform prompt. Verify the labels are clear.
- Run several games — Cloistered Youth should transform at least some of the time.
- Check other "may" triggers too: Thraben Sentry, Screeching Bat, Murder of Crows, Curiosity draw, Mentor of the Meek draw.

---

## Bug 9: Stack targets not displayed (MEDIUM)

**File:** `mtg-player/src/llm.rs` lines 767-778

**Problem:** When spells are on the stack, the display shows the spell name and controller but not what it targets. The `StackItemView` struct has a `targets: Vec<Target>` field that is ignored.

**Fix:** Include targets in the stack display. Something like `Dead Weight targeting Falkenrath Noble (opponent's)`. Use `obj_name()` to resolve target IDs to names. Note: targets are shown in the recent events log (e.g. `p0 cast Dead Weight targeting Falkenrath Noble`), but the stack display itself doesn't show them.

**How to verify the fix:**
- Grep for `Stack:` in the log. Before the fix: `Stack: Dead Weight (your)`. After the fix: `Stack: Dead Weight targeting Falkenrath Noble (opponent's)`.
- Check that the AI can see what removal spells are targeting before they resolve.

**Scenarios where this matters:**

1. **Counter timing:** Opponent casts Dead Weight on your best creature. You have Dissipate in hand. Without seeing the target on the stack, you can't tell if it's worth countering. The AI might counter a spell targeting a token when it should save the counter for a spell targeting its bomb. **Verify:** When a removal spell is on the stack, the AI's GEMINI_THOUGHT references the specific target — "Dead Weight is targeting my Reaper from the Abyss, I should counter it" vs "Dead Weight is targeting a Spirit token, not worth countering."

2. **Response timing:** Opponent casts Brimstone Volley targeting you (player, not creature). If you can see it targets you and not your creature, you might play differently (e.g. not waste a protection spell). **Verify:** The AI distinguishes between spells targeting creatures vs players.

**How to test:**
- Cast a targeted spell (e.g. Dead Weight, Brimstone Volley). Check the stack display in the prompt to the opponent (who gets priority to respond). Verify the target is shown.
- Write a unit test that constructs a `StackItemView` with targets and verifies the display format.
- Run a game with counterspells. Verify the AI's GEMINI_THOUGHT for counter decisions references what the spell is targeting.

---

## Bug 10: Graveyard creature P/T missing (LOW)

**File:** `mtg-player/src/llm.rs` around lines 797-804

**Problem:** The graveyard lists card names but no power/toughness. This matters for cards like Corpse Lunge that deal damage equal to exiled creature's power. The AI can't evaluate which creature to exile without seeing stats.

**Fix:** Add P/T to creature names in the graveyard display, similar to how hand cards show stats. You'll need access to the card registry or the card data to get P/T for graveyard cards. Check how the `CardView` struct stores this info.

**How to verify the fix:**
- Grep for `graveyard:` lines in the log. Before: `Fiend Hunter, Markov Patrician`. After: `Fiend Hunter 1/3, Markov Patrician 3/1`.
- When Corpse Lunge is cast, the AI should be able to see which graveyard creature has the highest power.

**Scenarios where this matters:**

1. **Corpse Lunge targeting:** You have Corpse Lunge in hand and 3 creatures in your graveyard: a 1/1, a 3/3, and a 5/5. The opponent has a 4/4 creature. You need to exile the 5/5 to deal enough damage. Without P/T in the graveyard, the AI picks blind or relies on memory. **Verify:** The AI's GEMINI_THOUGHT for Corpse Lunge mentions specific power values — "I'll exile my Reaper from the Abyss (6/6) to deal 6 damage."

2. **Unburial Rites / Ghoulcaller's Chant target selection:** Choosing which creature to return from the graveyard. With stats visible, the AI can compare options. **Verify:** The AI references P/T when choosing reanimation targets.

3. **Skaab creature exile costs:** Makeshift Mauler and Skaab Goliath require exiling creature cards from the graveyard. The AI should see which creatures are available and their stats. **Verify:** The exile choice shows creature names with P/T.

**How to test:**
- Run a game where creatures die. Check the graveyard display in prompts to verify P/T is shown.
- Cast Corpse Lunge and verify the AI makes an informed exile choice based on power values.

---

## Bug 11: Prompt noise reduction (LOW - optional)

**Problem:** ~46% of prompt content is "passes priority" messages and empty step transitions. A typical turn-2 prompt has 30+ lines of `Step: Upkeep / p0 passes priority / p1 passes priority / Step: Draw / ...` for events where nothing happened.

**Possible fix:** Filter the game log before sending to the AI. Remove consecutive "passes priority" lines that add no information. Collapse step transitions where nothing happened. For example, instead of showing every step, only show steps where something happened. This would roughly halve prompt token usage.

Be careful not to remove information that matters — e.g. "no spells were cast last turn" matters for werewolf transforms, and the current step matters for knowing what phase the AI is in. The phase information at the top of the board state display (`Turn 5 - Main 1 (your turn)`) already tells the AI what phase it's in, so step markers in the event log are mostly redundant.

**How to verify the fix:**
- Compare prompt lengths before and after. `grep "PROMPT" game.log | awk '{print NR, length}'` — average prompt length should decrease significantly.
- Verify game correctness is unchanged — all the same actions should be available, werewolves should still transform correctly.

**How to test:**
- Run a full game with the filter and without. Compare results to ensure no behavioral differences.
- Specifically test werewolf transforms — they depend on "no spells were cast last turn" which must still be detectable.

---

## Bug 12: Audit other complicated cards for harness issues

After fixing bugs 1-11, check these cards for similar display/choice/parsing problems. Many of these have "may" abilities, complex targeting, or unusual interactions that could hit the same class of bugs:

**"May" abilities (same class as Cloistered Youth Bug 8):**
- **Screeching Bat** — "At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform." Check that the pay/don't pay choice is clear.
- **Murder of Crows** — "Whenever another creature dies, you may draw a card. If you do, discard a card." Two-step choice: draw, then discard. Check both steps show card names.
- **Curiosity** — "Whenever enchanted creature deals damage, you may draw a card." Check the may choice is presented clearly.
- **Mentor of the Meek** — "Whenever a creature with power 2 or less enters, you may pay {1}. If you do, draw a card." Check pay choice is clear (not "Pay {1} / Don't pay (countered)").
- **Bitterheart Witch** — "When this dies, you may search your library for a Curse card." Check the search works.
- **Delver of Secrets** — "At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform." Check the reveal choice shows the card name.

**Complex targeting / multiple choices:**
- **Fiend Hunter** — Exiles a creature on ETB, returns it on LTB. Check the exile target choice shows creature names (not obj#IDs). Check the LTB return works correctly.
- **Morkrut Banshee** — "Morbid — When this enters, if a creature died this turn, target creature gets -4/-4." Check the target choice shows creature names and P/T.
- **Tribute to Hunger** — "Target opponent sacrifices a creature." The opponent chooses which creature. Check the sacrifice choice shows creature names and stats.
- **Corpse Lunge** — "Exile a creature card from your graveyard. Corpse Lunge deals damage equal to that card's power to target creature." Two choices: which graveyard creature to exile (needs P/T — Bug 10), and which creature to target.
- **Forbidden Alchemy** — "Look at the top four cards. Put one into your hand and the rest into your graveyard." Check the card choices show names.
- **Travel Preparations** — "Put a +1/+1 counter on target creature." In one log, the AI targeted an opponent's creature by mistake. Check that the target list makes it clear which creatures are yours vs opponent's.
- **Feeling of Dread** — "Tap up to two target creatures." Multi-target. Check both targets are selectable and show names.
- **Blasphemous Act** — "Costs {1} less for each creature on the battlefield." Check the cost reduction is shown or the engine auto-calculates it.

**Discard/sacrifice choices:**
- **Liliana of the Veil** — "+1: Each player discards a card." The discard choice needs card names.
- **Altar's Reap** — "As an additional cost, sacrifice a creature." Check the sacrifice choice shows creature names.
- **Skirsdag Cultist** — "{R}, {T}, Sacrifice a creature: Deal 2 damage." Check sacrifice target is named.

**For each card:** Build a deck containing it, run a game, and verify:
1. All choice prompts show card names (not obj#IDs)
2. "May" choices have clear yes/no labels (not confusing "Pay/Don't pay")  
3. Multi-step choices complete correctly (e.g. Murder of Crows draw → discard)
4. The AI's GEMINI_THOUGHT shows it understood the choice it was making

---

## Bug 13: Update system prompt to explain UI changes

**File:** `mtg-player/src/llm.rs` — the `GAME_RULES` constant and `GEMINI_RESPONSE_FORMAT` constant near the top of the file.

**Problem:** After fixing bugs 4-10, the UI will show new information (keywords, P/T in more places, targets on stack, etc.) but the system prompt doesn't explain what any of it means. The AI needs to understand:

1. **Keywords on creatures** — After Bug 4, creatures show keywords like `Abbey Griffin 2/2 flying, vigilance`. The system prompt should explain what each keyword means in gameplay terms and how they affect combat:
   - Flying: can only be blocked by creatures with flying or reach
   - Deathtouch: any damage it deals to a creature destroys it
   - Lifelink: damage dealt also gains that much life for the controller
   - First strike: deals combat damage before creatures without first strike
   - Vigilance: doesn't tap when attacking
   - Hexproof: can't be targeted by opponent's spells/abilities
   - Reach: can block creatures with flying
   - Intimidate: can only be blocked by artifact creatures or creatures sharing a color
   - Trample: excess combat damage carries over to the defending player

2. **Effective P/T** — Creatures on the board, in attacker lists, and in blocker lists now show effective power/toughness (including boosts from auras, counters, anthem effects). The system prompt should explain that the displayed P/T is the creature's CURRENT stats, not its base stats. Example: "Abbey Griffin 3/3 (Skeletal Grimace)" means the Griffin is effectively a 3/3 due to Skeletal Grimace's +1/+1 bonus.

3. **Blocker format** — After Bug 1, the blocker response format may have changed. If you switched to numbered actions, update the system prompt to match. If you kept the `blocker:attacker` format but added fallback parsing, document the expected format.

4. **Attacker format** — After Bug 2, if you changed how "all" works or added heuristics, make sure the system prompt instructions for declaring attackers are clear.

5. **Stack targets** — After Bug 9, spells on the stack show their targets. Add a note explaining this so the AI knows to check the stack for targeting info when deciding whether to counter or respond.

**How to verify:**
- Read the system prompt in the log (it's logged at game start as SYSTEM). All new UI elements should be explained.
- Run a game and check the AI's GEMINI_THOUGHT. It should reference the new information naturally — e.g. "The opponent's Moon Heron has flying, so I need a flyer or reach creature to block it."
- Verify the AI doesn't get confused by the new format — no thoughts like "I don't understand what 'flying' means after the creature name."

**How to test:**
- Run a game and manually read 5-10 GEMINI_THOUGHT entries. The AI should demonstrate understanding of keywords, effective P/T, and other new display elements.

---

## Testing approach

For each bug, you can test using one of these approaches:

1. **Full game test:** `cargo run --release -p mtg-runner -- --model gemini:gemini-2.5-flash:low:low` (cheap, fast). Watch the log for the specific behavior.

2. **Unit test in mtg-player:** Write a test that constructs a `GameView` with specific board state and verifies the prompt output contains the expected information.

3. **Engine test:** For engine-level fixes (view struct changes), write tests in the engine crate.

Fix the bugs in the numbered order (1-10). Each one builds on previous fixes (e.g. Bug 6 benefits from Bug 4's keyword additions). Test after each fix.
