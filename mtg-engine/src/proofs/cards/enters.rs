//! The permanents with an ability that triggers as they enter the
//! battlefield (CR 603.6a), or a choice made as they enter (614.12).
//!
//! - Armored Skaab: "When this creature enters, mill four cards."
//! - Crossway Vampire: "When this creature enters, target creature can't
//!   block this turn."
//! - Geistcatcher's Rig: "When this creature enters, you may have it deal 4
//!   damage to target creature with flying."
//! - Slayer of the Wicked: "When this creature enters, you may destroy
//!   target Vampire, Werewolf, or Zombie."
//! - Village Bell-Ringer: "Flash. When this creature enters, untap all
//!   creatures you control."
//! - Hollowhenge Scavenger: "Morbid — When this creature enters, if a
//!   creature died this turn, you gain 5 life."
//! - Morkrut Banshee: "Morbid — When this creature enters, if a creature
//!   died this turn, target creature gets -4/-4 until end of turn."
//! - Woodland Sleuth: "Morbid — When this creature enters, if a creature
//!   died this turn, return a creature card at random from your graveyard
//!   to your hand."
//! - Ghoulraiser: "When this creature enters, return a Zombie card at random
//!   from your graveyard to your hand."
//! - Snapcaster Mage: "Flash. When this creature enters, target instant or
//!   sorcery card in your graveyard gains flashback until end of turn. The
//!   flashback cost is equal to its mana cost."
//! - Witchbane Orb: "When this artifact enters, destroy all Curses attached
//!   to you. You have hexproof."
//! - Fiend Hunter: "When this creature enters, you may exile another target
//!   creature. When this creature leaves the battlefield, return the
//!   exiled card to the battlefield under its owner's control."
//!
//! The engine puts an enters trigger on the stack for a permanent whose
//! card says it has one and whose condition holds (CR 603.4: an
//! intervening "if" is checked as it triggers and again as it resolves),
//! with the target its card declares; each proof pins those, and runs the
//! card's code for the trigger resolving, the permanent on the
//! battlefield or gone from it by then (CR 113.7a: the ability resolves
//! all the same), under either player's control, beside a card and a
//! token of any characteristics.

use super::{act, acted, any_object, any_target, asked_for, board, card_proof, triggers, Act, Form, Kinds, Snapshot};
use crate::actions::Target;
use crate::cards::{CardBehavior, CardRegistry, TargetFilter, TargetRequirement as R, TriggerKind};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameObject, GameState, PendingEffect, TemporaryEffect as T};
use crate::types::{CardType, Keyword, Zone};

use crate::cards::isd;

const ANY: Kinds = Kinds { types: &[CardType::Creature, CardType::Enchantment], subtypes: &["Curse"], keywords: &[] };

/// The board: the card's permanent, owned by player 0, on the battlefield
/// or gone to their graveyard, controlled (or last controlled) by `you`;
/// a card and a token beside it.
struct Board {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    others: [ObjectId; 2],
    you: PlayerId,
}

fn entered(card: Box<dyn CardBehavior>) -> Board {
    let (registry, mut state, blank, card) = board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let gone: bool = kani::any();
    let me = state.create_object(card, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Battlefield }, None, None);
    let o = state.get_object_mut(me).expect("just created");
    if gone {
        o.last_controller = Some(you);
        state.players[0].graveyard_order.push(me);
    } else {
        o.controller = you;
    }
    state.creature_died_this_turn = kani::any();
    let others = [any_object(&mut state, blank, &ANY, Form::Card), any_object(&mut state, blank, &ANY, Form::Token)];
    Board { registry, state, me, others, you }
}

impl Board {
    fn card(&self) -> CardId {
        self.state.get_object(self.me).expect("on the board").card_id
    }

    fn get(&self, id: ObjectId) -> &GameObject {
        self.state.get_object(id).expect("on the board")
    }

    fn those(&self, pred: impl Fn(&GameObject) -> bool) -> Vec<ObjectId> {
        self.others.iter().copied().filter(|id| pred(self.get(*id))).collect()
    }

    /// Those of `cards` in `you`'s graveyard that `is` holds for, in the
    /// graveyard's order (CR 404.2), which is the order the board put them
    /// there and so the order given.
    fn in_your_graveyard(&self, cards: [ObjectId; 4], is: impl Fn(ObjectId) -> bool) -> Vec<ObjectId> {
        cards.into_iter().filter(|id| self.get(*id).zone == Zone::Graveyard && self.get(*id).owner == self.you && is(*id)).collect()
    }

