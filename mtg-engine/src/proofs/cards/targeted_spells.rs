//! The spells that act on what they target: a creature, a land, a card in
//! a graveyard, a player.
//!
//! - Swords to Plowshares, {W} Instant: "Exile target creature. Its
//!   controller gains life equal to its power."
//! - Prey Upon, {G} Sorcery: "Target creature you control fights target
//!   creature you don't control."
//! - Silent Departure, {U} Sorcery: "Return target creature to its owner's
//!   hand. Flashback {4}{U}"
//! - Grasp of Phantoms, {3}{U} Sorcery: "Put target creature on top of its
//!   owner's library. Flashback {7}{U}"
//! - Maw of the Mire, {4}{B} Sorcery: "Destroy target land. You gain 4
//!   life."
//! - Purify the Grave, {W} Instant: "Exile target card from a graveyard.
//!   Flashback {W}"
//! - Traitorous Blood, {1}{R}{R} Sorcery: "Gain control of target creature
//!   until end of turn. Untap it. It gains trample and haste until end of
//!   turn. Flashback {1}{R}"
//! - Corpse Lunge, {2}{B} Instant: "As an additional cost to cast this
//!   spell, exile a creature card from your graveyard. Corpse Lunge deals
//!   damage equal to the exiled card's power to target creature."
//! - Harvest Pyre, {1}{R} Instant: "As an additional cost to cast this
//!   spell, exile X cards from your graveyard. Harvest Pyre deals X damage
//!   to target creature."
//! - Ghoulcaller's Chant, {B} Sorcery: "Choose one — • Return target
//!   creature card from your graveyard to your hand. • Return two target
//!   Zombie cards from your graveyard to your hand."
//! - Memory's Journey, {1}{U} Instant: "Target player shuffles up to three
//!   target cards from their graveyard into their library. Flashback {G}"
//! - Unburial Rites, {4}{B} Sorcery: "Return target creature card from your
//!   graveyard to the battlefield. Flashback {3}{W}"
//! - Sever the Bloodline, {3}{B} Sorcery: "Exile target creature and all
//!   other creatures with the same name as that creature. Flashback
//!   {5}{B}{B}"
//! - Blasphemous Act, {8}{R} Sorcery: "This spell costs {1} less to cast
//!   for each creature on the battlefield. Blasphemous Act deals 13 damage
//!   to each creature."
//!
//! Each proof pins what the spell targets (CR 115.1, 601.2c) and runs its
//! resolution over a spell cast by player 0 and a board of a card and a
//! token of any characteristics, each target one of those or gone illegal.
//! A target that was legal as the spell was cast and is not as it resolves
//! is `Target::Illegal` by then (CR 608.2b), which the engine decides
//! (`proofs::actions`); a target still an object here may be anywhere, so
//! that the proof covers what the card does with one it no longer finds
//! where it was.

use super::{acted, any_object, asked_for, board, card_proof, Act, Form, Kinds, Snapshot};
use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardRegistry, TargetFilter, TargetRequirement as R};
use crate::damage::DamageKind;
use crate::events::DamageTarget;
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameObject, GameState, LibraryPosition, PendingEffect, TemporaryEffect as T};
use crate::types::{CardType, Keyword, ManaSymbol, Zone};

use crate::cards::isd;

const ANY: Kinds = Kinds { types: &[CardType::Creature, CardType::Land], subtypes: &["Zombie"], keywords: &[] };

/// The board: the spell, cast by player 0, and a card and a token.
struct Board {
    registry: CardRegistry,
    state: GameState,
    spell: ObjectId,
    others: [ObjectId; 2],
}

fn spell(card: Box<dyn CardBehavior>) -> Board {
    let (registry, mut state, blank, card) = board(card);
    let others = [any_object(&mut state, blank, &ANY, Form::Card), any_object(&mut state, blank, &ANY, Form::Token)];
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    Board { registry, state, spell, others }
}

impl Board {
    fn get(&self, id: ObjectId) -> &GameObject {
        self.state.get_object(id).expect("on the board")
    }

