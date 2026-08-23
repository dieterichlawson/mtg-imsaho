---
id: ghost_quarter-02
status: new
card: Ghost Quarter
audit_run_id: 2026-04-19-ghost_quarter-audit
audit_model: sonnet
audit_tokens: 25466
audit_duration: 1899
---

## Audit Finding

**Oracle text:**
> Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.

**Code:**
> if basic_lands.is_empty() {
    // No basic lands to find — shuffle anyway per oracle text.
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    state.get_player_mut(target_controller).library_order.shuffle(&mut rng);
    return;
}

**Description:**
When the target land's controller has no basic lands in their library, the card skips the 'may' choice entirely and immediately shuffles the library. The oracle text's 'may' means the player must always be offered the choice to search or decline. If the player declines, no library search occurs and — because the search never happened — no shuffle should occur either. The current code incorrectly shuffles regardless of what the player would have chosen, removing the choice entirely. The correct behaviour is to present the same optional ChooseTarget prompt (with an empty options list, which the player can only decline) so that: (a) the player's agency is preserved, and (b) the library is only shuffled when the player accepts the search. When basic lands do exist the code uses `ChooseTarget { optional: true }` correctly — the no-basics branch should follow the same pattern.

**Engine path:** mtg-engine/src/cards/isd/ghost_quarter.rs:91

## Tests

### ghost_quarter_may_choice_offered_when_no_basics
Scenario: Ghost Quarter destroys an opponent's land; the opponent's library contains no basic lands. The opponent should be presented with the 'may search' choice; if they decline, the library should not be shuffled. Currently the library is always shuffled and no choice is presented.

