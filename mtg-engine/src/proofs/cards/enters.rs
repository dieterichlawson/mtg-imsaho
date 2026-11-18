//! The permanents with an ability that triggers as they enter the
//! battlefield (CR 603.6a), or a choice made as they enter (614.12).
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
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

use super::{
    acted, any_object, board, card_proof, triggers, Form,
    Kinds, Snapshot,
};
use crate::actions::Target;
use crate::cards::{CardBehavior, CardRegistry, TargetFilter, TargetRequirement as R, TriggerKind};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameState, TemporaryEffect as T};
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
    any_object(&mut state, blank, &ANY, Form::Card);
    any_object(&mut state, blank, &ANY, Form::Token);
    Board { registry, state, me, you }
}

impl Board {
    fn card(&self) -> CardId {
        self.state.get_object(self.me).expect("on the board").card_id
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
    village_bell_ringer_declares_its_trigger: isd::village_bell_ringer::VillageBellRinger => None, false;
    hollowhenge_scavenger_declares_its_morbid_trigger: isd::hollowhenge_scavenger::HollowhengeScavenger => None, true;
    morkrut_banshee_declares_its_morbid_trigger: isd::morkrut_banshee::MorkrutBanshee => Some(R::Creature), true;
    woodland_sleuth_declares_its_morbid_trigger: isd::woodland_sleuth::WoodlandSleuth => None, true;
    ghoulraiser_declares_its_trigger: isd::ghoulraiser::Ghoulraiser => None, false;
    snapcaster_mage_declares_its_trigger: isd::snapcaster_mage::SnapcasterMage => Some(R::GraveyardCardOwnedByCaster), false;
    witchbane_orb_declares_its_trigger: isd::witchbane_orb::WitchbaneOrb => None, false;
}

card_proof! {
    /// Slayer of the Wicked: as [`declares_an_enters_trigger`] says, its
    /// target a Vampire, Werewolf or Zombie creature, unconditioned. The
    /// target is matched as a pattern, not compared with a requirement
    /// built and handed in: two target requirements read from the heap,
    /// one holding a list of names, are what CBMC ran out of memory on
    /// (`prints_effects` says why), and only this card's held a list.
    fn slayer_of_the_wicked_declares_its_trigger() {
        let card = isd::slayer_of_the_wicked::SlayerOfTheWicked;
        assert!(card.has_etb_handler());
        let declared = std::mem::ManuallyDrop::new(triggers(&card));
        assert!(matches!(declared.as_slice(), [(TriggerKind::EntersBattlefield,
            Some(R::CreatureWithFilter(TargetFilter::SubtypeOrCardType { subtypes, card_types })))]
            if card_types.is_empty() && subtypes.len() == 3
                && subtypes[0] == "Vampire" && subtypes[1] == "Werewolf" && subtypes[2] == "Zombie"));
        let (registry, mut state, _) = super::blank_board();
        state.creature_died_this_turn = kani::any();
        assert!(card.should_trigger(&state, ObjectId(1), &TriggerKind::EntersBattlefield, &registry));
        std::mem::forget((registry, state));
    }
}

// A graveyard of these boards holds at most four cards (the card's own,
// the card beside it, the Zombie and the blank), and the cards' scans of
// it go round at most four times: a bound of five, not the harness's
// seven, which ran Ghoulraiser's and Woodland Sleuth's proofs out of
// memory in the solver at seven gigabytes.
// loop bound: <cards::isd::ghoulraiser::Ghoulraiser as cards::CardBehavior>::on_enter_battlefield 5 ghoulraiser
// loop bound: <cards::isd::woodland_sleuth::WoodlandSleuth as cards::CardBehavior>::on_enter_battlefield 5 woodland_sleuth

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