    fn on(&self, id: ObjectId) -> bool {
        self.get(id).zone == Zone::Battlefield
    }

    /// Either object, or a target gone illegal.
    fn any_object_target(&self) -> Target {
        match kani::any::<u8>() % 3 {
            0 => Target::Object(self.others[0]),
            1 => Target::Object(self.others[1]),
            _ => Target::Illegal,
        }
    }

    fn resolve(&mut self, targets: &[Target]) -> Snapshot {
        let before = Snapshot::of(&self.state);
        let card = self.get(self.spell).card_id;
        self.registry.get(card).expect("registered").on_resolve(&mut self.state, self.spell, targets, &self.registry);
        before
    }

    fn requirement(&self) -> std::mem::ManuallyDrop<R> {
        std::mem::ManuallyDrop::new(self.registry.get(self.get(self.spell).card_id).expect("registered").target_requirement())
    }
}

/// `card` targets as `requirement` says, and, holding one target that is
/// either object or gone illegal, asks for `acts(board, object)` when the
/// target is an object for which `applies` holds, and for nothing
/// otherwise; and changes nothing else.
fn acts_on_its_target(
    card: Box<dyn CardBehavior>, requirement: R, applies: fn(&Board, ObjectId) -> bool, acts: fn(&Board, ObjectId) -> Vec<Act>,
) {
    let mut b = spell(card);
    assert!(*b.requirement() == requirement);
    let target = b.any_object_target();
    let expected = match target {
        Target::Object(id) if applies(&b, id) => acts(&b, id),
        _ => vec![],
    };
    let before = b.resolve(&[target]);
    assert!(asked_for(&expected) && Snapshot::of(&b.state) == before);
    std::mem::forget(b);
}