    /// The enters trigger resolving with `targets`; the board before.
    fn resolve(&mut self, targets: &[Target]) -> Snapshot {
        let before = Snapshot::of(&self.state);
        let card = self.card();
        self.registry.get(card).expect("registered").on_enter_battlefield(&mut self.state, self.me, targets, &self.registry);
        before
    }
}

/// `card` has an enters trigger the engine puts on the stack (CR 603.6a),
/// targeting as `target` says, and with no other triggered ability; and,
/// if `morbid`, it triggers only if a creature died this turn (CR 603.4),
/// any other event it watches being unconditioned.
fn declares_an_enters_trigger(card: &dyn CardBehavior, target: Option<R>, morbid: bool) {
    assert!(card.has_etb_handler());
    assert!(triggers(card) == [(TriggerKind::EntersBattlefield, target)]);
    let (registry, mut state, _) = super::blank_board();
    let died: bool = kani::any();
    state.creature_died_this_turn = died;
    assert!(card.should_trigger(&state, ObjectId(1), &TriggerKind::EntersBattlefield, &registry) == (died || !morbid));
    std::mem::forget((registry, state));
}

macro_rules! declares {
    ($($name:ident: $card:expr => $target:expr, $morbid:expr);* $(;)?) => {
        $(
            card_proof! {
                fn $name() {
                    declares_an_enters_trigger(&$card, $target, $morbid);
                }
            }
        )*
    };
}

declares! {
    armored_skaab_declares_its_trigger: isd::armored_skaab::ArmoredSkaab => None, false;
    crossway_vampire_declares_its_trigger: isd::crossway_vampire::CrosswayVampire => Some(R::Creature), false;
    geistcatchers_rig_declares_its_trigger: isd::geistcatchers_rig::GeistcatchersRig
        => Some(R::CreatureWithFilter(TargetFilter::HasKeyword(Keyword::Flying))), false;
    slayer_of_the_wicked_declares_its_trigger: isd::slayer_of_the_wicked::SlayerOfTheWicked
        => Some(R::CreatureWithFilter(TargetFilter::SubtypeOrCardType {
            subtypes: vec!["Vampire".into(), "Werewolf".into(), "Zombie".into()], card_types: vec![],
        })), false;
    village_bell_ringer_declares_its_trigger: isd::village_bell_ringer::VillageBellRinger => None, false;
    hollowhenge_scavenger_declares_its_morbid_trigger: isd::hollowhenge_scavenger::HollowhengeScavenger => None, true;
    morkrut_banshee_declares_its_morbid_trigger: isd::morkrut_banshee::MorkrutBanshee => Some(R::Creature), true;
    woodland_sleuth_declares_its_morbid_trigger: isd::woodland_sleuth::WoodlandSleuth => None, true;
    ghoulraiser_declares_its_trigger: isd::ghoulraiser::Ghoulraiser => None, false;
    snapcaster_mage_declares_its_trigger: isd::snapcaster_mage::SnapcasterMage => Some(R::GraveyardCardOwnedByCaster), false;
    witchbane_orb_declares_its_trigger: isd::witchbane_orb::WitchbaneOrb => None, false;
}

