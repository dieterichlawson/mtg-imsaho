//! The permanents with activated abilities (CR 602): each ability as the
//! engine reads it to offer and pay for it, pinned to its Oracle text, and
//! what the ability does as it resolves.
//!
//! - Darkthicket Wolf: "{2}{G}: This creature gets +2/+2 until end of turn.
//!   Activate only once each turn."
//! - Kessig Wolf: "{1}{R}: This creature gains first strike until end of
//!   turn."
//! - Feral Ridgewolf: "Trample. {1}{R}: This creature gets +2/+0 until end
//!   of turn."
//! - Manor Skeleton: "Haste. {1}{B}: Regenerate this creature."
//! - Lantern Spirit: "Flying. {U}: Return this creature to its owner's
//!   hand."
//! - Manor Gargoyle: "Defender. This creature has indestructible as long as
//!   it has defender. {1}: Until end of turn, this creature loses defender
//!   and gains flying."
//! - Selfless Cathar: "{1}{W}, Sacrifice this creature: Creatures you
//!   control get +1/+1 until end of turn."
//! - Silverchase Fox: "{1}{W}, Sacrifice this creature: Exile target
//!   enchantment."
//! - Brain Weevil: "Intimidate. Sacrifice this creature: Target player
//!   discards two cards. Activate only as a sorcery."
//! - Elder of Laurels: "{3}{G}: Target creature gets +X/+X until end of
//!   turn, where X is the number of creatures you control."
//! - Disciple of Griselbrand: "{1}, Sacrifice a creature: You gain life
//!   equal to the sacrificed creature's toughness."
//! - Full Moon's Rise: "Werewolf creatures you control get +1/+0 and have
//!   trample. Sacrifice this enchantment: Regenerate all Werewolf creatures
//!   you control."
//! - Ghoulcaller's Bell: "{T}: Each player mills a card."
//! - Skirsdag Cultist: "{R}, {T}, Sacrifice a creature: This creature deals
//!   2 damage to any target."
//! - Skirsdag High Priest: "Morbid — {T}, Tap two untapped creatures you
//!   control: Create a 5/5 black Demon creature token with flying. Activate
//!   only if a creature died this turn."
//! - Stitcher's Apprentice: "{1}{U}, {T}: Create a 2/2 blue Homunculus
//!   creature token, then sacrifice a creature."
//! - Tree of Redemption: "Defender. {T}: Exchange your life total with this
//!   creature's toughness."
//! - Traveler's Amulet: "{1}, Sacrifice this artifact: Search your library
//!   for a basic land card, reveal it, put it into your hand, then
//!   shuffle."
//! - Mikaeus, the Lunarch: "{T}: Put a +1/+1 counter on Mikaeus. {T},
//!   Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other
//!   creature you control."
//! - Grimoire of the Dead: "{1}, {T}, Discard a card: Put a study counter
//!   on this artifact. {T}, Remove three study counters from this artifact
//!   and sacrifice it: Put all creature cards from all graveyards onto the
//!   battlefield under your control. They're black Zombies in addition to
//!   their other colors and types."
//! - Back from the Brink: "Exile a creature card from your graveyard and
//!   pay its mana cost: Create a token that's a copy of that card.
//!   Activate only as a sorcery."
//! - Graveyard Shovel: "{2}, {T}: Target player exiles a card from their
//!   graveyard. If it's a creature card, you gain 2 life."
//! - Gavony Township: "{T}: Add {C}. {2}{G}{W}, {T}: Put a +1/+1 counter
//!   on each creature you control."
//! - Ghost Quarter: "{T}: Add {C}. {T}, Sacrifice this land: Destroy target
//!   land. Its controller may search their library for a basic land card,
//!   put it onto the battlefield, then shuffle."
//! - Kessig Wolf Run: "{T}: Add {C}. {X}{R}{G}, {T}: Target creature gets
//!   +X/+0 and gains trample until end of turn."
//! - Moorland Haunt: "{T}: Add {C}. {W}{U}, {T}, Exile a creature card from
//!   your graveyard: Create a 1/1 white Spirit creature token with flying."
//! - Nephalia Drownyard: "{T}: Add {C}. {1}{U}{B}, {T}: Target player mills
//!   three cards."
//! - Stensia Bloodhall: "{T}: Add {C}. {3}{B}{R}, {T}: This land deals 2
//!   damage to target player or planeswalker."
//! - Grimgrin, Corpse-Born: "Sacrifice another creature: Untap Grimgrin
//!   and put a +1/+1 counter on it."
//! - Evil Twin's copy: "{U}{B}, {T}: Destroy target creature with the same
//!   name as this creature."
//! - Mirror-Mad Phantasm: "Flying. {1}{U}: This creature's owner shuffles
//!   it into their library. If that player does, they reveal cards from
//!   the top of that library until a card named Mirror-Mad Phantasm is
//!   revealed. The player puts that card onto the battlefield and all
//!   other cards revealed this way into their graveyard."
//!
//! An ability's costs are paid as it is activated (CR 602.2b), by the
//! engine, which `proofs::actions` proves pays exactly the costs pinned
//! here; what is proved here is the rest. Each resolution is over the
//! permanent on the battlefield or gone from it by then (sacrificed as a
//! cost, or killed in response: the ability resolves all the same, CR
//! 113.7a), under either player's control; "you" is the player who
//! activated it, its controller as it was activated (CR 113.8); and a card
//! and a token of any characteristics beside it.

use super::{acted, any_object, any_target, asked_each, asked_for, board, both, card_proof, case_by_case, each_case, each_is, has_subtype, new_ids, prints_where, target_at, were, Act, Form, Kinds, Snapshot};
use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::damage::DamageKind;
use crate::events::DamageTarget;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameObject, GameState, LibraryPosition, PendingEffect, ResolutionChoiceKind, TemporaryEffect as T};
use crate::types::{
    CardType, Color, ContinuousEffect as E, CounterType, CreatureFilter as F, EffectCondition as If, EffectScope, Keyword,
    ManaCost, ManaSymbol, Zone,
};

use crate::cards::isd;

/// An activated ability as the engine reads it to offer it and to pay for
/// it (CR 602.1, 602.2): its mana cost, whether {T} is part of the cost,
/// what it sacrifices, the counters it removes, how many untapped
/// creatures it taps, what it targets, and the limits on when it may be
/// activated. Its description is for people.
#[derive(Clone, Debug, PartialEq)]
pub struct Ability {
    pub cost: ManaCost,
    pub tap: bool,
    pub sacrifice: SacrificeCost,
    pub remove_counters: Option<(CounterType, u32)>,
    pub tap_creatures: Option<usize>,
    pub target: Option<TargetRequirement>,
    pub once_per_turn: bool,
    pub sorcery_speed: bool,
}

impl Default for Ability {
    fn default() -> Self {
        Ability {
            cost: ManaCost::free(),
            tap: false,
            sacrifice: SacrificeCost::None,
            remove_counters: None,
            tap_creatures: None,
            target: None,
            once_per_turn: false,
            sorcery_speed: false,
        }
    }
}

impl Ability {
    pub fn of(def: &ActivatedAbilityDef) -> Self {
        Ability {
            cost: def.cost.clone(),
            tap: def.requires_tap,
            sacrifice: def.sacrifice_cost.clone(),
            remove_counters: def.counter_cost,
            tap_creatures: def.tap_cost.map(|c| c.count),
            target: def.target_requirement.clone(),
            once_per_turn: def.once_per_turn,
            sorcery_speed: def.sorcery_speed_only,
        }
    }
}

/// A mana cost of `generic` and then `colored`.
fn mana(generic: u32, colored: &[Color]) -> ManaCost {
    let mut symbols = Vec::with_capacity(1 + colored.len());
    if generic > 0 {
        symbols.push(ManaSymbol::Generic(generic));
    }
    for c in colored {
        symbols.push(ManaSymbol::Colored(*c));
    }
    ManaCost::new(symbols)
}

/// The board: the card's permanent, on the battlefield or gone to its
/// owner's graveyard, controlled (or last controlled) by `you`, who
/// activated its ability; and a card and a token beside it.
struct Board {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    others: [ObjectId; 2],
    you: PlayerId,
}

const ANY_CREATURE: Kinds = Kinds { types: &[CardType::Creature, CardType::Enchantment], subtypes: &[], keywords: &[] };

fn permanent(card: Box<dyn CardBehavior>, kinds: &Kinds, gone: bool) -> Board {
    permanent_of(card, kinds, gone, PlayerId(u8::from(kani::any::<bool>())))
}

/// [`permanent`]'s board, controlled by `you`.
fn permanent_of(card: Box<dyn CardBehavior>, kinds: &Kinds, gone: bool, you: PlayerId) -> Board {
    permanent_at(card, kinds, gone && kani::any(), you)
}

/// [`permanent_of`]'s board, the permanent gone exactly if `gone`.
fn permanent_at(card: Box<dyn CardBehavior>, kinds: &Kinds, gone: bool, you: PlayerId) -> Board {
    let (registry, mut state, blank, card) = board(card);
    let me = state.create_object(card, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Battlefield }, None, None);
    let o = state.get_object_mut(me).expect("just created");
    if gone {
        o.last_controller = Some(you);
        state.players[0].graveyard_order.push(me);
    } else {
        o.controller = you;
    }
    let others = [any_object(&mut state, blank, kinds, Form::Card), any_object(&mut state, blank, kinds, Form::Token)];
    state.resolving_ability_activator = Some(you);
    Board { registry, state, me, others, you }
}

impl Board {
    fn card(&self) -> CardId {
        self.state.get_object(self.me).expect("on the board").card_id
    }

    fn on(&self) -> bool {
        self.state.get_object(self.me).expect("on the board").zone == Zone::Battlefield
    }

    fn those(&self, pred: impl Fn(&GameObject) -> bool) -> Vec<ObjectId> {
        self.others.iter().copied().filter(|id| pred(self.state.get_object(*id).expect("on the board"))).collect()
    }

    /// The card's ability `index` resolving with `targets`; the board
    /// before.
    fn resolve(&mut self, index: usize, targets: &[Target]) -> Snapshot {
        let before = Snapshot::of(&self.state);
        let card = self.card();
        self.registry.get(card).expect("registered").resolve_activated_ability(&mut self.state, self.me, index, targets, &self.registry);
        before
    }

    /// The until-end-of-turn effects added since `before`.
    fn added(&self, before: &Snapshot) -> &[T] {
        &self.state.until_end_of_turn[before.until_end_of_turn()..]
    }

    /// Nothing changed since `before` but by the actions asked for and the
    /// effects added.
    fn nothing_else_changed(&self, before: &Snapshot) -> bool {
        Snapshot::of(&self.state).without_until_end_of_turn() == before.without_until_end_of_turn()
    }
}

/// The card's abilities as the engine reads them with its permanent on
/// the battlefield, a creature having died this turn or not: exactly
/// `expected(died)`, each at its index.
fn has_abilities(card: Box<dyn CardBehavior>, expected: impl Fn(bool) -> Vec<Ability>) {
    has_abilities_when(card, kani::any(), &expected);
}

