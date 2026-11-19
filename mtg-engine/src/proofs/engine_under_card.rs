//! A card resolving with the engine itself under it: no recorder stands in
//! for an engine action here. The card's own hook runs (a spell's
//! `on_resolve`, a permanent's enters trigger), and under it the
//! engine's own destruction, zone change, damage, drawing, token making
//! and continuous effects; what is asserted is the board afterwards, read
//! back through the engine's own characteristics, against the card's text.
//! `docs/plans/cards-in-the-loop.md`, B2, has the measurements these were
//! shaped by.
//!
//! What every proof still takes out: `format!` and the game log, which no
//! rule reads, and growing and freeing an allocation (`proofs::cards`
//! says why of each).
//!
//! What a proof here does not cover: the spell's trip through the stack
//! (its targets re-checked, the card put where it goes after), which is
//! the engine's and proved of it (`proofs::actions::resolving`); with the
//! engine's own code under it that step ran out of memory at 12 GB even
//! on a board with nothing open. Every card through cast, stack,
//! resolution, state-based actions and triggers together, the engine's
//! own, is `tests/card_semantics.rs` on every small board.
//!
//! What keeps one in reach, each measured: a player the solver leaves open
//! is a library or a hand it leaves open, so a card that draws has a
//! harness for each player (open, Think Twice ran out of memory at 12 GB;
//! fixed, it is under a minute and a gigabyte); and a list the solver may
//! or may not have pushed to is one of either length to every scan after,
//! so an effect that may be there is always pushed, as one kind or
//! another.

use crate::actions::Target;
use crate::cards::CardBehavior;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{CombatState, GameState, StackEntry, TemporaryEffect};
use crate::types::{CardType, Color, CounterType, Keyword, ManaType, Zone};

/// What a spell's target is on the battlefield: a permanent of the blank
/// card, made each of these.
#[derive(Clone, Copy, PartialEq)]
enum Permanent {
    /// A creature, a 2/2 or a 2/3.
    Creature,
    /// A creature of power 4, a 4/2 or a 4/3 (Smite the Monstrous).
    Monstrous,
    /// A Spirit creature (Urgent Exorcism).
    Spirit,
    /// A Human creature (Bonds of Faith).
    Human,
    /// A creature attacking the first player (Rebuke).
    Attacker,
    Artifact,
    Enchantment,
    Land,
    /// A planeswalker with three loyalty counters.
    Planeswalker,
}

impl Permanent {
    fn is_creature(self) -> bool {
        matches!(self, Permanent::Creature | Permanent::Monstrous | Permanent::Spirit | Permanent::Human | Permanent::Attacker)
    }
}

/// A permanent of the blank card on the battlefield, the second player's
/// card under either player's control, tapped or not, `what` it is; a
/// creature has a point of damage or none and a toughness of 2 or 3.
fn a_permanent(state: &mut GameState, blank: CardId, what: Permanent) -> ObjectId {
    let (power, toughness) = match what {
        Permanent::Monstrous => (Some(4), Some(2)),
        _ if what.is_creature() => (Some(2), Some(2)),
        _ => (None, None),
    };
    let id = state.create_object(blank, PlayerId(1), Zone::Battlefield, power, toughness);
    let o = state.get_object_mut(id).expect("just created");
    o.card_types = vec![match what {
        Permanent::Artifact => CardType::Artifact,
        Permanent::Enchantment => CardType::Enchantment,
        Permanent::Land => CardType::Land,
        Permanent::Planeswalker => CardType::Planeswalker,
        _ => CardType::Creature,
    }];
    o.controller = PlayerId(u8::from(kani::any::<bool>()));
    o.tapped = kani::any();
    if what.is_creature() {
        o.damage_marked = u32::from(kani::any::<bool>());
        if kani::any() {
            o.toughness = Some(3);
        }
    }
    if what == Permanent::Spirit {
        o.subtypes = vec!["Spirit".to_string()];
    }
    if what == Permanent::Human {
        o.subtypes = vec!["Human".to_string()];
    }
    if what == Permanent::Planeswalker {
        o.counters.insert(CounterType::Loyalty, 3);
    }
    if what == Permanent::Attacker {
        let mut combat = CombatState::new();
        combat.attackers.insert(id, PlayerId(0));
        state.combat = Some(combat);
    }
    id
}

/// "Destroy target <permanent>." resolving at a permanent its text names,
/// the engine's own destruction and zone change under the card's
/// `on_resolve`: the permanent is put into its owner's graveyard (CR
/// 701.7a, 400.3) and is no longer in combat (506.4), unless it is
/// indestructible and nothing happens to it (702.12b), or it has a
/// regeneration shield and is tapped, its damage removed, out of combat
/// and the shield used up instead (701.15a). For the second player's
/// permanent under either player, tapped or not, shielded or not,
/// indestructible or not; a creature damaged or not, of toughness 2 or 3.
fn destroys((card, what): (Box<dyn CardBehavior>, Permanent)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let owner = PlayerId(1);
    let target = a_permanent(&mut state, blank, what);
    state.get_object_mut(target).expect("just created").regeneration_shields = u32::from(kani::any::<bool>());
    // Indestructible as an effect gives it until end of turn: a card's
    // printed keywords are its card's, and the blank card prints none. One
    // effect either way, of indestructible or of flying: a list the solver
    // may or may not have pushed to is one of either length to every scan
    // of it after.
    let indestructible: bool = kani::any();
    let keyword = if indestructible { Keyword::Indestructible } else { Keyword::Flying };
    state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target, keyword });
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let before = state.get_object(target).expect("just created");
    let (shield, tapped, damage, controller) = (before.regeneration_shields > 0, before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    let attacking = state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&target));
    if indestructible {
        assert!(o.zone == Zone::Battlefield && o.regeneration_shields == u32::from(shield) && o.tapped == tapped && o.damage_marked == damage);
        assert!(o.controller == controller && attacking == (what == Permanent::Attacker));
    } else if shield {
        assert!(o.zone == Zone::Battlefield && o.tapped && o.damage_marked == 0 && o.regeneration_shields == 0);
        assert!(o.controller == controller && !attacking);
    } else {
        assert!(o.zone == Zone::Graveyard && o.controller == owner && !o.tapped && o.damage_marked == 0);
        assert!(state.players[1].graveyard_order.contains(&target) && !attacking);
    }
    kani::cover!(o.zone == Zone::Graveyard, "destroyed");
    kani::cover!(o.zone == Zone::Battlefield && !indestructible, "regenerated");
    std::mem::forget((registry, state));
}

/// "Target creature gets +P/+T [and gains a keyword] until end of turn."
/// resolving at a creature, read back through the engine's own power,
/// toughness and keywords (CR 613.4c, 613.1f): that much more of each than
/// before, the keyword if the text gives one, untapped if the text untaps
/// it (701.21a) and otherwise tapped as it was, and nothing else about
/// the creature changed.
fn pumps((card, (power_up, toughness_up), keyword, untaps): (Box<dyn CardBehavior>, (i32, i32), Option<Keyword>, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let (power, toughness) = (state.effective_power(target, &registry), state.effective_toughness(target, &registry));
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    assert!(state.effective_power(target, &registry) == power.map(|p| p + power_up));
    assert!(state.effective_toughness(target, &registry) == toughness.map(|t| t + toughness_up));
    if let Some(keyword) = keyword {
        assert!(state.has_keyword(target, keyword, &registry));
    }
    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == (tapped && !untaps) && o.damage_marked == damage && o.controller == controller);
    kani::cover!(toughness == Some(3), "a 2/3");
    std::mem::forget((registry, state));
}

/// What a spell does to each of up to two target creatures.
#[derive(Clone, Copy)]
enum ToEach {
    /// Puts a +1/+1 counter on it (Travel Preparations).
    Counter,
    /// Taps it (Feeling of Dread).
    Tap,
    /// It can't block this turn (Nightbird's Clutches).
    CantBlock,
}

/// "... each of up to two target creatures" resolving with `targets` of
/// two creatures on the battlefield as its targets, the first `targets` of
/// them: each one targeted has had `does` done to it, read back through
/// the engine (a counter and the +1/+1 it gives, CR 122.1a, 613.4c; tapped,
/// 701.21a; unable to block), and each not targeted is as it was.
fn does_to_each_of_up_to_two((card, does, targets): (Box<dyn CardBehavior>, ToEach, usize)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let creatures = [a_permanent(&mut state, blank, Permanent::Creature), a_permanent(&mut state, blank, Permanent::Creature)];
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let chosen = [Target::Object(creatures[0]), Target::Object(creatures[1])];
    let mut before = [(None, None, false, 0); 2];
    for (i, &c) in creatures.iter().enumerate() {
        let o = state.get_object(c).expect("just created");
        before[i] = (state.effective_power(c, &registry), state.effective_toughness(c, &registry), o.tapped, o.damage_marked);
    }

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &chosen[..targets], &registry);

    for (i, &c) in creatures.iter().enumerate() {
        let (power, toughness, tapped, damage) = before[i];
        let hit = i < targets;
        let o = state.get_object(c).expect("still an object");
        let counters = o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
        assert!(o.zone == Zone::Battlefield && o.damage_marked == damage);
        let grown = i32::from(hit && matches!(does, ToEach::Counter));
        assert!(counters == u32::from(hit && matches!(does, ToEach::Counter)));
        assert!(o.tapped == (tapped || (hit && matches!(does, ToEach::Tap))));
        assert!(state.effective_power(c, &registry) == power.map(|p| p + grown));
        assert!(state.effective_toughness(c, &registry) == toughness.map(|t| t + grown));
        assert!(state.can_block(c, &registry) != (hit && matches!(does, ToEach::CantBlock)));
    }
    std::mem::forget((registry, state));
}

/// What a spell gives each creature of one side until end of turn.
#[derive(Clone, Copy)]
enum ToAll {
    /// Creatures you control get +2/+0 (Rally the Peasants).
    YoursPlusTwo,
    /// Vampire creatures you control get +2/+0 and gain first strike
    /// (Vampiric Fury).
    YourVampiresPlusTwoAndFirstStrike,
    /// Creatures your opponents control get -4/-0 (Hysterical Blindness).
    TheirsMinusFour,
}

/// A spell that gives every creature of one side something until end of
/// turn, resolving for the first player on a board of two creatures, each
/// under either player (the second a Vampire or not): each creature the
/// text names has it, read back through the engine's own power, toughness
/// and keywords (CR 611.2c: the creatures there as it resolves), and each
/// other creature is as it was.
fn gives_each((card, does): (Box<dyn CardBehavior>, ToAll)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let creatures = [a_permanent(&mut state, blank, Permanent::Creature), a_permanent(&mut state, blank, Permanent::Creature)];
    // One subtype either way, as one effect either way in `destroys`.
    let vampire: bool = kani::any();
    state.get_object_mut(creatures[1]).expect("just created").subtypes = vec![if vampire { "Vampire" } else { "Human" }.to_string()];
    let you = PlayerId(0);
    let spell = state.create_object(card, you, Zone::Stack, None, None);
    let mut before = [(None, None, you); 2];
    for (i, &c) in creatures.iter().enumerate() {
        before[i] = (state.effective_power(c, &registry), state.effective_toughness(c, &registry), state.get_object(c).expect("just created").controller);
    }

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    for (i, &c) in creatures.iter().enumerate() {
        let (power, toughness, controller) = before[i];
        let (up, first_strike) = match does {
            ToAll::YoursPlusTwo => (if controller == you { 2 } else { 0 }, false),
            ToAll::YourVampiresPlusTwoAndFirstStrike => {
                let named = controller == you && i == 1 && vampire;
                (if named { 2 } else { 0 }, named)
            }
            ToAll::TheirsMinusFour => (if controller == you { 0 } else { -4 }, false),
        };
        assert!(state.effective_power(c, &registry) == power.map(|p| p + up));
        assert!(state.effective_toughness(c, &registry) == toughness);
        assert!(state.has_keyword(c, Keyword::FirstStrike, &registry) == first_strike);
    }
    std::mem::forget((registry, state));
}

/// "Draw N cards." resolving for the player `you` names (each in a harness
/// of its own: a player the solver leaves open is a library it leaves
/// open, and the draw ran out of memory), each player with a library of
/// `cards` cards: the top N of its controller's library, or all of it if
/// there are fewer, are in their hand and no longer in their library (CR
/// 121.1, 121.2), top first, and with fewer than N to draw they have drawn
/// from an empty library (704.5b); the other player's library and hand
/// are as they were.
fn draws((card, n, cards, you): (Box<dyn CardBehavior>, usize, usize, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let mut libraries = [[None; 3]; 2];
    for p in 0..2u8 {
        for k in 0..cards {
            let id = state.create_object(blank, PlayerId(p), Zone::Library, None, None);
            if p == 0 {
                state.players[0].library_order.push(id);
            } else {
                state.players[1].library_order.push(id);
            }
            libraries[p as usize][k] = Some(id);
        }
    }
    let spell = state.create_object(card, PlayerId(u8::from(you)), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    let (mine, theirs) = if you { (1, 0) } else { (0, 1) };
    assert!(state.players[theirs].library_order.len() == cards && !state.players[theirs].has_drawn_from_empty);
    for k in 0..cards {
        let theirs_still = libraries[theirs][k].expect("made");
        assert!(state.get_object(theirs_still).expect("still an object").zone == Zone::Library);
        let o = state.get_object(libraries[mine][k].expect("made")).expect("still an object");
        assert!(o.zone == if k < n { Zone::Hand } else { Zone::Library });
        assert!(o.controller == PlayerId(u8::from(you)));
    }
    assert!(state.players[mine].library_order.len() == cards.saturating_sub(n));
    if cards > n {
        assert!(state.players[mine].library_order[0] == libraries[mine][n].expect("made"));
    }
    assert!(state.players[mine].has_drawn_from_empty == (cards < n));
    std::mem::forget((registry, state));
}

/// "Create N P/T <colour> <subtype> creature tokens [with a keyword]."
/// resolving for either player on a board with nothing that changes how
/// tokens are made: N new permanents, each a token, a creature of that
/// power, toughness, colour and subtype (and keyword), under the spell's
/// controller (CR 111.1, 111.2), read back through the engine's own
/// characteristics.
fn makes_tokens((card, n, (power, toughness), colour, subtype, keyword): (Box<dyn CardBehavior>, u64, (i32, i32), Color, &str, Option<Keyword>)) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let spell = state.create_object(card, you, Zone::Stack, None, None);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    assert!(state.objects.len() == before + usize::try_from(n).expect("a few"));
    for k in 1..=n {
        let id = ObjectId(spell.0 + k);
        let t = state.get_object(id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
        assert!(state.is_creature(id, &registry));
        if let Some(keyword) = keyword {
            assert!(state.has_keyword(id, keyword, &registry));
        }
        assert!(state.effective_power(id, &registry) == Some(power) && state.effective_toughness(id, &registry) == Some(toughness));
        assert!(t.colors == [colour] && t.subtypes.len() == 1 && t.subtypes[0] == subtype);
    }
    std::mem::forget((registry, state));
}

/// How much a damage spell deals as it resolves: a constant of each
/// harness. Left to the solver (Brimstone Volley's 3 or 5 in one harness),
/// the engine's damage ran out of memory at 7 GB.
#[derive(Clone, Copy)]
enum Amount {
    Fixed(u32),
    /// 5 if a creature died this turn, else 3 (Brimstone Volley's morbid,
    /// CR 700.4): which, the harness says.
    Morbid { died: bool },
    /// The X it was cast with (Devil's Play, CR 107.3).
    X(u32),
}

/// The spell `card` on the stack, the first player's, and how much damage
/// its text says it deals, the board set up as `amount` says.
fn a_damage_spell(state: &mut GameState, card: CardId, amount: Amount) -> (ObjectId, u32) {
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let n = match amount {
        Amount::Fixed(n) => n,
        Amount::Morbid { died } => {
            state.creature_died_this_turn = died;
            if died { 5 } else { 3 }
        }
        Amount::X(x) => {
            state.get_object_mut(spell).expect("just created").x_value = Some(x);
            x
        }
    };
    (spell, n)
}

/// "~ deals N damage to <target>." resolving at a creature, by the
/// engine's own damage: N more damage is marked on it than before (CR
/// 120.3e), and it is otherwise as it was. State-based actions are not
/// run here: what lethal damage does is theirs.
fn deals_to_a_creature((card, amount): (Box<dyn CardBehavior>, Amount)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let (spell, n) = a_damage_spell(&mut state, card, amount);
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == damage + n && o.tapped == tapped && o.controller == controller);
    assert!(state.players[0].life == life[0] && state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// The same at a player, either one: that player has N life fewer (CR
/// 120.3a), and the other as many as before.
fn deals_to_a_player((card, amount): (Box<dyn CardBehavior>, Amount)) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let (spell, n) = a_damage_spell(&mut state, card, amount);
    let at = u8::from(kani::any::<bool>());
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Player(PlayerId(at))], &registry);

    for p in 0..2u8 {
        let lost = if p == at { i32::try_from(n).expect("small") } else { 0 };
        assert!(state.players[p as usize].life == life[p as usize] - lost);
    }
    std::mem::forget((registry, state));
}

/// The same at a planeswalker with three loyalty counters: N of them are
/// removed, or all three if N is more (CR 120.3c), and no damage is marked
/// on it.
fn deals_to_a_planeswalker((card, amount): (Box<dyn CardBehavior>, Amount)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_permanent(&mut state, blank, Permanent::Planeswalker);
    let (spell, n) = a_damage_spell(&mut state, card, amount);
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == 0);
    assert!(o.counters.get(&CounterType::Loyalty).copied().unwrap_or(0) == 3u32.saturating_sub(n));
    assert!(state.players[0].life == life[0] && state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// What an Aura's text says of the creature it enchants, as the engine
/// reads it back.
#[derive(Clone, Copy)]
struct Enchanted {
    /// What it gets.
    pt: (i32, i32),
    /// A keyword it has.
    keyword: Option<Keyword>,
    cant_attack: bool,
    cant_block: bool,
    must_attack: bool,
    doesnt_untap: bool,
}

/// An Aura whose text changes nothing the engine reads off the creature.
const AS_IT_WAS: Enchanted = Enchanted { pt: (0, 0), keyword: None, cant_attack: false, cant_block: false, must_attack: false, doesnt_untap: false };

/// An Aura with "enchant creature" resolving at a creature, the engine's
/// own zone change, attachment and continuous effects under the card's
/// `on_resolve`: the Aura is on the battlefield under its controller,
/// attached to that creature (CR 303.4f, 608.3), and the creature, read
/// back through the engine's own layers, is what the Aura's text `says` of
/// it (613.4c, 613.1f) and otherwise as it was. For either player's Aura
/// at the second player's creature (`what` it is) under either player,
/// tapped or not, damaged or not, of toughness 2 or 3.
fn enchants((card, what, says): (Box<dyn CardBehavior>, Permanent, Enchanted)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_permanent(&mut state, blank, what);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let aura = state.create_object(card, you, Zone::Stack, None, None);
    let (power, toughness) = (state.effective_power(target, &registry), state.effective_toughness(target, &registry));
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_resolve(&mut state, aura, &[Target::Object(target)], &registry);

    let a = state.get_object(aura).expect("still an object");
    assert!(a.zone == Zone::Battlefield && a.attached_to == Some(target) && a.attached_to_player.is_none() && a.controller == you);
    assert!(state.effective_power(target, &registry) == power.map(|p| p + says.pt.0));
    assert!(state.effective_toughness(target, &registry) == toughness.map(|t| t + says.pt.1));
    if let Some(keyword) = says.keyword {
        assert!(state.has_keyword(target, keyword, &registry));
    }
    assert!(state.can_attack(target, &registry) != says.cant_attack);
    assert!(state.can_block(target, &registry) != says.cant_block);
    assert!(state.must_attack(target, &registry) == says.must_attack);
    assert!(state.untaps_normally(target, &registry) != says.doesnt_untap);
    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.damage_marked == damage && o.controller == controller);
    std::mem::forget((registry, state));
}

/// What a Curse's text says of the creatures its enchanted player
/// controls, as the engine reads it back.
#[derive(Clone, Copy)]
enum Cursed {
    /// Nothing the engine reads off a creature: its text is a trigger.
    Nothing,
    /// They get -1/-1 (Curse of Death's Hold).
    MinusOne,
    /// They attack each combat if able (Curse of the Nightly Hunt).
    MustAttack,
}

/// An Aura with "enchant player" (a Curse) resolving at a player, either
/// one, beside a creature under either player: the Aura is on the
/// battlefield under its controller, attached to that player and to no
/// object (CR 303.4f, 608.3), and the creature, read back through the
/// engine's own layers, is what the Curse's text `says` of it if the
/// enchanted player controls it, and as it was if the other player does.
fn curses((card, says): (Box<dyn CardBehavior>, Cursed)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let creature = a_permanent(&mut state, blank, Permanent::Creature);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let cursed = PlayerId(u8::from(kani::any::<bool>()));
    let curse = state.create_object(card, you, Zone::Stack, None, None);
    let (power, toughness) = (state.effective_power(creature, &registry), state.effective_toughness(creature, &registry));
    let theirs = state.get_object(creature).expect("just created").controller == cursed;

    registry.get(card).expect("registered").on_resolve(&mut state, curse, &[Target::Player(cursed)], &registry);

    let a = state.get_object(curse).expect("still an object");
    assert!(a.zone == Zone::Battlefield && a.attached_to_player == Some(cursed) && a.attached_to.is_none() && a.controller == you);
    let down = i32::from(theirs && matches!(says, Cursed::MinusOne));
    assert!(state.effective_power(creature, &registry) == power.map(|p| p - down));
    assert!(state.effective_toughness(creature, &registry) == toughness.map(|t| t - down));
    assert!(state.must_attack(creature, &registry) == (theirs && matches!(says, Cursed::MustAttack)));
    kani::cover!(theirs, "the enchanted player's creature");
    std::mem::forget((registry, state));
}

