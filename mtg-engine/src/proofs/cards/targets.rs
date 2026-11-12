//! What the damage, destroy and counter spells may target, as each card's own code
//! says it: the requirement it names, and that it accepts whatever that
//! requirement offers (it adds no condition of its own).
//!
//! What each requirement then offers is the engine's, proved once for
//! every card in `proofs::actions::targeting` (the requirement's own
//! logic) and `proofs::actions::predicates` (what each filter, and
//! "can be targeted by", mean). Together: each card's targets are exactly
//! those its text names, less those its caster may not target.

use super::{any_target, blank_board, card_proof};
use crate::cards::{CardBehavior, TargetFilter, TargetRequirement};
use crate::ids::PlayerId;
use crate::types::CardType;

/// `card` names `requirement`, and accepts any target its caster could be
/// offered.
fn names(card: &dyn CardBehavior, requirement: TargetRequirement) {
    let (registry, mut state, blank) = blank_board();
    let object = state.create_object(blank, PlayerId(1), crate::types::Zone::Battlefield, None, None);
    let other = state.create_object(blank, PlayerId(0), crate::types::Zone::Battlefield, None, None);
    assert!(card.target_requirement() == requirement);
    let caster = PlayerId(u8::from(kani::any::<bool>()));
    assert!(card.is_valid_target(&state, caster, &any_target([object, other]), &registry));
    std::mem::forget((registry, state));
}

card_proof! {
    /// Geistflame: "any target" (CR 115.4).
    fn geistflame_targets_any_target() {
        names(&crate::cards::isd::geistflame::Geistflame, TargetRequirement::AnyTarget);
    }
}

card_proof! {
    /// Lightning Bolt: "any target".
    fn lightning_bolt_targets_any_target() {
        names(&crate::cards::lightning_bolt::LightningBolt, TargetRequirement::AnyTarget);
    }
}

card_proof! {
    /// Brimstone Volley: "any target".
    fn brimstone_volley_targets_any_target() {
        names(&crate::cards::isd::brimstone_volley::BrimstoneVolley, TargetRequirement::AnyTarget);
    }
}

card_proof! {
    /// Devil's Play: "any target".
    fn devils_play_targets_any_target() {
        names(&crate::cards::isd::devils_play::DevilsPlay, TargetRequirement::AnyTarget);
    }
}

card_proof! {
    /// Lava Axe: "target player or planeswalker".
    fn lava_axe_targets_a_player_or_planeswalker() {
        names(&crate::cards::lava_axe::LavaAxe, TargetRequirement::PlayerOrPlaneswalker);
    }
}

card_proof! {
    /// Ancient Grudge: "target artifact".
    fn ancient_grudge_targets_an_artifact() {
        names(&crate::cards::isd::ancient_grudge::AncientGrudge,
            TargetRequirement::PermanentWithFilter(TargetFilter::HasCardType(vec![CardType::Artifact])));
    }
}

card_proof! {
    /// Naturalize: "target artifact or enchantment".
    fn naturalize_targets_an_artifact_or_enchantment() {
        names(&crate::cards::isd::naturalize::Naturalize,
            TargetRequirement::PermanentWithFilter(TargetFilter::HasCardType(vec![CardType::Artifact, CardType::Enchantment])));
    }
}

card_proof! {
    /// Rebuke: "target attacking creature".
    fn rebuke_targets_an_attacking_creature() {
        names(&crate::cards::isd::rebuke::Rebuke, TargetRequirement::CreatureWithFilter(TargetFilter::Attacking));
    }
}

card_proof! {
    /// Smite the Monstrous: "target creature with power 4 or greater".
    fn smite_the_monstrous_targets_a_creature_with_power_four_or_greater() {
        names(&crate::cards::isd::smite_the_monstrous::SmiteTheMonstrous,
            TargetRequirement::CreatureWithFilter(TargetFilter::PowerAtLeast(4)));
    }
}

card_proof! {
    /// Urgent Exorcism: "target Spirit or enchantment".
    fn urgent_exorcism_targets_a_spirit_or_enchantment() {
        names(&crate::cards::isd::urgent_exorcism::UrgentExorcism,
            TargetRequirement::PermanentWithFilter(TargetFilter::SubtypeOrCardType {
                subtypes: vec!["Spirit".into()],
                card_types: vec![CardType::Enchantment],
            }));
    }
}