/// [`has_abilities`], a creature having died this turn if `died`.
fn has_abilities_when(card: Box<dyn CardBehavior>, died: bool, expected: &impl Fn(bool) -> Vec<Ability>) {
    let mut b = permanent(card, &ANY_CREATURE, false);
    b.state.creature_died_this_turn = died;
    let card = b.card();
    let defs = std::mem::ManuallyDrop::new(b.registry.get(card).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    let expected = std::mem::ManuallyDrop::new(expected(died));
    assert!(defs.len() == expected.len());
    for (i, (def, e)) in defs.iter().zip(expected.iter()).enumerate() {
        assert!(def.ability_index == i && Ability::of(def) == *e);
    }
    std::mem::forget(b);
}

/// One ability, whatever has died.
fn one(ability: Ability) -> impl Fn(bool) -> Vec<Ability> {
    move |_| vec![ability.clone()]
}

/// The card's one ability as the engine reads it with its permanent on
/// the battlefield: exactly `expected`, at `index` (a land's ability
/// after its mana ability, which is index 0, CR 605).
fn has_ability_at(card: Box<dyn CardBehavior>, index: usize, expected: Ability) {
    let b = permanent(card, &ANY_CREATURE, false);
    let card = b.card();
    let defs = std::mem::ManuallyDrop::new(b.registry.get(card).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    assert!(defs.len() == 1 && defs[0].ability_index == index && Ability::of(&defs[0]) == expected);
    std::mem::forget((b, expected));
}

macro_rules! abilities {
    ($($name:ident: $card:expr => $expected:expr),* $(,)?) => {
        $(
            card_proof! {
                fn $name() {
                    has_abilities(Box::new($card), $expected);
                }
            }
        )*
    };
}

/// Skirsdag High Priest's morbid ability, after a death and only then: a
/// harness for each, whether one died a constant in it. Its abilities are
/// a list of a length that decides, and a list expected of a length the
/// solver left open is read past its end, as the lists of expected actions
/// are: in one harness the proof ran out of memory at ten gigabytes.
fn skirsdag_high_priest_has_its_ability(died: bool) {
    has_abilities_when(Box::new(isd::skirsdag_high_priest::SkirsdagHighPriest), died, &|died: bool| {
        if died { vec![Ability { tap: true, tap_creatures: Some(2), ..Ability::default() }] } else { vec![] }
    });
}

case_by_case! { skirsdag_high_priest_has_its_ability;
    skirsdag_high_priest_has_no_ability_without_a_death: false,
    skirsdag_high_priest_has_its_ability_after_a_death: true,
}

abilities! {
    darkthicket_wolf_has_its_ability: isd::darkthicket_wolf::DarkthicketWolf
        => one(Ability { cost: mana(2, &[Color::Green]), once_per_turn: true, ..Ability::default() }),
    kessig_wolf_has_its_ability: isd::kessig_wolf::KessigWolf => one(Ability { cost: mana(1, &[Color::Red]), ..Ability::default() }),
    feral_ridgewolf_has_its_ability: isd::feral_ridgewolf::FeralRidgewolf => one(Ability { cost: mana(1, &[Color::Red]), ..Ability::default() }),
    manor_skeleton_has_its_ability: isd::manor_skeleton::ManorSkeleton => one(Ability { cost: mana(1, &[Color::Black]), ..Ability::default() }),
    lantern_spirit_has_its_ability: isd::lantern_spirit::LanternSpirit => one(Ability { cost: mana(0, &[Color::Blue]), ..Ability::default() }),
    manor_gargoyle_has_its_ability: isd::manor_gargoyle::ManorGargoyle => one(Ability { cost: mana(1, &[]), ..Ability::default() }),
    selfless_cathar_has_its_ability: isd::selfless_cathar::SelflessCathar
        => one(Ability { cost: mana(1, &[Color::White]), sacrifice: SacrificeCost::SacrificeThis, ..Ability::default() }),
    silverchase_fox_has_its_ability: isd::silverchase_fox::SilverchaseFox => one(Ability {
        cost: mana(1, &[Color::White]), sacrifice: SacrificeCost::SacrificeThis,
        target: Some(TargetRequirement::PermanentWithFilter(TargetFilter::HasCardType(vec![CardType::Enchantment]))),
        ..Ability::default()
    }),
    brain_weevil_has_its_ability: isd::brain_weevil::BrainWeevil => one(Ability {
        sacrifice: SacrificeCost::SacrificeThis, target: Some(TargetRequirement::PlayerOnly), sorcery_speed: true, ..Ability::default()
    }),
    elder_of_laurels_has_its_ability: isd::elder_of_laurels::ElderOfLaurels
        => one(Ability { cost: mana(3, &[Color::Green]), target: Some(TargetRequirement::Creature), ..Ability::default() }),
    disciple_of_griselbrand_has_its_ability: isd::disciple_of_griselbrand::DiscipleOfGriselbrand
        => one(Ability { cost: mana(1, &[]), sacrifice: SacrificeCost::SacrificeCreature, ..Ability::default() }),
    full_moons_rise_has_its_ability: isd::full_moons_rise::FullMoonsRise
        => one(Ability { sacrifice: SacrificeCost::SacrificeThis, ..Ability::default() }),
    ghoulcallers_bell_has_its_ability: isd::ghoulcallers_bell::GhoulcallersBell => one(Ability { tap: true, ..Ability::default() }),
    skirsdag_cultist_has_its_ability: isd::skirsdag_cultist::SkirsdagCultist => one(Ability {
        cost: mana(0, &[Color::Red]), tap: true, sacrifice: SacrificeCost::SacrificeCreature,
        target: Some(TargetRequirement::AnyTarget), ..Ability::default()
    }),
    stitchers_apprentice_has_its_ability: isd::stitchers_apprentice::StitchersApprentice
        => one(Ability { cost: mana(1, &[Color::Blue]), tap: true, ..Ability::default() }),
    tree_of_redemption_has_its_ability: isd::tree_of_redemption::TreeOfRedemption => one(Ability { tap: true, ..Ability::default() }),
    travelers_amulet_has_its_ability: isd::travelers_amulet::TravelersAmulet
        => one(Ability { cost: mana(1, &[]), sacrifice: SacrificeCost::SacrificeThis, ..Ability::default() }),
    mikaeus_the_lunarch_has_its_abilities: isd::mikaeus_the_lunarch::MikaeusTheLunarch => |_| vec![
        Ability { tap: true, ..Ability::default() },
        Ability { tap: true, remove_counters: Some((CounterType::PlusOnePlusOne, 1)), ..Ability::default() },
    ],
    grimgrin_corpse_born_has_its_ability: isd::grimgrin_corpse_born::GrimgrinCorpseBorn
        => one(Ability { sacrifice: SacrificeCost::SacrificeAnotherCreature, ..Ability::default() }),
    heretics_punishment_has_its_ability: isd::heretics_punishment::HereticsPunishment
        => one(Ability { cost: mana(3, &[Color::Red]), target: Some(TargetRequirement::AnyTarget), ..Ability::default() }),
    mindshrieker_has_its_ability: isd::mindshrieker::Mindshrieker
        => one(Ability { cost: mana(2, &[]), target: Some(TargetRequirement::PlayerOnly), ..Ability::default() }),
    olivia_voldaren_has_her_abilities: isd::olivia_voldaren::OliviaVoldaren => |_| vec![
        Ability { cost: mana(1, &[Color::Red]), target: Some(TargetRequirement::CreatureWithFilter(TargetFilter::Another)), ..Ability::default() },
        Ability {
            cost: mana(3, &[Color::Black, Color::Black]),
            target: Some(TargetRequirement::CreatureWithFilter(TargetFilter::HasSubtype("Vampire".into()))),
            ..Ability::default()
        },
    ],
    graveyard_shovel_has_its_ability: isd::graveyard_shovel::GraveyardShovel
        => one(Ability { cost: mana(2, &[]), tap: true, target: Some(TargetRequirement::PlayerOnly), ..Ability::default() }),
    mirror_mad_phantasm_has_its_ability: isd::mirror_mad_phantasm::MirrorMadPhantasm
        => one(Ability { cost: mana(1, &[Color::Blue]), ..Ability::default() }),
    avacynian_priest_has_its_ability: isd::avacynian_priest::AvacynianPriest
        => one(Ability { cost: mana(1, &[]), tap: true, target: Some(TargetRequirement::Creature), ..Ability::default() }),
}

/// The ability, with no targets, adds exactly `effects(me)` until end of
/// turn and asks for nothing. Whether the permanent is still there does
/// not matter to it: an effect on a permanent that has left the battlefield
/// is one the engine ends as the ability finishes resolving (CR 400.7,
/// `GameState::prune_effects_on_departed_objects`, proved in
/// `proofs::actions`).
fn lasts_the_turn<const N: usize>(card: Box<dyn CardBehavior>, effects: fn(ObjectId) -> [T; N]) {
    let mut b = permanent(card, &ANY_CREATURE, true);
    let before = b.resolve(0, &[]);
    let expected = std::mem::ManuallyDrop::new(effects(b.me));
    assert!(b.added(&before) == *expected && acted() == 0 && b.nothing_else_changed(&before));
    std::mem::forget(b);
}

card_proof! {
    /// Darkthicket Wolf: +2/+2 until end of turn.
    fn darkthicket_wolf_gets_plus_two_plus_two() {
        lasts_the_turn(Box::new(isd::darkthicket_wolf::DarkthicketWolf), |me| [T::ModifyPT { target: me, power_mod: 2, toughness_mod: 2 }]);
    }
}

card_proof! {
    /// Kessig Wolf: first strike until end of turn.
    fn kessig_wolf_gains_first_strike() {
        lasts_the_turn(Box::new(isd::kessig_wolf::KessigWolf), |me| [T::GrantKeyword { target: me, keyword: Keyword::FirstStrike }]);
    }
}

card_proof! {
    /// Feral Ridgewolf: +2/+0 until end of turn.
    fn feral_ridgewolf_gets_plus_two_plus_zero() {
        lasts_the_turn(Box::new(isd::feral_ridgewolf::FeralRidgewolf), |me| [T::ModifyPT { target: me, power_mod: 2, toughness_mod: 0 }]);
    }
}

card_proof! {
    /// Manor Gargoyle: indestructible as long as it has defender (CR
    /// 611.3a); its ability, until end of turn, flying and no defender.
    fn manor_gargoyle_loses_defender_and_gains_flying() {
        assert!(prints_where(&isd::manor_gargoyle::ManorGargoyle, |e| matches!(e, [E::When {
            condition: If::SelfHasKeyword(Keyword::Defender), effect,
        }] if matches!(**effect, E::GrantKeyword { keyword: Keyword::Indestructible, scope: EffectScope::OnSelf }))));
        lasts_the_turn(Box::new(isd::manor_gargoyle::ManorGargoyle), |me| [
            T::GrantKeyword { target: me, keyword: Keyword::Flying },
            T::RemoveKeyword { target: me, keyword: Keyword::Defender },
        ]);
    }
}

card_proof! {
    /// Manor Skeleton: a regeneration shield on itself (whether it can
    /// take one off the battlefield is the action's), and nothing else.
    fn manor_skeleton_regenerates() {
        let mut b = permanent(Box::new(isd::manor_skeleton::ManorSkeleton), &ANY_CREATURE, true);
        let before = b.resolve(0, &[]);
        assert!(asked_for(&[Act::Regenerate(b.me)]) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Lantern Spirit: returned to its owner's hand if it is still on the
    /// battlefield (CR 400.7: a card that has left is a new object, which
    /// "this creature" is not); and nothing else.
    fn lantern_spirit_returns_to_its_owners_hand() {
        let mut b = permanent(Box::new(isd::lantern_spirit::LanternSpirit), &ANY_CREATURE, true);
        let on = b.on();
        let before = b.resolve(0, &[]);
        if on {
            assert!(asked_for(&[Act::Move(b.me, Zone::Hand)]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Selfless Cathar: +1/+1 until end of turn on each creature the player
    /// who activated it controls as it resolves, and nothing else.
    fn selfless_cathar_pumps_your_creatures() {
        let mut b = permanent(Box::new(isd::selfless_cathar::SelflessCathar), &ANY_CREATURE, true);
        let mine = b.on() && b.state.get_object(b.me).expect("on the board").controller == b.you;
        let before = b.resolve(0, &[]);
        let mut pumped = Vec::with_capacity(3);
        if mine {
            pumped.push(b.me);
        }
        pumped.extend(b.those(|o| o.zone == Zone::Battlefield && o.controller == b.you && o.card_types.contains(&CardType::Creature)));
        assert!(each_is(b.added(&before), pumped.len(), |i| T::ModifyPT { target: pumped[i], power_mod: 1, toughness_mod: 1 }));
        assert!(acted() == 0 && b.nothing_else_changed(&before));
        std::mem::forget(b);
    }
}

card_proof! {
    /// Silverchase Fox: its target exiled (an enchantment as it was chosen;
    /// one that has since stopped being a legal target is the engine's to
    /// set aside, CR 608.2b); with its target gone, nothing.
    fn silverchase_fox_exiles_its_target() {
        let mut b = permanent(Box::new(isd::silverchase_fox::SilverchaseFox), &ANY_CREATURE, true);
        let target = any_target(b.others);
        let before = b.resolve(0, &[target.clone()]);
        match target {
            Target::Object(id) => {
                assert!(asked_for(&[Act::Move(id, Zone::Exile)]));
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
    /// Brain Weevil: its target player discards two cards of their choice;
    /// with its target gone, nothing.
    fn brain_weevil_makes_its_target_discard_two() {
        let mut b = permanent(Box::new(isd::brain_weevil::BrainWeevil), &ANY_CREATURE, true);
        let target = any_target(b.others);
        let before = b.resolve(0, &[target.clone()]);
        match target {
            Target::Player(p) => {
                assert!(asked_for(&[Act::DiscardN { player: p, count: 2, source: b.me }]));
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
    /// Elder of Laurels: its target, if still a creature on the
    /// battlefield, gets +X/+X until end of turn, X the number of creatures
    /// the player who activated it controls as it resolves; and nothing
    /// else.
    fn elder_of_laurels_pumps_by_your_creature_count() {
        let mut b = permanent(Box::new(isd::elder_of_laurels::ElderOfLaurels), &ANY_CREATURE, true);
        let target = if kani::any() { Target::Object(b.others[0]) } else { Target::Object(b.others[1]) };
        let mine = b.on() && b.state.get_object(b.me).expect("on the board").controller == b.you;
        let x = i32::from(mine) + i32::try_from(b.those(|o| {
            o.zone == Zone::Battlefield && o.controller == b.you && o.card_types.contains(&CardType::Creature)
        }).len()).expect("small");
        let before = b.resolve(0, &[target.clone()]);
        let Target::Object(id) = target else { unreachable!() };
        let on = usize::from(b.state.get_object(id).expect("on the board").zone == Zone::Battlefield);
        assert!(each_is(b.added(&before), on, |_| T::ModifyPT { target: id, power_mod: x, toughness_mod: x }));
        assert!(acted() == 0 && b.nothing_else_changed(&before));
        std::mem::forget(b);
    }
}

card_proof! {
    /// Disciple of Griselbrand: life equal to the sacrificed creature's
    /// toughness as it last existed (CR 608.2h), for the player who
    /// activated it; none for a toughness of 0 or less (CR 107.1b: a
    /// negative amount of life is none); and nothing else.
    fn disciple_of_griselbrand_gains_the_sacrificed_creatures_toughness() {
        let mut b = permanent(Box::new(isd::disciple_of_griselbrand::DiscipleOfGriselbrand), &ANY_CREATURE, true);
        let toughness: i8 = kani::any();
        kani::assume((-2..=6).contains(&toughness));
        b.state.last_activated_sacrifice = Some(b.others[0]);
        b.state.last_activated_sacrifice_toughness = Some(i32::from(toughness));
        let before = b.resolve(0, &[]);
        if toughness > 0 {
            assert!(asked_for(&[Act::Life(b.you, i32::from(toughness))]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Full Moon's Rise: Werewolf creatures its controller controls get
    /// +1/+0 and have trample. Its ability is the proof below.
    fn full_moons_rise_pumps_your_werewolves() {
        let werewolves_you_control = |f: &F| both(f, |f| matches!(f, F::ControlledByYou), |f| has_subtype(f, "Werewolf"));
        assert!(prints_where(&isd::full_moons_rise::FullMoonsRise, |e| matches!(e, [
            E::ModifyPT { power: 1, toughness: 0, scope: EffectScope::Global(a) },
            E::GrantKeyword { keyword: Keyword::Trample, scope: EffectScope::Global(b) },
        ] if werewolves_you_control(a) && werewolves_you_control(b))));
    }
}

/// Full Moon's Rise's ability: a regeneration shield on each Werewolf
/// creature the player who activated it controls as it resolves, and
/// nothing else. With each of the other two objects a Werewolf or not, a
/// constant in each harness: subtypes the solver chose among were strings
/// each one of several allocations, every Werewolf creature the ability
/// looks for compared with them byte by byte, and the proof ran out of
/// memory at ten gigabytes; with the subtypes constant and the four boards
/// in one harness, at four.
fn full_moons_rise_regenerates(werewolves: [bool; 2]) {
    const CREATURES: Kinds = Kinds { types: &[CardType::Creature], subtypes: &[], keywords: &[] };
    let mut b = permanent(Box::new(isd::full_moons_rise::FullMoonsRise), &CREATURES, true);
    for (id, werewolf) in b.others.into_iter().zip(werewolves) {
        if werewolf {
            b.state.get_object_mut(id).expect("on the board").subtypes = vec!["Werewolf".into()];
        }
    }
    let before = b.resolve(0, &[]);
    let shielded = b.those(|o| {
        o.zone == Zone::Battlefield && o.controller == b.you && o.card_types.contains(&CardType::Creature)
            && o.subtypes.iter().any(|s| s == "Werewolf")
    });
    assert!(asked_each(shielded.len(), |i| Act::Regenerate(shielded[i])) && Snapshot::of(&b.state) == before);
    std::mem::forget((b, shielded));
}

case_by_case! { full_moons_rise_regenerates;
    full_moons_rise_regenerates_with_no_other_werewolf: [false, false],
    full_moons_rise_regenerates_with_the_token_a_werewolf: [false, true],
    full_moons_rise_regenerates_with_the_card_a_werewolf: [true, false],
    full_moons_rise_regenerates_with_both_werewolves: [true, true],
}

card_proof! {
    /// Ghoulcaller's Bell: each player mills a card, and nothing else.
    fn ghoulcallers_bell_mills_each_player_one() {
        let mut b = permanent(Box::new(isd::ghoulcallers_bell::GhoulcallersBell), &ANY_CREATURE, true);
        let before = b.resolve(0, &[]);
        assert!(asked_for(&[Act::Mill(PlayerId(0), 1), Act::Mill(PlayerId(1), 1)]) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Skirsdag Cultist: 2 damage from the Cultist to its target, an object
    /// or a player (CR 120.3); with its target gone, nothing.
    fn skirsdag_cultist_deals_two_to_any_target() {
        let mut b = permanent(Box::new(isd::skirsdag_cultist::SkirsdagCultist), &ANY_CREATURE, true);
        let target = any_target(b.others);
        let before = b.resolve(0, &[target.clone()]);
        let damaged = match target {
            Target::Object(id) => Some(DamageTarget::Object(id)),
            Target::Player(p) => Some(DamageTarget::Player(p)),
            Target::Illegal => None,
        };
        match damaged {
            Some(target) => {
                assert!(asked_for(&[Act::Damage { source: b.me, target, amount: 2, kind: DamageKind::NonCombat }]));
            }
            None => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Skirsdag High Priest: a 5/5 black Demon creature token with flying
    /// for the player who activated it, and nothing else.
    fn skirsdag_high_priest_makes_a_demon() {
        let mut b = permanent(Box::new(isd::skirsdag_high_priest::SkirsdagHighPriest), &ANY_CREATURE, true);
        let before = b.resolve(0, &[]);
        assert!(asked_for(&[Act::Tokens {
            n: 1, controller: b.you, power: 5, toughness: 5, colors: vec![Color::Black],
            types: vec![CardType::Creature], keywords: vec![Keyword::Flying], subtypes: vec!["Demon".into()],
        }]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! { making_tokens
    /// Stitcher's Apprentice: a 2/2 blue Homunculus creature token for the
    /// player who activated it, then that player sacrifices a creature
    /// they control as it is then, the Homunculus among them: they are
    /// asked which, and must choose one (CR 701.21a).
    fn stitchers_apprentice_makes_a_homunculus_then_sacrifices_a_creature() {
        let mut b = permanent(Box::new(isd::stitchers_apprentice::StitchersApprentice), &ANY_CREATURE, true);
        let mine = b.on() && b.state.get_object(b.me).expect("on the board").controller == b.you;
        let homunculus = new_ids(&b.state, 1)[0];
        let before = b.resolve(0, &[]);
        let mut options = Vec::with_capacity(4);
        if mine {
            options.push(Target::Object(b.me));
        }
        for id in b.those(|o| o.zone == Zone::Battlefield && o.controller == b.you && o.card_types.contains(&CardType::Creature)) {
            options.push(Target::Object(id));
        }
        options.push(Target::Object(homunculus));
        assert!(asked_for(&[
            Act::Tokens {
                n: 1, controller: b.you, power: 2, toughness: 2, colors: vec![Color::Blue],
                types: vec![CardType::Creature], keywords: vec![], subtypes: vec!["Homunculus".into()],
            },
            Act::Choose {
                source: b.me, player: b.you, options,
                effect: PendingEffect::SacrificeCreature { source_name: "Stitcher's Apprentice".into() }, optional: false,
            },
        ]));
        assert!(b.added(&before).is_empty());
        std::mem::forget((b, before));
    }
}

card_proof! {
    /// Tree of Redemption, if still on the battlefield: its controller's
    /// life total becomes its toughness, and its toughness becomes their
    /// life total as it was (CR 701.10 exchange; the new toughness a
    /// layer-7b value, under any effects that modify it, 613.4b); and
    /// nothing else. Gone, nothing: there is no toughness to exchange.
    fn tree_of_redemption_exchanges_your_life_and_its_toughness() {
        let mut b = permanent(Box::new(isd::tree_of_redemption::TreeOfRedemption), &ANY_CREATURE, true);
        // Its printed 0/13, as every card object is made (`setup_game`).
        let printed = b.registry.card_data(b.card()).expect("registered");
        let (power, toughness) = (printed.power, printed.toughness);
        let o = b.state.get_object_mut(b.me).expect("on the board");
        (o.power, o.toughness) = (power, toughness);
        let life: i8 = kani::any();
        kani::assume((-2..=30).contains(&life));
        if b.you == PlayerId(0) {
            b.state.players[0].life = i32::from(life);
        } else {
            b.state.players[1].life = i32::from(life);
        }
        let on = b.on();
        let toughness = b.state.effective_toughness(b.me, &b.registry);
        let before = b.resolve(0, &[]);
        if on {
            let toughness = toughness.expect("a creature on the battlefield has a toughness");
            assert!(asked_for(&[Act::Life(b.you, toughness - i32::from(life)), Act::SetPT(b.me, None, Some(i32::from(life)))]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

/// Traveler's Amulet: the player who activated it searches their
/// library for a basic land card (the basic lands in it, in library
/// order) to put into their hand, untapped, and must; the search
/// reveals and shuffles (`proofs::actions`). Over a library of a Forest
/// and a nonbasic land card in either order, the first player's, the
/// player who activated it either. The order, the library's owner and
/// the player who activated it are constants in each harness: an order
/// the solver chose made each card in the library either card, every
/// read of it a question of which, and a player it chose an index into
/// the players, through which the card reads the library it searches;
/// the proof ran out of memory at ten gigabytes, and with the activator
/// alone left open, at four; and the four in one harness, two gigabytes
/// each, at four.
///
/// With the Forest on top if `forest_first`, activated by `you`.
fn travelers_amulet(forest_first: bool, you: PlayerId) {
    let mut b = permanent_of(Box::new(isd::travelers_amulet::TravelersAmulet), &ANY_CREATURE, true, you);
    let forest_card = b.registry.register(Box::new(crate::cards::forest::Forest));
    let blank = CardId(1);
    let owner = PlayerId(0);
    let forest = b.state.create_object(forest_card, owner, Zone::Library, None, None);
    let other = b.state.create_object(blank, owner, Zone::Library, None, None);
    b.state.get_object_mut(other).expect("just created").card_types = vec![CardType::Land];
    b.state.players[0].library_order = if forest_first { vec![forest, other] } else { vec![other, forest] };
    let before = b.resolve(0, &[]);
    let candidates = if owner == b.you { vec![forest] } else { vec![] };
    assert!(asked_for(&[Act::Search { source: b.me, player: b.you, candidates, to: Zone::Hand, tapped: false, optional: false }]));
    assert!(Snapshot::of(&b.state) == before);
    std::mem::forget(b);
}

case_by_case! { |(forest_first, p)| travelers_amulet(forest_first, PlayerId(p));
    travelers_amulet_finds_the_forest_on_top_for_the_first_player: (true, 0),
    travelers_amulet_finds_the_forest_under_a_land_for_the_first_player: (false, 0),
    travelers_amulet_finds_the_forest_on_top_for_the_second_player: (true, 1),
    travelers_amulet_finds_the_forest_under_a_land_for_the_second_player: (false, 1),
}

/// A card in a library a proof makes: Grizzly Bears (a creature card, mana
/// value 2), Lava Axe (a sorcery card, mana value 5), or the blank card
/// (neither, and no mana cost).
#[derive(Clone, Copy)]
enum InLibrary {
    Bears,
    Axe,
    Blank,
}

/// A library for `player` of `cards`, top first. The cards are constants: a
/// library whose length the solver left open was a list of every length,
/// and a card it left open an index into the registry it left open, which
/// every read of the card (its mana value, whether it is a creature) is
/// then a question of; with both, each of these proofs ran out of memory
/// at ten gigabytes.
fn library_of(b: &mut Board, player: PlayerId, cards: &[InLibrary]) -> Vec<ObjectId> {
    let bears = b.registry.register(Box::new(crate::cards::grizzly_bears::GrizzlyBears));
    let axe = b.registry.register(Box::new(crate::cards::lava_axe::LavaAxe));
    let mut library = Vec::with_capacity(2);
    for card in cards {
        let card = match card { InLibrary::Bears => bears, InLibrary::Axe => axe, InLibrary::Blank => CardId(1) };
        library.push(b.state.create_object(card, player, Zone::Library, None, None));
    }
    if player == PlayerId(0) {
        b.state.players[0].library_order = library.clone();
    } else {
        b.state.players[1].library_order = library.clone();
    }
    library
}

/// Cellar Door: "{3}, {T}: Target player puts the bottom card of their
/// library into their graveyard. If it's a creature card, you create a
/// 2/2 black Zombie creature token." With that library empty, nothing;
/// with its target gone illegal, nothing.
fn cellar_door(cards: &[InLibrary]) {
    let mut b = permanent(Box::new(isd::cellar_door::CellarDoor), &ANY_CREATURE, true);
    let card = b.card();
    let defs = std::mem::ManuallyDrop::new(b.registry.get(card).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    assert!(defs.len() == 1 && defs[0].ability_index == 0
        && Ability::of(&defs[0]) == Ability { cost: mana(3, &[]), tap: true, target: Some(TargetRequirement::PlayerOnly), ..Ability::default() });
    // The board's controller is either player, so the target is either
    // the controller or the opponent.
    let player = PlayerId(0);
    let library = library_of(&mut b, player, cards);
    let target = if kani::any() { Target::Player(player) } else { Target::Illegal };
    let bottom = library.last().copied();
    let creature = bottom.is_some_and(|id| b.state.name_of(id, &b.registry) == "Grizzly Bears");
    let before = b.resolve(0, &[target.clone()]);
    match (target, bottom) {
        (Target::Player(_), Some(id)) if creature => {
            assert!(asked_for(&[Act::MillCard(id), Act::Tokens {
                n: 1, controller: b.you, power: 2, toughness: 2, colors: vec![Color::Black],
                types: vec![CardType::Creature], keywords: vec![], subtypes: vec!["Zombie".into()],
            }]));
        }
        (Target::Player(_), Some(id)) => {
            assert!(asked_for(&[Act::MillCard(id)]));
        }
        _ => {
            assert!(acted() == 0);
        }
    }
    assert!(Snapshot::of(&b.state) == before);
    std::mem::forget((b, library));
}

/// Mindshrieker: "{2}: Target player mills a card. This creature gets
/// +X/+X until end of turn, where X is the milled card's mana value."
/// No card milled, or one of mana value 0, or Mindshrieker gone: no
/// bonus (CR 400.7). With its target gone illegal, nothing.
fn mindshrieker(cards: &[InLibrary]) {
    let mut b = permanent(Box::new(isd::mindshrieker::Mindshrieker), &ANY_CREATURE, true);
    // As in `cellar_door`.
    let player = PlayerId(0);
    let library = library_of(&mut b, player, cards);
    let target = if kani::any() { Target::Player(player) } else { Target::Illegal };
    let top = library.first().copied();
    let mana_value = top.map_or(0, |id| match b.state.name_of(id, &b.registry).as_str() {
        "Grizzly Bears" => 2,
        "Lava Axe" => 5,
        _ => 0,
    });
    let on = b.on();
    let before = b.resolve(0, &[target.clone()]);
    let added = b.added(&before);
    match target {
        Target::Player(p) => {
            assert!(asked_for(&[Act::Mill(p, 1)]));
            let grows = usize::from(on && mana_value > 0);
            assert!(each_is(added, grows, |_| T::ModifyPT { target: b.me, power_mod: mana_value, toughness_mod: mana_value }));
        }
        _ => {
            assert!(acted() == 0 && added.is_empty());
        }
    }
    assert!(b.nothing_else_changed(&before));
    std::mem::forget((b, library));
}

/// Heretic's Punishment: "{3}{R}: Choose any target, then mill three
/// cards. This enchantment deals damage to that permanent or player
/// equal to the greatest mana value among the milled cards." With its
/// target gone illegal it does nothing, the milling included (CR
/// 608.2b). Over a library of the cards given, up to two, of mana value 0,
/// 2 or 5, of the enchantment's controller `you`. The controller is a
/// constant in each harness: the card reads the top of its controller's
/// library through an index by the controller, and with one the solver
/// chose each of these ran out of memory at four gigabytes, the empty
/// library's included; and the two in one harness, a couple of gigabytes
/// each, out of four.
fn heretics_punishment(you: PlayerId, cards: &[InLibrary]) {
    let mut b = permanent_of(Box::new(isd::heretics_punishment::HereticsPunishment), &ANY_CREATURE, true, you);
    let library = library_of(&mut b, you, cards);
    let target = any_target(b.others);
    let mana_value = |id: &ObjectId| match b.state.name_of(*id, &b.registry).as_str() {
        "Grizzly Bears" => 2,
        "Lava Axe" => 5,
        _ => 0,
    };
    let greatest = library.iter().map(mana_value).max().unwrap_or(0);
    let before = b.resolve(0, &[target.clone()]);
    let damaged = match target {
        Target::Object(id) => Some(DamageTarget::Object(id)),
        Target::Player(p) => Some(DamageTarget::Player(p)),
        Target::Illegal => None,
    };
    match damaged {
        Some(t) => {
            assert!(asked_for(&[Act::Mill(you, 3), Act::Damage { source: b.me, target: t, amount: greatest, kind: DamageKind::NonCombat }]));
        }
        None => {
            assert!(acted() == 0);
        }
    }
    assert!(Snapshot::of(&b.state) == before);
    std::mem::forget((b, library));
}

/// The library proofs above, a harness for each library: the top card for
/// Mindshrieker, which mills one; and the bottom card, and one above it of
/// another kind, for Cellar Door, which mills the bottom one.
macro_rules! over_each_library {
    ($($name:ident: $proof:ident[$($card:ident),*];)*) => {$(
        card_proof! {
            /// The proof above it, for one library.
            fn $name() {
                $proof(&[$(InLibrary::$card),*]);
            }
        }
    )*};
}

over_each_library! {
    mindshrieker_milling_from_an_empty_library_grows_by_nothing: mindshrieker[];
    mindshrieker_milling_grizzly_bears_grows_by_its_mana_value: mindshrieker[Bears];
    mindshrieker_milling_lava_axe_grows_by_its_mana_value: mindshrieker[Axe];
    mindshrieker_milling_a_blank_card_grows_by_its_mana_value: mindshrieker[Blank];
    cellar_door_over_an_empty_library_does_nothing: cellar_door[];
    cellar_door_milling_grizzly_bears_makes_a_zombie_if_a_creature: cellar_door[Bears];
    cellar_door_milling_lava_axe_makes_a_zombie_if_a_creature: cellar_door[Axe];
    cellar_door_milling_a_blank_card_makes_a_zombie_if_a_creature: cellar_door[Blank];
    cellar_door_milling_lava_axe_under_grizzly_bears_makes_a_zombie_if_a_creature: cellar_door[Bears, Axe];
    cellar_door_milling_a_blank_card_under_grizzly_bears_makes_a_zombie_if_a_creature: cellar_door[Bears, Blank];
    cellar_door_milling_grizzly_bears_under_lava_axe_makes_a_zombie_if_a_creature: cellar_door[Axe, Bears];
    cellar_door_milling_a_blank_card_under_lava_axe_makes_a_zombie_if_a_creature: cellar_door[Axe, Blank];
    cellar_door_milling_grizzly_bears_under_a_blank_card_makes_a_zombie_if_a_creature: cellar_door[Blank, Bears];
    cellar_door_milling_lava_axe_under_a_blank_card_makes_a_zombie_if_a_creature: cellar_door[Blank, Axe];
}

// Heretic's Punishment, which mills three, over every library of up to two
// cards, for each controller.
case_by_case! { |p| heretics_punishment(PlayerId(p), &[]);
    heretics_punishment_over_an_empty_library_deals_nothing_for_the_first_player: 0,
    heretics_punishment_over_an_empty_library_deals_nothing_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Bears]);
    heretics_punishment_milling_grizzly_bears_deals_its_mana_value_for_the_first_player: 0,
    heretics_punishment_milling_grizzly_bears_deals_its_mana_value_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Axe]);
    heretics_punishment_milling_lava_axe_deals_its_mana_value_for_the_first_player: 0,
    heretics_punishment_milling_lava_axe_deals_its_mana_value_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Blank]);
    heretics_punishment_milling_a_blank_card_deals_its_mana_value_for_the_first_player: 0,
    heretics_punishment_milling_a_blank_card_deals_its_mana_value_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Bears, InLibrary::Bears]);
    heretics_punishment_milling_grizzly_bears_then_grizzly_bears_deals_the_greater_for_the_first_player: 0,
    heretics_punishment_milling_grizzly_bears_then_grizzly_bears_deals_the_greater_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Bears, InLibrary::Axe]);
    heretics_punishment_milling_grizzly_bears_then_lava_axe_deals_the_greater_for_the_first_player: 0,
    heretics_punishment_milling_grizzly_bears_then_lava_axe_deals_the_greater_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Bears, InLibrary::Blank]);
    heretics_punishment_milling_grizzly_bears_then_a_blank_card_deals_the_greater_for_the_first_player: 0,
    heretics_punishment_milling_grizzly_bears_then_a_blank_card_deals_the_greater_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Axe, InLibrary::Bears]);
    heretics_punishment_milling_lava_axe_then_grizzly_bears_deals_the_greater_for_the_first_player: 0,
    heretics_punishment_milling_lava_axe_then_grizzly_bears_deals_the_greater_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Axe, InLibrary::Axe]);
    heretics_punishment_milling_lava_axe_then_lava_axe_deals_the_greater_for_the_first_player: 0,
    heretics_punishment_milling_lava_axe_then_lava_axe_deals_the_greater_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Axe, InLibrary::Blank]);
    heretics_punishment_milling_lava_axe_then_a_blank_card_deals_the_greater_for_the_first_player: 0,
    heretics_punishment_milling_lava_axe_then_a_blank_card_deals_the_greater_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Blank, InLibrary::Bears]);
    heretics_punishment_milling_a_blank_card_then_grizzly_bears_deals_the_greater_for_the_first_player: 0,
    heretics_punishment_milling_a_blank_card_then_grizzly_bears_deals_the_greater_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Blank, InLibrary::Axe]);
    heretics_punishment_milling_a_blank_card_then_lava_axe_deals_the_greater_for_the_first_player: 0,
    heretics_punishment_milling_a_blank_card_then_lava_axe_deals_the_greater_for_the_second_player: 1,
}
case_by_case! { |p| heretics_punishment(PlayerId(p), &[InLibrary::Blank, InLibrary::Blank]);
    heretics_punishment_milling_a_blank_card_then_a_blank_card_deals_the_greater_for_the_first_player: 0,
    heretics_punishment_milling_a_blank_card_then_a_blank_card_deals_the_greater_for_the_second_player: 1,
}



card_proof! {
    /// Avacynian Priest's target: a creature on the battlefield that is not
    /// a Human (the engine asks the requirement, then the card); its
    /// ability is in the table above. Whether the candidate is a Human,
    /// and the player choosing, are constants in each call: a list of
    /// subtypes the solver left open, with the target it chose and the
    /// ability, ran the proof out of memory at ten gigabytes, and the
    /// ability with the target, the Human-ness a constant, at four.
    fn avacynian_priest_targets_a_non_human_creature() {
        each_case(4, |case| avacynian_priest_targets(case % 2 == 1, PlayerId(case / 2)));
    }
}

/// `avacynian_priest_targets_a_non_human_creature`, for a candidate that
/// is a Human or not, the Priest `you`'s.
fn avacynian_priest_targets(human: bool, you: PlayerId) {
    const PEOPLE: Kinds = Kinds { types: &[CardType::Creature], subtypes: &[], keywords: &[] };
    let mut b = permanent_of(Box::new(isd::avacynian_priest::AvacynianPriest), &PEOPLE, true, you);
    let candidate = b.others[0];
    if human {
        b.state.get_object_mut(candidate).expect("on the board").subtypes = vec!["Human".into()];
    }
    let o = b.state.get_object(candidate).expect("on the board");
    let creature = o.zone == Zone::Battlefield && o.card_types.contains(&CardType::Creature);
    let priest = isd::avacynian_priest::AvacynianPriest;
    assert!(priest.is_valid_target(&b.state, b.you, &Target::Object(candidate), &b.registry) == (creature && !human));
    assert!(!priest.is_valid_target(&b.state, b.you, &Target::Player(b.you), &b.registry));
    std::mem::forget(b);
}

card_proof! {
    /// Avacynian Priest's ability, resolving: its target tapped (whether it
    /// can be is the action's); with its target gone illegal, nothing.
    fn avacynian_priest_taps_its_target() {
        let mut b = permanent(Box::new(isd::avacynian_priest::AvacynianPriest), &ANY_CREATURE, true);
        let target = any_target(b.others);
        let before = b.resolve(0, &[target.clone()]);
        match target {
            Target::Object(id) => {
                assert!(asked_for(&[Act::Tap(id)]));
            }
            _ => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}


card_proof! { adding_counters
    /// Olivia Voldaren's first ability: "{1}{R}: Olivia Voldaren deals 1
    /// damage to another target creature. That creature becomes a Vampire
    /// in addition to its other types. Put a +1/+1 counter on Olivia
    /// Voldaren." With its target gone, nothing at all.
    fn olivia_voldaren_pings_and_turns_its_target_into_a_vampire() {
        let mut b = permanent(Box::new(isd::olivia_voldaren::OliviaVoldaren), &ANY_CREATURE, true);
        let target = if kani::any() { Target::Object(b.others[0]) } else { Target::Illegal };
        let was_vampire: bool = kani::any();
        if was_vampire {
            b.state.get_object_mut(b.others[0]).expect("on the board").subtypes.push("Vampire".into());
        }
        let on = b.state.get_object(b.others[0]).expect("on the board").zone == Zone::Battlefield;
        b.resolve(0, &[target.clone()]);
        if target == Target::Object(b.others[0]) && on {
            assert!(asked_for(&[
                Act::Effect(Target::Object(b.others[0]), PendingEffect::DealDamage { amount: 1, source_id: b.me }),
                Act::Counters(b.me, CounterType::PlusOnePlusOne, 1),
            ]));
            let subtypes = &b.state.get_object(b.others[0]).expect("on the board").subtypes;
            assert!(subtypes.iter().filter(|s| *s == "Vampire").count() == 1);
        } else {
            assert!(acted() == 0);
        }
        std::mem::forget(b);
    }
}

card_proof! {
    /// Olivia Voldaren's second ability: "{3}{B}{B}: Gain control of target
    /// Vampire for as long as you control Olivia Voldaren." Its target
    /// still a Vampire on the battlefield and Olivia still the activating
    /// player's, the control effect (CR 611.2b); otherwise nothing (a
    /// duration that has already ended, CR 611.2b, the card's ruling).
    fn olivia_voldaren_steals_a_vampire_for_as_long_as_you_control_her() {
        const VAMPIRES: Kinds = Kinds { types: &[CardType::Creature], subtypes: &["Vampire"], keywords: &[] };
        let mut b = permanent(Box::new(isd::olivia_voldaren::OliviaVoldaren), &VAMPIRES, true);
        let activator = PlayerId(u8::from(kani::any::<bool>()));
        b.state.resolving_ability_activator = Some(activator);
        let target = any_target(b.others);
        let olivia = b.state.get_object(b.me).expect("on the board");
        let still_hers = olivia.zone == Zone::Battlefield && olivia.controller == activator;
        let before = b.resolve(1, &[target.clone()]);
        let vampire = |id: ObjectId| {
            let o = b.state.get_object(id).expect("on the board");
            o.zone == Zone::Battlefield && o.subtypes.iter().any(|s| s == "Vampire")
        };
        match target {
            Target::Object(id) if vampire(id) && still_hers => {
                assert!(asked_for(&[Act::ControlWhile { object: id, source: b.me }]));
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
    /// Mikaeus, the Lunarch: its first ability, a +1/+1 counter on Mikaeus
    /// (one on a Mikaeus that has left the battlefield lands nowhere, which
    /// is `add_counters`' to do); its second, the counter removed as its cost
    /// (CR 602.2b, pinned above), a +1/+1 counter on each other creature the
    /// player who activated it controls as it resolves. Nothing else.
    fn mikaeus_the_lunarch_puts_counters_on_itself_or_on_your_other_creatures() {
        let mut b = permanent(Box::new(isd::mikaeus_the_lunarch::MikaeusTheLunarch), &ANY_CREATURE, true);
        let index = usize::from(kani::any::<bool>());
        let yours = b.those(|o| o.zone == Zone::Battlefield && o.controller == b.you && o.card_types.contains(&CardType::Creature));
        let before = b.resolve(index, &[]);
        if index == 0 {
            assert!(asked_for(&[Act::Counters(b.me, CounterType::PlusOnePlusOne, 1)]));
        } else {
            assert!(asked_each(yours.len(), |i| Act::Counters(yours[i], CounterType::PlusOnePlusOne, 1)));
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Mirror-Mad Phantasm, its ability resolving with the Phantasm still
    /// on the battlefield: its owner, not the player who activated it, puts
    /// it into their library and shuffles (to shuffle a card in is to put
    /// it in, then shuffle: `LibraryPosition`'s doc); then reveals from the top of that
    /// library, as the shuffle left it, until a card named Mirror-Mad
    /// Phantasm, which goes onto the battlefield under the owner's control
    /// (CR 110.2a), the cards revealed before it into the graveyard, and
    /// the cards below it left where they are. With no such card, the whole
    /// library is revealed and goes to the graveyard. A token named
    /// Mirror-Mad Phantasm is not a card (CR 109.1) and goes with the rest.
    /// The Phantasm gone by then, nothing: there is no Phantasm to shuffle
    /// in, so the player has not ("If that player does").
    ///
    /// Over the owner's library, as the shuffle left it, of three objects,
    /// each a card or a token and each named Mirror-Mad Phantasm or not (an
    /// object in a library has its printed name, a card that changes zones
    /// being a new object, CR 400.7, whose name `move_object` writes back),
    /// and the
    /// other player's library of a card named Mirror-Mad Phantasm, which
    /// the reveal must not reach whoever activated the ability.
    fn mirror_mad_phantasm_reveals_until_a_phantasm_card() {
        const NAME: &str = "Mirror-Mad Phantasm";
        let (registry, mut state, blank, card) = board(Box::new(isd::mirror_mad_phantasm::MirrorMadPhantasm));
        let you = PlayerId(u8::from(kani::any::<bool>()));
        let on: bool = kani::any();
        let me = state.create_object(card, PlayerId(0), if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
        if on {
            state.get_object_mut(me).expect("just created").controller = you;
        } else {
            state.get_object_mut(me).expect("just created").last_controller = Some(you);
            state.players[0].graveyard_order.push(me);
        }
        state.resolving_ability_activator = Some(you);
        let mut library = [me; 3];
        let mut phantasm = [false; 3];
        for (slot, named) in library.iter_mut().zip(&mut phantasm) {
            let is_named: bool = kani::any();
            let token: bool = kani::any();
            let id = state.create_object(if is_named { card } else { blank }, PlayerId(0), Zone::Library, None, None);
            let o = state.get_object_mut(id).expect("just created");
            o.name = String::from(if is_named { NAME } else { "Blank" });
            o.is_token = token;
            state.players[0].library_order.push(id);
            *slot = id;
            *named = is_named && !token;
        }
        let theirs = state.create_object(card, PlayerId(1), Zone::Library, None, None);
        state.get_object_mut(theirs).expect("just created").name = String::from(NAME);
        state.players[1].library_order.push(theirs);

        let before = Snapshot::of(&state);
        registry.get(card).expect("registered").resolve_activated_ability(&mut state, me, 0, &[], &registry);
        if !on {
            assert!(acted() == 0 && Snapshot::of(&state) == before);
            std::mem::forget((registry, state));
            return;
        }
        let found = phantasm.iter().position(|named| *named);
        let revealed = found.unwrap_or(library.len());
        assert!(acted() == 2 + revealed + usize::from(found.is_some()));
        assert!(were(0, 1, |_| Act::ToLibrary(me, LibraryPosition::Bottom)) && were(1, 1, |_| Act::Shuffle(PlayerId(0))));
        assert!(were(2, revealed, |j| Act::Move(library[j], Zone::Graveyard)));
        if let Some(i) = found {
            assert!(were(2 + revealed, 1, |_| Act::Move(library[i], Zone::Battlefield)));
            assert!(state.get_object(library[i]).expect("on the board").controller == PlayerId(0));
        }
        let mut after = before.clone();
        after.players[0].1 = super::zone_rows(found.map_or(&[][..], |i| &library[i + 1..]));
        assert!(Snapshot::of(&state) == after);
        std::mem::forget((registry, state));
    }
}

/// `n` cards of the blank card in `p`'s hand, and one in the other
/// player's.
fn hands(state: &mut GameState, blank: CardId, p: PlayerId, n: usize) -> Vec<ObjectId> {
    let mut hand = Vec::with_capacity(2);
    for _ in 0..n {
        hand.push(state.create_object(blank, p, Zone::Hand, None, None));
    }
    state.create_object(blank, PlayerId(1 - p.0), Zone::Hand, None, None);
    hand
}

/// Grimoire of the Dead's abilities: "{1}, {T}, Discard a card", offered
/// only with a card in its controller's hand to discard (CR 601.2h: a cost
/// that cannot be paid cannot be chosen); and "{T}, Remove three study
/// counters ... and sacrifice it", always. With `n` cards in its
/// controller's hand.
fn grimoire_of_the_dead_has_its_abilities_with(n: usize) {
    let mut b = permanent(Box::new(isd::grimoire_of_the_dead::GrimoireOfTheDead), &ANY_CREATURE, false);
    let you = b.you;
    let hand = hands(&mut b.state, CardId(1), you, n);
    let card = b.card();
    let defs = std::mem::ManuallyDrop::new(b.registry.get(card).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    let discard = Ability { cost: mana(1, &[]), tap: true, ..Ability::default() };
    let sacrifice = Ability {
        tap: true, sacrifice: SacrificeCost::SacrificeThis, remove_counters: Some((CounterType::Study, 3)), ..Ability::default()
    };
    if hand.is_empty() {
        assert!(defs.len() == 1 && defs[0].ability_index == 1 && Ability::of(&defs[0]) == sacrifice);
    } else {
        assert!(defs.len() == 2 && defs[0].ability_index == 0 && Ability::of(&defs[0]) == discard);
        assert!(defs[1].ability_index == 1 && Ability::of(&defs[1]) == sacrifice);
    }
    std::mem::forget((b, hand, discard, sacrifice));
}

/// Grimoire of the Dead, paying for its first ability: the card in its
/// controller's hand discarded, or with more than one, they are asked
/// which (CR 601.2b: the choice is theirs, made while paying), from their
/// own hand alone. The second ability's costs are the engine's alone
/// (pinned above). Nothing else. With `n` cards in its controller's
/// hand.
fn grimoire_of_the_dead_discards_as_a_cost_with(n: usize) {
    let mut b = permanent(Box::new(isd::grimoire_of_the_dead::GrimoireOfTheDead), &ANY_CREATURE, false);
    let you = b.you;
    let hand = hands(&mut b.state, CardId(1), you, n);
    let index = usize::from(kani::any::<bool>());
    let before = Snapshot::of(&b.state);
    let card = b.card();
    b.registry.get(card).expect("registered").pay_activation_cost(&mut b.state, b.me, index, &[], &b.registry);
    if index == 0 && hand.len() == 1 {
        assert!(asked_for(&[Act::Discard(hand[0])]) && Snapshot::of(&b.state) == before);
    } else if index == 0 && hand.len() == 2 {
        let asked = matches!(&b.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::ChooseCardFromHand {
                player: from, cards, discard_immediately: true, remaining: 1, ..
            },
        }) if *player == you && *from == you && *source == b.me && *cards == hand);
        assert!(asked && acted() == 0);
    } else {
        assert!(acted() == 0 && Snapshot::of(&b.state) == before);
    }
    std::mem::forget((b, hand));
}

// The proofs above, a harness for each size of hand up to 2: a hand whose
// size the solver chose is a list of every length, and every card made
// after it an id it leaves open; the Grimoire's proofs ran out of memory at
// ten gigabytes, and with the sizes in one harness each, at four.
case_by_case! { grimoire_of_the_dead_has_its_abilities_with;
    grimoire_of_the_dead_has_its_abilities_with_an_empty_hand: 0,
    grimoire_of_the_dead_has_its_abilities_with_a_card_in_hand: 1,
    grimoire_of_the_dead_has_its_abilities_with_two_cards_in_hand: 2,
}

case_by_case! { grimoire_of_the_dead_discards_as_a_cost_with;
    grimoire_of_the_dead_discards_nothing_from_an_empty_hand: 0,
    grimoire_of_the_dead_discards_its_controllers_only_card: 1,
    grimoire_of_the_dead_asks_which_of_two_cards_to_discard: 2,
}

card_proof! {
    /// Grimoire of the Dead's first ability resolving: a study counter on
    /// it, and nothing else.
    fn grimoire_of_the_dead_puts_a_study_counter_on_itself() {
        let mut b = permanent(Box::new(isd::grimoire_of_the_dead::GrimoireOfTheDead), &ANY_CREATURE, true);
        let before = b.resolve(0, &[]);
        assert!(asked_for(&[Act::Counters(b.me, CounterType::Study, 1)]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Grimoire of the Dead's second ability resolving: every creature
    /// card in every graveyard (no token, CR 109.1, and not itself,
    /// sacrificed to pay for it, which is not a creature) put onto the
    /// battlefield under the control of the player who activated it, each
    /// of them then black and a Zombie besides whatever else it is, once
    /// (CR 205.1b: in addition to its other types), and nothing else made
    /// either. Apart from the first ability's proof above: the two in one
    /// harness ran out of memory at ten gigabytes.
    fn grimoire_of_the_dead_returns_every_creature_card_as_a_black_zombie() {
        let mut b = permanent(Box::new(isd::grimoire_of_the_dead::GrimoireOfTheDead), &ANY_CREATURE, true);
        let returned = b.those(|o| o.zone == Zone::Graveyard && !o.is_token && o.card_types.contains(&CardType::Creature));
        let black = b.others.map(|id| b.state.get_object(id).expect("on the board").colors.contains(&Color::Black));
        let before = b.resolve(1, &[]);
        assert!(asked_each(returned.len(), |i| Act::MoveUnder(returned[i], Zone::Battlefield, b.you)));
        for (i, id) in b.others.iter().enumerate() {
            let o = b.state.get_object(*id).expect("on the board");
            let zombies = o.subtypes.iter().filter(|s| *s == "Zombie").count();
            let blacks = o.colors.iter().filter(|c| **c == Color::Black).count();
            if returned.contains(id) {
                assert!(zombies == 1 && blacks == 1);
            } else {
                assert!(zombies == 0 && (blacks == 1) == black[i]);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget((b, returned));
    }
}

/// Back from the Brink's board: the enchantment on the battlefield under
/// either player; in player 0's graveyard, in this order, a Mikaeus, the
/// Lunarch ({X}{W}), a second one or not (it is in player 0's hand
/// otherwise), a card of the blank card that is a creature or not, and a
/// creature token; each named as it is printed (an object in a graveyard
/// has its printed name: `move_object` writes it back).
struct Brink {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    you: PlayerId,
    mikaeus: [ObjectId; 2],
    blank: ObjectId,
    token: ObjectId,
}

fn brink() -> Brink {
    brink_shaped(kani::any(), kani::any())
}

/// [`brink`]'s board, with the second Mikaeus in the graveyard if
/// `second_dead`, and the blank card a creature if `blank_creature`.
fn brink_shaped(second_dead: bool, blank_creature: bool) -> Brink {
    let (mut registry, mut state, blank_card, card) = board(Box::new(isd::back_from_the_brink::BackFromTheBrink));
    let mikaeus_card = registry.register(Box::new(isd::mikaeus_the_lunarch::MikaeusTheLunarch));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let me = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
    state.get_object_mut(me).expect("just created").controller = you;
    let graveyard = |state: &mut GameState, card: CardId, name: &str, zone: Zone| {
        let id = state.create_object(card, PlayerId(0), zone, None, None);
        state.get_object_mut(id).expect("just created").name = String::from(name);
        if zone == Zone::Graveyard {
            state.players[0].graveyard_order.push(id);
        }
        id
    };
    let first = graveyard(&mut state, mikaeus_card, "Mikaeus, the Lunarch", Zone::Graveyard);
    let second = graveyard(&mut state, mikaeus_card, "Mikaeus, the Lunarch", if second_dead { Zone::Graveyard } else { Zone::Hand });
    let blank = graveyard(&mut state, blank_card, "Blank", Zone::Graveyard);
    let o = state.get_object_mut(blank).expect("just created");
    o.card_types = Vec::with_capacity(1);
    if blank_creature {
        o.card_types.push(CardType::Creature);
    }
    let token = graveyard(&mut state, CardId(0), "Blank", Zone::Graveyard);
    let o = state.get_object_mut(token).expect("just created");
    o.is_token = true;
    o.card_types = vec![CardType::Creature];
    Brink { registry, state, me, you, mikaeus: [first, second], blank, token }
}

impl Brink {
    fn card(&self) -> CardId {
        self.state.get_object(self.me).expect("on the board").card_id
    }

    fn creature(&self, id: ObjectId) -> bool {
        self.state.get_object(id).expect("on the board").card_types.contains(&CardType::Creature)
    }
}

/// Back from the Brink's abilities: one for each creature card in its
/// controller's graveyard (no token, CR 109.1), two cards of one name
/// being one ability (the same cost, the same token, CR 707.2), each
/// costing that card's mana cost with X as 0 (CR 107.3e: no X is
/// announced, the card not being cast) and activated only as a
/// sorcery; the ability's index names the card. None for a player
/// with no creature card in their graveyard.
///
/// For one board, a constant in each harness: a board the solver chose had
/// a graveyard of every length, and so a list of abilities of every
/// length, each compared, and the proof ran out of memory at ten
/// gigabytes; the four boards in one harness, at four.
fn back_from_the_brink_has_its_abilities((second_dead, blank_creature): (bool, bool)) {
    let b = brink_shaped(second_dead, blank_creature);
    let defs = std::mem::ManuallyDrop::new(b.registry.get(b.card()).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    if b.you == PlayerId(1) {
        assert!(defs.is_empty());
    } else {
        let mut expected = Vec::with_capacity(2);
        expected.push((b.mikaeus[0], Ability { cost: mana(0, &[Color::White]), sorcery_speed: true, ..Ability::default() }));
        if b.creature(b.blank) {
            expected.push((b.blank, Ability { sorcery_speed: true, ..Ability::default() }));
        }
        assert!(defs.len() == expected.len());
        for (def, (id, ability)) in defs.iter().zip(&expected) {
            assert!(def.ability_index == usize::try_from(id.0).expect("small") && Ability::of(def) == *ability);
        }
        std::mem::forget(expected);
    }
    std::mem::forget(b);
}

case_by_case! { back_from_the_brink_has_its_abilities;
    back_from_the_brink_has_an_ability_for_one_mikaeus: (false, false),
    back_from_the_brink_has_an_ability_for_one_mikaeus_and_a_creature_card: (false, true),
    back_from_the_brink_has_one_ability_for_two_mikaeus: (true, false),
    back_from_the_brink_has_one_ability_for_two_mikaeus_and_one_for_a_creature_card: (true, true),
}

card_proof! {
    /// Back from the Brink, paying: the card its ability names exiled from
    /// its controller's graveyard (CR 602.2b, a cost), if it is a card
    /// there; otherwise nothing is. Resolving: a token copy of that card
    /// (CR 707.2) for the player who activated it, wherever the card is by
    /// then (CR 608.2h: an object's last known information). Nothing else.
    fn back_from_the_brink_exiles_the_card_and_copies_it() {
        let mut b = brink();
        let named = match kani::any::<u8>() % 4 {
            0 => b.mikaeus[0],
            1 => b.mikaeus[1],
            2 => b.blank,
            _ => b.token,
        };
        let index = usize::try_from(named.0).expect("small");
        let o = b.state.get_object(named).expect("on the board");
        let exiled = o.zone == Zone::Graveyard && !o.is_token && b.you == PlayerId(0);
        let before = Snapshot::of(&b.state);
        let card = b.card();
        b.registry.get(card).expect("registered").pay_activation_cost(&mut b.state, b.me, index, &[], &b.registry);
        if exiled {
            assert!(asked_for(&[Act::Move(named, Zone::Exile)]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        let activator = PlayerId(u8::from(kani::any::<bool>()));
        b.state.resolving_ability_activator = Some(activator);
        let n = acted();
        b.registry.get(card).expect("registered").resolve_activated_ability(&mut b.state, b.me, index, &[], &b.registry);
        assert!(acted() == n + 1 && Some(&Act::TokenCopy(named, activator)) == super::act(n));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Graveyard Shovel: its target player exiles a card from their
    /// graveyard (no token, CR 109.1): the one there is, or with more,
    /// one they are asked to choose; with none, nothing. If it is a
    /// creature card, the player who activated the Shovel gains 2 life.
    /// Its target gone, nothing. Over the Shovel itself in player 0's
    /// graveyard or not, and a card and a token of any characteristics.
    fn graveyard_shovel_exiles_a_card_and_gains_life_for_a_creature() {
        let shovel = isd::graveyard_shovel::GraveyardShovel;
        let mut b = permanent(Box::new(shovel), &ANY_CREATURE, true);
        let target = any_target(b.others);
        let Target::Player(p) = target else {
            let before = b.resolve(0, &[target]);
            assert!(acted() == 0 && Snapshot::of(&b.state) == before);
            std::mem::forget(b);
            return;
        };
        let mut cards = Vec::with_capacity(3);
        for id in [b.me, b.others[0], b.others[1]] {
            let o = b.state.get_object(id).expect("on the board");
            if o.zone == Zone::Graveyard && o.owner == p && !o.is_token {
                cards.push(id);
            }
        }
        let before = b.resolve(0, &[target]);
        // Each list of a length known where it is compared.
        let exiled = |b: &Board, id: ObjectId| {
            if b.state.get_object(id).expect("on the board").card_types.contains(&CardType::Creature) {
                asked_for(&[Act::Move(id, Zone::Exile), Act::Life(b.you, 2)])
            } else {
                asked_for(&[Act::Move(id, Zone::Exile)])
            }
        };
        match cards.len() {
            0 => {
                assert!(acted() == 0);
            }
            1 => {
                assert!(exiled(&b, cards[0]));
            }
            _ => {
                let asked = matches!(&b.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                    player, source, choice: ResolutionChoiceKind::ChooseTarget {
                        options, optional: false, effect: PendingEffect::CardEffect { source_id, .. }, ..
                    },
                }) if *player == p && *source == b.me && *source_id == b.me
                    && options.len() == 2 && options[0] == Target::Object(cards[0]) && options[1] == Target::Object(cards[1]));
                assert!(asked && acted() == 0);
                b.state.awaiting_action = None;
                let chosen = cards[usize::from(kani::any::<bool>())];
                let card = b.card();
                b.registry.get(card).expect("registered").resolve_card_effect(&mut b.state, b.me, "", &Target::Object(chosen), &b.registry);
                assert!(exiled(&b, chosen));
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget((b, cards));
    }
}

/// Evil Twin's ability: on a permanent that entered as a copy with it
/// (CR 706.9a: the exception its copy effect makes), and on nothing
/// else, a declined Evil Twin being a 0/0 with no ability; costing
/// {U}{B} and {T}, and targeting a creature with the same name as the
/// permanent (the engine's `SameNameAsSource`). Its targets and its
/// resolution are the two proofs below: the three in one harness ran
/// out of memory at ten gigabytes. Whether it is a copy, and whether it
/// is on the battlefield, are constants in each harness: they decide the
/// length of the list of abilities, read past its end if the solver
/// chose them, and the proof ran out of memory at four gigabytes so.
///
/// For an Evil Twin that entered as a copy if `copy`, gone from the
/// battlefield if `gone`.
fn evil_twin_has_its_ability((copy, gone): (bool, bool)) {
    let mut b = permanent_at(Box::new(isd::evil_twin::EvilTwin), &ANY_CREATURE, gone, PlayerId(u8::from(kani::any::<bool>())));
    if copy {
        let card = b.card();
        b.state.get_object_mut(b.me).expect("on the board").copy_grantor = Some(card);
    }
    let card = b.card();
    let twin = b.registry.get(card).expect("registered");
    let defs = std::mem::ManuallyDrop::new(twin.activated_abilities(&b.state, b.me, &b.registry));
    if copy && b.on() {
        let ability = Ability {
            cost: mana(0, &[Color::Blue, Color::Black]), tap: true,
            target: Some(TargetRequirement::CreatureWithFilter(TargetFilter::SameNameAsSource)),
            ..Ability::default()
        };
        assert!(defs.len() == 1 && defs[0].ability_index == 0 && Ability::of(&defs[0]) == ability);
        std::mem::forget(ability);
    } else {
        assert!(defs.is_empty());
    }
    std::mem::forget(b);
}

case_by_case! { evil_twin_has_its_ability;
    evil_twin_as_a_copy_has_its_ability: (true, false),
    evil_twin_declined_has_no_ability: (false, false),
    evil_twin_as_a_copy_gone_has_no_ability: (true, true),
    evil_twin_declined_gone_has_no_ability: (false, true),
}

card_proof! {
    /// Evil Twin's target: as the card says, a creature on the battlefield
    /// (the name is the requirement's, the engine's); no player, nothing
    /// gone.
    fn evil_twin_targets_a_creature() {
        let b = permanent(Box::new(isd::evil_twin::EvilTwin), &ANY_CREATURE, true);
        let twin = isd::evil_twin::EvilTwin;
        for id in b.others {
            let o = b.state.get_object(id).expect("on the board");
            let creature = o.zone == Zone::Battlefield && o.card_types.contains(&CardType::Creature);
            assert!(twin.is_valid_target(&b.state, b.you, &Target::Object(id), &b.registry) == creature);
        }
        assert!(!twin.is_valid_target(&b.state, b.you, &Target::Player(b.you), &b.registry));
        assert!(!twin.is_valid_target(&b.state, b.you, &Target::Illegal, &b.registry));
        std::mem::forget(b);
    }
}

card_proof! {
    /// Evil Twin's ability resolving: its target destroyed; its target
    /// gone, nothing.
    fn evil_twin_destroys_its_target() {
        let mut b = permanent(Box::new(isd::evil_twin::EvilTwin), &ANY_CREATURE, true);
        let target = any_target(b.others);
        let before = b.resolve(0, &[target.clone()]);
        match target {
            Target::Object(id) => {
                assert!(asked_for(&[Act::Destroy(id)]));
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
    /// Grimgrin, Corpse-Born's ability: the creature sacrificed as its cost
    /// (pinned above), Grimgrin untapped and given a +1/+1 counter (on a
    /// Grimgrin gone by then, neither lands: `untap`'s and
    /// `add_counters`'). Nothing else.
    fn grimgrin_corpse_born_untaps_and_grows() {
        let mut b = permanent(Box::new(isd::grimgrin_corpse_born::GrimgrinCorpseBorn), &ANY_CREATURE, true);
        let before = b.resolve(0, &[]);
        assert!(asked_for(&[Act::Untap(b.me), Act::Counters(b.me, CounterType::PlusOnePlusOne, 1)]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Gavony Township's ability: {2}{G}{W} and {T}.
    fn gavony_township_has_its_ability() {
        has_ability_at(Box::new(isd::gavony_township::GavonyTownship), 1,
            Ability { cost: mana(2, &[Color::Green, Color::White]), tap: true, ..Ability::default() });
    }
}

card_proof! {
    /// Gavony Township: a +1/+1 counter on each creature the player who
    /// activated it controls as it resolves, and nothing else.
    fn gavony_township_puts_a_counter_on_each_of_your_creatures() {
        let mut b = permanent(Box::new(isd::gavony_township::GavonyTownship), &ANY_CREATURE, true);
        let yours = b.those(|o| o.zone == Zone::Battlefield && o.controller == b.you && o.card_types.contains(&CardType::Creature));
        let before = b.resolve(1, &[]);
        assert!(asked_each(yours.len(), |i| Act::Counters(yours[i], CounterType::PlusOnePlusOne, 1)) && Snapshot::of(&b.state) == before);
        std::mem::forget((b, yours));
    }
}

card_proof! {
    /// Ghost Quarter's ability: {T} and sacrificing itself, targeting a
    /// land.
    fn ghost_quarter_has_its_ability() {
        has_ability_at(Box::new(isd::ghost_quarter::GhostQuarter), 1, Ability {
            tap: true, sacrifice: SacrificeCost::SacrificeThis,
            target: Some(TargetRequirement::PermanentWithFilter(TargetFilter::HasCardType(vec![CardType::Land]))),
            ..Ability::default()
        });
    }
}

/// Ghost Quarter: its target destroyed, and then, destroyed or not
/// (the card's ruling), its controller as it was may search their
/// library for a basic land card to put onto the battlefield untapped:
/// offered every basic land card in that library, in order ("may": the
/// search is optional, and its shuffle the search's). Its target gone,
/// nothing. Over a library of a Forest and a nonbasic land card in
/// either order for each player.
///
/// For the target at place `i` of [`any_target`]'s list, a constant in
/// each harness: a target the solver chose, beside the libraries' orders,
/// ran the proof out of memory at ten gigabytes.
fn ghost_quarter_resolves(i: u8) {
    let mut b = permanent(Box::new(isd::ghost_quarter::GhostQuarter), &ANY_CREATURE, true);
    let forest_card = b.registry.register(Box::new(crate::cards::forest::Forest));
    let township_card = b.registry.register(Box::new(isd::gavony_township::GavonyTownship));
    let mut forests = [b.me; 2];
    for (p, forest) in forests.iter_mut().enumerate() {
        let owner = PlayerId(p as u8);
        *forest = b.state.create_object(forest_card, owner, Zone::Library, None, None);
        let other = b.state.create_object(township_card, owner, Zone::Library, None, None);
        let order = if kani::any() { [*forest, other] } else { [other, *forest] };
        b.state.players[p].library_order.extend_from_slice(&order);
    }
    let target = target_at(b.others, i);
    let land = match target {
        Target::Object(id) => {
            let o = b.state.get_object(id).expect("on the board");
            (o.zone == Zone::Battlefield).then_some((id, o.controller))
        }
        _ => None,
    };
    let before = b.resolve(1, &[target]);
    match land {
        Some((id, controller)) => {
            let candidates = vec![forests[usize::from(controller.0)]];
            assert!(asked_for(&[
                Act::Destroy(id),
                Act::Search { source: b.me, player: controller, candidates, to: Zone::Battlefield, tapped: false, optional: true },
            ]));
        }
        None => {
            assert!(acted() == 0);
        }
    }
    assert!(Snapshot::of(&b.state) == before);
    std::mem::forget(b);
}

case_by_case! { unwind(8) ghost_quarter_resolves;
    ghost_quarter_destroys_the_card_it_targets_and_offers_a_basic: 0,
    ghost_quarter_destroys_the_token_it_targets_and_offers_a_basic: 1,
    ghost_quarter_targeting_the_first_player_does_nothing: 2,
    ghost_quarter_targeting_the_second_player_does_nothing: 3,
    ghost_quarter_with_its_target_gone_does_nothing: 4,
}

card_proof! {
    /// Kessig Wolf Run's ability: {X}{R}{G} and {T}, targeting a creature
    /// (which, as the card says, must be on the battlefield); resolving,
    /// its target gets +X/+0 and trample until end of turn, X the value
    /// it was activated with (CR 107.3m); its target gone, nothing.
    /// Nothing else. Over X from 0 to 3.
    fn kessig_wolf_run_pumps_its_target() {
        let mut symbols = Vec::with_capacity(3);
        symbols.extend([ManaSymbol::X, ManaSymbol::Colored(Color::Red), ManaSymbol::Colored(Color::Green)]);
        has_ability_at(Box::new(isd::kessig_wolf_run::KessigWolfRun), 1,
            Ability { cost: ManaCost::new(symbols), tap: true, target: Some(TargetRequirement::Creature), ..Ability::default() });
        let mut b = permanent(Box::new(isd::kessig_wolf_run::KessigWolfRun), &ANY_CREATURE, true);
        let card = b.card();
        let run = b.registry.get(card).expect("registered");
        for id in b.others {
            let on = b.state.get_object(id).expect("on the board").zone == Zone::Battlefield;
            assert!(run.is_valid_target(&b.state, b.you, &Target::Object(id), &b.registry) == on);
        }
        assert!(!run.is_valid_target(&b.state, b.you, &Target::Player(b.you), &b.registry));
        let x: u32 = kani::any();
        kani::assume(x <= 3);
        b.state.last_activated_x_value = Some(x);
        let target = any_target(b.others);
        let before = b.resolve(1, &[target.clone()]);
        let added = b.added(&before);
        match target {
            Target::Object(id) => {
                let x = i32::try_from(x).expect("small");
                assert!(added == [T::ModifyPT { target: id, power_mod: x, toughness_mod: 0 }, T::GrantKeyword { target: id, keyword: Keyword::Trample }]);
            }
            _ => {
                assert!(added.is_empty());
            }
        }
        assert!(acted() == 0 && b.nothing_else_changed(&before));
        std::mem::forget(b);
    }
}

/// Moorland Haunt's board: [`permanent`]'s, and another card of the blank
/// card, a creature or not, in either player's graveyard.
fn haunt() -> (Board, ObjectId) {
    let b = permanent(Box::new(isd::moorland_haunt::MoorlandHaunt), &ANY_CREATURE, false);
    haunt_beside(b, PlayerId(u8::from(kani::any::<bool>())), kani::any())
}

/// [`haunt`]'s board on `b`, the other card in `owner`'s graveyard and a
/// creature card if `creature`.
fn haunt_beside(mut b: Board, owner: PlayerId, creature: bool) -> (Board, ObjectId) {
    let card = b.state.create_object(CardId(1), owner, Zone::Graveyard, None, None);
    let o = b.state.get_object_mut(card).expect("just created");
    o.card_types = Vec::with_capacity(1);
    if creature {
        o.card_types.push(CardType::Creature);
    }
    if owner == PlayerId(0) {
        b.state.players[0].graveyard_order.push(card);
    } else {
        b.state.players[1].graveyard_order.push(card);
    }
    (b, card)
}

impl Board {
    /// The creature cards in `p`'s graveyard among `ids`, in the
    /// graveyard's order, which is the order the board put them there.
    fn creature_cards_in_graveyard(&self, p: PlayerId, ids: &[ObjectId]) -> Vec<ObjectId> {
        let mut found = Vec::with_capacity(ids.len());
        for id in ids {
            let o = self.state.get_object(*id).expect("on the board");
            if o.zone == Zone::Graveyard && o.owner == p && !o.is_token && o.card_types.contains(&CardType::Creature) {
                found.push(*id);
            }
        }
        found
    }
}

/// Moorland Haunt's ability: {W}{U} and {T}, offered while its controller
/// has a creature card in their graveyard to exile (CR 601.2h: a cost that
/// cannot be paid cannot be chosen), and not otherwise. For a Haunt `you`
/// control, and the other card in either player's graveyard, a creature
/// card or not, each a constant in each harness: the engine looks for one
/// through the graveyard of the player the solver chose, an index into
/// the players it leaves open, and the proof ran out of memory at ten
/// gigabytes.
fn haunt_has_its_ability((you, owner, creature): (u8, u8, bool)) {
    let you = PlayerId(you);
    let b = permanent_of(Box::new(isd::moorland_haunt::MoorlandHaunt), &ANY_CREATURE, false, you);
    let (b, card) = haunt_beside(b, PlayerId(owner), creature);
    let exilable = b.creature_cards_in_graveyard(you, &[b.others[0], card]);
    let haunt = b.card();
    let defs = std::mem::ManuallyDrop::new(b.registry.get(haunt).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    if exilable.is_empty() {
        assert!(defs.is_empty());
    } else {
        let ability = Ability { cost: mana(0, &[Color::White, Color::Blue]), tap: true, ..Ability::default() };
        assert!(defs.len() == 1 && defs[0].ability_index == 1 && Ability::of(&defs[0]) == ability);
        std::mem::forget(ability);
    }
    std::mem::forget((b, exilable));
}

case_by_case! { haunt_has_its_ability;
    moorland_haunt_of_the_first_player_with_its_creature_card_has_its_ability: (0, 0, true),
    moorland_haunt_of_the_first_player_with_a_noncreature_card_has_none: (0, 0, false),
    moorland_haunt_of_the_first_player_with_the_others_creature_card_has_none: (0, 1, true),
    moorland_haunt_of_the_first_player_with_the_others_noncreature_card_has_none: (0, 1, false),
    moorland_haunt_of_the_second_player_with_the_others_creature_card_has_none: (1, 0, true),
    moorland_haunt_of_the_second_player_with_the_others_noncreature_card_has_none: (1, 0, false),
    moorland_haunt_of_the_second_player_with_its_creature_card_has_its_ability: (1, 1, true),
    moorland_haunt_of_the_second_player_with_a_noncreature_card_has_none: (1, 1, false),
}

card_proof! {
    /// Moorland Haunt, paying: the creature card in its controller's
    /// graveyard exiled (CR 602.2b, a cost), or with more than one, they
    /// are asked which (CR 601.2b) and it is exiled on the answer; then,
    /// resolving, a 1/1 white Spirit creature token with flying for the
    /// player who activated it. Nothing else.
    fn moorland_haunt_exiles_a_creature_card_and_makes_a_spirit() {
        let (mut b, card) = haunt();
        let you = b.state.get_object(b.me).expect("on the board").controller;
        let exilable = b.creature_cards_in_graveyard(you, &[b.others[0], card]);
        let haunt = b.card();
        let before = Snapshot::of(&b.state);
        b.registry.get(haunt).expect("registered").pay_activation_cost(&mut b.state, b.me, 1, &[], &b.registry);
        match exilable.len() {
            0 => {
                assert!(acted() == 0);
            }
            1 => {
                assert!(asked_for(&[Act::Move(exilable[0], Zone::Exile)]));
            }
            _ => {
                let Some(Act::Choose { source, player, options, effect: PendingEffect::CardEffect { source_id, .. }, optional: false }) = super::act(0) else {
                    panic!("asks which card")
                };
                assert!(acted() == 1 && *source == b.me && *player == you && *source_id == b.me);
                assert!(options.len() == 2 && options[0] == Target::Object(exilable[0]) && options[1] == Target::Object(exilable[1]));
                let chosen = exilable[usize::from(kani::any::<bool>())];
                b.registry.get(haunt).expect("registered").resolve_card_effect(&mut b.state, b.me, "", &Target::Object(chosen), &b.registry);
                assert!(acted() == 2 && Some(&Act::Move(chosen, Zone::Exile)) == super::act(1));
            }
        }
        let n = acted();
        b.registry.get(haunt).expect("registered").resolve_activated_ability(&mut b.state, b.me, 1, &[], &b.registry);
        assert!(acted() == n + 1 && Some(&Act::Tokens {
            n: 1, controller: b.you, power: 1, toughness: 1, colors: vec![Color::White],
            types: vec![CardType::Creature], keywords: vec![Keyword::Flying], subtypes: vec!["Spirit".into()],
        }) == super::act(n));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget((b, exilable));
    }
}

card_proof! {
    /// Nephalia Drownyard: {1}{U}{B} and {T}, targeting a player, who mills
    /// three cards; its target gone, nothing.
    fn nephalia_drownyard_mills_its_target_three() {
        has_ability_at(Box::new(isd::nephalia_drownyard::NephaliaDrownyard), 1,
            Ability { cost: mana(1, &[Color::Blue, Color::Black]), tap: true, target: Some(TargetRequirement::PlayerOnly), ..Ability::default() });
        let mut b = permanent(Box::new(isd::nephalia_drownyard::NephaliaDrownyard), &ANY_CREATURE, true);
        let target = any_target(b.others);
        let before = b.resolve(1, &[target.clone()]);
        match target {
            Target::Player(p) => {
                assert!(asked_for(&[Act::Mill(p, 3)]));
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
    /// Stensia Bloodhall: {3}{B}{R} and {T}, targeting a player or
    /// planeswalker, which takes 2 damage from it (one gone illegal by
    /// then is the effect's to skip, CR 608.2b).
    fn stensia_bloodhall_deals_two_to_its_target() {
        has_ability_at(Box::new(isd::stensia_bloodhall::StensiaBloodhall), 1,
            Ability { cost: mana(3, &[Color::Black, Color::Red]), tap: true, target: Some(TargetRequirement::PlayerOrPlaneswalker), ..Ability::default() });
        let mut b = permanent(Box::new(isd::stensia_bloodhall::StensiaBloodhall), &ANY_CREATURE, true);
        let target = any_target(b.others);
        let before = b.resolve(1, &[target.clone()]);
        assert!(asked_for(&[Act::Effect(target, PendingEffect::DealDamage { amount: 2, source_id: b.me })]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// What each of these cards accepts as a target, besides what its
    /// requirement asks (the engine asks both, CR 608.2b): Heretic's
    /// Punishment, any target, a permanent on the battlefield or a player;
    /// Olivia Voldaren, a creature on the battlefield; Graveyard Shovel, a
    /// player; and none of them a target gone illegal.
    fn targeted_abilities_accept_what_their_text_targets() {
        let mut b = permanent(Box::new(isd::olivia_voldaren::OliviaVoldaren), &ANY_CREATURE, true);
        let shovel = b.registry.register(Box::new(isd::graveyard_shovel::GraveyardShovel));
        let punishment = b.registry.register(Box::new(isd::heretics_punishment::HereticsPunishment));
        let olivia = b.card();
        let p = PlayerId(u8::from(kani::any::<bool>()));
        for id in [b.me, b.others[0], b.others[1]] {
            let o = b.state.get_object(id).expect("on the board");
            let on = o.zone == Zone::Battlefield;
            // Olivia a creature card, printed so; the blank card's objects
            // creatures by the types they were given.
            let creature = on && (id == b.me || o.card_types.contains(&CardType::Creature));
            let t = Target::Object(id);
            assert!(b.registry.get(punishment).expect("registered").is_valid_target(&b.state, b.you, &t, &b.registry) == on);
            assert!(b.registry.get(olivia).expect("registered").is_valid_target(&b.state, b.you, &t, &b.registry) == creature);
            assert!(!b.registry.get(shovel).expect("registered").is_valid_target(&b.state, b.you, &t, &b.registry));
        }
        for (card, player_ok) in [(punishment, true), (olivia, false), (shovel, true)] {
            let behavior = b.registry.get(card).expect("registered");
            assert!(behavior.is_valid_target(&b.state, b.you, &Target::Player(p), &b.registry) == player_ok);
            assert!(!behavior.is_valid_target(&b.state, b.you, &Target::Illegal, &b.registry));
        }
        std::mem::forget(b);
    }
}

/// The ability of `source` resolving, put on the stack from its back face
/// or not (CR 712.8d-e: the face up as it was activated, whichever is up
/// now).
fn resolving_from(source: ObjectId, card: CardId, activator: PlayerId, back: bool) -> crate::state::StackEntry {
    crate::state::StackEntry::Ability {
        source_id: source, ability_index: 0, behavior_card_id: card, targets: Vec::new(), activator, x_value: None,
        target_requirement: None, sacrificed: None, sacrificed_toughness: None, loyalty: false, source_transforms: 0,
        source_back_face: back,
    }
}

/// Daybreak Ranger: face up, "{T}: This creature deals 2 damage to
/// target creature with flying"; transformed, Nightfall Predator's
/// "{R}, {T}: This creature fights target creature" (CR 701.14a); off
/// the battlefield, neither. Its resolution is the proof below: the two
/// in one harness, over a target the solver chose, ran out of memory at
/// ten gigabytes. Where it is and which face is up are constants in each
/// harness: the abilities are a list of a length they decide, read past
/// its end if the solver chose them.
///
/// With the Ranger in its owner's graveyard for 0, front face up on the
/// battlefield for 1, and transformed there for 2.
fn daybreak_ranger_has_its_abilities_placed(placed: u8) {
    let mut b = permanent_at(Box::new(isd::daybreak_ranger::DaybreakRanger), &ANY_CREATURE, placed == 0, PlayerId(u8::from(kani::any::<bool>())));
    let transformed = placed == 2;
    b.state.get_object_mut(b.me).expect("on the board").is_transformed = transformed;
    let card = b.card();
    let defs = std::mem::ManuallyDrop::new(b.registry.get(card).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    if !b.on() {
        assert!(defs.is_empty());
    } else if transformed {
        let fight = Ability { cost: mana(0, &[Color::Red]), tap: true, target: Some(TargetRequirement::Creature), ..Ability::default() };
        assert!(defs.len() == 1 && defs[0].ability_index == 0 && Ability::of(&defs[0]) == fight);
        std::mem::forget(fight);
    } else {
        let ping = Ability {
            tap: true, target: Some(TargetRequirement::CreatureWithFilter(TargetFilter::HasKeyword(Keyword::Flying))), ..Ability::default()
        };
        assert!(defs.len() == 1 && defs[0].ability_index == 0 && Ability::of(&defs[0]) == ping);
        std::mem::forget(ping);
    }
    std::mem::forget(b);
}

case_by_case! { daybreak_ranger_has_its_abilities_placed;
    daybreak_ranger_gone_has_no_ability: 0,
    daybreak_ranger_face_up_pings_a_flyer: 1,
    nightfall_predator_fights: 2,
}

card_proof! {
    /// Daybreak Ranger's ability resolving as the face it was activated
    /// from says, whichever face is up by then (CR 712.8d-e: Moonmist in
    /// response turns no ping into a fight): 2 damage to its target, or a
    /// fight with it; its target gone, nothing.
    fn daybreak_ranger_pings_a_flyer_or_nightfall_predator_fights() {
        let mut b = permanent(Box::new(isd::daybreak_ranger::DaybreakRanger), &ANY_CREATURE, true);
        let transformed = b.on() && kani::any();
        b.state.get_object_mut(b.me).expect("on the board").is_transformed = transformed;
        let card = b.card();
        let back: bool = kani::any();
        b.state.resolving_ability = Some(resolving_from(b.me, card, b.you, back));
        let target = any_target(b.others);
        let before = b.resolve(0, &[target.clone()]);
        match target {
            Target::Object(id) if back => {
                assert!(asked_for(&[Act::Fight(b.me, id)]));
            }
            Target::Object(id) => {
                assert!(asked_for(&[Act::Effect(Target::Object(id), PendingEffect::DealDamage { amount: 2, source_id: b.me })]));
            }
            _ => {
                assert!(acted() == 0);
            }
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

/// Ulvenwald Mystics: face up, no ability; transformed, Ulvenwald
/// Primordials' "{G}: Regenerate this creature"; resolving, a
/// regeneration shield on it. Nothing else. Where it is and which face is
/// up are constants in each harness: whether it has the ability is a list
/// of abilities of a length they decide, read past its end if the solver
/// chose them, and the proof ran out of memory at ten gigabytes so. With
/// it in its owner's graveyard for 0, front face up on the battlefield for
/// 1, and transformed there for 2.
fn ulvenwald_mystics_placed(placed: u8) {
    let mut b = permanent_at(Box::new(isd::ulvenwald_mystics::UlvenwaldMystics), &ANY_CREATURE, placed == 0, PlayerId(u8::from(kani::any::<bool>())));
    let transformed = placed == 2;
    b.state.get_object_mut(b.me).expect("on the board").is_transformed = transformed;
    let card = b.card();
    let defs = std::mem::ManuallyDrop::new(b.registry.get(card).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    if transformed {
        let regenerate = Ability { cost: mana(0, &[Color::Green]), ..Ability::default() };
        assert!(defs.len() == 1 && defs[0].ability_index == 0 && Ability::of(&defs[0]) == regenerate);
        std::mem::forget(regenerate);
    } else {
        assert!(defs.is_empty());
    }
    let before = b.resolve(0, &[]);
    assert!(asked_for(&[Act::Regenerate(b.me)]) && Snapshot::of(&b.state) == before);
    std::mem::forget(b);
}

case_by_case! { ulvenwald_mystics_placed;
    ulvenwald_mystics_gone_has_no_ability: 0,
    ulvenwald_mystics_face_up_has_no_ability: 1,
    ulvenwald_primordials_regenerates: 2,
}