/// "Counter target spell." resolving at another spell on the stack, the
/// player `whose` names its owner, the engine's own stack and zone change
/// under the card's `on_resolve`: the spell countered is off the stack and
/// in its owner's graveyard (CR 701.5a), or in exile if the text says
/// "exile it instead" (614.1a), and the counterspell is what is left on
/// the stack.
fn counters((card, exiled, whose): (Box<dyn CardBehavior>, bool, u8)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let owner = PlayerId(whose);
    let target = state.create_object(blank, owner, Zone::Stack, None, None);
    state.put_on_stack(StackEntry::Spell(target));
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.put_on_stack(StackEntry::Spell(spell));

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == if exiled { Zone::Exile } else { Zone::Graveyard } && o.owner == owner && o.controller == owner);
    assert!(state.stack.len() == 1 && state.stack[0].as_spell() == Some(spell));
    assert!(state.players[whose as usize].graveyard_order.contains(&target) != exiled);
    std::mem::forget((registry, state));
}

/// Bump in the Night ("Target opponent loses 3 life.") resolving at a
/// player, either one: that player has 3 life fewer (CR 119.3), and the
/// other as many as before.
fn bump_in_the_night(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::bump_in_the_night::BumpInTheNight));
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let at = u8::from(kani::any::<bool>());
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Player(PlayerId(at))], &registry);

    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] - if p == at { 3 } else { 0 });
    }
    std::mem::forget((registry, state));
}

/// Infernal Plunge ("Add {R}{R}{R}.") resolving for either player: three
/// red mana more in its controller's pool than before and none of another
/// colour (CR 106.4), and the other player's pool as it was.
fn infernal_plunge(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::infernal_plunge::InfernalPlunge));
    let you = u8::from(kani::any::<bool>());
    let spell = state.create_object(card, PlayerId(you), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    for p in 0..2u8 {
        let pool = &state.players[p as usize].mana_pool;
        assert!(pool.get(ManaType::Red) == if p == you { 3 } else { 0 });
        assert!(pool.total() == if p == you { 3 } else { 0 });
    }
    std::mem::forget((registry, state));
}

/// Silent Departure ("Return target creature to its owner's hand.")
/// resolving at a creature, the engine's own zone change under it: the
/// creature is in its owner's hand, under its owner, as a new object with
/// no damage and untapped (CR 400.7), whoever controlled it.
fn silent_departure(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::silent_departure::SilentDeparture));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Hand && o.controller == PlayerId(1) && o.owner == PlayerId(1) && !o.tapped && o.damage_marked == 0);
    std::mem::forget((registry, state));
}

/// `cards` cards of the blank card in `who`'s library, top first.
fn a_library(state: &mut GameState, blank: CardId, who: u8, cards: usize) -> [Option<ObjectId>; 4] {
    let mut library = [None; 4];
    for slot in library.iter_mut().take(cards) {
        let id = state.create_object(blank, PlayerId(who), Zone::Library, None, None);
        // A branch on the player, not an index by it: `proofs::actions::
        // library` says why.
        if who == 0 {
            state.players[0].library_order.push(id);
        } else {
            state.players[1].library_order.push(id);
        }
        *slot = Some(id);
    }
    library
}

/// "Target player mills N cards." resolving at the player `who` names (a
/// harness for each: `draws` says why), their library of `cards` cards:
/// the top N of it, or all of it if there are fewer, are in their
/// graveyard, top first (CR 701.17a), the rest are their library in the
/// order they were, and milling an empty library is not drawing from one
/// (701.17b, 704.5b).
fn mills((card, n, cards, who): (Box<dyn CardBehavior>, usize, usize, u8)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let library = a_library(&mut state, blank, who, cards);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Player(PlayerId(who))], &registry);

    let milled = n.min(cards);
    let player = &state.players[who as usize];
    assert!(player.library_order.len() == cards - milled && player.graveyard_order.len() == milled && !player.has_drawn_from_empty);
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k < milled {
            assert!(o.zone == Zone::Graveyard && player.graveyard_order[k] == id);
        } else {
            assert!(o.zone == Zone::Library && player.library_order[k - milled] == id);
        }
    }
    std::mem::forget((registry, state));
}

/// A card of the blank card in `zone`, the player `whose` names its owner;
/// a creature card (a 2/2) if `creature`.
fn a_card_in(state: &mut GameState, blank: CardId, whose: u8, zone: Zone, creature: bool) -> ObjectId {
    let pt = if creature { Some(2) } else { None };
    let id = state.create_object(blank, PlayerId(whose), zone, pt, pt);
    if zone == Zone::Graveyard {
        if whose == 0 {
            state.players[0].graveyard_order.push(id);
        } else {
            state.players[1].graveyard_order.push(id);
        }
    }
    id
}

/// A spell that moves its one target card from a zone that is not the
/// battlefield to another, resolving for the first player at a card of
/// the player `whose` names, the engine's own zone change under it: the card is in the zone
/// the text names, under its owner (CR 400.3, 108.3), no longer in the
/// graveyard's order if it came from there, and on the battlefield
/// untapped and summoning sick (302.6).
fn moves_a_card((card, from, to, whose): (Box<dyn CardBehavior>, Zone, Zone, u8)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_card_in(&mut state, blank, whose, from, true);
    let beside = a_card_in(&mut state, blank, whose, from, false);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == to && o.owner == PlayerId(whose) && o.controller == PlayerId(whose) && !o.tapped);
    if to == Zone::Battlefield {
        assert!(o.summoning_sick && state.is_creature(target, &registry));
    }
    assert!(!state.players[whose as usize].graveyard_order.contains(&target));
    assert!(state.get_object(beside).expect("still an object").zone == from);
    if from == Zone::Graveyard {
        assert!(state.players[whose as usize].graveyard_order.as_slice() == [beside]);
    }
    std::mem::forget((registry, state));
}

/// Gnaw to the Bone ("You gain 2 life for each creature card in your
/// graveyard.") resolving for the first player, `creatures` creature cards
/// and one other card in their graveyard and a creature card in the other
/// player's: they have twice that many life more (CR 119.3), and the
/// other player as much as before.
fn gnaw_to_the_bone(creatures: usize) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::gnaw_to_the_bone::GnawToTheBone));
    for _ in 0..creatures {
        a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
    }
    a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    a_card_in(&mut state, blank, 1, Zone::Graveyard, true);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    assert!(state.players[0].life == life[0] + 2 * i32::try_from(creatures).expect("a few"));
    assert!(state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// Grasp of Phantoms ("Put target creature on top of its owner's
/// library.") resolving at the second player's creature under either
/// player, their library of `cards` cards: the creature is the top card
/// of its owner's library (CR 401.4), a card there and no longer a
/// permanent, and the cards that were there are under it in the order
/// they were.
fn grasp_of_phantoms(cards: usize) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::grasp_of_phantoms::GraspOfPhantoms));
    let library = a_library(&mut state, blank, 1, cards);
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Library && o.controller == PlayerId(1) && !o.tapped && o.damage_marked == 0);
    let order = &state.players[1].library_order;
    assert!(order.len() == cards + 1 && order[0] == target);
    for k in 0..cards {
        assert!(order[k + 1] == library[k].expect("made"));
    }
    std::mem::forget((registry, state));
}

/// Swords to Plowshares ("Exile target creature. Its controller gains
/// life equal to its power.") resolving at a creature of power 2, or of
/// power 4, under either player: the creature is in exile, its owner's
/// (CR 406.1), and the player who controlled it has that much life more
/// (119.3), the other as much as before.
fn swords_to_plowshares(what: Permanent) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::swords_to_plowshares::SwordsToPlowshares));
    let target = a_permanent(&mut state, blank, what);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let controller = state.get_object(target).expect("just created").controller;
    let power = if what == Permanent::Monstrous { 4 } else { 2 };
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Exile && o.controller == PlayerId(1) && o.owner == PlayerId(1));
    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] + if PlayerId(p) == controller { power } else { 0 });
    }
    std::mem::forget((registry, state));
}

/// Prey Upon ("Target creature you control fights target creature you
/// don't control.") resolving at two creatures of power 2, the engine's
/// own fight and damage under it: each has 2 more damage marked on it
/// than before (CR 701.14a, 120.3e), and is otherwise as it was.
/// State-based actions are not run here.
fn prey_upon(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::prey_upon::PreyUpon));
    let fighters = [a_permanent(&mut state, blank, Permanent::Creature), a_permanent(&mut state, blank, Permanent::Creature)];
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let mut before = [(false, 0); 2];
    for (i, &f) in fighters.iter().enumerate() {
        let o = state.get_object(f).expect("just created");
        before[i] = (o.tapped, o.damage_marked);
    }

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(fighters[0]), Target::Object(fighters[1])], &registry);

    for (i, &f) in fighters.iter().enumerate() {
        let o = state.get_object(f).expect("still an object");
        assert!(o.zone == Zone::Battlefield && o.damage_marked == before[i].1 + 2 && o.tapped == before[i].0);
    }
    std::mem::forget((registry, state));
}

/// Spider Spawning ("Create a 1/2 green Spider creature token with reach
/// for each creature card in your graveyard.") resolving for the first
/// player, `creatures` creature cards and one other card in their
/// graveyard and a creature card in the other player's: that many new
/// permanents, each a token, a 1/2 green Spider creature with reach under
/// them (CR 111.1, 111.2), read back through the engine's own
/// characteristics.
fn spider_spawning(creatures: u64) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::spider_spawning::SpiderSpawning));
    for _ in 0..creatures {
        a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
    }
    a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    a_card_in(&mut state, blank, 1, Zone::Graveyard, true);
    let you = PlayerId(0);
    let spell = state.create_object(card, you, Zone::Stack, None, None);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    assert!(state.objects.len() == before + usize::try_from(creatures).expect("a few"));
    for k in 1..=creatures {
        let id = ObjectId(spell.0 + k);
        let t = state.get_object(id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
        assert!(state.is_creature(id, &registry) && state.has_keyword(id, Keyword::Reach, &registry));
        assert!(state.effective_power(id, &registry) == Some(1) && state.effective_toughness(id, &registry) == Some(2));
        assert!(t.colors == [Color::Green] && t.subtypes.len() == 1 && t.subtypes[0] == "Spider");
    }
    std::mem::forget((registry, state));
}

/// "~ deals N damage to each creature [without flying]." resolving on a
/// board of two creatures, each under either player, and an artifact, the
/// engine's own damage under it, all of it queued and then dealt at once
/// (CR 120.3e): each creature the text names has N more damage marked on
/// it than before, and nothing else has any. The
/// second creature has flying or not as `flying` says (a constant of the
/// harness: left open, how many are dealt damage is open).
fn deals_to_each_creature((card, n, spares_fliers, flying): (Box<dyn CardBehavior>, u32, bool, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let creatures = [a_permanent(&mut state, blank, Permanent::Creature), a_permanent(&mut state, blank, Permanent::Creature)];
    let artifact = a_permanent(&mut state, blank, Permanent::Artifact);
    let keyword = if flying { Keyword::Flying } else { Keyword::Reach };
    state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target: creatures[1], keyword });
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let before = [state.get_object(creatures[0]).expect("just created").damage_marked, state.get_object(creatures[1]).expect("just created").damage_marked];
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    for (i, &c) in creatures.iter().enumerate() {
        let spared = spares_fliers && flying && i == 1;
        let o = state.get_object(c).expect("still an object");
        assert!(o.zone == Zone::Battlefield && o.damage_marked == before[i] + if spared { 0 } else { n });
    }
    assert!(state.get_object(artifact).expect("still an object").damage_marked == 0);
    assert!(state.players[0].life == life[0] && state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// Ghoulcaller's Chant (either mode: "Return target creature card from
/// your graveyard to your hand." or "Return two target Zombie cards...")
/// resolving for the first player with `targets` of the two creature
/// cards in their graveyard as its targets, a third card beside them:
/// each card targeted is in their hand and no longer in their graveyard
/// (CR 400.3), and the rest are where they were, in the order they were.
fn ghoulcallers_chant(targets: usize) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::ghoulcallers_chant::GhoulcallersChant));
    let cards = [a_card_in(&mut state, blank, 0, Zone::Graveyard, true), a_card_in(&mut state, blank, 0, Zone::Graveyard, true)];
    let beside = a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let chosen = [Target::Object(cards[0]), Target::Object(cards[1])];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &chosen[..targets], &registry);

    for (i, &c) in cards.iter().enumerate() {
        let o = state.get_object(c).expect("still an object");
        assert!(o.zone == if i < targets { Zone::Hand } else { Zone::Graveyard } && o.controller == PlayerId(0));
        assert!(state.players[0].graveyard_order.contains(&c) == (i >= targets));
    }
    assert!(state.get_object(beside).expect("still an object").zone == Zone::Graveyard);
    assert!(state.players[0].graveyard_order.len() == 3 - targets);
    std::mem::forget((registry, state));
}

/// Corpse Lunge ("Corpse Lunge deals damage equal to the exiled card's
/// power to target creature.") resolving at a creature, the card exiled to
/// cast it of power `power` (a constant of the harness, as every amount of
/// damage is): that much more damage is marked on the creature than
/// before (CR 120.3e), none for a power of 0 (120.8), and it is otherwise
/// as it was.
fn corpse_lunge(power: u32) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::corpse_lunge::CorpseLunge));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.get_object_mut(spell).expect("just created").card_state.insert(crate::cards::exiled_to_cost_power_key(0), ObjectId(u64::from(power)));
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == damage + power && o.tapped == tapped && o.controller == controller);
    std::mem::forget((registry, state));
}

/// Lost in the Mist ("Counter target spell. Return target permanent to
/// its owner's hand.") resolving at the second player's spell on the stack
/// and their creature on the battlefield under either player: the spell
/// is off the stack and in its owner's graveyard (CR 701.5a), and the
/// permanent is in its owner's hand (400.3).
fn lost_in_the_mist(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::lost_in_the_mist::LostInTheMist));
    let permanent = a_permanent(&mut state, blank, Permanent::Creature);
    let countered = state.create_object(blank, PlayerId(1), Zone::Stack, None, None);
    state.put_on_stack(StackEntry::Spell(countered));
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.put_on_stack(StackEntry::Spell(spell));

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(countered), Target::Object(permanent)], &registry);

    let c = state.get_object(countered).expect("still an object");
    assert!(c.zone == Zone::Graveyard && c.controller == PlayerId(1));
    assert!(state.stack.len() == 1 && state.stack[0].as_spell() == Some(spell));
    let o = state.get_object(permanent).expect("still an object");
    assert!(o.zone == Zone::Hand && o.controller == PlayerId(1) && !o.tapped && o.damage_marked == 0);
    std::mem::forget((registry, state));
}

/// Traitorous Blood ("Gain control of target creature until end of turn.
/// Untap it. It gains trample and haste until end of turn.") resolving
/// for the first player at a creature under either player, tapped or
/// not: they control it (CR 613.1b), it is untapped (701.21a) and has
/// trample and haste, read back through the engine's own keywords, and
/// it is otherwise as it was.
fn traitorous_blood(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::traitorous_blood::TraitorousBlood));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let you = PlayerId(0);
    let spell = state.create_object(card, you, Zone::Stack, None, None);
    let damage = state.get_object(target).expect("just created").damage_marked;

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.controller == you && o.owner == PlayerId(1) && !o.tapped && o.damage_marked == damage);
    assert!(state.has_keyword(target, Keyword::Trample, &registry) && state.has_keyword(target, Keyword::Haste, &registry));
    std::mem::forget((registry, state));
}

/// Harvest Pyre ("Harvest Pyre deals X damage to target creature.", X the
/// cards exiled to cast it) resolving at a creature with `exiled` cards
/// exiled (a constant of the harness, as every amount of damage is): that
/// much more damage is marked on the creature than before (CR 120.3e),
/// and it is otherwise as it was.
fn harvest_pyre(exiled: u32) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::harvest_pyre::HarvestPyre));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.get_object_mut(spell).expect("just created").card_state.insert(crate::cards::EXILE_COUNT.to_string(), ObjectId(u64::from(exiled)));
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == damage + exiled && o.tapped == tapped && o.controller == controller);
    std::mem::forget((registry, state));
}

/// Into the Maw of Hell ("Destroy target land. Into the Maw of Hell deals
/// 13 damage to target creature.") resolving at a land and a creature:
/// the land is in its owner's graveyard (CR 701.7a) and the creature has
/// 13 more damage marked on it than before (120.3e).
fn into_the_maw_of_hell(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::into_the_maw_of_hell::IntoTheMawOfHell));
    let land = a_permanent(&mut state, blank, Permanent::Land);
    let creature = a_permanent(&mut state, blank, Permanent::Creature);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let damage = state.get_object(creature).expect("just created").damage_marked;

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(land), Target::Object(creature)], &registry);

    let l = state.get_object(land).expect("still an object");
    assert!(l.zone == Zone::Graveyard && l.controller == PlayerId(1));
    let c = state.get_object(creature).expect("still an object");
    assert!(c.zone == Zone::Battlefield && c.damage_marked == damage + 13);
    std::mem::forget((registry, state));
}

/// Maw of the Mire ("Destroy target land. You gain 4 life.") resolving
/// for the first player at a land: the land is in its owner's graveyard
/// (CR 701.7a), and they have 4 life more (119.3), the other player as
/// much as before.
fn maw_of_the_mire(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::maw_of_the_mire::MawOfTheMire));
    let land = a_permanent(&mut state, blank, Permanent::Land);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(land)], &registry);

    let l = state.get_object(land).expect("still an object");
    assert!(l.zone == Zone::Graveyard && l.controller == PlayerId(1));
    assert!(state.players[0].life == life[0] + 4 && state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// Paraselene ("Destroy all enchantments. You gain 1 life for each
/// enchantment destroyed this way.") resolving for the first player on a
/// board of two enchantments, each under either player, and a creature;
/// the second enchantment indestructible or not as `indestructible` says:
/// each enchantment that is not indestructible is in its owner's
/// graveyard (CR 701.7a, 702.12b), the creature is where it was, and they
/// have 1 life more for each one destroyed (119.3).
fn paraselene(indestructible: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::paraselene::Paraselene));
    let enchantments = [a_permanent(&mut state, blank, Permanent::Enchantment), a_permanent(&mut state, blank, Permanent::Enchantment)];
    let creature = a_permanent(&mut state, blank, Permanent::Creature);
    let keyword = if indestructible { Keyword::Indestructible } else { Keyword::Flying };
    state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target: enchantments[1], keyword });
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    assert!(state.get_object(enchantments[0]).expect("still an object").zone == Zone::Graveyard);
    assert!(state.get_object(enchantments[1]).expect("still an object").zone == if indestructible { Zone::Battlefield } else { Zone::Graveyard });
    assert!(state.get_object(creature).expect("still an object").zone == Zone::Battlefield);
    assert!(state.players[0].life == life[0] + if indestructible { 1 } else { 2 } && state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// Wreath of Geists ("Enchanted creature gets +X/+X, where X is the number
/// of creature cards in your graveyard.") resolving for the first player
/// at a creature, `creatures` creature cards and one other card in their
/// graveyard and a creature card in the other player's: the Aura is
/// attached to the creature (CR 303.4f), which has that much more power
/// and toughness, read back through the engine's own layers (613.4c), and
/// is otherwise as it was.
fn wreath_of_geists(creatures: usize) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::wreath_of_geists::WreathOfGeists));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    for _ in 0..creatures {
        a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
    }
    a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    a_card_in(&mut state, blank, 1, Zone::Graveyard, true);
    let aura = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let (power, toughness) = (state.effective_power(target, &registry), state.effective_toughness(target, &registry));
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_resolve(&mut state, aura, &[Target::Object(target)], &registry);

    let a = state.get_object(aura).expect("still an object");
    assert!(a.zone == Zone::Battlefield && a.attached_to == Some(target) && a.controller == PlayerId(0));
    let x = i32::try_from(creatures).expect("a few");
    assert!(state.effective_power(target, &registry) == power.map(|p| p + x));
    assert!(state.effective_toughness(target, &registry) == toughness.map(|t| t + x));
    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.damage_marked == damage && o.controller == controller);
    std::mem::forget((registry, state));
}

/// Sever the Bloodline ("Exile target creature and all other creatures
/// with the same name as that creature.") resolving at a creature on a
/// board with another creature of its name and one of another name, each
/// under either player: the two of that name are in exile (CR 406.1), and
/// the third is where it was.
fn sever_the_bloodline(_: ()) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::sever_the_bloodline::SeverTheBloodline));
    let unlike = registry.register(Box::new(crate::proofs::cards::OtherBlank));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let namesake = a_permanent(&mut state, blank, Permanent::Creature);
    let stranger = a_permanent(&mut state, unlike, Permanent::Creature);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    for id in [target, namesake] {
        let o = state.get_object(id).expect("still an object");
        assert!(o.zone == Zone::Exile && o.controller == PlayerId(1));
    }
    assert!(state.get_object(stranger).expect("still an object").zone == Zone::Battlefield);
    std::mem::forget((registry, state));
}

