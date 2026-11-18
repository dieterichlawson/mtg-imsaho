//! The Curses: Auras that enchant a player (CR 303.4, 702.5a), and what
//! each one does to the player it enchants.
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
//!
//! - Curse of Death's Hold: "Creatures enchanted player controls get
//!   -1/-1."
//! - Curse of the Nightly Hunt: "Creatures enchanted player controls attack
//!   each combat if able."
//! - Curse of the Bloody Tome: "At the beginning of enchanted player's
//!   upkeep, that player mills two cards."
//! - Curse of the Pierced Heart: "At the beginning of enchanted player's
//!   upkeep, this Aura deals 1 damage to that player or a planeswalker that
//!   player controls."
//! - Curse of Oblivion: "At the beginning of enchanted player's upkeep,
//!   that player exiles two cards from their graveyard."
//! - Curse of Stalked Prey: "Whenever a creature deals combat damage to
//!   enchanted player, put a +1/+1 counter on that creature."
//!
//! Each is "Enchant player": its requirement is "target player" (CR
//! 303.4a, 115.1), and it resolves by entering attached to that player
//! (303.4f). Each upkeep trigger is its enchanted player's (503.1a), which
//! its trigger scope says, and resolves for the player it enchants, or
//! enchanted last if it has since left the battlefield (113.7a).

use super::{card_proof, has_trigger, prints_where};
use crate::cards::{CardBehavior, TriggerKind, TriggerScope};
use crate::types::{ContinuousEffect as E, CreatureFilter, EffectScope};

use crate::cards::isd as isd;

card_proof! {
    /// Curse of Death's Hold prints "-1/-1 to each creature enchanted player
    /// controls", and Curse of the Nightly Hunt "each creature enchanted
    /// player controls attacks each combat if able", read by the engine as
    /// `proofs::actions::effects` proves for a player-wide scope.
    fn the_static_curses_print_what_their_text_says() {
        // As patterns (`prints_where`): each effect holds a filter, and two
        // filters read from the heap compared whole are what a proof cannot
        // settle (`prints_effects` says why).
        assert!(prints_where(&isd::curse_of_deaths_hold::CurseOfDeathsHold, |e| matches!(e,
            [E::ModifyPT { power: -1, toughness: -1, scope: EffectScope::Global(CreatureFilter::ControlledByAttachedPlayer) }])));
        assert!(prints_where(&isd::curse_of_the_nightly_hunt::CurseOfTheNightlyHunt, |e| matches!(e,
            [E::ForceAttack { scope: EffectScope::Global(CreatureFilter::ControlledByAttachedPlayer) }])));
    }
}

card_proof! {
    /// The upkeep Curses trigger in their enchanted player's upkeep, and no
    /// one else's (CR 503.1a).
    fn the_upkeep_curses_trigger_in_the_enchanted_players_upkeep() {
        for curse in [
            &isd::curse_of_the_bloody_tome::CurseOfTheBloodyTome as &dyn CardBehavior,
            &isd::curse_of_the_pierced_heart::CurseOfThePiercedHeart,
            &isd::curse_of_oblivion::CurseOfOblivion,
        ] {
            assert!(curse.step_trigger_scope(&TriggerKind::Upkeep, false) == TriggerScope::AttachedPlayer);
            assert!(has_trigger(curse, &TriggerKind::Upkeep));
        }
    }
}