card_proof! {
    /// Swords to Plowshares: its target, if still on the battlefield,
    /// exiled, and then whoever controlled it gains life equal to its power
    /// as it last was there (CR 608.2h); none for a power of 0 or less.
    fn swords_to_plowshares_exiles_and_its_controller_gains_its_power() {
        let mut b = spell(Box::new(crate::cards::swords_to_plowshares::SwordsToPlowshares));
        assert!(*b.requirement() == R::Creature);
        let target = b.any_object_target();
        let expected = match target {
            Target::Object(id) if b.on(id) => {
                let power = b.state.effective_power(id, &b.registry).unwrap_or(0);
                let mut acts = vec![Act::Move(id, Zone::Exile)];
                if power > 0 {
                    acts.push(Act::Life(b.get(id).controller, power));
                }
                acts
            }
            _ => vec![],
        };
        let before = b.resolve(&[target]);
        assert!(asked_for(&expected) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Prey Upon: a creature its caster controls and one they don't; the
    /// two fight if both targets are still legal, and neither does
    /// anything otherwise (CR 701.12b, and the card's ruling).
    fn prey_upon_makes_its_targets_fight() {
        let mut b = spell(Box::new(isd::prey_upon::PreyUpon));
        assert!(matches!(&*b.requirement(), R::TwoTargets(yours, theirs)
            if matches!(**yours, R::CreatureWithFilter(TargetFilter::YouControl))
                && matches!(**theirs, R::CreatureWithFilter(TargetFilter::YouDontControl))));
        let first = if kani::any() { Target::Object(b.others[0]) } else { Target::Illegal };
        let second = if kani::any() { Target::Object(b.others[1]) } else { Target::Illegal };
        let expected = match (&first, &second) {
            (Target::Object(a), Target::Object(c)) => vec![Act::Fight(*a, *c)],
            _ => vec![],
        };
        let before = b.resolve(&[first, second]);
        assert!(asked_for(&expected) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Silent Departure: its target, if still on the battlefield, returned
    /// to its owner's hand.
    fn silent_departure_returns_its_target_to_its_owners_hand() {
        acts_on_its_target(Box::new(isd::silent_departure::SilentDeparture), R::Creature,
            Board::on, |_, id| vec![Act::Move(id, Zone::Hand)]);
    }
}

card_proof! {
    /// Grasp of Phantoms: its target, if still on the battlefield, put on
    /// top of its owner's library.
    fn grasp_of_phantoms_puts_its_target_on_top_of_its_owners_library() {
        acts_on_its_target(Box::new(isd::grasp_of_phantoms::GraspOfPhantoms), R::Creature,
            Board::on, |_, id| vec![Act::ToLibrary(id, LibraryPosition::Top)]);
    }
}

card_proof! {
    /// Maw of the Mire: "target land"; it destroyed (whether it still can
    /// be is the action's), then 4 life for its caster. With its target
    /// gone illegal it does nothing at all, the life included: a spell
    /// whose every target is illegal does not resolve (CR 608.2b).
    fn maw_of_the_mire_destroys_a_land_and_gains_four() {
        acts_on_its_target(Box::new(isd::maw_of_the_mire::MawOfTheMire),
            R::PermanentWithFilter(TargetFilter::HasCardType(vec![CardType::Land])),
            |_, _| true, |_, id| vec![Act::Destroy(id), Act::Life(PlayerId(0), 4)]);
    }
}

card_proof! {
    /// Purify the Grave: "target card from a graveyard"; it exiled.
    fn purify_the_grave_exiles_a_card_from_a_graveyard() {
        acts_on_its_target(Box::new(isd::purify_the_grave::PurifyTheGrave), R::GraveyardCard,
            |_, _| true, |_, id| vec![Act::Move(id, Zone::Exile)]);
    }
}

card_proof! {
    /// Unburial Rites: "target creature card from your graveyard"; it
    /// returned to the battlefield.
    fn unburial_rites_returns_a_creature_card_to_the_battlefield() {
        acts_on_its_target(Box::new(isd::unburial_rites::UnburialRites), R::GraveyardCreature,
            |_, _| true, |_, id| vec![Act::Move(id, Zone::Battlefield)]);
    }
}

card_proof! {
    /// Traitorous Blood: its target, if still on the battlefield, under its
    /// caster's control until end of turn (a control effect with the latest
    /// timestamp, CR 613.7), untapped, and with trample and haste until end
    /// of turn; and nothing else.
    fn traitorous_blood_steals_untaps_and_hastes_its_target() {
        let mut b = spell(Box::new(isd::traitorous_blood::TraitorousBlood));
        assert!(*b.requirement() == R::Creature);
        let target = b.any_object_target();
        let before = b.resolve(&[target.clone()]);
        let added = &b.state.until_end_of_turn[before.until_end_of_turn()..];
        match target {
            Target::Object(id) if b.on(id) => {
                assert!(asked_for(&[Act::Control(id, PlayerId(0)), Act::Untap(id)]));
                let latest = b.state.next_effect_timestamp;
                assert!(added == [
                    T::ChangeControl { target: id, controller: PlayerId(0), timestamp: latest },
                    T::GrantKeyword { target: id, keyword: Keyword::Haste },
                    T::GrantKeyword { target: id, keyword: Keyword::Trample },
                ]);
            }
            _ => {
                assert!(acted() == 0 && added.is_empty());
            }
        }
        assert!(Snapshot::of(&b.state).without_until_end_of_turn() == before.without_until_end_of_turn());
        std::mem::forget(b);
    }
}

card_proof! {
    /// Corpse Lunge: its additional cost exiles a creature card from its
    /// caster's graveyard (CR 601.2h), which the engine pays and records;
    /// it deals damage equal to that card's power, as it is in exile, to
    /// its target, none for a power of 0 or less. With no card recorded
    /// (none exiled), 0.
    fn corpse_lunge_deals_the_exiled_cards_power() {
        assert!(isd::corpse_lunge::CorpseLunge.card_data().additional_cost == Some(AdditionalCost::ExileCreaturesFromGraveyard(1)));
        let mut b = spell(Box::new(isd::corpse_lunge::CorpseLunge));
        assert!(*b.requirement() == R::Creature);
        let blank = b.get(b.others[0]).card_id;
        let power: i8 = kani::any();
        kani::assume((-1..=6).contains(&power));
        let exiled = b.state.create_object(blank, PlayerId(0), Zone::Exile, Some(i32::from(power)), Some(1));
        b.state.get_object_mut(exiled).expect("just created").card_types = vec![CardType::Creature];
        let recorded: bool = kani::any();
        if recorded {
            b.state.get_object_mut(b.spell).expect("on the stack").card_state.insert(crate::cards::exiled_to_cost_key(0), exiled);
        }
        let damage = if recorded { u32::try_from(power.max(0)).expect("small") } else { 0 };
        let target = b.any_object_target();
        let before = b.resolve(&[target.clone()]);
        match target {
            Target::Object(id) => {
                assert!(asked_for(&[Act::Damage { source: b.spell, target: DamageTarget::Object(id), amount: damage, kind: DamageKind::NonCombat }]));
            }
            _ => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Harvest Pyre: its additional cost exiles X cards from its caster's
    /// graveyard, X chosen as it is cast, which the engine pays and
    /// records; it deals X damage to its target.
    fn harvest_pyre_deals_x_damage() {
        assert!(isd::harvest_pyre::HarvestPyre.card_data().additional_cost == Some(AdditionalCost::ExileXFromGraveyard));
        let mut b = spell(Box::new(isd::harvest_pyre::HarvestPyre));
        assert!(*b.requirement() == R::Creature);
        let x: u8 = kani::any();
        kani::assume(x <= 8);
        b.state.get_object_mut(b.spell).expect("on the stack").card_state
            .insert(crate::cards::EXILE_COUNT.to_string(), ObjectId(u64::from(x)));
        let target = b.any_object_target();
        let before = b.resolve(&[target.clone()]);
        match target {
            Target::Object(id) => {
                assert!(asked_for(&[Act::Effect(Target::Object(id), PendingEffect::DealDamage { amount: u32::from(x), source_id: b.spell })]));
            }
            _ => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Ghoulcaller's Chant: one mode or the other (CR 700.2); each target
    /// card still in a graveyard returned to its owner's hand, in order.
    fn ghoulcallers_chant_returns_its_targets_to_hand() {
        let mut b = spell(Box::new(isd::ghoulcallers_chant::GhoulcallersChant));
        let zombie = |r: &R| matches!(r, R::GraveyardCreatureOfSubtype(s) if s == "Zombie");
        assert!(matches!(&*b.requirement(), R::ModalChoice(modes) if modes.len() == 2
            && matches!(modes[0], R::GraveyardCreature)
            && matches!(&modes[1], R::TwoTargets(x, y) if zombie(x) && zombie(y))));
        let targets = if kani::any() { vec![b.any_object_target()] } else { vec![b.any_object_target(), b.any_object_target()] };
        let expected: Vec<Act> = targets.iter()
            .filter_map(|t| match t {
                Target::Object(id) if b.get(*id).zone == Zone::Graveyard => Some(Act::Move(*id, Zone::Hand)),
                _ => None,
            })
            .collect();
        let before = b.resolve(&targets);
        assert!(asked_for(&expected) && Snapshot::of(&b.state) == before);
        std::mem::forget((b, targets));
    }
}

card_proof! {
    /// Memory's Journey: a player, and up to three cards from that player's
    /// graveyard; each target card still in that graveyard put into its
    /// owner's library, and then that library shuffled. With its player
    /// target gone illegal, nothing.
    fn memorys_journey_shuffles_its_targets_into_their_library() {
        let mut b = spell(Box::new(isd::memorys_journey::MemorysJourney));
        assert!(matches!(&*b.requirement(), R::TwoTargets(player, cards)
            if matches!(**player, R::PlayerOnly)
                && matches!(&**cards, R::UpToTargets(3, card) if matches!(**card, R::GraveyardCardOwnedByTargetPlayer))));
        let player = match kani::any::<u8>() % 3 {
            0 => Target::Player(PlayerId(0)),
            1 => Target::Player(PlayerId(1)),
            _ => Target::Illegal,
        };
        let mut targets = vec![player.clone()];
        if kani::any() {
            targets.push(b.any_object_target());
        }
        let expected = match player {
            Target::Player(p) => {
                let mut acts: Vec<Act> = targets.iter()
                    .filter_map(|t| match t {
                        Target::Object(id) if b.get(*id).zone == Zone::Graveyard && b.get(*id).owner == p => {
                            Some(Act::ToLibrary(*id, LibraryPosition::Bottom))
                        }
                        _ => None,
                    })
                    .collect();
                acts.push(Act::Shuffle(p));
                acts
            }
            _ => vec![],
        };
        let before = b.resolve(&targets);
        assert!(asked_for(&expected) && Snapshot::of(&b.state) == before);
        std::mem::forget((b, targets));
    }
}

card_proof! {
    /// Sever the Bloodline: its target, if still on the battlefield, and
    /// every other creature on the battlefield with its name (CR 201.2),
    /// exiled; and nothing else. The token is named as the card is, or not.
    fn sever_the_bloodline_exiles_every_creature_with_its_targets_name() {
        let mut b = spell(Box::new(isd::sever_the_bloodline::SeverTheBloodline));
        assert!(*b.requirement() == R::Creature);
        let token = b.others[1];
        b.state.get_object_mut(token).expect("on the board").name = if kani::any() { "Blank".into() } else { "Spirit".into() };
        let target = b.any_object_target();
        let expected = match target {
            Target::Object(id) if b.on(id) => {
                let name = b.state.name_of(id, &b.registry);
                b.others.iter().copied()
                    .filter(|o| b.on(*o) && b.get(*o).card_types.contains(&CardType::Creature) && b.state.name_of(*o, &b.registry) == name)
                    .map(|o| Act::Move(o, Zone::Exile))
                    .collect()
            }
            _ => vec![],
        };
        let before = b.resolve(&[target]);
        assert!(asked_for(&expected) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Blasphemous Act: {1} less for each creature on the battlefield,
    /// from its generic part only (CR 601.2f), so never below {R}; 13
    /// damage from it to each creature, all at once; and nothing else.
    fn blasphemous_act_costs_less_and_deals_thirteen_to_each_creature() {
        let mut b = spell(Box::new(isd::blasphemous_act::BlasphemousAct));
        let creatures: Vec<ObjectId> = b.others.iter().copied()
            .filter(|id| b.on(*id) && b.get(*id).card_types.contains(&CardType::Creature)).collect();
        let cost = isd::blasphemous_act::BlasphemousAct.modified_cost(&b.state, &b.registry);
        let n = u32::try_from(creatures.len()).expect("small");
        match cost {
            None => {
                assert!(n == 0);
            }
            Some(c) => {
                let generic: u32 = c.symbols.iter().map(|s| match s { ManaSymbol::Generic(g) => *g, _ => 0 }).sum();
                let red = c.symbols.iter().filter(|s| **s == ManaSymbol::Colored(crate::types::Color::Red)).count();
                assert!(n > 0 && generic == 8 - n.min(8) && red == 1 && c.symbols.len() <= 2);
            }
        }
        let before = b.resolve(&[]);
        let mut expected: Vec<Act> = creatures.into_iter()
            .map(|id| Act::Queue { source: b.spell, target: DamageTarget::Object(id), amount: 13, kind: DamageKind::NonCombat })
            .collect();
        expected.push(Act::DealQueued);
        assert!(asked_for(&expected) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Into the Maw of Hell: a land and a creature (CR 115.1); the land
    /// destroyed and the creature dealt 13 damage by the spell, each if it
    /// is still on the battlefield, either one without the other.
    fn into_the_maw_of_hell_destroys_a_land_and_deals_thirteen() {
        let mut b = spell(Box::new(isd::into_the_maw_of_hell::IntoTheMawOfHell));
        assert!(matches!(&*b.requirement(), R::TwoTargets(land, creature)
            if matches!(&**land, R::PermanentWithFilter(TargetFilter::HasCardType(types)) if types.as_slice() == [CardType::Land])
                && matches!(**creature, R::Creature)));
        let land = if kani::any() { Target::Object(b.others[0]) } else { Target::Illegal };
        let creature = if kani::any() { Target::Object(b.others[1]) } else { Target::Illegal };
        let mut expected = Vec::with_capacity(2);
        if let Target::Object(id) = land {
            if b.on(id) {
                expected.push(Act::Destroy(id));
            }
        }
        if let Target::Object(id) = creature {
            if b.on(id) {
                expected.push(Act::Effect(Target::Object(id), PendingEffect::DealDamage { amount: 13, source_id: b.spell }));
            }
        }
        let before = b.resolve(&[land, creature]);
        assert!(asked_for(&expected) && Snapshot::of(&b.state) == before);
        std::mem::forget((b, expected));
    }
}

card_proof! {
    /// Lost in the Mist: a spell and a permanent; the spell countered (CR
    /// 701.6a; whether it still is one is the action's), and the permanent,
    /// if still on the battlefield, returned to its owner's hand; either
    /// without the other.
    fn lost_in_the_mist_counters_and_bounces() {
        let mut b = spell(Box::new(isd::lost_in_the_mist::LostInTheMist));
        assert!(matches!(&*b.requirement(), R::TwoTargets(spell, permanent)
            if matches!(**spell, R::Spell) && matches!(**permanent, R::PermanentWithFilter(TargetFilter::Any))));
        let blank = b.get(b.others[0]).card_id;
        let countered = b.state.create_object(blank, PlayerId(1), Zone::Stack, None, None);
        let first = if kani::any() { Target::Object(countered) } else { Target::Illegal };
        let second = b.any_object_target();
        let mut expected = Vec::with_capacity(2);
        if let Target::Object(id) = first {
            expected.push(Act::Counter { spell: id, exile: false });
        }
        if let Target::Object(id) = second {
            if b.on(id) {
                expected.push(Act::Move(id, Zone::Hand));
            }
        }
        let before = b.resolve(&[first, second]);
        assert!(asked_for(&expected) && Snapshot::of(&b.state) == before);
        std::mem::forget((b, expected));
    }
}

card_proof! {
    /// Runic Repetition: "target exiled card with flashback you own" — in
    /// exile, its caster's, and a card whose face has flashback (Think
    /// Twice has, Lightning Bolt has not); it returned to its owner's hand.
    fn runic_repetition_returns_an_exiled_flashback_card() {
        let mut b = spell(Box::new(isd::runic_repetition::RunicRepetition));
        assert!(*b.requirement() == R::ExileCard);
        let twice = b.registry.register(Box::new(isd::think_twice::ThinkTwice));
        let bolt = b.registry.register(Box::new(crate::cards::lightning_bolt::LightningBolt));
        let flashback: bool = kani::any();
        let owner = PlayerId(u8::from(kani::any::<bool>()));
        let exiled: bool = kani::any();
        let card = b.state.create_object(if flashback { twice } else { bolt }, owner, if exiled { Zone::Exile } else { Zone::Graveyard }, None, None);
        let caster = PlayerId(u8::from(kani::any::<bool>()));
        let valid = isd::runic_repetition::RunicRepetition.is_valid_target(&b.state, caster, &Target::Object(card), &b.registry);
        assert!(valid == (exiled && owner == caster && flashback));
        assert!(!isd::runic_repetition::RunicRepetition.is_valid_target(&b.state, caster, &Target::Player(caster), &b.registry));
        let target = if kani::any() { Target::Object(card) } else { Target::Illegal };
        let before = b.resolve(&[target.clone()]);
        if target == Target::Object(card) {
            assert!(asked_for(&[Act::Move(card, Zone::Hand)]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Night Terrors: "target player"; the spell's controller chooses a
    /// nonland card of that player's hand (the only one without being
    /// asked), which is exiled; with none, nothing. Over a hand of up to
    /// two cards, each Grizzly Bears or a Forest.
    fn night_terrors_exiles_a_nonland_card_you_choose_from_their_hand() {
        let mut b = spell(Box::new(isd::night_terrors::NightTerrors));
        assert!(*b.requirement() == R::PlayerOnly);
        let bears = b.registry.register(Box::new(crate::cards::grizzly_bears::GrizzlyBears));
        let forest = b.registry.register(Box::new(crate::cards::forest::Forest));
        let player = PlayerId(u8::from(kani::any::<bool>()));
        let n: u8 = kani::any();
        kani::assume(n <= 2);
        let mut nonland = Vec::with_capacity(2);
        for _ in 0..n {
            let land: bool = kani::any();
            let id = b.state.create_object(if land { forest } else { bears }, player, Zone::Hand, None, None);
            if !land {
                nonland.push(id);
            }
        }
        let target = if kani::any() { Target::Player(player) } else { Target::Illegal };
        let before = b.resolve(&[target.clone()]);
        match (target, nonland.as_slice()) {
            (Target::Player(_), [only]) => {
                assert!(asked_for(&[Act::Move(*only, Zone::Exile)]));
            }
            (Target::Player(_), [a, c]) => {
                assert!(asked_for(&[Act::Choose {
                    source: b.spell, player: PlayerId(0), options: vec![Target::Object(*a), Target::Object(*c)],
                    effect: PendingEffect::CardEffect { source_id: b.spell, key: String::new() }, optional: false,
                }]));
                let chosen = if kani::any() { *a } else { *c };
                let card = b.get(b.spell).card_id;
                b.registry.get(card).expect("registered").resolve_card_effect(&mut b.state, b.spell, "", &Target::Object(chosen), &b.registry);
                assert!(super::act(1) == Some(&Act::Move(chosen, Zone::Exile)) && acted() == 2);
            }
            _ => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget((b, nonland));
    }
}

card_proof! {
    /// Tribute to Hunger: "target opponent", who chooses a creature they
    /// control and must sacrifice it (the only one is still theirs to
    /// choose: the engine applies a one-option choice, `proofs::actions`);
    /// with none, nothing. Then the spell's controller gains life equal to
    /// that creature's toughness as it was sacrificed, none for 0.
    fn tribute_to_hunger_makes_them_sacrifice_and_you_gain_its_toughness() {
        let mut b = spell(Box::new(isd::tribute_to_hunger::TributeToHunger));
        assert!(*b.requirement() == R::OpponentOnly);
        let target = if kani::any() { Target::Player(PlayerId(1)) } else { Target::Illegal };
        let theirs: Vec<ObjectId> = b.others.iter().copied().filter(|id| {
            let o = b.get(*id);
            o.zone == Zone::Battlefield && o.controller == PlayerId(1) && o.card_types.contains(&CardType::Creature)
        }).collect();
        let before = b.resolve(&[target.clone()]);
        if target == Target::Illegal || theirs.is_empty() {
            assert!(acted() == 0);
        } else {
            let options: Vec<Target> = theirs.iter().map(|id| Target::Object(*id)).collect();
            assert!(asked_for(&[Act::Choose {
                source: b.spell, player: PlayerId(1), options,
                effect: PendingEffect::CardEffect { source_id: b.spell, key: String::new() }, optional: false,
            }]));
            let chosen = theirs[usize::from(kani::any::<bool>()) % theirs.len()];
            let toughness = b.state.effective_toughness(chosen, &b.registry).unwrap_or(0);
            let card = b.get(b.spell).card_id;
            b.registry.get(card).expect("registered").resolve_card_effect(&mut b.state, b.spell, "", &Target::Object(chosen), &b.registry);
            assert!(super::act(1) == Some(&Act::Sacrifice(chosen)));
            if toughness > 0 {
                assert!(acted() == 3 && super::act(2) == Some(&Act::Life(PlayerId(0), toughness)));
            } else {
                assert!(acted() == 2);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget((b, theirs));
    }
}