/// Rolling Temblor ("Rolling Temblor deals 2 damage to each creature
/// without flying.") resolving on a board of one creature, under either
/// player, and nothing else: the creature has 2 more damage marked on it
/// than before if it has no flying and as much as before if it has (CR
/// 120.3e). Proved of the creature without flying; with flying it ran
/// out of memory at 8 GB. The creature alone: the card asks of each
/// permanent whether it is a creature and has flying before it queues
/// the damage, and with a second permanent there, a creature or not and
/// nothing about either left open, the harness ran out of memory at 5
/// and at 8 GB; a board of two is the small boards'.
fn rolling_temblor(flying: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::rolling_temblor::RollingTemblor));
    let creature = a_permanent(&mut state, blank, Permanent::Creature);
    if flying {
        state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target: creature, keyword: Keyword::Flying });
    }
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let before = state.get_object(creature).expect("just created");
    let (tapped, damage) = (before.tapped, before.damage_marked);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    let o = state.get_object(creature).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.damage_marked == damage + if flying { 0 } else { 2 });
    std::mem::forget((registry, state));
}

/// Tribute to Hunger ("Target opponent sacrifices a creature. You gain
/// life equal to that creature's toughness.") resolving for the first
/// player at the second, who controls two creatures, the first player
/// one; and then the effect its question holds, applied by the engine at
/// the `pick`th option. The opponent is asked, and must answer,
/// among exactly the creatures they control (CR 701.21a: the player
/// sacrificing chooses); the one chosen is in their graveyard, the other
/// and the first player's are where they were, and the first player has
/// life more by its toughness (119.3).
///
/// Between the two is the engine's answering of a choice
/// (`engine::actions::choices::resolve_choice`), which checks the answer
/// is one of the options and applies that effect to it; with the engine
/// itself under it that dispatcher ran out of memory at 5 GB on a board
/// with nothing open, as the stack's resolution does, and it is proved of
/// itself (`proofs::actions::answering`).
fn tribute_to_hunger(pick: usize) {
    use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::tribute_to_hunger::TributeToHunger));
    let theirs = [a_permanent(&mut state, blank, Permanent::Creature), a_permanent(&mut state, blank, Permanent::Creature)];
    let yours = a_permanent(&mut state, blank, Permanent::Creature);
    for id in theirs {
        state.get_object_mut(id).expect("just created").controller = PlayerId(1);
    }
    state.get_object_mut(yours).expect("just created").controller = PlayerId(0);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Player(PlayerId(1))], &registry);

    // Read where it lies: taken out or cloned, the question is one CBMC
    // copies whole (`engine::actions::choices` says the same of itself).
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. }) => {
            assert!(*player == PlayerId(1) && !*optional);
            assert!(options.len() == 2 && options[0] == Target::Object(theirs[0]) && options[1] == Target::Object(theirs[1]));
            assert!(matches!(effect, PendingEffect::CardEffect { source_id, key } if *source_id == spell && key.is_empty()));
        }
        _ => panic!("the opponent is asked which creature"),
    }
    crate::proofs::cards::clear_prompt(&mut state);
    let effect = PendingEffect::CardEffect { source_id: spell, key: String::new() };
    let chosen = theirs[pick];
    let toughness = state.effective_toughness(chosen, &registry).expect("a creature");
    crate::engine::apply_pending_effect(&mut state, &Target::Object(chosen), &effect, &registry);

    assert!(state.awaiting_action.is_none());
    assert!(state.get_object(chosen).expect("still an object").zone == Zone::Graveyard);
    assert!(state.get_object(theirs[1 - pick]).expect("still an object").zone == Zone::Battlefield);
    assert!(state.get_object(yours).expect("still an object").zone == Zone::Battlefield);
    assert!(state.players[0].life == life[0] + toughness && state.players[1].life == life[1]);
    std::mem::forget((registry, state, effect));
}

// ---- Permanents: their enters triggers, each hook run for a permanent
// of the card already on the battlefield, as the trigger resolves. ----

/// A permanent of `card` on the battlefield under `you`: the source of a
/// trigger of its own.
fn a_source(state: &mut GameState, card: CardId, you: PlayerId) -> ObjectId {
    state.create_object(card, you, Zone::Battlefield, None, None)
}

/// Armored Skaab ("When this creature enters, mill four cards.") entered
/// under the player `who` names (a harness for each: `draws` says why),
/// their library of `cards` cards: the top four of it, or all of it if
/// there are fewer, are in their graveyard, top first (CR 701.17a), and
/// milling an empty library is not drawing from one (701.17b).
fn armored_skaab((cards, who): (usize, u8)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::armored_skaab::ArmoredSkaab));
    let library = a_library(&mut state, blank, who, cards);
    let skaab = a_source(&mut state, card, PlayerId(who));

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, skaab, &[], &registry);

    let milled = cards.min(4);
    let player = &state.players[who as usize];
    assert!(player.library_order.len() == cards - milled && player.graveyard_order.len() == milled && !player.has_drawn_from_empty);
    for k in 0..milled {
        let id = library[k].expect("made");
        assert!(state.get_object(id).expect("still an object").zone == Zone::Graveyard && player.graveyard_order[k] == id);
    }
    assert!(state.get_object(skaab).expect("still an object").zone == Zone::Battlefield);
    std::mem::forget((registry, state));
}

/// Claustrophobia ("When this Aura enters, tap enchanted creature.")
/// entered attached to a creature, tapped or not: the creature is tapped
/// (CR 701.21a), and otherwise as it was.
fn claustrophobia_enters(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::claustrophobia::Claustrophobia));
    let creature = a_permanent(&mut state, blank, Permanent::Creature);
    let aura = a_source(&mut state, card, PlayerId(u8::from(kani::any::<bool>())));
    state.attach(aura, creature);
    let before = state.get_object(creature).expect("just created");
    let (damage, controller) = (before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, aura, &[], &registry);

    let o = state.get_object(creature).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped && o.damage_marked == damage && o.controller == controller);
    assert!(state.get_object(aura).expect("still an object").attached_to == Some(creature));
    std::mem::forget((registry, state));
}

/// Crossway Vampire ("When this creature enters, target creature can't
/// block this turn.") entered, its trigger resolving at a creature: that
/// creature cannot block, as the engine's own query answers, and is
/// otherwise as it was.
fn crossway_vampire(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::crossway_vampire::CrosswayVampire));
    let creature = a_permanent(&mut state, blank, Permanent::Creature);
    let vampire = a_source(&mut state, card, PlayerId(0));
    let before = state.get_object(creature).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);
    assert!(state.can_block(creature, &registry));

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, vampire, &[Target::Object(creature)], &registry);

    assert!(!state.can_block(creature, &registry));
    let o = state.get_object(creature).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.damage_marked == damage && o.controller == controller);
    std::mem::forget((registry, state));
}

/// Geist-Honored Monk ("When this creature enters, create two 1/1 white
/// Spirit creature tokens with flying.") entered under either player:
/// two new permanents, each a token, a 1/1 white Spirit creature with
/// flying under that player (CR 111.1, 111.2).
fn geist_honored_monk(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::geist_honored_monk::GeistHonoredMonk));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let monk = a_source(&mut state, card, you);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, monk, &[], &registry);

    assert!(state.objects.len() == before + 2);
    for k in 1..=2 {
        let id = ObjectId(monk.0 + k);
        let t = state.get_object(id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
        assert!(state.is_creature(id, &registry) && state.has_keyword(id, Keyword::Flying, &registry));
        assert!(state.effective_power(id, &registry) == Some(1) && state.effective_toughness(id, &registry) == Some(1));
        assert!(t.colors == [Color::White] && t.subtypes.len() == 1 && t.subtypes[0] == "Spirit");
    }
    std::mem::forget((registry, state));
}

/// Hollowhenge Scavenger ("Morbid — When this creature enters, if a
/// creature died this turn, you gain 5 life.") entered under either
/// player, a creature having died this turn or not (`died`): its
/// controller has 5 life more if one did and as much as before if none
/// did (CR 700.4, 119.3), and the other player as much as before.
fn hollowhenge_scavenger(died: bool) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::hollowhenge_scavenger::HollowhengeScavenger));
    let you = u8::from(kani::any::<bool>());
    let scavenger = a_source(&mut state, card, PlayerId(you));
    state.creature_died_this_turn = died;
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, scavenger, &[], &registry);

    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] + if p == you && died { 5 } else { 0 });
    }
    std::mem::forget((registry, state));
}

/// Morkrut Banshee ("Morbid — When this creature enters, if a creature
/// died this turn, target creature gets -4/-4 until end of turn.")
/// entered, its trigger resolving at a creature, a creature having died
/// this turn or not (`died`): the target has 4 less power and toughness,
/// read back through the engine's layers (CR 613.4c), if one did, and is
/// as it was if none did.
fn morkrut_banshee(died: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::morkrut_banshee::MorkrutBanshee));
    let creature = a_permanent(&mut state, blank, Permanent::Creature);
    let banshee = a_source(&mut state, card, PlayerId(0));
    state.creature_died_this_turn = died;
    let (power, toughness) = (state.effective_power(creature, &registry), state.effective_toughness(creature, &registry));
    let before = state.get_object(creature).expect("just created");
    let (tapped, damage) = (before.tapped, before.damage_marked);

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, banshee, &[Target::Object(creature)], &registry);

    let down = if died { 4 } else { 0 };
    assert!(state.effective_power(creature, &registry) == power.map(|p| p - down));
    assert!(state.effective_toughness(creature, &registry) == toughness.map(|t| t - down));
    let o = state.get_object(creature).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.damage_marked == damage);
    std::mem::forget((registry, state));
}

/// A "you may <do something to> target ..." enters trigger resolving at
/// its target: its hook asks its controller, who may decline, about
/// exactly that target (CR 603.5), and the question holds `effect`; that
/// effect, applied by the engine at the target (`tribute_to_hunger` says
/// why not through the answer's dispatcher), leaves the board as
/// `after` says. What it returns is the board for `after` to be asked of.
/// `holds` and `effect` are given the trigger's source and its card's
/// printed name, which an effect may carry for the log: read from the
/// registry, not spelled here, where the engine names no card
/// (`tests/engine_knows_no_cards.rs`).
fn may_do_to_its_target(card: Box<dyn CardBehavior>, what: Permanent, holds: &dyn Fn(&crate::state::PendingEffect, ObjectId, &str) -> bool,
    effect: &dyn Fn(ObjectId, &str) -> crate::state::PendingEffect) -> (crate::cards::CardRegistry, GameState, ObjectId, (bool, u32, PlayerId))
{
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_permanent(&mut state, blank, what);
    let you = PlayerId(0);
    let source = a_source(&mut state, card, you);
    let name = registry.printed_name(card, false).expect("its face").to_string();
    let before = state.get_object(target).expect("just created");
    let was = (before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, source, &[Target::Object(target)], &registry);

    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. }) => {
            assert!(*player == you && *optional);
            assert!(options.len() == 1 && options[0] == Target::Object(target));
            assert!(holds(effect, source, &name));
        }
        _ => panic!("its controller is asked whether to"),
    }
    crate::proofs::cards::clear_prompt(&mut state);
    let effect = effect(source, &name);
    crate::engine::apply_pending_effect(&mut state, &Target::Object(target), &effect, &registry);
    std::mem::forget((effect, name));
    (registry, state, target, was)
}

/// Slayer of the Wicked ("When this creature enters, you may destroy
/// target Vampire, Werewolf, or Zombie.") entered, its trigger resolving
/// at a creature: its controller is asked, and the destruction the
/// question holds puts the creature into its owner's graveyard (CR
/// 701.7a).
fn slayer_of_the_wicked(_: ()) {
    use crate::state::PendingEffect;
    let (registry, state, target, _) = may_do_to_its_target(Box::new(crate::cards::isd::slayer_of_the_wicked::SlayerOfTheWicked), Permanent::Creature,
        &|effect, _, name| matches!(effect, PendingEffect::Destroy { source_name } if source_name == name),
        &|_, name| PendingEffect::Destroy { source_name: name.to_string() });
    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Graveyard && o.controller == PlayerId(1));
    std::mem::forget((registry, state));
}

/// Geistcatcher's Rig ("When this creature enters, you may have it deal 4
/// damage to target creature with flying.") entered, its trigger
/// resolving at a creature: its controller is asked, and the damage the
/// question holds, 4 from the Rig, is marked on the creature (CR
/// 120.3e), which is otherwise as it was.
fn geistcatchers_rig(_: ()) {
    use crate::state::PendingEffect;
    let (registry, state, target, (tapped, damage, controller)) = may_do_to_its_target(Box::new(crate::cards::isd::geistcatchers_rig::GeistcatchersRig), Permanent::Creature,
        &|effect, source, _| matches!(effect, PendingEffect::DealDamage { amount: 4, source_id } if *source_id == source),
        &|source, _| PendingEffect::DealDamage { amount: 4, source_id: source });
    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == damage + 4 && o.tapped == tapped && o.controller == controller);
    std::mem::forget((registry, state));
}

// ---- Permanents: triggers a death or combat damage sets off, each hook
// run as the trigger resolves, for a source still on the battlefield or
// gone from it by then. ----

/// Which trigger of a permanent's resolves: each a hook of its card.
#[derive(Clone, Copy, PartialEq)]
enum Fired {
    /// "When this creature dies".
    Died,
    /// "Whenever a creature dies", or another creature, or one of a kind:
    /// which deaths set it off is the card's own answer
    /// (`should_trigger_on_creature_dies`) and the engine's collecting,
    /// proved of it (`proofs::actions::triggers`).
    CreatureDied,
    /// "Whenever this creature deals combat damage to a player".
    DamagedPlayer,
    /// "Whenever another creature enters".
    CreatureEntered,
}

/// A creature of `card`, a 2/2 as far as the engine's layers start from,
/// on the battlefield: `owner`'s card under `you` (CR 108.3, 110.2).
fn a_creature_of(state: &mut GameState, card: CardId, owner: PlayerId, you: PlayerId) -> ObjectId {
    let id = state.create_object(card, owner, Zone::Battlefield, Some(2), Some(2));
    let o = state.get_object_mut(id).expect("just created");
    o.card_types = vec![CardType::Creature];
    o.controller = you;
    id
}

/// The trigger `fired` names of `source`'s resolving, `targets` chosen for
/// it as it went on the stack: the card's own hook, for a creature that
/// died (the blank card's, in its owner's graveyard), 2 combat damage
/// dealt to the player who does not control the source, or a creature
/// that entered, as the trigger is.
fn fire(state: &mut GameState, registry: &crate::cards::CardRegistry, source: ObjectId, fired: Fired, other: ObjectId, targets: &[Target]) {
    let card = state.get_object(source).expect("its source").card_id;
    let behavior = registry.get(card).expect("registered");
    match fired {
        Fired::Died => behavior.on_dies(state, source, targets, registry),
        Fired::CreatureDied => behavior.on_any_creature_dies(state, source, other, PlayerId(1), &[], 2, false, targets, registry),
        Fired::DamagedPlayer => {
            let damaged = PlayerId(1 - state.last_known_controller(source).0);
            behavior.on_combat_damage_to_player(state, source, damaged, 2, registry);
        }
        Fired::CreatureEntered => behavior.on_any_creature_enters(state, source, other, PlayerId(1), registry),
    }
}

/// "..., put N +1/+1 counters on this creature." resolving for a creature
/// under either player (one player's card: a zone change for an owner the
/// solver leaves open is a graveyard it leaves open, and ran out of
/// memory at 4 GB), still on the battlefield (`there`) or put into its
/// graveyard since by the engine's own zone change: there, it has N
/// counters more and that much more power and toughness, read back
/// through the engine's layers (CR 122.1a, 613.4c), and is otherwise as it
/// was; gone, it is a new object that no counter is put on (121.1,
/// 400.7). The other creature on the board is as it was either way.
fn counters_itself((card, n, fired, there): (Box<dyn CardBehavior>, u32, Fired, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let other = a_permanent(&mut state, blank, Permanent::Creature);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let source = a_creature_of(&mut state, card, PlayerId(1), you);
    state.get_object_mut(source).expect("just created").tapped = kani::any();
    let tapped = state.get_object(source).expect("just created").tapped;
    if !there {
        state.move_object(source, Zone::Graveyard, &registry);
    }
    let was = (state.effective_power(other, &registry), state.effective_toughness(other, &registry));

    fire(&mut state, &registry, source, fired, other, &[]);

    let o = state.get_object(source).expect("still an object");
    let counters = o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    if there {
        let up = i32::try_from(n).expect("small");
        assert!(o.zone == Zone::Battlefield && counters == n && o.tapped == tapped && o.controller == you && o.damage_marked == 0);
        assert!(state.effective_power(source, &registry) == Some(2 + up) && state.effective_toughness(source, &registry) == Some(2 + up));
    } else {
        assert!(o.zone == Zone::Graveyard && counters == 0);
    }
    let o = state.get_object(other).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.counters.get(&CounterType::PlusOnePlusOne).is_none());
    assert!((state.effective_power(other, &registry), state.effective_toughness(other, &registry)) == was);
    std::mem::forget((registry, state));
}

/// "Whenever <a creature> deals combat damage to <a player>, put a +1/+1
/// counter on it." resolving, "it" the creature that dealt the damage and
/// not the trigger's source: that creature, still on the battlefield
/// (`there`), has one counter more and one more power and toughness (CR
/// 122.1a); put into its graveyard since, no counter is put on it
/// (121.1). The source has none either way.
fn counters_what_dealt((card, there): (Box<dyn CardBehavior>, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let dealt = a_permanent(&mut state, blank, Permanent::Creature);
    let source = a_source(&mut state, card, PlayerId(u8::from(kani::any::<bool>())));
    let before = state.get_object(dealt).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);
    let (power, toughness) = (state.effective_power(dealt, &registry), state.effective_toughness(dealt, &registry));
    if !there {
        state.move_object(dealt, Zone::Graveyard, &registry);
    }

    registry.get(card).expect("registered").on_any_combat_damage_to_player(&mut state, source, dealt, PlayerId(1), 2, &registry);

    let o = state.get_object(dealt).expect("still an object");
    let counters = o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    if there {
        assert!(o.zone == Zone::Battlefield && counters == 1 && o.tapped == tapped && o.damage_marked == damage && o.controller == controller);
        assert!(state.effective_power(dealt, &registry) == power.map(|p| p + 1));
        assert!(state.effective_toughness(dealt, &registry) == toughness.map(|t| t + 1));
    } else {
        assert!(o.zone == Zone::Graveyard && counters == 0);
    }
    let o = state.get_object(source).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.counters.get(&CounterType::PlusOnePlusOne).is_none());
    std::mem::forget((registry, state));
}

/// "When this creature dies, create N 1/1 white Spirit creature tokens
/// with flying." resolving, the creature one player's card that died
/// under either player's control, put into its owner's graveyard by the
/// engine's own zone change: N new permanents, each a token, a 1/1 white
/// Spirit creature with flying, under the player who controlled the
/// creature as it died and not its owner (CR 603.10a, 608.2h, 111.2).
fn dies_into_spirits((card, n): (Box<dyn CardBehavior>, u64)) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let source = a_creature_of(&mut state, card, PlayerId(1), you);
    state.move_object(source, Zone::Graveyard, &registry);
    assert!(state.get_object(source).expect("still an object").zone == Zone::Graveyard);
    let before = state.objects.len();

    fire(&mut state, &registry, source, Fired::Died, source, &[]);

    assert!(state.objects.len() == before + usize::try_from(n).expect("a few"));
    for k in 1..=n {
        let id = ObjectId(source.0 + k);
        let t = state.get_object(id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
        assert!(state.is_creature(id, &registry) && state.has_keyword(id, Keyword::Flying, &registry));
        assert!(state.effective_power(id, &registry) == Some(1) && state.effective_toughness(id, &registry) == Some(1));
        assert!(t.colors == [Color::White] && t.subtypes.len() == 1 && t.subtypes[0] == "Spirit");
    }
    assert!(state.get_object(source).expect("still an object").zone == Zone::Graveyard);
    std::mem::forget((registry, state));
}

/// Elder Cathar ("When this creature dies, put a +1/+1 counter on target
/// creature you control. If that creature is a Human, put two +1/+1
/// counters on it instead.") dead, its trigger resolving at a creature
/// that is `what`: one counter on it, or two on a Human, and that much
/// more power and toughness (CR 122.1a).
fn elder_cathar((what, n): (Permanent, u32)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::elder_cathar::ElderCathar));
    let target = a_permanent(&mut state, blank, what);
    let source = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    state.move_object(source, Zone::Graveyard, &registry);
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);
    let (power, toughness) = (state.effective_power(target, &registry), state.effective_toughness(target, &registry));

    fire(&mut state, &registry, source, Fired::Died, source, &[Target::Object(target)]);

    let up = i32::try_from(n).expect("small");
    let o = state.get_object(target).expect("still an object");
    assert!(o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0) == n);
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.damage_marked == damage && o.controller == controller);
    assert!(state.effective_power(target, &registry) == power.map(|p| p + up));
    assert!(state.effective_toughness(target, &registry) == toughness.map(|t| t + up));
    std::mem::forget((registry, state));
}

