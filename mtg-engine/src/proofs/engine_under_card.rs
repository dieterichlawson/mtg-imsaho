//! A card resolving with the engine itself under it: no recorder stands in
//! for an engine action here. The card's own hook runs, and under it the
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

// Damage to each creature, and from what a cost exiled. Not Rolling
// Temblor ("2 damage to each creature without flying"), yet: it asks of
// each creature whether it has flying before it queues the damage, and
// with the creatures' scalars open both rows ran out of memory at 7 GB.
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

// More spells of their own: counted from a graveyard, and by name. Not
// yet, each out of memory at 8 GB as first written: Spare from Evil
// (three creatures and the engine's protection read back), and Tribute to
// Hunger, the first spell here to ask a question and have the engine's
// own answering (`engine::actions::choices::resolve_choice`) resolve it.
engine_under_card! {
    wreath_of_geists_with_no_creature_card_gives_nothing: wreath_of_geists(0);
    wreath_of_geists_with_one_creature_card_gives_one_and_one: wreath_of_geists(1);
    wreath_of_geists_with_two_creature_cards_gives_two_and_two: wreath_of_geists(2);
    sever_the_bloodline_exiles_every_creature_of_its_targets_name: sever_the_bloodline(());
}
