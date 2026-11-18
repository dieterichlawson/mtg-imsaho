//! The permanents with activated abilities (CR 602): each ability as the
//! engine reads it to offer and pay for it, pinned to its Oracle text, and
//! what the ability does as it resolves.
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
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
//!   Its second ability hands the whole return to the engine as one event
//!   (`GameState::put_onto_battlefield_together`, #1064-#1067), and is
//!   played, not proved: `tests/enters_as_copy.rs`,
//!   `tests/cards_complex_creatures.rs` and the generated boards of
//!   `tests/card_semantics.rs`.
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

use super::{acted, any_object, any_target, board, both, card_proof, case_by_case, each_case, each_is, has_subtype, kept, prints_where, Form, Kinds, Snapshot};
use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameObject, GameState, TemporaryEffect as T};
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
    /// Whether `def` is this ability, each field compared where it lies,
    /// this (the expected value) on the left.
    ///
    /// Not by building an `Ability` of `def` and comparing the two: that
    /// cloned the engine's target requirement, a type that holds itself,
    /// and an ability read out of a list built in one branch or another is
    /// of a variant the solver leaves open, so the clone went down every
    /// shape a requirement can take. It was half the symbolic execution of
    /// Moorland Haunt's proof, where it ran out of memory.
    pub fn describes(&self, def: &ActivatedAbilityDef) -> bool {
        self.cost == def.cost
            && self.tap == def.requires_tap
            && self.sacrifice == def.sacrifice_cost
            && self.remove_counters == def.counter_cost
            && self.tap_creatures == def.tap_cost.map(|c| c.count)
            && self.target == def.target_requirement
            && self.once_per_turn == def.once_per_turn
            && self.sorcery_speed == def.sorcery_speed_only
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
        kept(&self.others, |id| pred(self.state.get_object(id).expect("on the board")))
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
        assert!(def.ability_index == i && e.describes(def));
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
    assert!(defs.len() == 1 && defs[0].ability_index == index && expected.describes(&defs[0]));
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
/// counters ... and sacrifice it", always. With `n` cards in the hand of
/// its controller, player `you`.
fn grimoire_of_the_dead_has_its_abilities_with((n, you): (usize, u8)) {
    let mut b = permanent_of(Box::new(isd::grimoire_of_the_dead::GrimoireOfTheDead), &ANY_CREATURE, false, PlayerId(you));
    let you = b.you;
    let hand = hands(&mut b.state, CardId(1), you, n);
    let card = b.card();
    let defs = std::mem::ManuallyDrop::new(b.registry.get(card).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    let discard = Ability { cost: mana(1, &[]), tap: true, ..Ability::default() };
    let sacrifice = Ability {
        tap: true, sacrifice: SacrificeCost::SacrificeThis, remove_counters: Some((CounterType::Study, 3)), ..Ability::default()
    };
    if hand.is_empty() {
        assert!(defs.len() == 1 && defs[0].ability_index == 1 && sacrifice.describes(&defs[0]));
    } else {
        assert!(defs.len() == 2 && defs[0].ability_index == 0 && discard.describes(&defs[0]));
        assert!(defs[1].ability_index == 1 && sacrifice.describes(&defs[1]));
    }
    std::mem::forget((b, hand, discard, sacrifice));
}

// The proofs above, a harness for each size of hand up to 2 and each
// controller: a hand whose size the solver chose is a list of every
// length, and every card made after it an id it leaves open; the
// Grimoire's proofs ran out of memory at ten gigabytes, with the sizes in
// one harness each at four, and with a harness per size, the controller
// the solver's (whose hand the engine finds by them), at three and a half.
case_by_case! { grimoire_of_the_dead_has_its_abilities_with;
    grimoire_of_the_dead_has_its_abilities_with_an_empty_hand_for_the_first_player: (0, 0),
    grimoire_of_the_dead_has_its_abilities_with_a_card_in_hand_for_the_first_player: (1, 0),
    grimoire_of_the_dead_has_its_abilities_with_two_cards_in_hand_for_the_first_player: (2, 0),
    grimoire_of_the_dead_has_its_abilities_with_an_empty_hand_for_the_second_player: (0, 1),
    grimoire_of_the_dead_has_its_abilities_with_a_card_in_hand_for_the_second_player: (1, 1),
    grimoire_of_the_dead_has_its_abilities_with_two_cards_in_hand_for_the_second_player: (2, 1),
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
}

/// [`brink`]'s board, with the second Mikaeus in the graveyard if
/// `second_dead`, the blank card a creature if `blank_creature`, and Back
/// from the Brink controlled by `you`.
fn brink_shaped(second_dead: bool, blank_creature: bool, you: PlayerId) -> Brink {
    let (mut registry, mut state, blank_card, card) = board(Box::new(isd::back_from_the_brink::BackFromTheBrink));
    let mikaeus_card = registry.register(Box::new(isd::mikaeus_the_lunarch::MikaeusTheLunarch));
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
    Brink { registry, state, me, you, mikaeus: [first, second], blank }
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
/// For one board and controller, a constant in each harness: a board the
/// solver chose had a graveyard of every length, and so a list of
/// abilities of every length, each compared, and the proof ran out of
/// memory at ten gigabytes; the four boards in one harness, at four; a
/// harness per board, the controller (whose graveyard the abilities are
/// of) the solver's, at three and a half.
fn back_from_the_brink_has_its_abilities((second_dead, blank_creature, you): (bool, bool, u8)) {
    let b = brink_shaped(second_dead, blank_creature, PlayerId(you));
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
            assert!(def.ability_index == usize::try_from(id.0).expect("small") && ability.describes(def));
        }
        std::mem::forget(expected);
    }
    std::mem::forget(b);
}

case_by_case! { back_from_the_brink_has_its_abilities;
    back_from_the_brink_has_an_ability_for_one_mikaeus: (false, false, 0),
    back_from_the_brink_has_an_ability_for_one_mikaeus_and_a_creature_card: (false, true, 0),
    back_from_the_brink_has_one_ability_for_two_mikaeus: (true, false, 0),
    back_from_the_brink_has_one_ability_for_two_mikaeus_and_one_for_a_creature_card: (true, true, 0),
    back_from_the_brink_of_the_other_player_has_none_over_one_mikaeus: (false, false, 1),
    back_from_the_brink_of_the_other_player_has_none_over_one_mikaeus_and_a_creature_card: (false, true, 1),
    back_from_the_brink_of_the_other_player_has_none_over_two_mikaeus: (true, false, 1),
    back_from_the_brink_of_the_other_player_has_none_over_two_mikaeus_and_a_creature_card: (true, true, 1),
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
        assert!(defs.len() == 1 && defs[0].ability_index == 0 && ability.describes(&defs[0]));
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
    /// Gavony Township's ability: {2}{G}{W} and {T}.
    fn gavony_township_has_its_ability() {
        has_ability_at(Box::new(isd::gavony_township::GavonyTownship), 1,
            Ability { cost: mana(2, &[Color::Green, Color::White]), tap: true, ..Ability::default() });
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

/// Moorland Haunt's board: `b`, and another card of the blank card in
/// `owner`'s graveyard, a creature card if `creature`.
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
        assert!(defs.len() == 1 && defs[0].ability_index == 1 && ability.describes(&defs[0]));
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

/// Daybreak Ranger: face up, "{T}: This creature deals 2 damage to
/// target creature with flying"; transformed, Nightfall Predator's
/// "{R}, {T}: This creature fights target creature" (CR 701.14a); off
/// the battlefield, neither. The two are numbered apart, 0 and 1, so the
/// one activated is named by its index alone (#986). Its resolution is the proof below: the two
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
        assert!(defs.len() == 1 && defs[0].ability_index == 1 && fight.describes(&defs[0]));
        std::mem::forget(fight);
    } else {
        let ping = Ability {
            tap: true, target: Some(TargetRequirement::CreatureWithFilter(TargetFilter::HasKeyword(Keyword::Flying))), ..Ability::default()
        };
        assert!(defs.len() == 1 && defs[0].ability_index == 0 && ping.describes(&defs[0]));
        std::mem::forget(ping);
    }
    std::mem::forget(b);
}

case_by_case! { daybreak_ranger_has_its_abilities_placed;
    daybreak_ranger_gone_has_no_ability: 0,
    daybreak_ranger_face_up_pings_a_flyer: 1,
    nightfall_predator_fights: 2,
}