/// A creature of `card` under the first player, dead if its trigger is
/// its own death's (put into the graveyard by the engine's own zone
/// change) and on the battlefield if it watches for another's, and the
/// creature that died for it to have watched: the source, and that one.
fn a_source_of(state: &mut GameState, registry: &crate::cards::CardRegistry, blank: CardId, card: CardId, fired: Fired) -> (ObjectId, ObjectId) {
    let source = a_creature_of(state, card, PlayerId(0), PlayerId(0));
    if fired == Fired::Died {
        state.move_object(source, Zone::Graveyard, registry);
        return (source, source);
    }
    (source, a_card_in(state, blank, 1, Zone::Graveyard, true))
}

/// A death trigger's "~ deals N damage to <target>" resolving at a
/// creature, the engine's own damage under the hook: N more damage is
/// marked on it than before (CR 120.3e), from a source that may no
/// longer be on the battlefield (608.2h), and it is otherwise as it was.
fn a_death_deals_to_a_creature((card, fired, n): (Box<dyn CardBehavior>, Fired, u32)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let (source, other) = a_source_of(&mut state, &registry, blank, card, fired);
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);
    let life = [state.players[0].life, state.players[1].life];

    fire(&mut state, &registry, source, fired, other, &[Target::Object(target)]);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == damage + n && o.tapped == tapped && o.controller == controller);
    assert!(state.players[0].life == life[0] && state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// The same at a player, either one: that player has N life fewer (CR
/// 120.3a), and the other as many as before.
fn a_death_deals_to_a_player((card, fired, n): (Box<dyn CardBehavior>, Fired, u32)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let (source, other) = a_source_of(&mut state, &registry, blank, card, fired);
    let at = u8::from(kani::any::<bool>());
    let life = [state.players[0].life, state.players[1].life];

    fire(&mut state, &registry, source, fired, other, &[Target::Player(PlayerId(at))]);

    for p in 0..2u8 {
        let lost = if p == at { i32::try_from(n).expect("small") } else { 0 };
        assert!(state.players[p as usize].life == life[p as usize] - lost);
    }
    std::mem::forget((registry, state));
}

/// The same at a planeswalker with three loyalty counters: N of them are
/// removed (CR 120.3c), and no damage is marked on it.
fn a_death_deals_to_a_planeswalker((card, fired, n): (Box<dyn CardBehavior>, Fired, u32)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_permanent(&mut state, blank, Permanent::Planeswalker);
    let (source, other) = a_source_of(&mut state, &registry, blank, card, fired);
    let life = [state.players[0].life, state.players[1].life];

    fire(&mut state, &registry, source, fired, other, &[Target::Object(target)]);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == 0);
    assert!(o.counters.get(&CounterType::Loyalty).copied().unwrap_or(0) == 3u32.saturating_sub(n));
    assert!(state.players[0].life == life[0] && state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// Falkenrath Noble ("Whenever this creature or another creature dies,
/// target player loses 1 life and you gain 1 life.") under either player,
/// the trigger its own death's or another's, resolving at either player:
/// that player has 1 life fewer and its controller, as the creature last
/// was controlled, 1 more (CR 119.3, 608.2h); a controller who is the
/// target has as much as before.
fn falkenrath_noble(fired: Fired) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::falkenrath_noble::FalkenrathNoble));
    let you = u8::from(kani::any::<bool>());
    let source = a_creature_of(&mut state, card, PlayerId(1), PlayerId(you));
    let other = if fired == Fired::Died {
        state.move_object(source, Zone::Graveyard, &registry);
        source
    } else {
        a_card_in(&mut state, blank, 1, Zone::Graveyard, true)
    };
    let at = u8::from(kani::any::<bool>());
    let life = [state.players[0].life, state.players[1].life];

    fire(&mut state, &registry, source, fired, other, &[Target::Player(PlayerId(at))]);

    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] - i32::from(p == at) + i32::from(p == you));
    }
    std::mem::forget((registry, state));
}

/// Selhoff Occultist ("Whenever this creature or another creature dies,
/// target player mills a card.") the trigger its own death's or
/// another's, resolving at the player `who` names (a harness for each:
/// `draws` says why), their library of `cards` cards: the top one is in
/// their graveyard (CR 701.17a), after whatever was put there before it,
/// the rest are their library in the order they were, and milling an
/// empty library is not drawing from one (701.17b).
fn selhoff_occultist((fired, cards, who): (Fired, usize, u8)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::selhoff_occultist::SelhoffOccultist));
    let library = a_library(&mut state, blank, who, cards);
    let (source, other) = a_source_of(&mut state, &registry, blank, card, fired);
    let yard = state.players[who as usize].graveyard_order.len();

    fire(&mut state, &registry, source, fired, other, &[Target::Player(PlayerId(who))]);

    let milled = cards.min(1);
    let player = &state.players[who as usize];
    assert!(player.library_order.len() == cards - milled && player.graveyard_order.len() == yard + milled && !player.has_drawn_from_empty);
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k < milled {
            assert!(o.zone == Zone::Graveyard && player.graveyard_order[yard + k] == id);
        } else {
            assert!(o.zone == Zone::Library && player.library_order[k - milled] == id);
        }
    }
    std::mem::forget((registry, state));
}

/// Abattoir Ghoul ("Whenever a creature dealt damage by this creature
/// this turn dies, you gain life equal to that creature's toughness.")
/// under either player, on the battlefield or dead itself by then
/// (`there`), for a creature that died with a toughness from -1 to 4:
/// its controller, as it last was controlled, has that much more life,
/// and none for a toughness of 0 or less (CR 119.3, 608.2h); the other
/// player as much as before.
fn abattoir_ghoul(there: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::abattoir_ghoul::AbattoirGhoul));
    let you = u8::from(kani::any::<bool>());
    let source = a_creature_of(&mut state, card, PlayerId(1), PlayerId(you));
    if !there {
        state.move_object(source, Zone::Graveyard, &registry);
    }
    let dead = a_card_in(&mut state, blank, 1, Zone::Graveyard, true);
    let toughness: i32 = kani::any();
    kani::assume((-1..=4).contains(&toughness));
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_any_creature_dies(&mut state, source, dead, PlayerId(1), &[source], toughness, false, &[], &registry);

    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] + if p == you { toughness.max(0) } else { 0 });
    }
    std::mem::forget((registry, state));
}

/// Galvanic Juggernaut ("Whenever another creature dies, untap this
/// creature.") one player's card under either player, tapped or not, on
/// the battlefield or gone from it by then
/// (`there`): there, it is untapped (CR 701.21b) and otherwise as it was;
/// gone, it is the card in the graveyard and nothing is done to it
/// (400.7).
fn galvanic_juggernaut(there: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::galvanic_juggernaut::GalvanicJuggernaut));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let source = a_creature_of(&mut state, card, PlayerId(1), you);
    state.get_object_mut(source).expect("just created").tapped = kani::any();
    if !there {
        state.move_object(source, Zone::Graveyard, &registry);
    }
    let dead = a_card_in(&mut state, blank, 1, Zone::Graveyard, true);

    fire(&mut state, &registry, source, Fired::CreatureDied, dead, &[]);

    let o = state.get_object(source).expect("still an object");
    assert!(!o.tapped && o.damage_marked == 0);
    assert!(if there { o.zone == Zone::Battlefield && o.controller == you } else { o.zone == Zone::Graveyard && o.controller == PlayerId(1) });
    std::mem::forget((registry, state));
}

/// Gutter Grime ("Whenever a nontoken creature you control dies, put a
/// slime counter on this enchantment, then create a green Ooze creature
/// token with 'This token's power and toughness are each equal to the
/// number of slime counters on Gutter Grime.'") one player's card under
/// either player, with a slime counter already or none, on the
/// battlefield or gone from it by then (`there`):
/// there, it has one slime counter more and its controller a new token, a
/// green Ooze creature whose power and toughness the engine reads as
/// that many (CR 604.3, 111.2); gone, no counter is put on it (121.1)
/// and the Ooze is made all the same, a 0/0 (the card's ruling of
/// 2011-09-22).
fn gutter_grime(there: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::gutter_grime::GutterGrime));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let grime = a_source(&mut state, card, PlayerId(1));
    state.get_object_mut(grime).expect("just created").controller = you;
    let had = u32::from(kani::any::<bool>());
    // One entry either way: a map the solver may or may not have put to is
    // one of either length to every read after.
    state.get_object_mut(grime).expect("just created").counters.insert(CounterType::Slime, had);
    if !there {
        state.move_object(grime, Zone::Graveyard, &registry);
    }
    let dead = a_card_in(&mut state, blank, 1, Zone::Graveyard, true);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_any_creature_dies(&mut state, grime, dead, you, &[], 2, false, &[], &registry);

    assert!(state.objects.len() == before + 1);
    let slime = state.get_object(grime).expect("still an object").counters.get(&CounterType::Slime).copied().unwrap_or(0);
    let size = if there {
        assert!(slime == had + 1);
        i32::try_from(slime).expect("small")
    } else {
        0
    };
    let id = ObjectId(dead.0 + 1);
    let t = state.get_object(id).expect("made");
    assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
    assert!(state.is_creature(id, &registry));
    assert!(state.effective_power(id, &registry) == Some(size) && state.effective_toughness(id, &registry) == Some(size));
    assert!(t.colors == [Color::Green] && t.subtypes.len() == 1 && t.subtypes[0] == "Ooze");
    std::mem::forget((registry, state));
}

/// Sturmgeist ("Whenever this creature deals combat damage to a player,
/// draw a card.") the player `who` names its controller (a harness for
/// each: `draws` says why), the other player's card, on the battlefield
/// or gone from it by then (`there`), their library of `cards` cards:
/// the top card of its controller's library, as it last was controlled,
/// is in their hand (CR 121.1, 608.2h), the rest their library in the
/// order they were; with none to draw they have drawn from an empty
/// library (704.5b).
fn sturmgeist((cards, who, there): (usize, u8, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::sturmgeist::Sturmgeist));
    let library = a_library(&mut state, blank, who, cards);
    let source = a_creature_of(&mut state, card, PlayerId(1 - who), PlayerId(who));
    if !there {
        state.move_object(source, Zone::Graveyard, &registry);
    }

    fire(&mut state, &registry, source, Fired::DamagedPlayer, source, &[]);

    let drawn = cards.min(1);
    let player = &state.players[who as usize];
    assert!(player.library_order.len() == cards - drawn && player.has_drawn_from_empty == (cards == 0));
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k < drawn {
            assert!(o.zone == Zone::Hand && o.controller == PlayerId(who));
        } else {
            assert!(o.zone == Zone::Library && player.library_order[k - drawn] == id);
        }
    }
    assert!(!state.players[1 - who as usize].has_drawn_from_empty);
    std::mem::forget((registry, state));
}

// ---- Permanents: what a step sets off (an upkeep, an end step), each
// hook run as the trigger resolves. ----

/// A Werewolf's "At the beginning of each upkeep, if no spells were cast
/// last turn, transform this creature." and its other face's "..., if a
/// player cast two or more spells last turn, transform this creature."
/// resolving, the creature showing its front face or its back (`back`)
/// under either player, each player having cast none, one or two spells
/// last turn: it has transformed if that face's condition holds as the
/// trigger resolves (CR 603.4, 701.28a) and not otherwise, and the engine
/// reads the power and toughness of the face it then shows (712.8). It is
/// otherwise as it was.
fn werewolf((card, back): (Box<dyn CardBehavior>, bool)) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let id = state.create_object(card, you, Zone::Battlefield, power, toughness);
    let o = state.get_object_mut(id).expect("just created");
    o.is_transformed = back;
    o.tapped = kani::any();
    let tapped = o.tapped;
    let cast: [u32; 2] = [kani::any(), kani::any()];
    kani::assume(cast[0] <= 2 && cast[1] <= 2);
    state.num_spells_cast_last_turn.insert(PlayerId(0), cast[0]);
    state.num_spells_cast_last_turn.insert(PlayerId(1), cast[1]);

    registry.get(card).expect("registered").on_upkeep(&mut state, id, &[], &registry);

    let turns = if back { cast[0] >= 2 || cast[1] >= 2 } else { cast[0] + cast[1] == 0 };
    let shows_back = back != turns;
    let o = state.get_object(id).expect("still an object");
    assert!(o.is_transformed == shows_back && o.zone == Zone::Battlefield && o.controller == you && o.tapped == tapped);
    let face = registry.printed_pt(card, shows_back).expect("that face");
    assert!(state.effective_power(id, &registry) == face.0 && state.effective_toughness(id, &registry) == face.1);
    kani::cover!(turns, "it transforms");
    kani::cover!(!turns, "it does not");
    std::mem::forget((registry, state));
}

/// "At the beginning of your upkeep, mill two cards." (Splinterfright)
/// and "At the beginning of enchanted player's upkeep, that player mills
/// two cards." (Curse of the Bloody Tome) resolving, the player `who`
/// names the one who mills (a harness for each: `draws` says why), their
/// library of `cards` cards: the top two of it, or all of it if there are
/// fewer, are in their graveyard, top first (CR 701.17a), the rest their
/// library in the order they were, and milling an empty library is not
/// drawing from one (701.17b).
fn mills_two_at_upkeep((card, cursed, cards, who): (Box<dyn CardBehavior>, bool, usize, u8)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let library = a_library(&mut state, blank, who, cards);
    // A Curse is the other player's, attached to the one who mills; a
    // creature is that player's own.
    let source = a_source(&mut state, card, PlayerId(if cursed { 1 - who } else { who }));
    if cursed {
        state.get_object_mut(source).expect("just created").attached_to_player = Some(PlayerId(who));
    }

    registry.get(card).expect("registered").on_upkeep(&mut state, source, &[], &registry);

    let milled = cards.min(2);
    let player = &state.players[who as usize];
    assert!(player.library_order.len() == cards - milled && player.graveyard_order.len() == milled && !player.has_drawn_from_empty);
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k < milled {
            assert!(o.zone == Zone::Graveyard && player.graveyard_order[k] == id);
        } else {
            assert!(o.zone == Zone::Library && player.library_order[k - milled] == id);
        }
    }
    assert!(state.players[1 - who as usize].graveyard_order.is_empty());
    std::mem::forget((registry, state));
}

/// Bloodgift Demon ("At the beginning of your upkeep, target player draws
/// a card and loses 1 life.") its trigger resolving at the player `who`
/// names (a harness for each: `draws` says why), their library of `cards`
/// cards: the top card of it is in their hand (CR 121.1) and they have 1
/// life fewer (119.3); with none to draw they have drawn from an empty
/// library (704.5b) and lost the life all the same. The other player has
/// as much life as before.
fn bloodgift_demon((cards, who): (usize, u8)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::bloodgift_demon::BloodgiftDemon));
    let library = a_library(&mut state, blank, who, cards);
    let demon = a_source(&mut state, card, PlayerId(0));
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_upkeep(&mut state, demon, &[Target::Player(PlayerId(who))], &registry);

    let drawn = cards.min(1);
    let player = &state.players[who as usize];
    assert!(player.library_order.len() == cards - drawn && player.has_drawn_from_empty == (cards == 0));
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k < drawn {
            assert!(o.zone == Zone::Hand);
        } else {
            assert!(o.zone == Zone::Library && player.library_order[k - drawn] == id);
        }
    }
    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] - i32::from(p == who));
    }
    std::mem::forget((registry, state));
}

/// Angel of Flight Alabaster ("At the beginning of your upkeep, return
/// target Spirit card from your graveyard to your hand.") its trigger
/// resolving at a card in its controller's graveyard, beside another
/// there: the card is in its owner's hand and no longer in the
/// graveyard's order (CR 400.3), and the other card is where it was.
fn angel_of_flight_alabaster(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::angel_of_flight_alabaster::AngelOfFlightAlabaster));
    let other = a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
    let spirit = a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
    state.get_object_mut(spirit).expect("just created").subtypes = vec!["Spirit".to_string()];
    let angel = a_source(&mut state, card, PlayerId(0));

    registry.get(card).expect("registered").on_upkeep(&mut state, angel, &[Target::Object(spirit)], &registry);

    let o = state.get_object(spirit).expect("still an object");
    assert!(o.zone == Zone::Hand && o.controller == PlayerId(0) && o.owner == PlayerId(0));
    let yard = &state.players[0].graveyard_order;
    assert!(yard.len() == 1 && yard[0] == other);
    assert!(state.get_object(other).expect("still an object").zone == Zone::Graveyard);
    std::mem::forget((registry, state));
}

/// Curse of the Pierced Heart ("At the beginning of enchanted player's
/// upkeep, this Aura deals 1 damage to that player or a planeswalker that
/// player controls.") attached to either player, who controls no
/// planeswalker: that player has 1 life fewer (CR 120.3a), the other as
/// much as before, and nothing is asked.
fn curse_of_the_pierced_heart(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::curse_of_the_pierced_heart::CurseOfThePiercedHeart));
    let cursed = u8::from(kani::any::<bool>());
    let curse = a_source(&mut state, card, PlayerId(1 - cursed));
    state.get_object_mut(curse).expect("just created").attached_to_player = Some(PlayerId(cursed));
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_upkeep(&mut state, curse, &[], &registry);

    assert!(state.awaiting_action.is_none());
    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] - i32::from(p == cursed));
    }
    std::mem::forget((registry, state));
}

/// The same with a planeswalker of three loyalty under the enchanted
/// player: the Curse's controller is asked which, the player or the
/// planeswalker and no other, and may not decline; the damage the
/// question holds, 1 from the Curse, applied by the engine at the one
/// `at_the_planeswalker` names (`tribute_to_hunger` says why not through
/// the answer's dispatcher), takes a loyalty counter from the
/// planeswalker (CR 120.3c) or 1 life from the player (120.3a), and
/// nothing from the other.
fn curse_of_the_pierced_heart_chooses(at_the_planeswalker: bool) {
    use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::curse_of_the_pierced_heart::CurseOfThePiercedHeart));
    let walker = a_permanent(&mut state, blank, Permanent::Planeswalker);
    state.get_object_mut(walker).expect("just created").controller = PlayerId(1);
    let curse = a_source(&mut state, card, PlayerId(0));
    state.get_object_mut(curse).expect("just created").attached_to_player = Some(PlayerId(1));
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_upkeep(&mut state, curse, &[], &registry);

    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. }) => {
            assert!(*player == PlayerId(0) && !*optional);
            assert!(options.len() == 2 && options[0] == Target::Player(PlayerId(1)) && options[1] == Target::Object(walker));
            assert!(matches!(effect, PendingEffect::DealDamage { amount: 1, source_id } if *source_id == curse));
        }
        _ => panic!("its controller is asked which"),
    }
    crate::proofs::cards::clear_prompt(&mut state);
    let effect = PendingEffect::DealDamage { amount: 1, source_id: curse };
    let at = if at_the_planeswalker { Target::Object(walker) } else { Target::Player(PlayerId(1)) };
    crate::engine::apply_pending_effect(&mut state, &at, &effect, &registry);

    let o = state.get_object(walker).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == 0);
    assert!(o.counters.get(&CounterType::Loyalty).copied().unwrap_or(0) == if at_the_planeswalker { 2 } else { 3 });
    assert!(state.players[0].life == life[0] && state.players[1].life == life[1] - i32::from(!at_the_planeswalker));
    std::mem::forget((registry, state, effect));
}

/// Endless Ranks of the Dead ("At the beginning of your upkeep, create X
/// 2/2 black Zombie creature tokens, where X is half the number of
/// Zombies you control, rounded down.") under either player, who
/// controls `zombies` Zombie creatures, beside one Zombie of the other
/// player's that is not counted: X new permanents, each a token, a 2/2
/// black Zombie creature under that player (CR 111.2, 107.1a).
fn endless_ranks_of_the_dead(zombies: usize) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::endless_ranks_of_the_dead::EndlessRanksOfTheDead));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    for k in 0..=zombies {
        let id = state.create_object(blank, you, Zone::Battlefield, Some(2), Some(2));
        let o = state.get_object_mut(id).expect("just created");
        o.card_types = vec![CardType::Creature];
        o.subtypes = vec!["Zombie".to_string()];
        // The last is the other player's.
        if k == zombies {
            o.controller = PlayerId(1 - you.0);
        }
    }
    let ranks = a_source(&mut state, card, you);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_upkeep(&mut state, ranks, &[], &registry);

    let made = zombies / 2;
    assert!(state.objects.len() == before + made);
    for k in 1..=made {
        let id = ObjectId(ranks.0 + u64::try_from(k).expect("a few"));
        let t = state.get_object(id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
        assert!(state.is_creature(id, &registry));
        assert!(state.effective_power(id, &registry) == Some(2) && state.effective_toughness(id, &registry) == Some(2));
        assert!(t.colors == [Color::Black] && t.subtypes.len() == 1 && t.subtypes[0] == "Zombie");
    }
    std::mem::forget((registry, state));
}

