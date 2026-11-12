//! The spells whose text is "~ deals N damage to <target>": their
//! resolution, for each target they could hold.
//!
//! - Geistflame, {R} Instant: "Geistflame deals 1 damage to any target.
//!   Flashback {3}{R}"
//! - Lightning Bolt, {R} Instant: "Lightning Bolt deals 3 damage to any
//!   target."
//! - Lava Axe, {4}{R} Sorcery: "Lava Axe deals 5 damage to target player or
//!   planeswalker."
//! - Brimstone Volley, {2}{R} Instant: "Brimstone Volley deals 3 damage to
//!   any target. Morbid — Brimstone Volley deals 5 damage instead if a
//!   creature died this turn."
//! - Devil's Play, {X}{R} Sorcery: "Devil's Play deals X damage to any
//!   target. Flashback {X}{R}{R}{R}"
//!
//! Their costs, types and flashback costs are their printed characteristics,
//! checked for every card against its Oracle record; "any target" and
//! "target player or planeswalker" (CR 115.4) are the engine's targeting,
//! proved for every requirement; flashback is the engine's casting. What is
//! each card's own is how much damage it asks for, and to what.
//!
//! Each proof is over every target the spell could hold as it resolves: the
//! other object on the board (a creature, artifact or enchantment of any
//! characteristics, on the battlefield or in a graveyard, either player's),
//! the spell itself, either player, or a target gone illegal (CR 608.2b).

use super::{any_target, card_proof, deals_damage_to_its_target, resolve, ANY_PERMANENT};
use crate::cards::isd::brimstone_volley::BrimstoneVolley;
use crate::cards::isd::devils_play::DevilsPlay;
use crate::cards::isd::geistflame::Geistflame;
use crate::cards::lava_axe::LavaAxe;
use crate::cards::lightning_bolt::LightningBolt;

card_proof! {
    /// Geistflame asks for 1 damage, dealt by it, to its target, and
    /// nothing else changes.
    fn geistflame_deals_one_damage_to_its_target() {
        let r = resolve(Box::new(Geistflame), &ANY_PERMANENT, |_, _| {}, |other, spell| any_target([other, spell]));
        assert!(deals_damage_to_its_target(&r, 1) && r.nothing_else_changed());
        kani::cover!(r.target == crate::actions::Target::Illegal, "a target gone illegal");
        std::mem::forget(r);
    }
}

card_proof! {
    /// Lightning Bolt asks for 3 damage, dealt by it, to its target, and
    /// nothing else changes.
    fn lightning_bolt_deals_three_damage_to_its_target() {
        let r = resolve(Box::new(LightningBolt), &ANY_PERMANENT, |_, _| {}, |other, spell| any_target([other, spell]));
        assert!(deals_damage_to_its_target(&r, 3) && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

card_proof! {
    /// Lava Axe asks for 5 damage, dealt by it, to its target, and nothing
    /// else changes.
    fn lava_axe_deals_five_damage_to_its_target() {
        let r = resolve(Box::new(LavaAxe), &ANY_PERMANENT, |_, _| {}, |other, spell| any_target([other, spell]));
        assert!(deals_damage_to_its_target(&r, 5) && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

card_proof! {
    /// Brimstone Volley asks for 5 damage to its target if a creature died
    /// this turn and 3 if none did (CR 700.4), dealt by it, and nothing
    /// else changes.
    fn brimstone_volley_deals_three_or_with_morbid_five() {
        let died: bool = kani::any();
        let r = resolve(Box::new(BrimstoneVolley), &ANY_PERMANENT,
            |state, _| state.creature_died_this_turn = died,
            |other, spell| any_target([other, spell]));
        assert!(deals_damage_to_its_target(&r, if died { 5 } else { 3 }) && r.nothing_else_changed());
        kani::cover!(died, "morbid");
        std::mem::forget(r);
    }
}

card_proof! {
    /// Devil's Play asks for X damage to its target, X being what it was
    /// cast with (CR 107.3a, 601.2b), for every X up to 5; dealt by it, and
    /// nothing else changes.
    fn devils_play_deals_x_damage_to_its_target() {
        let x: u32 = kani::any();
        kani::assume(x <= 5);
        let r = resolve(Box::new(DevilsPlay), &ANY_PERMANENT,
            |state, spell| state.get_object_mut(spell).expect("the spell").x_value = Some(x),
            |other, spell| any_target([other, spell]));
        assert!(deals_damage_to_its_target(&r, x) && r.nothing_else_changed());
        kani::cover!(x == 0, "X is 0");
        std::mem::forget(r);
    }
}
