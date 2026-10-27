//! `engine::apply_pending_effect`, the action a card asks for when it asks
//! the engine to deal damage to a target it holds.

use super::action_proof;
use crate::actions::Target;
use crate::damage::DamageKind;
use crate::events::DamageTarget;
use crate::ids::PlayerId;
use crate::proofs::cards::{acted, any_target, asked_for, board, Act, Snapshot};
use crate::state::PendingEffect;
use crate::types::Zone;

action_proof! {
    /// "Deals N damage to that target", a spell's or ability's, is N
    /// non-combat damage dealt by that source to that creature,
    /// planeswalker or player (CR 120.1, 120.2b), and to a target that has
    /// become illegal, none: an illegal target is neither changed nor acted
    /// on (608.2b). For every target, every amount up to 5 and either
    /// object as the source; nothing else changes.
    fn damage_to_a_target_is_noncombat_damage_from_its_source() {
        let (registry, mut state, blank, _) = board(Box::new(crate::cards::grizzly_bears::GrizzlyBears));
        let creature = state.create_object(blank, PlayerId(1), Zone::Battlefield, Some(2), Some(2));
        let spell = state.create_object(blank, PlayerId(0), Zone::Stack, None, None);
        let target = any_target([creature, spell]);
        let amount: u32 = kani::any();
        kani::assume(amount <= 5);
        let source = if kani::any() { spell } else { creature };
        let before = Snapshot::of(&state);

        crate::engine::apply_pending_effect(&mut state, &target, &PendingEffect::DealDamage { amount, source_id: source }, &registry);

        let dealt_to = match target {
            Target::Object(id) => Some(DamageTarget::Object(id)),
            Target::Player(p) => Some(DamageTarget::Player(p)),
            Target::Illegal => None,
        };
        match dealt_to {
            Some(t) => {
                assert!(asked_for(&[Act::Damage { source, target: t, amount, kind: DamageKind::NonCombat }]));
            }
            None => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&state) == before);
        kani::cover!(dealt_to.is_none(), "an illegal target");
        std::mem::forget((registry, state));
    }
}