/// Mayor of Avabruck's other face, Howlpack Alpha ("At the beginning of
/// your end step, create a 2/2 green Wolf creature token."), one
/// player's card under either player, on the battlefield or gone from it
/// by then (`there`): one new permanent, a token, a 2/2 green Wolf
/// creature under the player who last controlled the Alpha (CR 608.2h,
/// 111.2). Howlpack Alpha's own "each other creature you control that's
/// a Werewolf or a Wolf gets +1/+1" is the board's and not the token's:
/// the Wolf is a 3/3 beside the Alpha and a 2/2 without it (613.4c).
fn howlpack_alpha(there: bool) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::mayor_of_avabruck::MayorOfAvabruck));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let alpha = a_creature_of(&mut state, card, PlayerId(1), you);
    state.get_object_mut(alpha).expect("just created").is_transformed = true;
    if !there {
        state.move_object(alpha, Zone::Graveyard, &registry);
    }
    let before = state.objects.len();

    registry.get(card).expect("registered").on_end_step(&mut state, alpha, &[], &registry);

    assert!(state.objects.len() == before + 1);
    let id = ObjectId(alpha.0 + 1);
    let t = state.get_object(id).expect("made");
    assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
    assert!(state.is_creature(id, &registry));
    let size = if there { 3 } else { 2 };
    assert!(state.effective_power(id, &registry) == Some(size) && state.effective_toughness(id, &registry) == Some(size));
    assert!(t.colors == [Color::Green] && t.subtypes.len() == 1 && t.subtypes[0] == "Wolf");
    std::mem::forget((registry, state));
}

/// Cloistered Youth's other face, Unholy Fiend ("At the beginning of
/// your end step, you lose 1 life."), one player's card under either
/// player, on the battlefield or gone from it by then (`there`): the
/// player who last controlled it has 1 life fewer (CR 119.3, 608.2h),
/// the other as much as before.
fn unholy_fiend(there: bool) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::cloistered_youth::CloisteredYouth));
    let you = u8::from(kani::any::<bool>());
    let fiend = a_creature_of(&mut state, card, PlayerId(1), PlayerId(you));
    state.get_object_mut(fiend).expect("just created").is_transformed = true;
    if !there {
        state.move_object(fiend, Zone::Graveyard, &registry);
    }
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_end_step(&mut state, fiend, &[], &registry);

    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] - i32::from(p == you));
    }
    std::mem::forget((registry, state));
}

/// Reaper from the Abyss ("Morbid — At the beginning of each end step,
/// if a creature died this turn, destroy target non-Demon creature.")
/// its trigger resolving at a creature, a creature having died this turn
/// or not (`died`): with one dead the target is put into its owner's
/// graveyard by the engine's own destruction (CR 701.7a), and with none
/// it is where it was and as it was (603.4).
fn reaper_from_the_abyss(died: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::reaper_from_the_abyss::ReaperFromTheAbyss));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let reaper = a_source(&mut state, card, PlayerId(0));
    state.creature_died_this_turn = died;
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_end_step(&mut state, reaper, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    if died {
        assert!(o.zone == Zone::Graveyard && o.controller == PlayerId(1) && !o.tapped && o.damage_marked == 0);
        assert!(state.players[1].graveyard_order.contains(&target));
    } else {
        assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.damage_marked == damage && o.controller == controller);
    }
    assert!(state.get_object(reaper).expect("still an object").zone == Zone::Battlefield);
    std::mem::forget((registry, state));
}

/// Charmbreaker Devils ("Whenever you cast an instant or sorcery spell,
/// this creature gets +4/+0 until end of turn.") its trigger resolving:
/// it has 4 more power and as much toughness as before, read back through
/// the engine's layers (CR 613.4c), and is otherwise as it was.
fn charmbreaker_devils_cast(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::charmbreaker_devils::CharmbreakerDevils));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let devils = a_creature_of(&mut state, card, you, you);
    state.get_object_mut(devils).expect("just created").tapped = kani::any();
    let tapped = state.get_object(devils).expect("just created").tapped;
    let spell = state.create_object(blank, you, Zone::Stack, None, None);

    registry.get(card).expect("registered").on_spell_cast(&mut state, devils, you, spell, &[], &registry);

    assert!(state.effective_power(devils, &registry) == Some(6) && state.effective_toughness(devils, &registry) == Some(2));
    let o = state.get_object(devils).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.controller == you && o.damage_marked == 0);
    std::mem::forget((registry, state));
}

// ---- Permanents: what an attack or a block sets off. ----

/// Ashmouth Hound ("Whenever this creature blocks or becomes blocked by
/// a creature, this creature deals 1 damage to that creature.") under
/// the first player, blocking (`blocks`) or blocked by a creature: 1 more
/// damage is marked on that creature by the engine's own damage (CR
/// 120.3e), which is otherwise as it was, and none on the Hound.
fn ashmouth_hound(blocks: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::ashmouth_hound::AshmouthHound));
    let other = a_permanent(&mut state, blank, Permanent::Creature);
    let hound = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let before = state.get_object(other).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);

    let behavior = registry.get(card).expect("registered");
    if blocks {
        behavior.on_blocks(&mut state, hound, other, &registry);
    } else {
        behavior.on_becomes_blocked(&mut state, hound, other, &registry);
    }

    let o = state.get_object(other).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == damage + 1 && o.tapped == tapped && o.controller == controller);
    let h = state.get_object(hound).expect("still an object");
    assert!(h.zone == Zone::Battlefield && h.damage_marked == 0);
    std::mem::forget((registry, state));
}

/// Wooden Stake ("Whenever equipped creature blocks or becomes blocked
/// by a Vampire, destroy that creature. It can't be regenerated.") its
/// trigger resolving for the Vampire, the creature it equips blocking
/// (`blocks`) or blocked: the Vampire is put into its owner's graveyard
/// by the engine's own destruction (CR 701.7a), a regeneration shield on
/// it left unused (701.15c), unless it is indestructible and nothing
/// happens to it (702.12b).
fn wooden_stake(blocks: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::wooden_stake::WoodenStake));
    let vampire = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(vampire).expect("just created").regeneration_shields = u32::from(kani::any::<bool>());
    // One effect either way (`destroys` says why).
    let indestructible: bool = kani::any();
    let keyword = if indestructible { Keyword::Indestructible } else { Keyword::Flying };
    state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target: vampire, keyword });
    let stake = a_source(&mut state, card, PlayerId(0));
    let before = state.get_object(vampire).expect("just created");
    let (shields, tapped, damage, controller) = (before.regeneration_shields, before.tapped, before.damage_marked, before.controller);

    let behavior = registry.get(card).expect("registered");
    if blocks {
        behavior.on_blocks(&mut state, stake, vampire, &registry);
    } else {
        behavior.on_becomes_blocked(&mut state, stake, vampire, &registry);
    }

    let o = state.get_object(vampire).expect("still an object");
    if indestructible {
        assert!(o.zone == Zone::Battlefield && o.regeneration_shields == shields && o.tapped == tapped && o.damage_marked == damage && o.controller == controller);
    } else {
        assert!(o.zone == Zone::Graveyard && o.controller == PlayerId(1) && !o.tapped && o.damage_marked == 0);
        assert!(state.players[1].graveyard_order.contains(&vampire));
    }
    std::mem::forget((registry, state));
}

/// Hamlet Captain ("Whenever this creature attacks or blocks, other
/// Humans you control get +1/+1 until end of turn.") under the first
/// player, attacking (`attacks`) or blocking, beside a Human of its
/// controller's (`yours`) or of the other player's and a creature of its
/// controller's that is no Human: the Human has 1 more power and
/// toughness, read back through the engine's layers (CR 613.4c), if it
/// is its controller's and is as it was if not; the other creature and
/// the Captain itself are as they were.
fn hamlet_captain((attacks, yours): (bool, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::hamlet_captain::HamletCaptain));
    let human = a_permanent(&mut state, blank, Permanent::Human);
    state.get_object_mut(human).expect("just created").controller = PlayerId(u8::from(!yours));
    let other = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(other).expect("just created").controller = PlayerId(0);
    let captain = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let pt = |state: &GameState, id| (state.effective_power(id, &registry), state.effective_toughness(id, &registry));
    let was = [pt(&state, human), pt(&state, other), pt(&state, captain)];

    let behavior = registry.get(card).expect("registered");
    if attacks {
        behavior.on_attacks(&mut state, captain, crate::cards::AttackInfo::new(captain, PlayerId(1)), &[], &registry);
    } else {
        behavior.on_blocks(&mut state, captain, other, &registry);
    }

    let up = i32::from(yours);
    assert!(pt(&state, human) == (was[0].0.map(|p| p + up), was[0].1.map(|t| t + up)));
    assert!(pt(&state, other) == was[1] && pt(&state, captain) == was[2]);
    std::mem::forget((registry, state));
}

/// The first player's creature of `card` attacking the second player:
/// the combat the engine reads, and the attack as a trigger is told of
/// it.
fn attacking(state: &mut GameState, attacker: ObjectId) -> crate::cards::AttackInfo {
    let mut combat = CombatState::new();
    combat.attackers.insert(attacker, PlayerId(1));
    state.combat = Some(combat);
    crate::cards::AttackInfo::new(attacker, PlayerId(1))
}

/// Geist of Saint Traft ("Whenever this creature attacks, create a 4/4
/// white Angel creature token with flying that's tapped and attacking.
/// Exile that token at end of combat.") attacking the second player, who
/// controls no planeswalker: one new permanent, a token, a 4/4 white
/// Angel creature with flying under the Geist's controller, tapped and
/// attacking that player without having been declared (CR 508.4), and
/// the engine holds its exile for the end of combat (603.7).
fn geist_of_saint_traft(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::geist_of_saint_traft::GeistOfSaintTraft));
    let you = PlayerId(0);
    let geist = a_creature_of(&mut state, card, you, you);
    let attack = attacking(&mut state, geist);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_attacks(&mut state, geist, attack, &[], &registry);

    assert!(state.objects.len() == before + 1 && state.awaiting_action.is_none());
    let id = ObjectId(geist.0 + 1);
    let t = state.get_object(id).expect("made");
    assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && t.tapped);
    assert!(state.is_creature(id, &registry) && state.has_keyword(id, Keyword::Flying, &registry));
    assert!(state.effective_power(id, &registry) == Some(4) && state.effective_toughness(id, &registry) == Some(4));
    assert!(t.colors == [Color::White] && t.subtypes.len() == 1 && t.subtypes[0] == "Angel");
    let combat = state.combat.as_ref().expect("in combat");
    assert!(combat.attackers.get(&id) == Some(&PlayerId(1)) && combat.attackers.get(&geist) == Some(&PlayerId(1)));
    assert!(state.end_of_combat_exiles.len() == 1 && state.end_of_combat_exiles[0].target_id == id && state.end_of_combat_exiles[0].controller == you);
    std::mem::forget((registry, state));
}

/// Kessig Cagebreakers ("Whenever this creature attacks, create a 2/2
/// green Wolf creature token that's tapped and attacking for each
/// creature card in your graveyard.") attacking the second player, who
/// controls no planeswalker, with `creatures` creature cards and one
/// card that is no creature in its controller's graveyard: that many new
/// permanents, each a token, a 2/2 green Wolf creature under its
/// controller, tapped and attacking that player (CR 508.4).
fn kessig_cagebreakers(creatures: u64) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::kessig_cagebreakers::KessigCagebreakers));
    let you = PlayerId(0);
    a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    for _ in 0..creatures {
        let id = a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
        state.get_object_mut(id).expect("just created").card_types = vec![CardType::Creature];
    }
    let cagebreakers = a_creature_of(&mut state, card, you, you);
    let attack = attacking(&mut state, cagebreakers);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_attacks(&mut state, cagebreakers, attack, &[], &registry);

    assert!(state.objects.len() == before + usize::try_from(creatures).expect("a few") && state.awaiting_action.is_none());
    for k in 1..=creatures {
        let id = ObjectId(cagebreakers.0 + k);
        let t = state.get_object(id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && t.tapped);
        assert!(state.is_creature(id, &registry));
        assert!(state.effective_power(id, &registry) == Some(2) && state.effective_toughness(id, &registry) == Some(2));
        assert!(t.colors == [Color::Green] && t.subtypes.len() == 1 && t.subtypes[0] == "Wolf");
        assert!(state.combat.as_ref().expect("in combat").attackers.get(&id) == Some(&PlayerId(1)));
    }
    std::mem::forget((registry, state));
}

/// Grimgrin, Corpse-Born ("Whenever Grimgrin attacks, destroy target
/// creature defending player controls, then put a +1/+1 counter on
/// Grimgrin.") attacking, its trigger resolving at a creature: the
/// creature is put into its owner's graveyard by the engine's own
/// destruction (CR 701.7a), unless it is indestructible and nothing
/// happens to it, or it has a regeneration shield, which is used
/// instead; and Grimgrin has a +1/+1 counter whichever of those it was
/// (the card's ruling of 2013-07-01).
fn grimgrin_attacks(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::grimgrin_corpse_born::GrimgrinCorpseBorn));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(target).expect("just created").regeneration_shields = u32::from(kani::any::<bool>());
    let indestructible: bool = kani::any();
    let keyword = if indestructible { Keyword::Indestructible } else { Keyword::Flying };
    state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target, keyword });
    let grimgrin = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let shield = state.get_object(target).expect("just created").regeneration_shields > 0;

    registry.get(card).expect("registered").on_attacks(&mut state, grimgrin, crate::cards::AttackInfo::new(grimgrin, PlayerId(1)), &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    if indestructible {
        assert!(o.zone == Zone::Battlefield && o.regeneration_shields == u32::from(shield));
    } else if shield {
        assert!(o.zone == Zone::Battlefield && o.tapped && o.damage_marked == 0 && o.regeneration_shields == 0);
    } else {
        assert!(o.zone == Zone::Graveyard && state.players[1].graveyard_order.contains(&target));
    }
    let g = state.get_object(grimgrin).expect("still an object");
    assert!(g.zone == Zone::Battlefield && g.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0) == 1);
    assert!(state.effective_power(grimgrin, &registry) == Some(3) && state.effective_toughness(grimgrin, &registry) == Some(3));
    std::mem::forget((registry, state));
}

// ---- Permanents: activated abilities, each resolving (its cost paid
// and its trip through the stack the engine's: `proofs::actions::
// activating`, `resolving`). ----

/// "{cost}: This creature gets +P/+T [or gains a keyword] until end of
/// turn." resolving for a creature under either player: it has that much
/// more power and toughness than before, and the keyword if the text
/// gives one, read back through the engine's layers (CR 613.4c, 613.1f),
/// and is otherwise as it was. Resolved twice it has twice as much (each
/// resolution its own effect, 611.2c).
fn pumps_itself((card, (power_up, toughness_up), keyword): (Box<dyn CardBehavior>, (i32, i32), Option<Keyword>)) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let source = a_creature_of(&mut state, card, you, you);
    state.get_object_mut(source).expect("just created").tapped = kani::any();
    let tapped = state.get_object(source).expect("just created").tapped;
    if let Some(keyword) = keyword {
        assert!(!state.has_keyword(source, keyword, &registry));
    }

    let behavior = registry.get(card).expect("registered");
    behavior.resolve_activated_ability(&mut state, source, 0, &[], &registry);

    assert!(state.effective_power(source, &registry) == Some(2 + power_up) && state.effective_toughness(source, &registry) == Some(2 + toughness_up));
    if let Some(keyword) = keyword {
        assert!(state.has_keyword(source, keyword, &registry));
    }
    let o = state.get_object(source).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.controller == you && o.damage_marked == 0);

    behavior.resolve_activated_ability(&mut state, source, 0, &[], &registry);

    assert!(state.effective_power(source, &registry) == Some(2 + 2 * power_up) && state.effective_toughness(source, &registry) == Some(2 + 2 * toughness_up));
    std::mem::forget((registry, state));
}

/// Avacynian Priest ("{1}, {T}: Tap target non-Human creature.") its
/// ability resolving at a creature, tapped or not: the creature is
/// tapped (CR 701.21a) and otherwise as it was.
fn avacynian_priest(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::avacynian_priest::AvacynianPriest));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let priest = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let before = state.get_object(target).expect("just created");
    let (damage, controller) = (before.damage_marked, before.controller);

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, priest, 0, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped && o.damage_marked == damage && o.controller == controller);
    std::mem::forget((registry, state));
}

/// "{cost}: Create a P/T black <subtype> creature token with flying."
/// (Bloodline Keeper's Vampire, Skirsdag High Priest's Demon) resolving
/// for a creature one player owns under either player: one new
/// permanent, a token, that creature under the ability's controller (CR
/// 111.2), read back through the engine's own characteristics.
fn makes_a_flier((card, (power, toughness), subtype): (Box<dyn CardBehavior>, (i32, i32), &str)) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let source = a_creature_of(&mut state, card, PlayerId(1), you);
    let before = state.objects.len();

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, source, 0, &[], &registry);

    assert!(state.objects.len() == before + 1);
    let id = ObjectId(source.0 + 1);
    let t = state.get_object(id).expect("made");
    assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
    assert!(state.is_creature(id, &registry) && state.has_keyword(id, Keyword::Flying, &registry));
    assert!(state.effective_power(id, &registry) == Some(power) && state.effective_toughness(id, &registry) == Some(toughness));
    assert!(t.colors == [Color::Black] && t.subtypes.len() == 1 && t.subtypes[0] == subtype);
    std::mem::forget((registry, state));
}

/// Bloodline Keeper ("{B}: Transform this creature. Activate only if you
/// control five or more Vampires.") its second ability resolving, the
/// engine's own transform under it: it shows its other face, Lord of
/// Lineage, whose power and toughness the engine then reads (CR 701.28a,
/// 712.8), and is otherwise as it was.
fn bloodline_keeper_transforms(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::bloodline_keeper::BloodlineKeeper));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let keeper = state.create_object(card, you, Zone::Battlefield, power, toughness);
    state.get_object_mut(keeper).expect("just created").tapped = kani::any();
    let tapped = state.get_object(keeper).expect("just created").tapped;

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, keeper, 1, &[], &registry);

    let o = state.get_object(keeper).expect("still an object");
    assert!(o.is_transformed && o.zone == Zone::Battlefield && o.controller == you && o.tapped == tapped);
    let back = registry.printed_back_pt(card).expect("its back face");
    assert!(state.effective_power(keeper, &registry) == back.0 && state.effective_toughness(keeper, &registry) == back.1);
    std::mem::forget((registry, state));
}

/// Nephalia Drownyard ("{1}{U}{B}, {T}: Target player mills three
/// cards.") its ability resolving at the player `who` names (a harness
/// for each: `draws` says why), their library of `cards` cards: the top
/// three of it, or all of it if there are fewer, are in their graveyard,
/// top first (CR 701.17a), the rest their library in the order they
/// were.
fn nephalia_drownyard((cards, who): (usize, u8)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::nephalia_drownyard::NephaliaDrownyard));
    let library = a_library(&mut state, blank, who, cards);
    let land = a_source(&mut state, card, PlayerId(0));

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, land, 0, &[Target::Player(PlayerId(who))], &registry);

    let milled = cards.min(3);
    let player = &state.players[who as usize];
    assert!(player.library_order.len() == cards - milled && player.graveyard_order.len() == milled && !player.has_drawn_from_empty);
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k < milled {
            assert!(o.zone == Zone::Graveyard && player.graveyard_order[k] == id);
        } else {
            assert!(o.zone == Zone::Library && player.library_order[k - milled] == id);
        }
    }
    std::mem::forget((registry, state));
}

/// Ghoulcaller's Bell ("{T}: Each player mills a card.") its ability
/// resolving, the first player's library of `yours` cards and the second
/// player's of `theirs`: the top card of each is in its owner's
/// graveyard (CR 701.17a) and the rest of each library is as it was; a
/// player with no library mills nothing and has not drawn from an empty
/// one (701.17b).
fn ghoulcallers_bell((yours, theirs): (usize, usize)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::ghoulcallers_bell::GhoulcallersBell));
    let libraries = [a_library(&mut state, blank, 0, yours), a_library(&mut state, blank, 1, theirs)];
    let bell = a_source(&mut state, card, PlayerId(0));

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, bell, 0, &[], &registry);

    for (p, cards) in [(0usize, yours), (1, theirs)] {
        let milled = cards.min(1);
        let player = &state.players[p];
        assert!(player.library_order.len() == cards - milled && player.graveyard_order.len() == milled && !player.has_drawn_from_empty);
        for k in 0..cards {
            let id = libraries[p][k].expect("made");
            let o = state.get_object(id).expect("still an object");
            if k < milled {
                assert!(o.zone == Zone::Graveyard && player.graveyard_order[k] == id);
            } else {
                assert!(o.zone == Zone::Library && player.library_order[k - milled] == id);
            }
        }
    }
    std::mem::forget((registry, state));
}

/// Stensia Bloodhall ("{3}{B}{R}, {T}: This land deals 2 damage to
/// target player or planeswalker.") its ability resolving at a player,
/// either one: that player has 2 life fewer (CR 120.3a), and the other
/// as many as before.
fn stensia_bloodhall_at_a_player(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::stensia_bloodhall::StensiaBloodhall));
    let land = a_source(&mut state, card, PlayerId(0));
    let at = u8::from(kani::any::<bool>());
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, land, 0, &[Target::Player(PlayerId(at))], &registry);

    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] - if p == at { 2 } else { 0 });
    }
    std::mem::forget((registry, state));
}