card_proof! {
    /// Victim of Night: "target non-Vampire, non-Werewolf, non-Zombie
    /// creature".
    fn victim_of_night_targets_a_creature_that_is_no_vampire_werewolf_or_zombie() {
        names(&crate::cards::isd::victim_of_night::VictimOfNight,
            TargetRequirement::CreatureWithFilter(TargetFilter::NotSubtypes(vec![
                "Vampire".into(), "Werewolf".into(), "Zombie".into(),
            ])));
    }
}

card_proof! {
    /// Bramblecrush: "target noncreature permanent".
    fn bramblecrush_targets_a_noncreature_permanent() {
        names(&crate::cards::isd::bramblecrush::Bramblecrush, TargetRequirement::PermanentWithFilter(TargetFilter::Noncreature));
    }
}

/// `card` names "target spell", and accepts exactly a spell: an object on
/// the stack (CR 115.1a), whoever controls it, and neither a player nor a
/// target gone illegal (CR 608.2b).
fn targets_a_spell(card: &dyn CardBehavior) {
    assert!(card.target_requirement() == TargetRequirement::Spell);
    let (registry, mut state, blank) = blank_board();
    let zone = match kani::any::<u8>() % 3 {
        0 => crate::types::Zone::Stack,
        1 => crate::types::Zone::Battlefield,
        _ => crate::types::Zone::Graveyard,
    };
    let object = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), zone, None, None);
    let caster = PlayerId(u8::from(kani::any::<bool>()));
    let target = any_target([object, object]);
    let spell = target == crate::actions::Target::Object(object) && zone == crate::types::Zone::Stack;
    assert!(card.is_valid_target(&state, caster, &target, &registry) == spell);
    std::mem::forget((registry, state));
}

card_proof! {
    /// Counterspell: "target spell".
    fn counterspell_targets_a_spell() {
        targets_a_spell(&crate::cards::counterspell::Counterspell);
    }
}

card_proof! {
    /// Dissipate: "target spell".
    fn dissipate_targets_a_spell() {
        targets_a_spell(&crate::cards::isd::dissipate::Dissipate);
    }
}

card_proof! {
    /// Frightful Delusion: "target spell".
    fn frightful_delusion_targets_a_spell() {
        targets_a_spell(&crate::cards::isd::frightful_delusion::FrightfulDelusion);
    }
}

card_proof! {
    /// Into the Maw of Hell: "target land and target creature", each a
    /// permanent on the battlefield of that type (the card accepts either
    /// for either slot; which slot is which is the requirement's), no
    /// player, and no target gone illegal. Lost in the Mist: "target spell
    /// and target permanent", an object on the stack or the battlefield,
    /// likewise. Over an object of any zone and any card types of these.
    fn the_two_target_spells_accept_what_their_text_targets() {
        let (registry, mut state, blank) = blank_board();
        let zone = match kani::any::<u8>() % 3 {
            0 => crate::types::Zone::Stack,
            1 => crate::types::Zone::Battlefield,
            _ => crate::types::Zone::Graveyard,
        };
        let object = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), zone, None, None);
        let o = state.get_object_mut(object).expect("just created");
        o.card_types = Vec::with_capacity(2);
        let land: bool = kani::any();
        let creature: bool = kani::any();
        if land {
            o.card_types.push(CardType::Land);
        }
        if creature {
            o.card_types.push(CardType::Creature);
        }
        let caster = PlayerId(u8::from(kani::any::<bool>()));
        let maw = crate::cards::isd::into_the_maw_of_hell::IntoTheMawOfHell;
        let mist = crate::cards::isd::lost_in_the_mist::LostInTheMist;
        let t = crate::actions::Target::Object(object);
        let on = zone == crate::types::Zone::Battlefield;
        assert!(maw.is_valid_target(&state, caster, &t, &registry) == (on && (land || creature)));
        assert!(mist.is_valid_target(&state, caster, &t, &registry) == (on || zone == crate::types::Zone::Stack));
        for card in [&maw as &dyn CardBehavior, &mist] {
            assert!(!card.is_valid_target(&state, caster, &crate::actions::Target::Player(caster), &registry));
            assert!(!card.is_valid_target(&state, caster, &crate::actions::Target::Illegal, &registry));
        }
        std::mem::forget((registry, state));
    }
}