card_proof! {
    /// Armored Skaab: its controller mills four cards.
    fn armored_skaab_mills_four() {
        let mut b = entered(Box::new(isd::armored_skaab::ArmoredSkaab));
        let before = b.resolve(&[]);
        assert!(asked_for(&[Act::Mill(b.you, 4)]) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Crossway Vampire: its target can't block this turn (whether it is
    /// still a creature to stop is the effect's).
    fn crossway_vampire_stops_its_target_blocking() {
        let mut b = entered(Box::new(isd::crossway_vampire::CrosswayVampire));
        let target = any_target(b.others);
        let before = b.resolve(&[target.clone()]);
        assert!(asked_for(&[Act::Effect(target, PendingEffect::CantBlockThisTurn { source_name: "Crossway Vampire".into() })]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Geistcatcher's Rig: its controller may have it deal 4 damage to its
    /// target, and is asked whether to.
    fn geistcatchers_rig_may_deal_four_to_its_target() {
        let mut b = entered(Box::new(isd::geistcatchers_rig::GeistcatchersRig));
        let target = any_target(b.others);
        let before = b.resolve(&[target.clone()]);
        assert!(asked_for(&[Act::Choose {
            source: b.me, player: b.you, options: vec![target], optional: true,
            effect: PendingEffect::DealDamage { amount: 4, source_id: b.me },
        }]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Slayer of the Wicked: its controller may destroy its target, and is
    /// asked whether to.
    fn slayer_of_the_wicked_may_destroy_its_target() {
        let mut b = entered(Box::new(isd::slayer_of_the_wicked::SlayerOfTheWicked));
        let target = any_target(b.others);
        let before = b.resolve(&[target.clone()]);
        assert!(asked_for(&[Act::Choose {
            source: b.me, player: b.you, options: vec![target], optional: true,
            effect: PendingEffect::Destroy { source_name: "Slayer of the Wicked".into() },
        }]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Village Bell-Ringer: each tapped creature its controller controls
    /// untapped, itself among them if it is there; and nothing else.
    fn village_bell_ringer_untaps_your_creatures() {
        let mut b = entered(Box::new(isd::village_bell_ringer::VillageBellRinger));
        for id in [b.me, b.others[0], b.others[1]] {
            b.state.get_object_mut(id).expect("on the board").tapped = kani::any();
        }
        let me_too = {
            let o = b.get(b.me);
            o.zone == Zone::Battlefield && o.controller == b.you && o.tapped
        };
        let mut expected = Vec::with_capacity(3);
        if me_too {
            expected.push(Act::Untap(b.me));
        }
        expected.extend(b.those(|o| {
            o.zone == Zone::Battlefield && o.controller == b.you && o.tapped && o.card_types.contains(&CardType::Creature)
        }).into_iter().map(Act::Untap));
        let before = b.resolve(&[]);
        assert!(asked_for(&expected) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Hollowhenge Scavenger: 5 life for its controller if a creature died
    /// this turn, checked again as it resolves; nothing otherwise.
    fn hollowhenge_scavenger_gains_five_after_a_death() {
        let mut b = entered(Box::new(isd::hollowhenge_scavenger::HollowhengeScavenger));
        let died = b.state.creature_died_this_turn;
        let before = b.resolve(&[]);
        if died {
            assert!(asked_for(&[Act::Life(b.you, 5)]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Morkrut Banshee: -4/-4 until end of turn on its target if a creature
    /// died this turn; nothing otherwise.
    fn morkrut_banshee_shrinks_its_target_after_a_death() {
        let mut b = entered(Box::new(isd::morkrut_banshee::MorkrutBanshee));
        let died = b.state.creature_died_this_turn;
        let target = any_target(b.others);
        let before = b.resolve(&[target.clone()]);
        if died {
            assert!(asked_for(&[Act::Effect(target, PendingEffect::DebuffUntilEOT { power: -4, toughness: -4, source_name: "Morkrut Banshee".into() })]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

/// A board for the cards that return a card at random from a graveyard:
/// the card's permanent as in [`entered`], and in the graveyards a Zombie
/// creature card (Diregraf Ghoul) and the blank card, each either player's,
/// the blank a creature or not.
fn with_graveyard(card: Box<dyn CardBehavior>) -> (Board, [ObjectId; 2]) {
    let mut b = entered(card);
    let ghoul = b.registry.register(Box::new(isd::diregraf_ghoul::DiregrafGhoul));
    let blank = CardId(1);
    let mut cards = [ObjectId(0); 2];
    for (i, card) in [ghoul, blank].into_iter().enumerate() {
        let owner: bool = kani::any();
        let id = b.state.create_object(card, PlayerId(u8::from(owner)), Zone::Graveyard, None, None);
        if owner {
            b.state.players[1].graveyard_order.push(id);
        } else {
            b.state.players[0].graveyard_order.push(id);
        }
        cards[i] = id;
    }
    if kani::any() {
        b.state.get_object_mut(cards[1]).expect("just created").card_types = vec![CardType::Creature];
    }
    (b, cards)
}

/// The card asked for `returnable` chosen at random, one of them, and that
/// one returned to its owner's hand; or, with none, for nothing.
fn returned_one_at_random(returnable: &[ObjectId]) -> bool {
    if returnable.is_empty() {
        return acted() == 0;
    }
    let Some(Act::Move(id, Zone::Hand)) = act(1) else { return false };
    acted() == 2 && act(0) == Some(Act::Random(returnable.to_vec(), 1)) && returnable.contains(&id)
}

card_proof! {
    /// Ghoulraiser: a Zombie card in its controller's graveyard, chosen at
    /// random, returned to their hand; with none, nothing.
    fn ghoulraiser_returns_a_zombie_card_at_random() {
        let (mut b, cards) = with_graveyard(Box::new(isd::ghoulraiser::Ghoulraiser));
        // Ghoulraiser is itself a Zombie: its own card, if it is in that
        // graveyard by then, is one to choose from. The blank cards print
        // no subtype.
        let returnable = b.in_your_graveyard([b.me, b.others[0], cards[0], cards[1]], |id| id == b.me || id == cards[0]);
        let before = b.resolve(&[]);
        assert!(returned_one_at_random(&returnable) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Woodland Sleuth: if a creature died this turn, a creature card in
    /// its controller's graveyard, chosen at random, returned to their
    /// hand; with none, or with no death, nothing.
    fn woodland_sleuth_returns_a_creature_card_at_random_after_a_death() {
        let (mut b, cards) = with_graveyard(Box::new(isd::woodland_sleuth::WoodlandSleuth));
        let died = b.state.creature_died_this_turn;
        // The Sleuth and Diregraf Ghoul print "Creature"; a blank card is
        // one if it was made one.
        let returnable = b.in_your_graveyard([b.me, b.others[0], cards[0], cards[1]], |id| {
            id == b.me || id == cards[0] || b.get(id).card_types.contains(&CardType::Creature)
        });
        let before = b.resolve(&[]);
        if died {
            assert!(returned_one_at_random(&returnable));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Snapcaster Mage: its target an instant or sorcery card (CR 115.1),
    /// which gains flashback until end of turn at its mana cost. Over an
    /// instant card (Lightning Bolt), a sorcery card (Lava Axe) and the
    /// blank card, which is neither.
    fn snapcaster_mage_gives_its_target_flashback_at_its_mana_cost() {
        let mut b = entered(Box::new(isd::snapcaster_mage::SnapcasterMage));
        let bolt = b.registry.register(Box::new(crate::cards::lightning_bolt::LightningBolt));
        let axe = b.registry.register(Box::new(crate::cards::lava_axe::LavaAxe));
        let which = kani::any::<u8>() % 3;
        let card = match which { 0 => bolt, 1 => axe, _ => CardId(1) };
        let target = b.state.create_object(card, b.you, Zone::Graveyard, None, None);
        let snapcaster = isd::snapcaster_mage::SnapcasterMage;
        assert!(snapcaster.is_valid_target(&b.state, b.you, &Target::Object(target), &b.registry) == (which < 2));
        assert!(!snapcaster.is_valid_target(&b.state, b.you, &Target::Player(b.you), &b.registry));
        if which < 2 {
            let data = std::mem::ManuallyDrop::new(b.registry.get(card).expect("registered").card_data());
            let cost = data.cost.clone().expect("a cost");
            let before = b.resolve(&[Target::Object(target)]);
            let added = &b.state.until_end_of_turn[before.until_end_of_turn()..];
            assert!(added == [T::GrantFlashback { target, cost }] && acted() == 0);
            assert!(Snapshot::of(&b.state).without_until_end_of_turn() == before.without_until_end_of_turn());
        }
        std::mem::forget(b);
    }
}

card_proof! {
    /// Witchbane Orb: its controller has hexproof (CR 702.11d); every Curse
    /// on the battlefield attached to its controller destroyed, at once,
    /// and no other permanent.
    fn witchbane_orb_destroys_the_curses_on_you() {
        assert!(isd::witchbane_orb::WitchbaneOrb.grants_player_hexproof());
        let mut b = entered(Box::new(isd::witchbane_orb::WitchbaneOrb));
        let curse = b.registry.register(Box::new(isd::curse_of_deaths_hold::CurseOfDeathsHold));
        let on = b.state.create_object(curse, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, None, None);
        let attached_to = PlayerId(u8::from(kani::any::<bool>()));
        b.state.get_object_mut(on).expect("just created").attached_to_player = Some(attached_to);
        // A blank card attached to a player is no Curse.
        let other = b.others[0];
        if kani::any() {
            b.state.get_object_mut(other).expect("on the board").attached_to_player = Some(b.you);
        }
        let mut curses = Vec::with_capacity(1);
        if attached_to == b.you {
            curses.push(on);
        }
        let before = b.resolve(&[]);
        let Some(Act::DestroyAll(outcomes)) = act(0) else { panic!("destroys") };
        assert!(acted() == 1 && outcomes.iter().map(|(id, _)| *id).eq(curses.iter().copied()));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Fiend Hunter's triggers: as it enters, "another target creature"
    /// (CR 603.3d: chosen as it triggers); as it leaves, none.
    fn fiend_hunter_declares_its_triggers() {
        let hunter = isd::fiend_hunter::FiendHunter;
        assert!(hunter.has_etb_handler());
        assert!(triggers(&hunter) == [
            (TriggerKind::EntersBattlefield, Some(R::CreatureWithFilter(TargetFilter::Another))),
            (TriggerKind::LeavesBattlefield, None),
        ]);
    }
}

card_proof! {
    /// Fiend Hunter entering: its controller may exile its target ("you
    /// may": asked, with that target the one choice, and declining an
    /// answer); on yes, the target exiled, and the Hunter remembers which
    /// card it was, whether it is still on the battlefield or not (if it
    /// has left first, its leaves trigger has already found nothing, and
    /// the card stays exiled: the card's ruling). With no target (the
    /// trigger's target illegal, it does not resolve at all, CR 608.2b),
    /// nothing. Nothing else.
    fn fiend_hunter_may_exile_its_target() {
        let mut b = entered(Box::new(isd::fiend_hunter::FiendHunter));
        let target = if kani::any() { Some(Target::Object(b.others[usize::from(kani::any::<bool>())])) } else { None };
        let before = b.resolve(target.as_slice());
        let Some(target) = target else {
            assert!(acted() == 0 && Snapshot::of(&b.state) == before);
            std::mem::forget(b);
            return;
        };
        assert!(acted() == 1 && Snapshot::of(&b.state) == before);
        let Some(Act::Choose { source, player, options, effect: PendingEffect::CardEffect { source_id, key }, optional: true }) = act(0) else {
            panic!("asks whether to exile it")
        };
        assert!(source == b.me && player == b.you && source_id == b.me && key.is_empty() && options.len() == 1 && options[0] == target);
        let Target::Object(id) = target else { unreachable!() };
        let card = b.card();
        b.registry.get(card).expect("registered").resolve_card_effect(&mut b.state, b.me, "", &target, &b.registry);
        assert!(acted() == 2 && act(1) == Some(Act::Move(id, Zone::Exile)));
        assert!(b.get(b.me).card_state.get("exiled_creature") == Some(&id) && b.get(b.me).card_state.len() == 1);
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget((b, options, key));
    }
}

card_proof! {
    /// Fiend Hunter leaving: the card it exiled, if it is still in exile,
    /// returned to the battlefield under its owner's control (CR 110.2a);
    /// if it has left exile, or the Hunter exiled nothing, nothing.
    fn fiend_hunter_returns_the_exiled_card_under_its_owners_control() {
        let (registry, mut state, blank, card) = board(Box::new(isd::fiend_hunter::FiendHunter));
        let me = state.create_object(card, PlayerId(0), Zone::Graveyard, None, None);
        state.players[0].graveyard_order.push(me);
        state.get_object_mut(me).expect("just created").last_controller = Some(PlayerId(u8::from(kani::any::<bool>())));
        let owner = PlayerId(u8::from(kani::any::<bool>()));
        let exiled = state.create_object(blank, owner, if kani::any() { Zone::Exile } else { Zone::Hand }, None, None);
        let remembered: bool = kani::any();
        if remembered {
            state.get_object_mut(me).expect("just created").card_state.insert("exiled_creature".into(), exiled);
        }
        let still_exiled = state.get_object(exiled).expect("on the board").zone == Zone::Exile;
        let before = Snapshot::of(&state);
        registry.get(card).expect("registered").on_leave_battlefield(&mut state, me, &registry);
        if remembered && still_exiled {
            assert!(asked_for(&[Act::MoveUnder(exiled, Zone::Battlefield, owner)]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}