/// The same at a planeswalker with three loyalty counters: two of them
/// are removed (CR 120.3c), and no damage is marked on it.
fn stensia_bloodhall_at_a_planeswalker(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::stensia_bloodhall::StensiaBloodhall));
    let target = a_permanent(&mut state, blank, Permanent::Planeswalker);
    let land = a_source(&mut state, card, PlayerId(0));
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, land, 0, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == 0 && o.counters.get(&CounterType::Loyalty).copied().unwrap_or(0) == 1);
    assert!(state.players[0].life == life[0] && state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// Lantern Spirit ("{U}: Return this creature to its owner's hand.") one
/// player's card under either player, on the battlefield or gone from it
/// by then (`there`): there, it is in its owner's hand by the engine's
/// own zone change (CR 400.3); gone, it is the card in the graveyard and
/// stays there (400.7).
fn lantern_spirit(there: bool) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::lantern_spirit::LanternSpirit));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let spirit = a_creature_of(&mut state, card, PlayerId(1), you);
    if !there {
        state.move_object(spirit, Zone::Graveyard, &registry);
    }

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, spirit, 0, &[], &registry);

    let o = state.get_object(spirit).expect("still an object");
    assert!(o.zone == if there { Zone::Hand } else { Zone::Graveyard } && o.controller == PlayerId(1) && o.owner == PlayerId(1));
    assert!(state.players[1].graveyard_order.contains(&spirit) != there);
    std::mem::forget((registry, state));
}

/// Elder of Laurels ("{3}{G}: Target creature gets +X/+X until end of
/// turn, where X is the number of creatures you control.") under the
/// first player, its ability resolving at a creature of its controller's
/// (`yours`) or of the other player's: X is counted as the ability
/// resolves, the Elder itself among them and the target if it is its
/// controller's, so the target has 2 more power and toughness if it is
/// and 1 more if not (CR 107.3, 613.4c), and the Elder is as it was.
fn elder_of_laurels(yours: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::elder_of_laurels::ElderOfLaurels));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(target).expect("just created").controller = PlayerId(u8::from(!yours));
    let elder = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let (power, toughness) = (state.effective_power(target, &registry), state.effective_toughness(target, &registry));

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, elder, 0, &[Target::Object(target)], &registry);

    let x = if yours { 2 } else { 1 };
    assert!(state.effective_power(target, &registry) == power.map(|p| p + x) && state.effective_toughness(target, &registry) == toughness.map(|t| t + x));
    assert!(state.effective_power(elder, &registry) == Some(2) && state.effective_toughness(elder, &registry) == Some(2));
    std::mem::forget((registry, state));
}

/// Gavony Township ("{2}{G}{W}, {T}: Put a +1/+1 counter on each
/// creature you control.") under the first player, beside a creature of
/// theirs and one of the other player's: theirs has one +1/+1 counter
/// and that much more power and toughness (CR 122.1a), the other
/// player's has none and is as it was, and the land has none.
fn gavony_township(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::gavony_township::GavonyTownship));
    let yours = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(yours).expect("just created").controller = PlayerId(0);
    let theirs = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(theirs).expect("just created").controller = PlayerId(1);
    let land = a_source(&mut state, card, PlayerId(0));
    let pt = |state: &GameState, id| (state.effective_power(id, &registry), state.effective_toughness(id, &registry));
    let was = [pt(&state, yours), pt(&state, theirs)];

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, land, 0, &[], &registry);

    let counters = |id| state.get_object(id).expect("still an object").counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    assert!(counters(yours) == 1 && counters(theirs) == 0 && counters(land) == 0);
    assert!(pt(&state, yours) == (was[0].0.map(|p| p + 1), was[0].1.map(|t| t + 1)) && pt(&state, theirs) == was[1]);
    std::mem::forget((registry, state));
}

// ---- Permanents: Equipment, and more activated abilities. ----

/// What an Equipment's text says of the creature it equips, as the
/// engine reads it back.
#[derive(Clone, Copy)]
struct Equipped {
    /// What it gets.
    pt: (i32, i32),
    /// A keyword it has.
    keyword: Option<Keyword>,
}

/// "Equip {cost}" resolving at a creature its controller controls (`what`
/// it is: a Human where the text says more of one), the engine's own
/// attachment and continuous effects under the card's hook: the
/// Equipment is attached to the creature (CR 702.6a, 301.5b), and the
/// creature, read back through the engine's own layers, is what the
/// Equipment's text `says` of it (613.4c, 613.1f) and otherwise as it
/// was.
fn equips((card, what, says): (Box<dyn CardBehavior>, Permanent, Equipped)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_permanent(&mut state, blank, what);
    state.get_object_mut(target).expect("just created").controller = PlayerId(0);
    let equipment = a_source(&mut state, card, PlayerId(0));
    let (power, toughness) = (state.effective_power(target, &registry), state.effective_toughness(target, &registry));
    let before = state.get_object(target).expect("just created");
    let (tapped, damage) = (before.tapped, before.damage_marked);

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, equipment, 0, &[Target::Object(target)], &registry);

    let e = state.get_object(equipment).expect("still an object");
    assert!(e.zone == Zone::Battlefield && e.attached_to == Some(target) && e.controller == PlayerId(0));
    assert!(state.effective_power(target, &registry) == power.map(|p| p + says.pt.0));
    assert!(state.effective_toughness(target, &registry) == toughness.map(|t| t + says.pt.1));
    if let Some(keyword) = says.keyword {
        assert!(state.has_keyword(target, keyword, &registry));
    }
    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.damage_marked == damage && o.controller == PlayerId(0));
    std::mem::forget((registry, state));
}

/// The same at a creature the other player controls by then: "target
/// creature you control" is checked as the ability resolves (CR 608.2b),
/// and the Equipment stays unattached, the creature as it was.
fn equip_leaves_their_creature((card, says): (Box<dyn CardBehavior>, Equipped)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(target).expect("just created").controller = PlayerId(1);
    let equipment = a_source(&mut state, card, PlayerId(0));
    let (power, toughness) = (state.effective_power(target, &registry), state.effective_toughness(target, &registry));

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, equipment, 0, &[Target::Object(target)], &registry);

    assert!(state.get_object(equipment).expect("still an object").attached_to.is_none());
    assert!(state.effective_power(target, &registry) == power && state.effective_toughness(target, &registry) == toughness);
    if let Some(keyword) = says.keyword {
        assert!(!state.has_keyword(target, keyword, &registry));
    }
    std::mem::forget((registry, state));
}

/// "{cost}: Regenerate this creature." resolving for a creature under
/// either player, on the battlefield or gone from it by then (`there`):
/// there, it has one regeneration shield more than before (CR 701.19a);
/// gone, the card in the graveyard has none (400.7).
fn shields_itself((card, there): (Box<dyn CardBehavior>, bool)) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let source = a_creature_of(&mut state, card, PlayerId(1), you);
    let had = u32::from(kani::any::<bool>());
    state.get_object_mut(source).expect("just created").regeneration_shields = had;
    if !there {
        state.move_object(source, Zone::Graveyard, &registry);
    }
    let had = state.get_object(source).expect("still an object").regeneration_shields;

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, source, 0, &[], &registry);

    let o = state.get_object(source).expect("still an object");
    assert!(o.regeneration_shields == had + u32::from(there));
    assert!(o.zone == if there { Zone::Battlefield } else { Zone::Graveyard });
    std::mem::forget((registry, state));
}

/// Manor Gargoyle ("{1}: Until end of turn, this creature loses
/// defender and gains flying.") its ability resolving: the engine reads
/// it as having flying and not defender (CR 613.1f), where before it
/// read defender and no flying, and it is otherwise as it was.
fn manor_gargoyle(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::manor_gargoyle::ManorGargoyle));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let gargoyle = a_creature_of(&mut state, card, you, you);
    assert!(state.has_keyword(gargoyle, Keyword::Defender, &registry) && !state.has_keyword(gargoyle, Keyword::Flying, &registry));

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, gargoyle, 0, &[], &registry);

    assert!(!state.has_keyword(gargoyle, Keyword::Defender, &registry) && state.has_keyword(gargoyle, Keyword::Flying, &registry));
    assert!(state.effective_power(gargoyle, &registry) == Some(2) && state.effective_toughness(gargoyle, &registry) == Some(2));
    let o = state.get_object(gargoyle).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.controller == you);
    std::mem::forget((registry, state));
}

/// Mikaeus, the Lunarch ("{T}: Put a +1/+1 counter on Mikaeus." and
/// "{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on
/// each other creature you control.") under the first player beside a
/// creature of theirs and one of the other player's, the ability `index`
/// names resolving: the first puts one counter on Mikaeus and none on
/// another; the second one on the creature its controller controls and
/// none on Mikaeus or the other player's (CR 122.1a).
fn mikaeus(index: usize) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::mikaeus_the_lunarch::MikaeusTheLunarch));
    let yours = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(yours).expect("just created").controller = PlayerId(0);
    let theirs = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(theirs).expect("just created").controller = PlayerId(1);
    let mikaeus = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let pt = |state: &GameState, id| (state.effective_power(id, &registry), state.effective_toughness(id, &registry));
    let was = [pt(&state, yours), pt(&state, theirs), pt(&state, mikaeus)];

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, mikaeus, index, &[], &registry);

    let counters = |id| state.get_object(id).expect("still an object").counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    let (on_it, on_yours) = if index == 0 { (1, 0) } else { (0, 1) };
    assert!(counters(mikaeus) == on_it && counters(yours) == on_yours && counters(theirs) == 0);
    let up = |was: (Option<i32>, Option<i32>), n: u32| (was.0.map(|p| p + n as i32), was.1.map(|t| t + n as i32));
    assert!(pt(&state, yours) == up(was[0], on_yours) && pt(&state, theirs) == was[1] && pt(&state, mikaeus) == up(was[2], on_it));
    std::mem::forget((registry, state));
}

/// Selfless Cathar ("{1}{W}, Sacrifice this creature: Creatures you
/// control get +1/+1 until end of turn.") sacrificed, in its owner's
/// graveyard by the engine's own zone change, its ability resolving for
/// the first player beside a creature of theirs and one of the other
/// player's: theirs has 1 more power and toughness, read back through
/// the engine's layers (CR 613.4c), the other player's is as it was.
fn selfless_cathar(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::selfless_cathar::SelflessCathar));
    let yours = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(yours).expect("just created").controller = PlayerId(0);
    let theirs = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(theirs).expect("just created").controller = PlayerId(1);
    let cathar = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    state.move_object(cathar, Zone::Graveyard, &registry);
    let pt = |state: &GameState, id| (state.effective_power(id, &registry), state.effective_toughness(id, &registry));
    let was = [pt(&state, yours), pt(&state, theirs)];

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, cathar, 0, &[], &registry);

    assert!(pt(&state, yours) == (was[0].0.map(|p| p + 1), was[0].1.map(|t| t + 1)) && pt(&state, theirs) == was[1]);
    std::mem::forget((registry, state));
}

/// Silverchase Fox ("{1}{W}, Sacrifice this creature: Exile target
/// enchantment.") its ability resolving at an enchantment: the
/// enchantment is in exile, its owner's card (CR 406.1, 400.3), by the
/// engine's own zone change.
fn silverchase_fox(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::silverchase_fox::SilverchaseFox));
    let target = a_permanent(&mut state, blank, Permanent::Enchantment);
    let fox = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, fox, 0, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Exile && o.controller == PlayerId(1) && o.owner == PlayerId(1) && !o.tapped);
    std::mem::forget((registry, state));
}

/// "{cost}: ~ deals N damage to <target>." resolving at a creature, the
/// engine's own damage under the hook of the ability `index` names: N
/// more damage is marked on it than before (CR 120.3e), and it is
/// otherwise as it was.
fn an_ability_deals_to_a_creature((card, index, n): (Box<dyn CardBehavior>, usize, u32)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let source = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, source, index, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == damage + n && o.tapped == tapped && o.controller == controller);
    assert!(state.players[0].life == life[0] && state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// The same at a player, either one: that player has N life fewer (CR
/// 120.3a), and the other as many as before.
fn an_ability_deals_to_a_player((card, index, n): (Box<dyn CardBehavior>, usize, u32)) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let source = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let at = u8::from(kani::any::<bool>());
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, source, index, &[Target::Player(PlayerId(at))], &registry);

    for p in 0..2u8 {
        let lost = if p == at { i32::try_from(n).expect("small") } else { 0 };
        assert!(state.players[p as usize].life == life[p as usize] - lost);
    }
    std::mem::forget((registry, state));
}

/// The same at a planeswalker with three loyalty counters: N of them are
/// removed (CR 120.3c), and no damage is marked on it.
fn an_ability_deals_to_a_planeswalker((card, index, n): (Box<dyn CardBehavior>, usize, u32)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let target = a_permanent(&mut state, blank, Permanent::Planeswalker);
    let source = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, source, index, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == 0);
    assert!(o.counters.get(&CounterType::Loyalty).copied().unwrap_or(0) == 3u32.saturating_sub(n));
    assert!(state.players[0].life == life[0] && state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// Kessig Wolf Run ("{X}{R}{G}, {T}: Target creature gets +X/+0 and
/// gains trample until end of turn.") activated with `x` for X, its
/// ability resolving at a creature: it has X more power, as much
/// toughness, and trample, read back through the engine's layers (CR
/// 107.3, 613.4c, 613.1f), and is otherwise as it was.
fn kessig_wolf_run(x: u32) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::kessig_wolf_run::KessigWolfRun));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let land = a_source(&mut state, card, PlayerId(0));
    state.last_activated_x_value = Some(x);
    let (power, toughness) = (state.effective_power(target, &registry), state.effective_toughness(target, &registry));
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, land, 0, &[Target::Object(target)], &registry);

    assert!(state.effective_power(target, &registry) == power.map(|p| p + i32::try_from(x).expect("small")));
    assert!(state.effective_toughness(target, &registry) == toughness && state.has_keyword(target, Keyword::Trample, &registry));
    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.damage_marked == damage && o.controller == controller);
    std::mem::forget((registry, state));
}

/// Disciple of Griselbrand ("{1}, Sacrifice a creature: You gain life
/// equal to the sacrificed creature's toughness.") under either player,
/// its ability resolving for a creature sacrificed with a toughness from
/// -1 to 4: its controller has that much more life, and none for a
/// toughness of 0 or less (CR 119.3, 608.2h); the other player as much
/// as before.
fn disciple_of_griselbrand(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::disciple_of_griselbrand::DiscipleOfGriselbrand));
    let you = u8::from(kani::any::<bool>());
    let disciple = a_creature_of(&mut state, card, PlayerId(1), PlayerId(you));
    let sacrificed = a_card_in(&mut state, blank, 1, Zone::Graveyard, true);
    let toughness: i32 = kani::any();
    kani::assume((-1..=4).contains(&toughness));
    state.last_activated_sacrifice = Some(sacrificed);
    state.last_activated_sacrifice_toughness = Some(toughness);
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, disciple, 0, &[], &registry);

    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] + if p == you { toughness.max(0) } else { 0 });
    }
    std::mem::forget((registry, state));
}

/// Tree of Redemption ("{T}: Exchange your life total with this
/// creature's toughness.") a 0/13 under either player, who has from 1 to
/// 30 life: its controller's life total is 13 and its toughness, read
/// back through the engine's layers, is the life they had (CR 701.12c,
/// 613.4b); the other player's life is as it was.
fn tree_of_redemption(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::tree_of_redemption::TreeOfRedemption));
    let you = u8::from(kani::any::<bool>());
    let tree = state.create_object(card, PlayerId(you), Zone::Battlefield, Some(0), Some(13));
    let had: i32 = kani::any();
    kani::assume((1..=30).contains(&had));
    if you == 0 {
        state.players[0].life = had;
    } else {
        state.players[1].life = had;
    }
    let other = state.players[1 - you as usize].life;

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, tree, 0, &[], &registry);

    assert!(state.players[you as usize].life == 13 && state.players[1 - you as usize].life == other);
    assert!(state.effective_toughness(tree, &registry) == Some(had) && state.effective_power(tree, &registry) == Some(0));
    std::mem::forget((registry, state));
}

/// A harness for each row: the proof named, with what it is given. Every
/// loop is unwound to 7, or to what a table says (`unwind(9)`) where its
/// board holds more objects than that walks.
macro_rules! engine_under_card {
    ($($name:ident: $proof:ident($open:expr);)*) => {
        engine_under_card! { unwind(7) $($name: $proof($open);)* }
    };
    (unwind($n:literal) $($name:ident: $proof:ident($open:expr);)*) => {$(
        #[kani::proof]
        #[kani::unwind($n)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        fn $name() {
            $proof($open);
        }
    )*};
}

use crate::cards::isd::{ancient_grudge::AncientGrudge, bramblecrush::Bramblecrush, brimstone_volley::BrimstoneVolley,
    devils_play::DevilsPlay, geistflame::Geistflame, naturalize::Naturalize, rebuke::Rebuke,
    smite_the_monstrous::SmiteTheMonstrous, urgent_exorcism::UrgentExorcism, victim_of_night::VictimOfNight};
use crate::cards::{doom_blade::DoomBlade, lava_axe::LavaAxe, lightning_bolt::LightningBolt};

// "Destroy target ...": each card at each kind of permanent its text names.
engine_under_card! {
    doom_blade_destroys_a_creature: destroys((Box::new(DoomBlade), Permanent::Creature));
    victim_of_night_destroys_a_creature: destroys((Box::new(VictimOfNight), Permanent::Creature));
    smite_the_monstrous_destroys_a_creature_of_power_four: destroys((Box::new(SmiteTheMonstrous), Permanent::Monstrous));
    rebuke_destroys_an_attacking_creature: destroys((Box::new(Rebuke), Permanent::Attacker));
    ancient_grudge_destroys_an_artifact: destroys((Box::new(AncientGrudge), Permanent::Artifact));
    naturalize_destroys_an_artifact: destroys((Box::new(Naturalize), Permanent::Artifact));
    naturalize_destroys_an_enchantment: destroys((Box::new(Naturalize), Permanent::Enchantment));
    urgent_exorcism_destroys_a_spirit: destroys((Box::new(UrgentExorcism), Permanent::Spirit));
    urgent_exorcism_destroys_an_enchantment: destroys((Box::new(UrgentExorcism), Permanent::Enchantment));
    bramblecrush_destroys_an_artifact: destroys((Box::new(Bramblecrush), Permanent::Artifact));
    bramblecrush_destroys_an_enchantment: destroys((Box::new(Bramblecrush), Permanent::Enchantment));
    bramblecrush_destroys_a_land: destroys((Box::new(Bramblecrush), Permanent::Land));
    bramblecrush_destroys_a_planeswalker: destroys((Box::new(Bramblecrush), Permanent::Planeswalker));
}

// "~ deals N damage to ...": each card at each kind of target its text
// names ("any target" is a creature, a player or a planeswalker, CR 115.4).
engine_under_card! {
    geistflame_deals_one_damage_to_a_creature: deals_to_a_creature((Box::new(Geistflame), Amount::Fixed(1)));
    geistflame_deals_one_damage_to_a_player: deals_to_a_player((Box::new(Geistflame), Amount::Fixed(1)));
    geistflame_deals_one_damage_to_a_planeswalker: deals_to_a_planeswalker((Box::new(Geistflame), Amount::Fixed(1)));
    lightning_bolt_deals_three_damage_to_a_creature: deals_to_a_creature((Box::new(LightningBolt), Amount::Fixed(3)));
    lightning_bolt_deals_three_damage_to_a_player: deals_to_a_player((Box::new(LightningBolt), Amount::Fixed(3)));
    lightning_bolt_deals_three_damage_to_a_planeswalker: deals_to_a_planeswalker((Box::new(LightningBolt), Amount::Fixed(3)));
    lava_axe_deals_five_damage_to_a_player: deals_to_a_player((Box::new(LavaAxe), Amount::Fixed(5)));
    lava_axe_deals_five_damage_to_a_planeswalker: deals_to_a_planeswalker((Box::new(LavaAxe), Amount::Fixed(5)));
    brimstone_volley_deals_three_damage_to_a_creature: deals_to_a_creature((Box::new(BrimstoneVolley), Amount::Morbid { died: false }));
    brimstone_volley_deals_three_damage_to_a_player: deals_to_a_player((Box::new(BrimstoneVolley), Amount::Morbid { died: false }));
    brimstone_volley_deals_three_damage_to_a_planeswalker: deals_to_a_planeswalker((Box::new(BrimstoneVolley), Amount::Morbid { died: false }));
    brimstone_volley_with_morbid_deals_five_damage_to_a_creature: deals_to_a_creature((Box::new(BrimstoneVolley), Amount::Morbid { died: true }));
    brimstone_volley_with_morbid_deals_five_damage_to_a_player: deals_to_a_player((Box::new(BrimstoneVolley), Amount::Morbid { died: true }));
    brimstone_volley_with_morbid_deals_five_damage_to_a_planeswalker: deals_to_a_planeswalker((Box::new(BrimstoneVolley), Amount::Morbid { died: true }));
    devils_play_for_no_x_deals_no_damage_to_a_creature: deals_to_a_creature((Box::new(DevilsPlay), Amount::X(0)));
    devils_play_for_no_x_deals_no_damage_to_a_player: deals_to_a_player((Box::new(DevilsPlay), Amount::X(0)));
    devils_play_for_no_x_deals_no_damage_to_a_planeswalker: deals_to_a_planeswalker((Box::new(DevilsPlay), Amount::X(0)));
    devils_play_for_x_of_one_deals_one_damage_to_a_creature: deals_to_a_creature((Box::new(DevilsPlay), Amount::X(1)));
    devils_play_for_x_of_one_deals_one_damage_to_a_player: deals_to_a_player((Box::new(DevilsPlay), Amount::X(1)));
    devils_play_for_x_of_one_deals_one_damage_to_a_planeswalker: deals_to_a_planeswalker((Box::new(DevilsPlay), Amount::X(1)));
    devils_play_for_x_of_four_deals_four_damage_to_a_creature: deals_to_a_creature((Box::new(DevilsPlay), Amount::X(4)));
    devils_play_for_x_of_four_deals_four_damage_to_a_player: deals_to_a_player((Box::new(DevilsPlay), Amount::X(4)));
    devils_play_for_x_of_four_deals_four_damage_to_a_planeswalker: deals_to_a_planeswalker((Box::new(DevilsPlay), Amount::X(4)));
}

use crate::cards::isd::{altars_reap::AltarsReap, bonds_of_faith::BondsOfFaith, claustrophobia::Claustrophobia, curiosity::Curiosity,
    curse_of_deaths_hold::CurseOfDeathsHold, curse_of_oblivion::CurseOfOblivion, curse_of_stalked_prey::CurseOfStalkedPrey,
    curse_of_the_bloody_tome::CurseOfTheBloodyTome, curse_of_the_nightly_hunt::CurseOfTheNightlyHunt,
    curse_of_the_pierced_heart::CurseOfThePiercedHeart, dead_weight::DeadWeight, dissipate::Dissipate, feeling_of_dread::FeelingOfDread,
    furor_of_the_bitten::FurorOfTheBitten, ghostly_possession::GhostlyPossession, gruesome_deformity::GruesomeDeformity,
    hysterical_blindness::HystericalBlindness, midnight_haunting::MidnightHaunting, moan_of_the_unhallowed::MoanOfTheUnhallowed,
    moment_of_heroism::MomentOfHeroism, nightbirds_clutches::NightbirdsClutches, rally_the_peasants::RallyThePeasants,
    rangers_guile::RangersGuile, sensory_deprivation::SensoryDeprivation, skeletal_grimace::SkeletalGrimace,
    spectral_flight::SpectralFlight, spidery_grasp::SpideryGrasp, think_twice::ThinkTwice, travel_preparations::TravelPreparations,
    vampiric_fury::VampiricFury};
use crate::cards::{counterspell::Counterspell, divination::Divination, giant_growth::GiantGrowth, holy_strength::HolyStrength, pacifism::Pacifism};

// "Target creature gets +P/+T [and gains ...] until end of turn."
engine_under_card! {
    giant_growth_gives_its_target_three_and_three: pumps((Box::new(GiantGrowth), (3, 3), None, false));
    rangers_guile_gives_its_target_one_and_one_and_hexproof: pumps((Box::new(RangersGuile), (1, 1), Some(Keyword::Hexproof), false));
    moment_of_heroism_gives_its_target_two_and_two_and_lifelink: pumps((Box::new(MomentOfHeroism), (2, 2), Some(Keyword::Lifelink), false));
    spidery_grasp_untaps_its_target_and_gives_it_two_and_four_and_reach: pumps((Box::new(SpideryGrasp), (2, 4), Some(Keyword::Reach), true));
}

// "... each of up to two target creatures": with none, one and two targets.
engine_under_card! {
    travel_preparations_with_no_target_puts_no_counter: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 0));
    travel_preparations_puts_a_counter_on_its_one_target: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 1));
    travel_preparations_puts_a_counter_on_each_of_its_two_targets: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 2));
    feeling_of_dread_with_no_target_taps_nothing: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 0));
    feeling_of_dread_taps_its_one_target: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 1));
    feeling_of_dread_taps_each_of_its_two_targets: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 2));
    nightbirds_clutches_with_no_target_stops_no_block: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 0));
    nightbirds_clutches_stops_its_one_target_blocking: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 1));
    nightbirds_clutches_stops_each_of_its_two_targets_blocking: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 2));
}

// The creatures of one side, until end of turn.
engine_under_card! {
    rally_the_peasants_gives_your_creatures_two_power: gives_each((Box::new(RallyThePeasants), ToAll::YoursPlusTwo));
    vampiric_fury_gives_your_vampires_two_power_and_first_strike: gives_each((Box::new(VampiricFury), ToAll::YourVampiresPlusTwoAndFirstStrike));
    hysterical_blindness_takes_four_power_from_their_creatures: gives_each((Box::new(HystericalBlindness), ToAll::TheirsMinusFour));
}

// "Draw N cards.": for each player, from a library of none to more than N.
engine_under_card! {
    think_twice_draws_from_your_empty_library: draws((Box::new(ThinkTwice), 1, 0, false));
    think_twice_draws_from_their_empty_library: draws((Box::new(ThinkTwice), 1, 0, true));
    think_twice_draws_your_only_card: draws((Box::new(ThinkTwice), 1, 1, false));
    think_twice_draws_their_only_card: draws((Box::new(ThinkTwice), 1, 1, true));
    think_twice_draws_the_top_of_your_two: draws((Box::new(ThinkTwice), 1, 2, false));
    think_twice_draws_the_top_of_their_two: draws((Box::new(ThinkTwice), 1, 2, true));
    divination_draws_from_your_empty_library: draws((Box::new(Divination), 2, 0, false));
    divination_draws_from_their_empty_library: draws((Box::new(Divination), 2, 0, true));
    divination_draws_your_only_card: draws((Box::new(Divination), 2, 1, false));
    divination_draws_their_only_card: draws((Box::new(Divination), 2, 1, true));
    divination_draws_both_of_your_two: draws((Box::new(Divination), 2, 2, false));
    divination_draws_both_of_their_two: draws((Box::new(Divination), 2, 2, true));
    divination_draws_the_top_two_of_your_three: draws((Box::new(Divination), 2, 3, false));
    divination_draws_the_top_two_of_their_three: draws((Box::new(Divination), 2, 3, true));
    altars_reap_draws_from_your_empty_library: draws((Box::new(AltarsReap), 2, 0, false));
    altars_reap_draws_from_their_empty_library: draws((Box::new(AltarsReap), 2, 0, true));
    altars_reap_draws_your_only_card: draws((Box::new(AltarsReap), 2, 1, false));
    altars_reap_draws_their_only_card: draws((Box::new(AltarsReap), 2, 1, true));
    altars_reap_draws_both_of_your_two: draws((Box::new(AltarsReap), 2, 2, false));
    altars_reap_draws_both_of_their_two: draws((Box::new(AltarsReap), 2, 2, true));
    altars_reap_draws_the_top_two_of_your_three: draws((Box::new(AltarsReap), 2, 3, false));
    altars_reap_draws_the_top_two_of_their_three: draws((Box::new(AltarsReap), 2, 3, true));
}

// "Create N ... creature tokens."
engine_under_card! {
    midnight_haunting_makes_two_spirits: makes_tokens((Box::new(MidnightHaunting), 2, (1, 1), Color::White, "Spirit", Some(Keyword::Flying)));
    moan_of_the_unhallowed_makes_two_zombies: makes_tokens((Box::new(MoanOfTheUnhallowed), 2, (2, 2), Color::Black, "Zombie", None));
}

// "Enchant creature": the Aura attached, and the creature as its text says.
engine_under_card! {
    holy_strength_gives_the_creature_it_enchants_one_and_two: enchants((Box::new(HolyStrength), Permanent::Creature, Enchanted { pt: (1, 2), ..AS_IT_WAS }));
    dead_weight_takes_two_and_two_from_the_creature_it_enchants: enchants((Box::new(DeadWeight), Permanent::Creature, Enchanted { pt: (-2, -2), ..AS_IT_WAS }));
    sensory_deprivation_takes_three_power_from_the_creature_it_enchants: enchants((Box::new(SensoryDeprivation), Permanent::Creature, Enchanted { pt: (-3, 0), ..AS_IT_WAS }));
    spectral_flight_gives_the_creature_it_enchants_two_and_two_and_flying: enchants((Box::new(SpectralFlight), Permanent::Creature, Enchanted { pt: (2, 2), keyword: Some(Keyword::Flying), ..AS_IT_WAS }));
    furor_of_the_bitten_gives_two_and_two_and_makes_it_attack: enchants((Box::new(FurorOfTheBitten), Permanent::Creature, Enchanted { pt: (2, 2), must_attack: true, ..AS_IT_WAS }));
    skeletal_grimace_gives_the_creature_it_enchants_one_and_one: enchants((Box::new(SkeletalGrimace), Permanent::Creature, Enchanted { pt: (1, 1), ..AS_IT_WAS }));
    ghostly_possession_gives_the_creature_it_enchants_flying: enchants((Box::new(GhostlyPossession), Permanent::Creature, Enchanted { keyword: Some(Keyword::Flying), ..AS_IT_WAS }));
    gruesome_deformity_gives_the_creature_it_enchants_intimidate: enchants((Box::new(GruesomeDeformity), Permanent::Creature, Enchanted { keyword: Some(Keyword::Intimidate), ..AS_IT_WAS }));
    pacifism_stops_the_creature_it_enchants_attacking_and_blocking: enchants((Box::new(Pacifism), Permanent::Creature, Enchanted { cant_attack: true, cant_block: true, ..AS_IT_WAS }));
    claustrophobia_keeps_the_creature_it_enchants_from_untapping: enchants((Box::new(Claustrophobia), Permanent::Creature, Enchanted { doesnt_untap: true, ..AS_IT_WAS }));
    curiosity_attaches_to_the_creature_it_enchants: enchants((Box::new(Curiosity), Permanent::Creature, AS_IT_WAS));
    bonds_of_faith_gives_a_human_two_and_two: enchants((Box::new(BondsOfFaith), Permanent::Human, Enchanted { pt: (2, 2), ..AS_IT_WAS }));
    bonds_of_faith_stops_another_creature_attacking_and_blocking: enchants((Box::new(BondsOfFaith), Permanent::Creature, Enchanted { cant_attack: true, cant_block: true, ..AS_IT_WAS }));
}

// "Enchant player": the Curse attached, and that player's creatures as its
// text says.
engine_under_card! {
    curse_of_deaths_hold_takes_one_and_one_from_the_enchanted_players_creatures: curses((Box::new(CurseOfDeathsHold), Cursed::MinusOne));
    curse_of_the_nightly_hunt_makes_the_enchanted_players_creatures_attack: curses((Box::new(CurseOfTheNightlyHunt), Cursed::MustAttack));
    curse_of_oblivion_attaches_to_the_player_it_enchants: curses((Box::new(CurseOfOblivion), Cursed::Nothing));
    curse_of_stalked_prey_attaches_to_the_player_it_enchants: curses((Box::new(CurseOfStalkedPrey), Cursed::Nothing));
    curse_of_the_bloody_tome_attaches_to_the_player_it_enchants: curses((Box::new(CurseOfTheBloodyTome), Cursed::Nothing));
    curse_of_the_pierced_heart_attaches_to_the_player_it_enchants: curses((Box::new(CurseOfThePiercedHeart), Cursed::Nothing));
}

// Spells of their own.
engine_under_card! {
    counterspell_counters_your_own_spell: counters((Box::new(Counterspell), false, 0));
    counterspell_counters_their_spell: counters((Box::new(Counterspell), false, 1));
    dissipate_counters_and_exiles_your_own_spell: counters((Box::new(Dissipate), true, 0));
    dissipate_counters_and_exiles_their_spell: counters((Box::new(Dissipate), true, 1));
    bump_in_the_night_takes_three_life: bump_in_the_night(());
    infernal_plunge_adds_three_red_mana: infernal_plunge(());
    silent_departure_returns_a_creature_to_its_owners_hand: silent_departure(());
}

use crate::cards::isd::{blasphemous_act::BlasphemousAct, dream_twist::DreamTwist, purify_the_grave::PurifyTheGrave,
    runic_repetition::RunicRepetition, unburial_rites::UnburialRites};

// "Target player mills three cards.": at each player, a library of none,
// fewer than three, three and more.
engine_under_card! {
    dream_twist_mills_nothing_from_your_empty_library: mills((Box::new(DreamTwist), 3, 0, 0));
    dream_twist_mills_nothing_from_their_empty_library: mills((Box::new(DreamTwist), 3, 0, 1));
    dream_twist_mills_both_of_your_two: mills((Box::new(DreamTwist), 3, 2, 0));
    dream_twist_mills_both_of_their_two: mills((Box::new(DreamTwist), 3, 2, 1));
    dream_twist_mills_all_of_your_three: mills((Box::new(DreamTwist), 3, 3, 0));
    dream_twist_mills_all_of_their_three: mills((Box::new(DreamTwist), 3, 3, 1));
    dream_twist_mills_the_top_three_of_your_four: mills((Box::new(DreamTwist), 3, 4, 0));
    dream_twist_mills_the_top_three_of_their_four: mills((Box::new(DreamTwist), 3, 4, 1));
}

// A card moved from one zone that is not the battlefield to another.
engine_under_card! {
    purify_the_grave_exiles_a_card_from_your_graveyard: moves_a_card((Box::new(PurifyTheGrave), Zone::Graveyard, Zone::Exile, 0));
    purify_the_grave_exiles_a_card_from_their_graveyard: moves_a_card((Box::new(PurifyTheGrave), Zone::Graveyard, Zone::Exile, 1));
    runic_repetition_returns_an_exiled_card_to_your_hand: moves_a_card((Box::new(RunicRepetition), Zone::Exile, Zone::Hand, 0));
    unburial_rites_returns_a_creature_card_to_the_battlefield: moves_a_card((Box::new(UnburialRites), Zone::Graveyard, Zone::Battlefield, 0));
    ghoulcallers_chant_returns_its_one_target_to_your_hand: ghoulcallers_chant(1);
    ghoulcallers_chant_returns_its_two_targets_to_your_hand: ghoulcallers_chant(2);
}

// Counted from a graveyard.
engine_under_card! {
    gnaw_to_the_bone_with_no_creature_card_gains_nothing: gnaw_to_the_bone(0);
    gnaw_to_the_bone_with_one_creature_card_gains_two: gnaw_to_the_bone(1);
    gnaw_to_the_bone_with_two_creature_cards_gains_four: gnaw_to_the_bone(2);
    spider_spawning_with_no_creature_card_makes_no_spider: spider_spawning(0);
    spider_spawning_with_one_creature_card_makes_a_spider: spider_spawning(1);
}

// A board of seven objects, which a walk of them unwound to 7 does not end.
engine_under_card! {
    unwind(9)
    spider_spawning_with_two_creature_cards_makes_two_spiders: spider_spawning(2);
}

// Damage to each creature, and from what a cost exiled.
engine_under_card! {
    blasphemous_act_deals_thirteen_damage_to_each_creature: deals_to_each_creature((Box::new(BlasphemousAct), 13, false, false));
    blasphemous_act_deals_thirteen_damage_to_a_flier_too: deals_to_each_creature((Box::new(BlasphemousAct), 13, false, true));
    corpse_lunge_for_a_power_of_none_deals_no_damage: corpse_lunge(0);
    corpse_lunge_for_a_power_of_two_deals_two_damage: corpse_lunge(2);
    corpse_lunge_for_a_power_of_five_deals_five_damage: corpse_lunge(5);
}

// Spells of their own, at a creature.
engine_under_card! {
    grasp_of_phantoms_puts_a_creature_on_top_of_an_empty_library: grasp_of_phantoms(0);
    grasp_of_phantoms_puts_a_creature_on_top_of_a_library_of_two: grasp_of_phantoms(2);
    swords_to_plowshares_exiles_a_creature_and_gives_its_controller_two_life: swords_to_plowshares(Permanent::Creature);
    swords_to_plowshares_exiles_a_creature_of_power_four_and_gives_four_life: swords_to_plowshares(Permanent::Monstrous);
    prey_upon_has_two_creatures_fight: prey_upon(());
    lost_in_the_mist_counters_a_spell_and_returns_a_permanent: lost_in_the_mist(());
}

// Spells of their own, the last of those with no question to ask.
engine_under_card! {
    traitorous_blood_takes_a_creature_untaps_it_and_gives_it_trample_and_haste: traitorous_blood(());
    harvest_pyre_with_nothing_exiled_deals_no_damage: harvest_pyre(0);
    harvest_pyre_with_three_cards_exiled_deals_three_damage: harvest_pyre(3);
    into_the_maw_of_hell_destroys_a_land_and_deals_thirteen_damage: into_the_maw_of_hell(());
    maw_of_the_mire_destroys_a_land_and_gains_four_life: maw_of_the_mire(());
    paraselene_destroys_both_enchantments_and_gains_two_life: paraselene(false);
    paraselene_spares_an_indestructible_enchantment_and_gains_one_life: paraselene(true);
}

// More spells of their own: counted from a graveyard, and by name.
engine_under_card! {
    wreath_of_geists_with_no_creature_card_gives_nothing: wreath_of_geists(0);
    wreath_of_geists_with_one_creature_card_gives_one_and_one: wreath_of_geists(1);
    wreath_of_geists_with_two_creature_cards_gives_two_and_two: wreath_of_geists(2);
    sever_the_bloodline_exiles_every_creature_of_its_targets_name: sever_the_bloodline(());
}

// Each reshaped after it ran out of memory as first written: one creature
// alone for Rolling Temblor, and for Tribute to Hunger, the first spell
// here that asks a question, the effect its question holds applied by the
// engine without the answer's dispatcher. Not yet, each out of memory at
// 8 GB: Rolling Temblor beside a creature that has flying
// (`rolling_temblor(true)`), and Spare from Evil, with one creature of
// yours and one other and the engine's protection read back.
engine_under_card! {
    rolling_temblor_deals_two_damage_to_a_creature_without_flying: rolling_temblor(false);
    tribute_to_hunger_has_the_opponent_sacrifice_their_first_creature: tribute_to_hunger(0);
    tribute_to_hunger_has_the_opponent_sacrifice_their_second_creature: tribute_to_hunger(1);
}

// Permanents: enters triggers.
engine_under_card! {
    armored_skaab_mills_nothing_from_your_empty_library: armored_skaab((0, 0));
    armored_skaab_mills_nothing_from_their_empty_library: armored_skaab((0, 1));
    armored_skaab_mills_all_of_your_three: armored_skaab((3, 0));
    armored_skaab_mills_all_of_their_three: armored_skaab((3, 1));
    armored_skaab_mills_all_of_your_four: armored_skaab((4, 0));
    armored_skaab_mills_all_of_their_four: armored_skaab((4, 1));
    claustrophobia_taps_the_creature_it_enchants_as_it_enters: claustrophobia_enters(());
    crossway_vampire_stops_its_target_blocking: crossway_vampire(());
    geist_honored_monk_makes_two_spirits: geist_honored_monk(());
    hollowhenge_scavenger_gains_nothing_with_no_death: hollowhenge_scavenger(false);
    hollowhenge_scavenger_with_morbid_gains_five_life: hollowhenge_scavenger(true);
    morkrut_banshee_does_nothing_with_no_death: morkrut_banshee(false);
    morkrut_banshee_with_morbid_gives_its_target_minus_four: morkrut_banshee(true);
    slayer_of_the_wicked_asks_and_destroys_its_target: slayer_of_the_wicked(());
    geistcatchers_rig_asks_and_deals_four_damage_to_its_target: geistcatchers_rig(());
}

use crate::cards::isd::{bloodcrazed_neonate::BloodcrazedNeonate, champion_of_the_parish::ChampionOfTheParish,
    doomed_traveler::DoomedTraveler, falkenrath_marauders::FalkenrathMarauders,
    lumberknot::Lumberknot, mausoleum_guard::MausoleumGuard, pitchburn_devils::PitchburnDevils, rage_thrower::RageThrower,
    rakish_heir::RakishHeir, stromkirk_noble::StromkirkNoble, stromkirk_patrol::StromkirkPatrol, unruly_mob::UnrulyMob,
    village_cannibals::VillageCannibals};

// Permanents: a counter on the trigger's own source, there or gone.
engine_under_card! {
    bloodcrazed_neonate_gets_a_counter_for_its_combat_damage: counters_itself((Box::new(BloodcrazedNeonate), 1, Fired::DamagedPlayer, true));
    bloodcrazed_neonate_gone_gets_no_counter: counters_itself((Box::new(BloodcrazedNeonate), 1, Fired::DamagedPlayer, false));
    falkenrath_marauders_get_two_counters_for_their_combat_damage: counters_itself((Box::new(FalkenrathMarauders), 2, Fired::DamagedPlayer, true));
    falkenrath_marauders_gone_get_no_counter: counters_itself((Box::new(FalkenrathMarauders), 2, Fired::DamagedPlayer, false));
    stromkirk_noble_gets_a_counter_for_its_combat_damage: counters_itself((Box::new(StromkirkNoble), 1, Fired::DamagedPlayer, true));
    stromkirk_noble_gone_gets_no_counter: counters_itself((Box::new(StromkirkNoble), 1, Fired::DamagedPlayer, false));
    stromkirk_patrol_gets_a_counter_for_its_combat_damage: counters_itself((Box::new(StromkirkPatrol), 1, Fired::DamagedPlayer, true));
    stromkirk_patrol_gone_gets_no_counter: counters_itself((Box::new(StromkirkPatrol), 1, Fired::DamagedPlayer, false));
    lumberknot_gets_a_counter_for_another_creatures_death: counters_itself((Box::new(Lumberknot), 1, Fired::CreatureDied, true));
    lumberknot_gone_gets_no_counter_for_another_creatures_death: counters_itself((Box::new(Lumberknot), 1, Fired::CreatureDied, false));
    lumberknot_dead_gets_no_counter_for_its_own_death: counters_itself((Box::new(Lumberknot), 1, Fired::Died, false));
    unruly_mob_gets_a_counter_for_a_death: counters_itself((Box::new(UnrulyMob), 1, Fired::CreatureDied, true));
    unruly_mob_gone_gets_no_counter: counters_itself((Box::new(UnrulyMob), 1, Fired::CreatureDied, false));
    village_cannibals_get_a_counter_for_a_death: counters_itself((Box::new(VillageCannibals), 1, Fired::CreatureDied, true));
    village_cannibals_gone_get_no_counter: counters_itself((Box::new(VillageCannibals), 1, Fired::CreatureDied, false));
    champion_of_the_parish_gets_a_counter_for_an_entry: counters_itself((Box::new(ChampionOfTheParish), 1, Fired::CreatureEntered, true));
    champion_of_the_parish_gone_gets_no_counter: counters_itself((Box::new(ChampionOfTheParish), 1, Fired::CreatureEntered, false));
}

// Permanents: a counter on the creature that dealt the damage.
engine_under_card! {
    rakish_heir_puts_a_counter_on_the_vampire_that_dealt_the_damage: counters_what_dealt((Box::new(RakishHeir), true));
    rakish_heir_puts_no_counter_on_a_vampire_gone_since: counters_what_dealt((Box::new(RakishHeir), false));
    curse_of_stalked_prey_puts_a_counter_on_the_creature_that_dealt_the_damage: counters_what_dealt((Box::new(CurseOfStalkedPrey), true));
    curse_of_stalked_prey_puts_no_counter_on_a_creature_gone_since: counters_what_dealt((Box::new(CurseOfStalkedPrey), false));
}

// Permanents: what a death sets off.
engine_under_card! {
    doomed_traveler_dies_into_a_spirit: dies_into_spirits((Box::new(DoomedTraveler), 1));
    mausoleum_guard_dies_into_two_spirits: dies_into_spirits((Box::new(MausoleumGuard), 2));
    elder_cathar_dies_and_puts_a_counter_on_a_creature: elder_cathar((Permanent::Creature, 1));
    elder_cathar_dies_and_puts_two_counters_on_a_human: elder_cathar((Permanent::Human, 2));
    pitchburn_devils_dies_and_deals_three_to_a_creature: a_death_deals_to_a_creature((Box::new(PitchburnDevils), Fired::Died, 3));
    pitchburn_devils_dies_and_deals_three_to_a_player: a_death_deals_to_a_player((Box::new(PitchburnDevils), Fired::Died, 3));
    pitchburn_devils_dies_and_deals_three_to_a_planeswalker: a_death_deals_to_a_planeswalker((Box::new(PitchburnDevils), Fired::Died, 3));
    rage_thrower_deals_two_to_a_player_for_a_death: a_death_deals_to_a_player((Box::new(RageThrower), Fired::CreatureDied, 2));
    falkenrath_noble_dies_and_drains_a_player: falkenrath_noble(Fired::Died);
    falkenrath_noble_drains_a_player_for_another_creatures_death: falkenrath_noble(Fired::CreatureDied);
    selhoff_occultist_dies_and_mills_nothing_from_your_empty_library: selhoff_occultist((Fired::Died, 0, 0));
    selhoff_occultist_dies_and_mills_nothing_from_their_empty_library: selhoff_occultist((Fired::Died, 0, 1));
    selhoff_occultist_dies_and_mills_the_top_of_your_two: selhoff_occultist((Fired::Died, 2, 0));
    selhoff_occultist_dies_and_mills_the_top_of_their_two: selhoff_occultist((Fired::Died, 2, 1));
    selhoff_occultist_mills_the_top_of_your_two_for_another_creatures_death: selhoff_occultist((Fired::CreatureDied, 2, 0));
    selhoff_occultist_mills_the_top_of_their_two_for_another_creatures_death: selhoff_occultist((Fired::CreatureDied, 2, 1));
    abattoir_ghoul_gains_the_toughness_of_what_died: abattoir_ghoul(true);
    abattoir_ghoul_dead_itself_still_gains_the_toughness_of_what_died: abattoir_ghoul(false);
    galvanic_juggernaut_untaps_for_a_death: galvanic_juggernaut(true);
    galvanic_juggernaut_gone_is_not_untapped: galvanic_juggernaut(false);
    gutter_grime_gets_a_slime_counter_and_makes_an_ooze_that_size: gutter_grime(true);
    gutter_grime_gone_makes_an_ooze_of_no_size: gutter_grime(false);
}

// Permanents: a card drawn for combat damage.
engine_under_card! {
    sturmgeist_draws_your_top_card: sturmgeist((2, 0, true));
    sturmgeist_draws_their_top_card: sturmgeist((2, 1, true));
    sturmgeist_gone_still_draws_your_top_card: sturmgeist((2, 0, false));
    sturmgeist_gone_still_draws_their_top_card: sturmgeist((2, 1, false));
    sturmgeist_draws_from_your_empty_library: sturmgeist((0, 0, true));
    sturmgeist_draws_from_their_empty_library: sturmgeist((0, 1, true));
}

use crate::cards::isd::{daybreak_ranger::DaybreakRanger, gatstaf_shepherd::GatstafShepherd,
    grizzled_outcasts::GrizzledOutcasts, hanweir_watchkeep::HanweirWatchkeep, instigator_gang::InstigatorGang, kruin_outlaw::KruinOutlaw,
    mayor_of_avabruck::MayorOfAvabruck, reckless_waif::RecklessWaif, splinterfright::Splinterfright, tormented_pariah::TormentedPariah,
    ulvenwald_mystics::UlvenwaldMystics, village_ironsmith::VillageIronsmith, villagers_of_estwald::VillagersOfEstwald};

// Permanents: the Werewolves' upkeep triggers, each face of each.
engine_under_card! {
    daybreak_ranger_transforms_when_no_spell_was_cast: werewolf((Box::new(DaybreakRanger), false));
    nightfall_predator_transforms_back_when_a_player_cast_two: werewolf((Box::new(DaybreakRanger), true));
    gatstaf_shepherd_transforms_when_no_spell_was_cast: werewolf((Box::new(GatstafShepherd), false));
    gatstaf_howler_transforms_back_when_a_player_cast_two: werewolf((Box::new(GatstafShepherd), true));
    grizzled_outcasts_transform_when_no_spell_was_cast: werewolf((Box::new(GrizzledOutcasts), false));
    krallenhorde_wantons_transform_back_when_a_player_cast_two: werewolf((Box::new(GrizzledOutcasts), true));
    hanweir_watchkeep_transforms_when_no_spell_was_cast: werewolf((Box::new(HanweirWatchkeep), false));
    bane_of_hanweir_transforms_back_when_a_player_cast_two: werewolf((Box::new(HanweirWatchkeep), true));
    instigator_gang_transforms_when_no_spell_was_cast: werewolf((Box::new(InstigatorGang), false));
    wildblood_pack_transforms_back_when_a_player_cast_two: werewolf((Box::new(InstigatorGang), true));
    kruin_outlaw_transforms_when_no_spell_was_cast: werewolf((Box::new(KruinOutlaw), false));
    terror_of_kruin_pass_transforms_back_when_a_player_cast_two: werewolf((Box::new(KruinOutlaw), true));
    mayor_of_avabruck_transforms_when_no_spell_was_cast: werewolf((Box::new(MayorOfAvabruck), false));
    howlpack_alpha_transforms_back_when_a_player_cast_two: werewolf((Box::new(MayorOfAvabruck), true));
    reckless_waif_transforms_when_no_spell_was_cast: werewolf((Box::new(RecklessWaif), false));
    merciless_predator_transforms_back_when_a_player_cast_two: werewolf((Box::new(RecklessWaif), true));
    tormented_pariah_transforms_when_no_spell_was_cast: werewolf((Box::new(TormentedPariah), false));
    rampaging_werewolf_transforms_back_when_a_player_cast_two: werewolf((Box::new(TormentedPariah), true));
    ulvenwald_mystics_transform_when_no_spell_was_cast: werewolf((Box::new(UlvenwaldMystics), false));
    ulvenwald_primordials_transform_back_when_a_player_cast_two: werewolf((Box::new(UlvenwaldMystics), true));
    village_ironsmith_transforms_when_no_spell_was_cast: werewolf((Box::new(VillageIronsmith), false));
    ironfang_transforms_back_when_a_player_cast_two: werewolf((Box::new(VillageIronsmith), true));
    villagers_of_estwald_transform_when_no_spell_was_cast: werewolf((Box::new(VillagersOfEstwald), false));
    howlpack_of_estwald_transforms_back_when_a_player_cast_two: werewolf((Box::new(VillagersOfEstwald), true));
}

// Permanents: other upkeep triggers.
engine_under_card! {
    splinterfright_mills_nothing_from_your_empty_library: mills_two_at_upkeep((Box::new(Splinterfright), false, 0, 0));
    splinterfright_mills_the_one_card_of_their_library: mills_two_at_upkeep((Box::new(Splinterfright), false, 1, 1));
    splinterfright_mills_the_top_two_of_your_three: mills_two_at_upkeep((Box::new(Splinterfright), false, 3, 0));
    splinterfright_mills_the_top_two_of_their_three: mills_two_at_upkeep((Box::new(Splinterfright), false, 3, 1));
    curse_of_the_bloody_tome_mills_nothing_from_your_empty_library: mills_two_at_upkeep((Box::new(CurseOfTheBloodyTome), true, 0, 0));
    curse_of_the_bloody_tome_mills_the_one_card_of_their_library: mills_two_at_upkeep((Box::new(CurseOfTheBloodyTome), true, 1, 1));
    curse_of_the_bloody_tome_mills_the_top_two_of_your_three: mills_two_at_upkeep((Box::new(CurseOfTheBloodyTome), true, 3, 0));
    curse_of_the_bloody_tome_mills_the_top_two_of_their_three: mills_two_at_upkeep((Box::new(CurseOfTheBloodyTome), true, 3, 1));
    bloodgift_demon_has_you_draw_and_lose_a_life: bloodgift_demon((2, 0));
    bloodgift_demon_has_them_draw_and_lose_a_life: bloodgift_demon((2, 1));
    bloodgift_demon_has_you_draw_from_an_empty_library_and_lose_a_life: bloodgift_demon((0, 0));
    bloodgift_demon_has_them_draw_from_an_empty_library_and_lose_a_life: bloodgift_demon((0, 1));
    angel_of_flight_alabaster_returns_its_target_to_your_hand: angel_of_flight_alabaster(());
    curse_of_the_pierced_heart_deals_one_to_the_enchanted_player: curse_of_the_pierced_heart(());
    curse_of_the_pierced_heart_asks_and_deals_one_to_the_player: curse_of_the_pierced_heart_chooses(false);
    curse_of_the_pierced_heart_asks_and_deals_one_to_their_planeswalker: curse_of_the_pierced_heart_chooses(true);
    endless_ranks_of_the_dead_makes_nothing_with_no_zombie: endless_ranks_of_the_dead(0);
    endless_ranks_of_the_dead_makes_nothing_with_one_zombie: endless_ranks_of_the_dead(1);
    endless_ranks_of_the_dead_makes_a_zombie_with_two: endless_ranks_of_the_dead(2);
    endless_ranks_of_the_dead_makes_a_zombie_with_three: endless_ranks_of_the_dead(3);
}

// Permanents: end step triggers, and a cast trigger.
engine_under_card! {
    howlpack_alpha_makes_a_wolf_it_makes_bigger: howlpack_alpha(true);
    howlpack_alpha_gone_still_makes_a_wolf: howlpack_alpha(false);
    unholy_fiend_has_its_controller_lose_a_life: unholy_fiend(true);
    unholy_fiend_gone_still_has_its_controller_lose_a_life: unholy_fiend(false);
    reaper_from_the_abyss_does_nothing_with_no_death: reaper_from_the_abyss(false);
    reaper_from_the_abyss_with_morbid_destroys_its_target: reaper_from_the_abyss(true);
    charmbreaker_devils_get_four_power_for_a_spell_cast: charmbreaker_devils_cast(());
}

// Permanents: what an attack or a block sets off.
engine_under_card! {
    ashmouth_hound_deals_one_to_the_creature_it_blocks: ashmouth_hound(true);
    ashmouth_hound_deals_one_to_the_creature_that_blocks_it: ashmouth_hound(false);
    wooden_stake_destroys_the_vampire_its_creature_blocks: wooden_stake(true);
    wooden_stake_destroys_the_vampire_that_blocks_its_creature: wooden_stake(false);
    hamlet_captain_attacking_gives_your_human_one_and_one: hamlet_captain((true, true));
    hamlet_captain_attacking_gives_their_human_nothing: hamlet_captain((true, false));
    hamlet_captain_blocking_gives_your_human_one_and_one: hamlet_captain((false, true));
    hamlet_captain_blocking_gives_their_human_nothing: hamlet_captain((false, false));
    geist_of_saint_traft_attacks_with_an_angel_beside_it: geist_of_saint_traft(());
    kessig_cagebreakers_attack_alone_with_no_creature_card: kessig_cagebreakers(0);
    kessig_cagebreakers_attack_with_a_wolf_for_one_creature_card: kessig_cagebreakers(1);
    kessig_cagebreakers_attack_with_two_wolves_for_two_creature_cards: kessig_cagebreakers(2);
    grimgrin_attacks_destroys_its_target_and_gets_a_counter: grimgrin_attacks(());
}

use crate::cards::isd::{bloodline_keeper::BloodlineKeeper, darkthicket_wolf::DarkthicketWolf, feral_ridgewolf::FeralRidgewolf,
    kessig_wolf::KessigWolf, skirsdag_high_priest::SkirsdagHighPriest};

// Permanents: activated abilities.
engine_under_card! {
    darkthicket_wolf_gets_two_and_two: pumps_itself((Box::new(DarkthicketWolf), (2, 2), None));
    feral_ridgewolf_gets_two_power: pumps_itself((Box::new(FeralRidgewolf), (2, 0), None));
    kessig_wolf_gains_first_strike: pumps_itself((Box::new(KessigWolf), (0, 0), Some(Keyword::FirstStrike)));
    avacynian_priest_taps_its_target: avacynian_priest(());
    bloodline_keeper_makes_a_vampire: makes_a_flier((Box::new(BloodlineKeeper), (2, 2), "Vampire"));
    skirsdag_high_priest_makes_a_demon: makes_a_flier((Box::new(SkirsdagHighPriest), (5, 5), "Demon"));
    bloodline_keeper_transforms_into_lord_of_lineage: bloodline_keeper_transforms(());
    nephalia_drownyard_mills_nothing_from_your_empty_library: nephalia_drownyard((0, 0));
    nephalia_drownyard_mills_all_of_their_two: nephalia_drownyard((2, 1));
    nephalia_drownyard_mills_the_top_three_of_your_four: nephalia_drownyard((4, 0));
    nephalia_drownyard_mills_the_top_three_of_their_four: nephalia_drownyard((4, 1));
    ghoulcallers_bell_mills_nothing_from_two_empty_libraries: ghoulcallers_bell((0, 0));
    ghoulcallers_bell_mills_the_top_card_of_each_library: ghoulcallers_bell((2, 2));
    ghoulcallers_bell_mills_from_the_one_library_there_is: ghoulcallers_bell((0, 2));
    stensia_bloodhall_deals_two_to_a_player: stensia_bloodhall_at_a_player(());
    stensia_bloodhall_deals_two_to_a_planeswalker: stensia_bloodhall_at_a_planeswalker(());
    lantern_spirit_returns_to_its_owners_hand: lantern_spirit(true);
    lantern_spirit_gone_stays_where_it_is: lantern_spirit(false);
    elder_of_laurels_gives_your_creature_two_and_two: elder_of_laurels(true);
    elder_of_laurels_gives_their_creature_one_and_one: elder_of_laurels(false);
    gavony_township_puts_a_counter_on_each_creature_you_control: gavony_township(());
}

use crate::cards::isd::{blazing_torch::BlazingTorch, butchers_cleaver::ButchersCleaver, cobbled_wings::CobbledWings,
    daybreak_ranger::DaybreakRanger as Ranger, demonmail_hauberk::DemonmailHauberk, inquisitors_flail::InquisitorsFlail,
    manor_skeleton::ManorSkeleton, mask_of_avacyn::MaskOfAvacyn, runechanters_pike::RunechantersPike,
    sharpened_pitchfork::SharpenedPitchfork, silver_inlaid_dagger::SilverInlaidDagger, skirsdag_cultist::SkirsdagCultist,
    trepanation_blade::TrepanationBlade, ulvenwald_mystics::UlvenwaldMystics as Mystics, wooden_stake::WoodenStake};

/// An Equipment whose text changes nothing the engine reads off the
/// creature's power, toughness or keywords.
const NOTHING: Equipped = Equipped { pt: (0, 0), keyword: None };

// Permanents: Equipment attached by its equip ability, each at a
// creature, and at a Human where its text says more of one.
engine_under_card! {
    butchers_cleaver_gives_three_power: equips((Box::new(ButchersCleaver), Permanent::Creature, Equipped { pt: (3, 0), keyword: None }));
    butchers_cleaver_gives_a_human_lifelink_too: equips((Box::new(ButchersCleaver), Permanent::Human, Equipped { pt: (3, 0), keyword: Some(Keyword::Lifelink) }));
    cobbled_wings_gives_flying: equips((Box::new(CobbledWings), Permanent::Creature, Equipped { pt: (0, 0), keyword: Some(Keyword::Flying) }));
    demonmail_hauberk_gives_four_and_two: equips((Box::new(DemonmailHauberk), Permanent::Creature, Equipped { pt: (4, 2), keyword: None }));
    inquisitors_flail_is_attached: equips((Box::new(InquisitorsFlail), Permanent::Creature, NOTHING));
    mask_of_avacyn_gives_one_and_two_and_hexproof: equips((Box::new(MaskOfAvacyn), Permanent::Creature, Equipped { pt: (1, 2), keyword: Some(Keyword::Hexproof) }));
    runechanters_pike_gives_first_strike_with_no_spell_card_in_the_graveyard: equips((Box::new(RunechantersPike), Permanent::Creature, Equipped { pt: (0, 0), keyword: Some(Keyword::FirstStrike) }));
    sharpened_pitchfork_gives_first_strike: equips((Box::new(SharpenedPitchfork), Permanent::Creature, Equipped { pt: (0, 0), keyword: Some(Keyword::FirstStrike) }));
    sharpened_pitchfork_gives_a_human_one_and_one_too: equips((Box::new(SharpenedPitchfork), Permanent::Human, Equipped { pt: (1, 1), keyword: Some(Keyword::FirstStrike) }));
    silver_inlaid_dagger_gives_two_power: equips((Box::new(SilverInlaidDagger), Permanent::Creature, Equipped { pt: (2, 0), keyword: None }));
    silver_inlaid_dagger_gives_a_human_three_power: equips((Box::new(SilverInlaidDagger), Permanent::Human, Equipped { pt: (3, 0), keyword: None }));
    blazing_torch_is_attached: equips((Box::new(BlazingTorch), Permanent::Creature, NOTHING));
    trepanation_blade_is_attached: equips((Box::new(TrepanationBlade), Permanent::Creature, NOTHING));
    wooden_stake_gives_one_power: equips((Box::new(WoodenStake), Permanent::Creature, Equipped { pt: (1, 0), keyword: None }));
    cobbled_wings_is_not_attached_to_their_creature: equip_leaves_their_creature((Box::new(CobbledWings), Equipped { pt: (0, 0), keyword: Some(Keyword::Flying) }));
}

// Permanents: more activated abilities.
engine_under_card! {
    manor_skeleton_gets_a_regeneration_shield: shields_itself((Box::new(ManorSkeleton), true));
    manor_skeleton_gone_gets_no_shield: shields_itself((Box::new(ManorSkeleton), false));
    ulvenwald_primordials_get_a_regeneration_shield: shields_itself((Box::new(Mystics), true));
    manor_gargoyle_loses_defender_and_gains_flying: manor_gargoyle(());
    mikaeus_puts_a_counter_on_itself: mikaeus(0);
    mikaeus_puts_a_counter_on_each_other_creature_you_control: mikaeus(1);
    selfless_cathar_gives_your_creatures_one_and_one: selfless_cathar(());
    silverchase_fox_exiles_its_target: silverchase_fox(());
    skirsdag_cultist_deals_two_to_a_creature: an_ability_deals_to_a_creature((Box::new(SkirsdagCultist), 0, 2));
    skirsdag_cultist_deals_two_to_a_player: an_ability_deals_to_a_player((Box::new(SkirsdagCultist), 0, 2));
    skirsdag_cultist_deals_two_to_a_planeswalker: an_ability_deals_to_a_planeswalker((Box::new(SkirsdagCultist), 0, 2));
    daybreak_ranger_deals_two_to_its_target: an_ability_deals_to_a_creature((Box::new(Ranger), 0, 2));
    kessig_wolf_run_with_no_x_gives_trample: kessig_wolf_run(0);
    kessig_wolf_run_with_three_for_x_gives_three_power_and_trample: kessig_wolf_run(3);
    disciple_of_griselbrand_gains_the_toughness_of_what_was_sacrificed: disciple_of_griselbrand(());
    tree_of_redemption_exchanges_life_and_toughness: tree_of_redemption(());
}
