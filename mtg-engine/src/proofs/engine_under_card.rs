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
//! another. And a value the solver can prove constant is not one symbolic
//! execution has folded: a face a transform has just written is asserted
//! and then written back as that constant before the layers read it
//! (`werewolf_as_cast`), where reading it as left ran out of memory at
//! 7 GB and reading it so takes a minute and a half.

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

/// The same for a Werewolf either of whose faces prints a static ability
/// (Hanweir Watchkeep, Instigator Gang, Kruin Outlaw, Mayor of
/// Avabruck), with the spells each player cast last turn a harness's own
/// (`cast`), so that the face it shows afterwards is one face in each
/// harness: the layers reading a face left open follow both faces'
/// abilities over the board. It has transformed if the condition of the
/// face it showed holds for `cast`, and the engine reads the power and
/// toughness of the face it then shows.
fn werewolf_as_cast((card, back, cast): (Box<dyn CardBehavior>, bool, [u32; 2])) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let id = state.create_object(card, you, Zone::Battlefield, power, toughness);
    let o = state.get_object_mut(id).expect("just created");
    o.is_transformed = back;
    o.tapped = kani::any();
    let tapped = o.tapped;
    state.num_spells_cast_last_turn.insert(PlayerId(0), cast[0]);
    state.num_spells_cast_last_turn.insert(PlayerId(1), cast[1]);

    registry.get(card).expect("registered").on_upkeep(&mut state, id, &[], &registry);

    let turns = if back { cast[0] >= 2 || cast[1] >= 2 } else { cast[0] + cast[1] == 0 };
    let shows_back = back != turns;
    let o = state.get_object(id).expect("still an object");
    assert!(o.is_transformed == shows_back && o.zone == Zone::Battlefield && o.controller == you && o.tapped == tapped);
    // The face it shows, written back as the constant it has just been
    // asserted to be. The transform writes it down a path whose guards
    // the solver decides and symbolic execution does not fold, so that
    // the layers, reading it next, follow the static abilities of both
    // faces at once: measured, the hook alone and the read alone each
    // prove in a minute, and the read after the hook ran out of memory at
    // 7 GB. The write changes nothing on any run the assertion allows.
    state.get_object_mut(id).expect("still an object").is_transformed = shows_back;
    let face = registry.printed_pt(card, shows_back).expect("that face");
    assert!(state.effective_power(id, &registry) == face.0 && state.effective_toughness(id, &registry) == face.1);
    std::mem::forget((registry, state));
}

/// The same for Instigator Gang, without the power and toughness read
/// back: its faces' "attacking creatures you control get +1/+0" (and
/// +3/+0) is a filter of two conditions, and the layers matching it ran
/// out of memory at 5 GB with nothing on the board open and no trigger
/// run at all. What is asserted is the transform: the face it shows, and
/// that it is otherwise as it was. What the filter matches is proved of
/// the engine with the filter built where it is matched
/// (`proofs::actions::filters`), and the ability is read on every small
/// board.
fn werewolf_flips((card, back, cast): (Box<dyn CardBehavior>, bool, [u32; 2])) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let id = state.create_object(card, you, Zone::Battlefield, power, toughness);
    let o = state.get_object_mut(id).expect("just created");
    o.is_transformed = back;
    o.tapped = kani::any();
    let tapped = o.tapped;
    state.num_spells_cast_last_turn.insert(PlayerId(0), cast[0]);
    state.num_spells_cast_last_turn.insert(PlayerId(1), cast[1]);

    registry.get(card).expect("registered").on_upkeep(&mut state, id, &[], &registry);

    let turns = if back { cast[0] >= 2 || cast[1] >= 2 } else { cast[0] + cast[1] == 0 };
    let o = state.get_object(id).expect("still an object");
    assert!(o.is_transformed == (back != turns) && o.zone == Zone::Battlefield && o.controller == you && o.tapped == tapped);
    assert!(o.damage_marked == 0 && o.counters.is_empty());
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
/// player controls.") attached to the player `cursed` names (a harness
/// for each: left open, the harness went over 4 GB), who controls no
/// planeswalker: that player has 1 life fewer (CR 120.3a), the other as
/// much as before, and nothing is asked.
fn curse_of_the_pierced_heart(cursed: u8) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::curse_of_the_pierced_heart::CurseOfThePiercedHeart));
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
/// Zombies you control, rounded down.") under the player `who` names,
/// who controls `zombies` Zombie creatures, beside one Zombie of the
/// other player's (`theirs`) that is not counted, or none: X new
/// permanents, each a token, a 2/2 black Zombie creature under that
/// player (CR 111.2, 107.1a). One Zombie of theirs beside one of the
/// other player's makes none.
///
/// Four permanents at most once the token is made: with a fifth, a list
/// the engine collects the battlefield into outgrows the room its first
/// entry gave it, which a proof here does not allow (`no_growing`), so
/// three Zombies, and two beside another player's, are the small
/// boards'.
fn endless_ranks_of_the_dead((zombies, theirs, who): (usize, bool, u8)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::endless_ranks_of_the_dead::EndlessRanksOfTheDead));
    let you = PlayerId(who);
    for k in 0..zombies + usize::from(theirs) {
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
/// player's card under either player, its trigger resolving after it
/// has left the battlefield: one new permanent, a token, a 2/2 green
/// Wolf creature under the player who last controlled the Alpha (CR
/// 608.2h, 111.2), read back through the engine's layers.
///
/// Not proved with the Alpha still on the battlefield, where the Wolf is
/// a 3/3 by the Alpha's own "each other creature you control that's a
/// Werewolf or a Wolf gets +1/+1": the filter, read back from the card's
/// printed effects, is one the solver matches as every filter there is
/// (`werewolf_flips` says the same of Instigator Gang), and the token
/// entering beside it ran out of memory at 7 GB with the power not read
/// at all. That board is the small boards'.
fn howlpack_alpha(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::mayor_of_avabruck::MayorOfAvabruck));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let alpha = a_creature_of(&mut state, card, PlayerId(1), you);
    state.get_object_mut(alpha).expect("just created").is_transformed = true;
    state.move_object(alpha, Zone::Graveyard, &registry);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_end_step(&mut state, alpha, &[], &registry);

    assert!(state.objects.len() == before + 1);
    let id = ObjectId(alpha.0 + 1);
    let t = state.get_object(id).expect("made");
    assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
    assert!(state.is_creature(id, &registry));
    assert!(state.effective_power(id, &registry) == Some(2) && state.effective_toughness(id, &registry) == Some(2));
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

/// Kessig Cagebreakers ("Whenever this creature attacks, create a 2/2
/// green Wolf creature token that's tapped and attacking for each
/// creature card in your graveyard.") attacking the second player, who
/// controls no planeswalker, with `creatures` creature cards and one
/// card that is no creature in its controller's graveyard: that many new
/// permanents, each a token, a 2/2 green Wolf creature under its
/// controller, tapped and attacking that player (CR 508.4).
///
/// Proved with no creature card there, where it makes nothing. With one
/// or two, and for Geist of Saint Traft's Angel, a token put onto the
/// battlefield attacking ran out of memory at 7 GB (1.7 to 2.1 million
/// steps of symbolic execution). The engine's own proofs take that in
/// two halves, the offer of what the token attacks with the effect
/// recorded (`proofs::actions::choosing_targets`) and the effect by
/// itself (`proofs::actions::answering`); here it is one piece, and too
/// much. Those are the small boards'.
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

// ---- Permanents: triggers that ask "you may". The hook asks; the
// answer's own hook (`on_yes_no_choice`) is run for each answer, the
// engine's dispatch of an answer to it proved of itself
// (`proofs::actions::answering`; `tribute_to_hunger` says why not here). ----

/// The question a "you may" trigger of `source` asks: of the player
/// `you`, yes or no, on the source's behalf (CR 603.5). Read where it
/// lies and then taken down, as answered.
fn asked_yes_or_no(state: &mut GameState, source: ObjectId, you: PlayerId) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, source: asked_for, choice: ResolutionChoiceKind::YesNo { source_card, .. } }) => {
            assert!(*player == you && *asked_for == source && *source_card == source);
        }
        _ => panic!("its controller is asked yes or no"),
    }
    crate::proofs::cards::clear_prompt(state);
}

/// A "you may transform this creature" trigger of a double-faced
/// creature under either player, set off by another creature's death
/// (Thraben Sentry) or, `at_upkeep`, by its controller's upkeep
/// (Cloistered Youth): its controller is asked, and answering yes it
/// shows its other face, whose power and toughness the engine then
/// reads (CR 701.28a, 712.8); answering no it is as it was.
fn may_transform((card, at_upkeep, yes): (Box<dyn CardBehavior>, bool, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let source = state.create_object(card, you, Zone::Battlefield, power, toughness);
    state.get_object_mut(source).expect("just created").tapped = kani::any();
    let tapped = state.get_object(source).expect("just created").tapped;
    let dead = a_card_in(&mut state, blank, 1, Zone::Graveyard, true);

    let behavior = registry.get(card).expect("registered");
    if at_upkeep {
        behavior.on_upkeep(&mut state, source, &[], &registry);
    } else {
        behavior.on_any_creature_dies(&mut state, source, dead, you, &[], 2, false, &[], &registry);
    }
    asked_yes_or_no(&mut state, source, you);
    behavior.on_yes_no_choice(&mut state, source, yes, &registry);

    let o = state.get_object(source).expect("still an object");
    assert!(o.is_transformed == yes && o.zone == Zone::Battlefield && o.controller == you && o.tapped == tapped);
    let face = registry.printed_pt(card, yes).expect("that face");
    assert!(state.effective_power(source, &registry) == face.0 && state.effective_toughness(source, &registry) == face.1);
    assert!(state.awaiting_action.is_none());
    std::mem::forget((registry, state));
}

/// The same trigger resolving for a creature that has transformed
/// already (`gone` false: the first of two such triggers did it, Thraben
/// Sentry's ruling of 2011-09-22) or that has left the battlefield
/// (`gone`): nothing is asked, and nothing is done to it (CR 400.7).
fn may_transform_asks_nothing((card, at_upkeep, gone): (Box<dyn CardBehavior>, bool, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let source = state.create_object(card, PlayerId(1), Zone::Battlefield, power, toughness);
    state.get_object_mut(source).expect("just created").controller = you;
    if gone {
        state.move_object(source, Zone::Graveyard, &registry);
    } else {
        state.get_object_mut(source).expect("just created").is_transformed = true;
    }
    let dead = a_card_in(&mut state, blank, 1, Zone::Graveyard, true);

    let behavior = registry.get(card).expect("registered");
    if at_upkeep {
        behavior.on_upkeep(&mut state, source, &[], &registry);
    } else {
        behavior.on_any_creature_dies(&mut state, source, dead, you, &[], 2, false, &[], &registry);
    }

    assert!(state.awaiting_action.is_none());
    let o = state.get_object(source).expect("still an object");
    assert!(o.is_transformed != gone && o.zone == if gone { Zone::Graveyard } else { Zone::Battlefield });
    std::mem::forget((registry, state));
}

/// Curiosity ("Whenever enchanted creature deals damage to an opponent,
/// you may draw a card.") the player `who` names its controller (a
/// harness for each: `draws` says why), their library of `cards` cards:
/// they are asked, and answering yes the top card of their library is in
/// their hand (CR 121.1), or with none to draw they have drawn from an
/// empty library (704.5b); answering no nothing is drawn.
fn curiosity((cards, who, yes): (usize, u8, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::curiosity::Curiosity));
    let library = a_library(&mut state, blank, who, cards);
    let creature = a_permanent(&mut state, blank, Permanent::Creature);
    let aura = a_source(&mut state, card, PlayerId(who));
    state.attach(aura, creature);

    let behavior = registry.get(card).expect("registered");
    behavior.on_any_damage_to_player(&mut state, aura, creature, PlayerId(1 - who), 2, &registry);
    asked_yes_or_no(&mut state, aura, PlayerId(who));
    behavior.on_yes_no_choice(&mut state, aura, yes, &registry);

    let drawn = if yes { cards.min(1) } else { 0 };
    let player = &state.players[who as usize];
    assert!(player.library_order.len() == cards - drawn && player.has_drawn_from_empty == (yes && cards == 0));
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k < drawn {
            assert!(o.zone == Zone::Hand);
        } else {
            assert!(o.zone == Zone::Library && player.library_order[k - drawn] == id);
        }
    }
    std::mem::forget((registry, state));
}

/// Murder of Crows ("Whenever another creature dies, you may draw a
/// card. If you do, discard a card.") the player `who` names its
/// controller (a harness for each: `draws` says why), their hand empty
/// and their library of `cards` cards: they are asked; answering yes
/// with a card to draw, it is drawn and, the only card in their hand,
/// discarded, so that it is in their graveyard (CR 701.8a) and the rest
/// of the library as it was; with none to draw nothing is discarded
/// ("if you do"), and they have drawn from an empty library (704.5b);
/// answering no nothing moves.
fn murder_of_crows((cards, who, yes): (usize, u8, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::murder_of_crows::MurderOfCrows));
    let library = a_library(&mut state, blank, who, cards);
    let crows = a_creature_of(&mut state, card, PlayerId(who), PlayerId(who));
    let dead = a_card_in(&mut state, blank, 1 - who, Zone::Graveyard, true);
    let yard = state.players[who as usize].graveyard_order.len();

    let behavior = registry.get(card).expect("registered");
    behavior.on_any_creature_dies(&mut state, crows, dead, PlayerId(1 - who), &[], 2, false, &[], &registry);
    asked_yes_or_no(&mut state, crows, PlayerId(who));
    behavior.on_yes_no_choice(&mut state, crows, yes, &registry);

    let looted = if yes { cards.min(1) } else { 0 };
    let player = &state.players[who as usize];
    assert!(state.awaiting_action.is_none());
    assert!(player.library_order.len() == cards - looted && player.graveyard_order.len() == yard + looted);
    assert!(player.has_drawn_from_empty == (yes && cards == 0));
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k < looted {
            assert!(o.zone == Zone::Graveyard && player.graveyard_order[yard + k] == id);
        } else {
            assert!(o.zone == Zone::Library && player.library_order[k - looted] == id);
        }
    }
    std::mem::forget((registry, state));
}

// ---- Permanents: more enters triggers, watchers and abilities. ----

/// Village Bell-Ringer ("When this creature enters, untap all creatures
/// you control.") entered under the first player beside a creature of
/// theirs and one of the other player's, each tapped or not: theirs is
/// untapped (CR 701.21b), the other player's is as it was, and each is
/// otherwise unchanged.
fn village_bell_ringer(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::village_bell_ringer::VillageBellRinger));
    let yours = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(yours).expect("just created").controller = PlayerId(0);
    let theirs = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(theirs).expect("just created").controller = PlayerId(1);
    let ringer = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let was = state.get_object(theirs).expect("just created").tapped;
    let damage = [state.get_object(yours).expect("just created").damage_marked, state.get_object(theirs).expect("just created").damage_marked];

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, ringer, &[], &registry);

    let (y, t) = (state.get_object(yours).expect("still an object"), state.get_object(theirs).expect("still an object"));
    assert!(!y.tapped && y.zone == Zone::Battlefield && y.damage_marked == damage[0] && y.controller == PlayerId(0));
    assert!(t.tapped == was && t.zone == Zone::Battlefield && t.damage_marked == damage[1] && t.controller == PlayerId(1));
    std::mem::forget((registry, state));
}

/// Undead Alchemist ("Whenever a creature card is put into an opponent's
/// graveyard from their library, exile that card and create a 2/2 black
/// Zombie creature token.") under either player, its trigger resolving
/// for a card still in that graveyard (`there`) or taken out of it
/// since: the card is in exile if it was there to exile (CR 406.1,
/// 400.7), and either way its controller has one new permanent, a token,
/// a 2/2 black Zombie creature (111.2).
fn undead_alchemist(there: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::undead_alchemist::UndeadAlchemist));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let milled = a_card_in(&mut state, blank, 1, if there { Zone::Graveyard } else { Zone::Hand }, true);
    let alchemist = a_creature_of(&mut state, card, PlayerId(1), you);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_creature_card_milled(&mut state, alchemist, milled, PlayerId(1), &registry);

    let o = state.get_object(milled).expect("still an object");
    assert!(o.zone == if there { Zone::Exile } else { Zone::Hand } && o.owner == PlayerId(1));
    assert!(state.players[1].graveyard_order.is_empty());
    assert!(state.objects.len() == before + 1);
    let id = ObjectId(alchemist.0 + 1);
    let t = state.get_object(id).expect("made");
    assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
    assert!(state.is_creature(id, &registry));
    assert!(state.effective_power(id, &registry) == Some(2) && state.effective_toughness(id, &registry) == Some(2));
    assert!(t.colors == [Color::Black] && t.subtypes.len() == 1 && t.subtypes[0] == "Zombie");
    std::mem::forget((registry, state));
}

/// Deranged Assistant ("{T}: Add {C}. Mill a card.") its mana ability
/// activated by the player `who` names (a harness for each: `draws` says
/// why), their library of `cards` cards: the top card of it is in their
/// graveyard (CR 701.17a), the rest their library in the order they
/// were; with no card there nothing is milled, and that is not drawing
/// from an empty library (701.17b). The mana is the engine's
/// (`proofs::cards::mana`).
fn deranged_assistant((cards, who): (usize, u8)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::deranged_assistant::DerangedAssistant));
    let library = a_library(&mut state, blank, who, cards);
    let assistant = a_creature_of(&mut state, card, PlayerId(who), PlayerId(who));

    registry.get(card).expect("registered").on_activate_mana_ability(&mut state, assistant, 0, &registry);

    let milled = cards.min(1);
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

/// Cellar Door ("{3}, {T}: Target player mills a card from the bottom of
/// their library. If it's a creature card, you create a 2/2 black Zombie
/// creature token.") under the first player, its ability resolving at
/// the player `who` names (a harness for each: `draws` says why), their
/// library of `cards` cards, the bottom one a creature card or not
/// (`creature`): the bottom card is in its owner's graveyard and the
/// rest their library in the order they were (CR 701.17a); for a
/// creature card the Door's controller, not the player milled, has a new
/// 2/2 black Zombie creature token (111.2), and otherwise none.
fn cellar_door((cards, who, creature): (usize, u8, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::cellar_door::CellarDoor));
    let library = a_library(&mut state, blank, who, cards);
    if creature {
        let bottom = library[cards - 1].expect("made");
        let o = state.get_object_mut(bottom).expect("just created");
        o.card_types = vec![CardType::Creature];
        o.power = Some(2);
        o.toughness = Some(2);
    }
    let door = a_source(&mut state, card, PlayerId(0));
    let before = state.objects.len();

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, door, 0, &[Target::Player(PlayerId(who))], &registry);

    let milled = cards.min(1);
    let player = &state.players[who as usize];
    assert!(player.library_order.len() == cards - milled && player.graveyard_order.len() == milled && !player.has_drawn_from_empty);
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k + milled == cards {
            assert!(o.zone == Zone::Graveyard && player.graveyard_order[0] == id);
        } else {
            assert!(o.zone == Zone::Library && player.library_order[k] == id);
        }
    }
    let made = usize::from(creature && cards > 0);
    assert!(state.objects.len() == before + made);
    if made == 1 {
        let id = ObjectId(door.0 + 1);
        let t = state.get_object(id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == PlayerId(0) && t.owner == PlayerId(0));
        assert!(state.is_creature(id, &registry));
        assert!(state.effective_power(id, &registry) == Some(2) && state.effective_toughness(id, &registry) == Some(2));
        assert!(t.colors == [Color::Black] && t.subtypes.len() == 1 && t.subtypes[0] == "Zombie");
    }
    std::mem::forget((registry, state));
}

/// Ludevic's Test Subject ("{1}{U}: Put a hatchling counter on this
/// creature. Then if there are five or more hatchling counters on it,
/// remove all of them and transform it.") with `had` hatchling counters,
/// its ability resolving: short of five it has one more and shows the
/// face it showed; at five it has none and shows its other face,
/// Ludevic's Abomination, whose power and toughness the engine then
/// reads (CR 122.1, 701.28a, 712.8).
fn ludevics_test_subject(had: u32) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::ludevics_test_subject::LudevicsTestSubject));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let subject = state.create_object(card, you, Zone::Battlefield, power, toughness);
    // One entry either way (`gutter_grime` says why).
    state.get_object_mut(subject).expect("just created").counters.insert(CounterType::Hatchling, had);

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, subject, 0, &[], &registry);

    let hatched = had + 1 >= 5;
    let o = state.get_object(subject).expect("still an object");
    assert!(o.counters.get(&CounterType::Hatchling).copied().unwrap_or(0) == if hatched { 0 } else { had + 1 });
    assert!(o.is_transformed == hatched && o.zone == Zone::Battlefield && o.controller == you);
    let face = registry.printed_pt(card, hatched).expect("that face");
    assert!(state.effective_power(subject, &registry) == face.0 && state.effective_toughness(subject, &registry) == face.1);
    std::mem::forget((registry, state));
}

/// Full Moon's Rise ("Sacrifice this enchantment: Regenerate all Werewolf
/// creatures you control.") the first player's, its ability resolving
/// beside a Werewolf of theirs, a Werewolf of the other player's and a
/// creature of theirs that is no Werewolf: their Werewolf has one
/// regeneration shield more (CR 701.19a), and neither of the others has.
fn full_moons_rise(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::full_moons_rise::FullMoonsRise));
    let mut a = |controller: u8, werewolf: bool| {
        let id = a_permanent(&mut state, blank, Permanent::Creature);
        let o = state.get_object_mut(id).expect("just created");
        o.controller = PlayerId(controller);
        if werewolf {
            o.subtypes = vec!["Werewolf".to_string()];
        }
        id
    };
    let (yours, theirs, other) = (a(0, true), a(1, true), a(0, false));
    let rise = a_source(&mut state, card, PlayerId(0));

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, rise, 0, &[], &registry);

    let shields = |id| state.get_object(id).expect("still an object").regeneration_shields;
    assert!(shields(yours) == 1 && shields(theirs) == 0 && shields(other) == 0);
    std::mem::forget((registry, state));
}

/// Grimgrin, Corpse-Born ("Sacrifice another creature: Untap Grimgrin and
/// put a +1/+1 counter on it.") tapped or not, on the battlefield or
/// gone from it by then (`there`): there, it is untapped (CR 701.21b)
/// with one +1/+1 counter more; gone, the card in the graveyard has no
/// counter (121.1).
fn grimgrin_untaps(there: bool) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::grimgrin_corpse_born::GrimgrinCorpseBorn));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let grimgrin = a_creature_of(&mut state, card, PlayerId(1), you);
    state.get_object_mut(grimgrin).expect("just created").tapped = kani::any();
    if !there {
        state.move_object(grimgrin, Zone::Graveyard, &registry);
    }

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, grimgrin, 0, &[], &registry);

    let o = state.get_object(grimgrin).expect("still an object");
    let counters = o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    if there {
        assert!(o.zone == Zone::Battlefield && !o.tapped && counters == 1 && o.controller == you);
        assert!(state.effective_power(grimgrin, &registry) == Some(3) && state.effective_toughness(grimgrin, &registry) == Some(3));
    } else {
        assert!(o.zone == Zone::Graveyard && counters == 0);
    }
    std::mem::forget((registry, state));
}

/// Moorland Haunt ("{W}{U}, {T}, Exile a creature card from your
/// graveyard: Create a 1/1 white Spirit creature token with flying.")
/// under either player, its ability resolving: one new permanent, a
/// token, a 1/1 white Spirit creature with flying under its controller
/// (CR 111.2).
fn moorland_haunt(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::moorland_haunt::MoorlandHaunt));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let haunt = a_source(&mut state, card, PlayerId(1));
    state.get_object_mut(haunt).expect("just created").controller = you;
    let before = state.objects.len();

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, haunt, 0, &[], &registry);

    assert!(state.objects.len() == before + 1);
    let id = ObjectId(haunt.0 + 1);
    let t = state.get_object(id).expect("made");
    assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
    assert!(state.is_creature(id, &registry) && state.has_keyword(id, Keyword::Flying, &registry));
    assert!(state.effective_power(id, &registry) == Some(1) && state.effective_toughness(id, &registry) == Some(1));
    assert!(t.colors == [Color::White] && t.subtypes.len() == 1 && t.subtypes[0] == "Spirit");
    std::mem::forget((registry, state));
}

/// Evil Twin's "{U}{B}, {T}: Destroy target creature with the same name
/// as this creature." resolving at a creature: the creature is put into
/// its owner's graveyard by the engine's own destruction (CR 701.7a),
/// unless it is indestructible and nothing happens to it, or it has a
/// regeneration shield, which is used instead (701.15a).
fn evil_twin_destroys(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::evil_twin::EvilTwin));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(target).expect("just created").regeneration_shields = u32::from(kani::any::<bool>());
    // One effect either way (`destroys` says why).
    let indestructible: bool = kani::any();
    let keyword = if indestructible { Keyword::Indestructible } else { Keyword::Flying };
    state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target, keyword });
    let twin = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let shield = state.get_object(target).expect("just created").regeneration_shields > 0;

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, twin, 0, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    if indestructible {
        assert!(o.zone == Zone::Battlefield && o.regeneration_shields == u32::from(shield));
    } else if shield {
        assert!(o.zone == Zone::Battlefield && o.tapped && o.damage_marked == 0 && o.regeneration_shields == 0);
    } else {
        assert!(o.zone == Zone::Graveyard && state.players[1].graveyard_order.contains(&target));
    }
    std::mem::forget((registry, state));
}

// ---- Permanents: more damage, a coin, a random choice. ----

/// Balefire Dragon ("Whenever this creature deals combat damage to a
/// player, it deals that much damage to each creature that player
/// controls.") having dealt 6 to the second player, beside one creature,
/// that player's (`theirs`) or its own controller's: the engine's own
/// damage marks 6 more on that player's creature (CR 120.3e) and none on
/// the other's, and each is otherwise as it was. One creature, as for
/// Rolling Temblor: a second permanent is past what fits.
fn balefire_dragon(theirs: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::balefire_dragon::BalefireDragon));
    let creature = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(creature).expect("just created").controller = PlayerId(u8::from(theirs));
    let dragon = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let before = state.get_object(creature).expect("just created");
    let (tapped, damage) = (before.tapped, before.damage_marked);

    registry.get(card).expect("registered").on_combat_damage_to_player(&mut state, dragon, PlayerId(1), 6, &registry);

    let o = state.get_object(creature).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped == tapped && o.damage_marked == damage + if theirs { 6 } else { 0 });
    assert!(state.get_object(dragon).expect("still an object").damage_marked == 0);
    std::mem::forget((registry, state));
}

/// Burning Vengeance ("Whenever you cast a spell from your graveyard,
/// this enchantment deals 2 damage to any target.") its trigger
/// resolving at a creature or a planeswalker (`at` which), or, with
/// none, at either player: 2 more damage marked on the creature (CR
/// 120.3e), two loyalty counters gone from the planeswalker's three
/// (120.3c), or 2 life from that player (120.3a), and nothing else.
fn burning_vengeance(at: Option<Permanent>) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::burning_vengeance::BurningVengeance));
    let permanent = at.map(|what| a_permanent(&mut state, blank, what));
    let source = a_source(&mut state, card, PlayerId(0));
    let spell = state.create_object(blank, PlayerId(0), Zone::Stack, None, None);
    let player = u8::from(kani::any::<bool>());
    let target = permanent.map_or(Target::Player(PlayerId(player)), Target::Object);
    let damage = permanent.map_or(0, |id| state.get_object(id).expect("just created").damage_marked);
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").on_spell_cast(&mut state, source, PlayerId(0), spell, &[target], &registry);

    match (at, permanent) {
        (Some(Permanent::Planeswalker), Some(id)) => {
            let o = state.get_object(id).expect("still an object");
            assert!(o.damage_marked == 0 && o.counters.get(&CounterType::Loyalty).copied().unwrap_or(0) == 1);
            assert!(state.players[0].life == life[0] && state.players[1].life == life[1]);
        }
        (_, Some(id)) => {
            let o = state.get_object(id).expect("still an object");
            assert!(o.zone == Zone::Battlefield && o.damage_marked == damage + 2);
            assert!(state.players[0].life == life[0] && state.players[1].life == life[1]);
        }
        (_, None) => {
            for p in 0..2u8 {
                assert!(state.players[p as usize].life == life[p as usize] - if p == player { 2 } else { 0 });
            }
        }
    }
    std::mem::forget((registry, state));
}

/// Nightfall Predator, Daybreak Ranger's other face ("{R}, {T}: This
/// creature fights target creature."), its ability resolving at a 2/2:
/// the engine's own fight, each dealing damage equal to its power to the
/// other (CR 701.14a): 4 more marked on the target, the Predator's
/// printed power as its transformed face is read, and 2 on the Predator.
fn nightfall_predator_fights(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::daybreak_ranger::DaybreakRanger));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let predator = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    state.get_object_mut(predator).expect("just created").is_transformed = true;
    assert!(state.effective_power(predator, &registry) == Some(4));
    let damage = state.get_object(target).expect("just created").damage_marked;

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, predator, 1, &[Target::Object(target)], &registry);

    assert!(state.get_object(target).expect("still an object").damage_marked == damage + 4);
    assert!(state.get_object(predator).expect("still an object").damage_marked == 2);
    std::mem::forget((registry, state));
}

/// Olivia Voldaren ("{1}{R}: Olivia Voldaren deals 1 damage to another
/// target creature. That creature becomes a Vampire in addition to its
/// other types. Put a +1/+1 counter on Olivia Voldaren.") its first
/// ability resolving at a creature: 1 more damage marked on it (CR
/// 120.3e), Vampire among its subtypes as the engine reads them
/// (205.1b), and one +1/+1 counter on Olivia.
fn olivia_voldaren_bites(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::olivia_voldaren::OliviaVoldaren));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let olivia = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let before = state.get_object(target).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);
    assert!(!state.has_subtype(target, "Vampire", &registry));

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, olivia, 0, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == damage + 1 && o.tapped == tapped && o.controller == controller);
    assert!(state.has_subtype(target, "Vampire", &registry));
    let v = state.get_object(olivia).expect("still an object");
    assert!(v.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0) == 1 && v.damage_marked == 0);
    std::mem::forget((registry, state));
}

/// Creepy Doll ("Whenever this creature deals combat damage to a
/// creature, flip a coin. If you win the flip, destroy that creature.")
/// its trigger resolving for a creature, the game's random numbers in
/// any state: the creature is in its owner's graveyard by the engine's
/// own destruction (CR 701.7a) or on the battlefield as it was, and each
/// is a way the coin can fall (705.2).
fn creepy_doll(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::creepy_doll::CreepyDoll));
    let creature = a_permanent(&mut state, blank, Permanent::Creature);
    let doll = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    state.rng_state = kani::any();
    let before = state.get_object(creature).expect("just created");
    let (tapped, damage, controller) = (before.tapped, before.damage_marked, before.controller);

    registry.get(card).expect("registered").on_deals_combat_damage_to_creature(&mut state, doll, creature, 1, &registry);

    let o = state.get_object(creature).expect("still an object");
    if o.zone == Zone::Battlefield {
        assert!(o.tapped == tapped && o.damage_marked == damage && o.controller == controller);
    } else {
        assert!(o.zone == Zone::Graveyard && o.controller == PlayerId(1) && state.players[1].graveyard_order.contains(&creature));
    }
    kani::cover!(o.zone == Zone::Graveyard, "the flip won");
    kani::cover!(o.zone == Zone::Battlefield, "the flip lost");
    std::mem::forget((registry, state));
}

/// Woodland Sleuth ("Morbid — When this creature enters, if a creature
/// died this turn, return a creature card at random from your graveyard
/// to your hand.") entered under the first player with two creature
/// cards and one other card in their graveyard, a creature having died
/// this turn or not (`died`), the game's random numbers at `seed`: with
/// one dead, exactly one of the two creature cards is in their hand and
/// the other where it was (CR 701.9c), the `pick`th as that seed has it;
/// with none, nothing moves (603.4). The card that is no creature stays
/// either way.
///
/// A seed for each of the two, not the random numbers left open: open,
/// the card moved is one the solver leaves open, and the harness ran out
/// of memory at 7 GB. That a choice at random is one of the candidates,
/// whatever the numbers, is proved of the engine
/// (`proofs::actions::random`); Creepy Doll's coin above is proved with
/// them open.
fn woodland_sleuth((died, seed, pick): (bool, u64, usize)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::woodland_sleuth::WoodlandSleuth));
    let other = a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    let creatures = [a_card_in(&mut state, blank, 0, Zone::Graveyard, true), a_card_in(&mut state, blank, 0, Zone::Graveyard, true)];
    for id in creatures {
        state.get_object_mut(id).expect("just created").card_types = vec![CardType::Creature];
    }
    let sleuth = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    state.creature_died_this_turn = died;
    state.rng_state = seed;

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, sleuth, &[], &registry);

    let zone = |id| state.get_object(id).expect("still an object").zone;
    assert!(zone(other) == Zone::Graveyard);
    if died {
        assert!(zone(creatures[pick]) == Zone::Hand && zone(creatures[1 - pick]) == Zone::Graveyard);
        assert!(state.players[0].graveyard_order.len() == 2);
    } else {
        assert!(zone(creatures[0]) == Zone::Graveyard && zone(creatures[1]) == Zone::Graveyard);
        assert!(state.players[0].graveyard_order.len() == 3);
    }
    std::mem::forget((registry, state));
}

// ---- Permanents: exile and return, control, and cards read from a
// library or a graveyard by what they are. ----

/// Fiend Hunter ("When this creature enters, you may exile another
/// target creature. When this creature leaves the battlefield, return
/// the exiled card to the battlefield under its owner's control.")
/// entered, its trigger resolving at a creature: its controller is
/// asked, and the effect the question holds, applied by the engine,
/// puts the creature in exile (CR 406.1), the Hunter still on the
/// battlefield.
///
/// What the Hunter holds afterwards is read in a harness of its own,
/// and from the card's hook called as the engine calls it rather than
/// through the engine's dispatch of the effect. Measured: this harness
/// asked that as well ran out of memory at 7 GB, and so did the effect
/// applied through the dispatch on a board with nothing open, room
/// given to the Hunter's list first or not; the hook called directly
/// and then read proves in 82 s. The likely reason, not measured
/// further: the dispatch picks the card's behaviour out of the registry
/// by an id read back from the object, and to the solver what the card
/// wrote is from then on written by either card of the registry or by
/// neither.
fn fiend_hunter(_: ()) {
    use crate::state::PendingEffect;
    let (registry, state, target, _) = may_do_to_its_target(Box::new(crate::cards::isd::fiend_hunter::FiendHunter), Permanent::Creature,
        &|effect, source, _| matches!(effect, PendingEffect::CardEffect { source_id, key } if *source_id == source && key.is_empty()),
        &|source, _| PendingEffect::CardEffect { source_id: source, key: String::new() });
    let hunter = ObjectId(target.0 + 1);
    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Exile && o.owner == PlayerId(1));
    assert!(state.get_object(hunter).expect("still an object").zone == Zone::Battlefield);
    std::mem::forget((registry, state));
}

/// The card's own hook for that effect, at the second player's creature
/// for a Hunter on the battlefield: the creature is in exile, and the
/// Hunter holds it, and nothing else, as the card it exiled (CR
/// 607.2a).
fn fiend_hunter_holds(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::fiend_hunter::FiendHunter));
    let target = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    let hunter = a_source(&mut state, card, PlayerId(0));

    registry.get(card).expect("registered").resolve_card_effect(&mut state, hunter, "", &Target::Object(target), &registry);

    assert!(state.get_object(target).expect("still an object").zone == Zone::Exile);
    let h = state.get_object(hunter).expect("still an object");
    assert!(h.zone == Zone::Battlefield && h.card_state.len() == 1 && h.card_state.get(HUNTED) == Some(&target));
    std::mem::forget((registry, state));
}

/// The whole of it on a board with nothing open, the card's own hooks
/// one after another: its effect at the second player's creature, the
/// Hunter then put into its graveyard by the engine's own zone change,
/// and its second ability resolving. The creature card is in exile
/// between, and at the end on the battlefield under its owner's
/// control, untapped and undamaged (CR 610.3, 400.7).
fn fiend_hunter_exiles_and_returns(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::fiend_hunter::FiendHunter));
    let target = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    let hunter = a_source(&mut state, card, PlayerId(0));

    let behavior = registry.get(card).expect("registered");
    behavior.resolve_card_effect(&mut state, hunter, "", &Target::Object(target), &registry);
    assert!(state.get_object(target).expect("still an object").zone == Zone::Exile);
    state.move_object(hunter, Zone::Graveyard, &registry);
    behavior.on_leave_battlefield(&mut state, hunter, &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.controller == PlayerId(1) && o.owner == PlayerId(1) && !o.tapped && o.damage_marked == 0);
    assert!(state.get_object(hunter).expect("still an object").zone == Zone::Graveyard);
    std::mem::forget((registry, state));
}

/// The key Fiend Hunter keeps the card it exiled under, on its own
/// object.
const HUNTED: &str = "exiled_creature";

/// Fiend Hunter the first player's, a creature card of the second
/// player's in exile, the Hunter holding it as the card it exiled or
/// not (`holds`: what `fiend_hunter_holds` reads, or a Hunter that
/// exiled nothing). The Hunter is put into its graveyard by
/// the engine's own zone change and its second ability resolves: the
/// card it exiled is on the battlefield under its owner's control,
/// untapped and undamaged, a new object as it enters (CR 610.3, 400.7);
/// a card it did not exile stays in exile.
fn fiend_hunter_leaves(holds: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::fiend_hunter::FiendHunter));
    let target = state.create_object(blank, PlayerId(1), Zone::Exile, Some(2), Some(2));
    state.get_object_mut(target).expect("just created").card_types = vec![CardType::Creature];
    let hunter = a_source(&mut state, card, PlayerId(0));
    if holds {
        state.get_object_mut(hunter).expect("just created").card_state.insert(HUNTED.to_string(), target);
    }

    state.move_object(hunter, Zone::Graveyard, &registry);
    registry.get(card).expect("registered").on_leave_battlefield(&mut state, hunter, &registry);

    let o = state.get_object(target).expect("still an object");
    if holds {
        assert!(o.zone == Zone::Battlefield && o.controller == PlayerId(1) && o.owner == PlayerId(1) && !o.tapped && o.damage_marked == 0);
    } else {
        assert!(o.zone == Zone::Exile && o.owner == PlayerId(1));
    }
    assert!(state.get_object(hunter).expect("still an object").zone == Zone::Graveyard);
    std::mem::forget((registry, state));
}

/// Olivia Voldaren ("{3}{B}{B}: Gain control of target Vampire for as
/// long as you control Olivia Voldaren.") the first player's, its second
/// ability resolving at the second player's creature, a Vampire or not
/// (`vampire`): the first player controls a Vampire from then on (CR
/// 613.1b), and a creature that is no Vampire by then stays its
/// controller's.
fn olivia_voldaren_takes(vampire: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::olivia_voldaren::OliviaVoldaren));
    let target = a_permanent(&mut state, blank, Permanent::Creature);
    let o = state.get_object_mut(target).expect("just created");
    o.controller = PlayerId(1);
    if vampire {
        o.subtypes = vec!["Vampire".to_string()];
    }
    let olivia = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, olivia, 1, &[Target::Object(target)], &registry);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.controller == PlayerId(u8::from(!vampire)) && o.owner == PlayerId(1));
    assert!(state.get_object(olivia).expect("still an object").controller == PlayerId(0));
    std::mem::forget((registry, state));
}

/// Witchbane Orb ("When this artifact enters, destroy all Curses attached
/// to you.") entered under the first player, a Curse attached to each
/// player: the one attached to them is in its owner's graveyard by the
/// engine's own destruction (CR 701.7a), and the one attached to the
/// other player is where it was.
fn witchbane_orb(_: ()) {
    let (mut registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::witchbane_orb::WitchbaneOrb));
    let curse = registry.register(Box::new(crate::cards::isd::curse_of_the_bloody_tome::CurseOfTheBloodyTome));
    let mut a_curse_on = |player: u8| {
        let id = state.create_object(curse, PlayerId(1), Zone::Battlefield, None, None);
        state.get_object_mut(id).expect("just created").attached_to_player = Some(PlayerId(player));
        id
    };
    let (yours, theirs) = (a_curse_on(0), a_curse_on(1));
    let orb = a_source(&mut state, card, PlayerId(0));

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, orb, &[], &registry);

    let y = state.get_object(yours).expect("still an object");
    assert!(y.zone == Zone::Graveyard && y.attached_to_player.is_none() && state.players[1].graveyard_order.contains(&yours));
    let t = state.get_object(theirs).expect("still an object");
    assert!(t.zone == Zone::Battlefield && t.attached_to_player == Some(PlayerId(1)));
    assert!(state.get_object(orb).expect("still an object").zone == Zone::Battlefield);
    std::mem::forget((registry, state));
}

/// Trepanation Blade ("Whenever equipped creature attacks, defending
/// player reveals cards from the top of their library until they reveal
/// a land card. The creature gets +1/+0 until end of turn for each card
/// revealed this way. That player puts the revealed cards into their
/// graveyard.") its trigger resolving for the first player's creature
/// attacking the second, whose library is `cards` cards, the `land`th of
/// them a land if any is: the cards down to and with the land, or all of
/// them with none, are in that player's graveyard in the order they
/// lay (CR 701.17a), the rest their library as it was, and the attacker
/// has 1 more power for each, its toughness as it was (613.4c).
fn trepanation_blade((cards, land): (usize, Option<usize>)) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::trepanation_blade::TrepanationBlade));
    let forest = registry.register(Box::new(crate::cards::forest::Forest));
    let mut library = [None; 4];
    for (k, slot) in library.iter_mut().enumerate().take(cards) {
        let id = state.create_object(if land == Some(k) { forest } else { blank }, PlayerId(1), Zone::Library, None, None);
        state.players[1].library_order.push(id);
        *slot = Some(id);
    }
    let attacker = a_permanent(&mut state, blank, Permanent::Creature);
    state.get_object_mut(attacker).expect("just created").controller = PlayerId(0);
    let blade = a_source(&mut state, card, PlayerId(0));
    state.attach(blade, attacker);
    let (power, toughness) = (state.effective_power(attacker, &registry), state.effective_toughness(attacker, &registry));

    registry.get(card).expect("registered").on_attacks(&mut state, blade, crate::cards::AttackInfo::new(attacker, PlayerId(1)), &[], &registry);

    let revealed = land.map_or(cards, |k| k + 1);
    let player = &state.players[1];
    assert!(player.library_order.len() == cards - revealed && player.graveyard_order.len() == revealed && !player.has_drawn_from_empty);
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k < revealed {
            assert!(o.zone == Zone::Graveyard && player.graveyard_order[k] == id);
        } else {
            assert!(o.zone == Zone::Library && player.library_order[k - revealed] == id);
        }
    }
    let up = i32::try_from(revealed).expect("a few");
    assert!(state.effective_power(attacker, &registry) == power.map(|p| p + up) && state.effective_toughness(attacker, &registry) == toughness);
    std::mem::forget((registry, state));
}

/// A card of one kind returned at random from its controller's graveyard
/// to their hand: Ghoulraiser's enters trigger ("return a Zombie card at
/// random") and Charmbreaker Devils' upkeep trigger ("return an instant
/// or sorcery card at random"), each for the first player with two cards
/// of that kind (`kind`'s, a card the registry prints as one) and one
/// that is not in their graveyard, the game's random numbers at `seed`:
/// exactly one of the two is in their hand, the `pick`th as that seed
/// has it (CR 701.9c), and the other two cards are where they were.
/// `woodland_sleuth` says why a seed and not the numbers left open.
fn returns_one_at_random((card, kind, upkeep, seed, pick): (Box<dyn CardBehavior>, Box<dyn CardBehavior>, bool, u64, usize)) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let kind = registry.register(kind);
    let other = a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    let two = [a_card_in(&mut state, kind, 0, Zone::Graveyard, false), a_card_in(&mut state, kind, 0, Zone::Graveyard, false)];
    let source = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    state.rng_state = seed;

    let behavior = registry.get(card).expect("registered");
    if upkeep {
        behavior.on_upkeep(&mut state, source, &[], &registry);
    } else {
        behavior.on_enter_battlefield(&mut state, source, &[], &registry);
    }

    let zone = |id| state.get_object(id).expect("still an object").zone;
    assert!(zone(two[pick]) == Zone::Hand && zone(two[1 - pick]) == Zone::Graveyard && zone(other) == Zone::Graveyard);
    assert!(state.players[0].graveyard_order.len() == 2);
    std::mem::forget((registry, state));
}

/// Mindshrieker ("{2}: Target player mills a card. This creature gets
/// +X/+X until end of turn, where X is the milled card's mana value.")
/// its ability resolving at the second player, the top of whose library
/// is a card of mana value 2 (Doom Blade) over another: that card is in
/// their graveyard (CR 701.17a), the other their library, and the
/// Mindshrieker has 2 more power and toughness, read back through the
/// engine's layers (202.3, 613.4c).
fn mindshrieker(_: ()) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::mindshrieker::Mindshrieker));
    let blade = registry.register(Box::new(DoomBlade));
    let top = state.create_object(blade, PlayerId(1), Zone::Library, None, None);
    let under = state.create_object(blank, PlayerId(1), Zone::Library, None, None);
    state.players[1].library_order.push(top);
    state.players[1].library_order.push(under);
    let shrieker = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, shrieker, 0, &[Target::Player(PlayerId(1))], &registry);

    assert!(state.get_object(top).expect("still an object").zone == Zone::Graveyard && state.players[1].graveyard_order == [top]);
    assert!(state.get_object(under).expect("still an object").zone == Zone::Library && state.players[1].library_order == [under]);
    assert!(state.effective_power(shrieker, &registry) == Some(4) && state.effective_toughness(shrieker, &registry) == Some(4));
    std::mem::forget((registry, state));
}

/// Heretic's Punishment ("{3}{R}: Choose any target, then mill three
/// cards. This enchantment deals damage to that permanent or player
/// equal to the greatest mana value among the milled cards.") the first
/// player's, its ability resolving at either player, the top of its
/// controller's library a card of mana value 1 (Lightning Bolt), one of
/// 2 (Doom Blade), a card with no cost and a fourth: the top three are
/// in their graveyard in order (CR 701.17a), the fourth is their
/// library, and the player chosen has 2 life fewer (202.3, 120.3a).
fn heretics_punishment(_: ()) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::heretics_punishment::HereticsPunishment));
    let (bolt, blade) = (registry.register(Box::new(LightningBolt)), registry.register(Box::new(DoomBlade)));
    let mut library = [None; 4];
    for (slot, of) in library.iter_mut().zip([bolt, blade, blank, blank]) {
        let id = state.create_object(of, PlayerId(0), Zone::Library, None, None);
        state.players[0].library_order.push(id);
        *slot = Some(id);
    }
    let punishment = a_source(&mut state, card, PlayerId(0));
    let at = u8::from(kani::any::<bool>());
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, punishment, 0, &[Target::Player(PlayerId(at))], &registry);

    let player = &state.players[0];
    assert!(player.library_order.len() == 1 && player.graveyard_order.len() == 3);
    for k in 0..4 {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k < 3 {
            assert!(o.zone == Zone::Graveyard && player.graveyard_order[k] == id);
        } else {
            assert!(o.zone == Zone::Library && player.library_order[0] == id);
        }
    }
    for p in 0..2u8 {
        assert!(state.players[p as usize].life == life[p as usize] - if p == at { 2 } else { 0 });
    }
    std::mem::forget((registry, state));
}

// ---- Permanents: a card looked at, picked or discarded as the hook
// resolves. Where a player is asked, the question is read where it
// lies and the effect it holds applied by the engine
// (`engine::apply_pending_effect`), as the engine's dispatch of an
// answer does (`proofs::actions::answering`). ----

/// Delver of Secrets ("At the beginning of your upkeep, look at the top
/// card of your library. You may reveal that card. If an instant or
/// sorcery card is revealed this way, transform this creature.") the
/// first player's at their upkeep, the top card of their library an
/// instant (`instant`: Lightning Bolt) or a card that is neither, over
/// another: they are asked whether to reveal it, and answering yes with
/// an instant there the Delver shows its other face, whose power and
/// toughness the engine then reads (CR 701.28a, 712.8); answering no,
/// or yes with another card, it is as it was. The library is as it was
/// either way: revealing a card moves nothing (701.16a).
fn delver_of_secrets((instant, yes): (bool, bool)) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::delver_of_secrets::DelverOfSecrets));
    let bolt = registry.register(Box::new(LightningBolt));
    let top = state.create_object(if instant { bolt } else { blank }, PlayerId(0), Zone::Library, None, None);
    let under = state.create_object(blank, PlayerId(0), Zone::Library, None, None);
    state.players[0].library_order.push(top);
    state.players[0].library_order.push(under);
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let delver = state.create_object(card, PlayerId(0), Zone::Battlefield, power, toughness);

    let behavior = registry.get(card).expect("registered");
    behavior.on_upkeep(&mut state, delver, &[], &registry);
    asked_yes_or_no(&mut state, delver, PlayerId(0));
    behavior.on_yes_no_choice(&mut state, delver, yes, &registry);

    let shows_back = instant && yes;
    let o = state.get_object(delver).expect("still an object");
    assert!(o.is_transformed == shows_back && o.zone == Zone::Battlefield && o.controller == PlayerId(0));
    state.get_object_mut(delver).expect("still an object").is_transformed = shows_back;
    let face = registry.printed_pt(card, shows_back).expect("that face");
    assert!(state.effective_power(delver, &registry) == face.0 && state.effective_toughness(delver, &registry) == face.1);
    assert!(state.players[0].library_order == [top, under] && state.awaiting_action.is_none());
    std::mem::forget((registry, state));
}

/// The same trigger with no card to look at (`gone` false: an empty
/// library) or for a Delver put into its graveyard since (`gone`):
/// nothing is asked, and nothing is done to it (CR 400.7).
fn delver_of_secrets_asks_nothing(gone: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::delver_of_secrets::DelverOfSecrets));
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let delver = state.create_object(card, PlayerId(0), Zone::Battlefield, power, toughness);
    if gone {
        let top = state.create_object(blank, PlayerId(0), Zone::Library, None, None);
        state.players[0].library_order.push(top);
        state.move_object(delver, Zone::Graveyard, &registry);
    }

    registry.get(card).expect("registered").on_upkeep(&mut state, delver, &[], &registry);

    assert!(state.awaiting_action.is_none());
    let o = state.get_object(delver).expect("still an object");
    assert!(!o.is_transformed && o.zone == if gone { Zone::Graveyard } else { Zone::Battlefield });
    std::mem::forget((registry, state));
}

/// Curse of Oblivion ("At the beginning of enchanted player's upkeep,
/// that player exiles two cards from their graveyard.") attached to the
/// second player, `cards` cards in their graveyard and one in the other
/// player's: with two or fewer, each is in exile by the engine's own
/// zone change and nothing is asked (CR 406.1); with three, the
/// enchanted player is asked for exactly two of the three, and the
/// effect the question holds, applied to each of two (`spared` is the
/// third), exiles them. The other player's card is where it was.
fn curse_of_oblivion((cards, spared): (usize, usize)) {
    use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(CurseOfOblivion));
    let mut theirs = [None; 3];
    for slot in theirs.iter_mut().take(cards) {
        *slot = Some(a_card_in(&mut state, blank, 1, Zone::Graveyard, false));
    }
    let yours = a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    let curse = a_source(&mut state, card, PlayerId(0));
    state.get_object_mut(curse).expect("just created").attached_to_player = Some(PlayerId(1));

    registry.get(card).expect("registered").on_upkeep(&mut state, curse, &[], &registry);

    if cards == 3 {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseObjectSet { options, min, max, effect, .. }, .. }) => {
                assert!(*player == PlayerId(1) && *min == 2 && *max == 2 && options.len() == 3);
                for k in 0..3 {
                    assert!(Some(options[k]) == theirs[k]);
                }
                assert!(matches!(effect, PendingEffect::CardEffect { source_id, key } if *source_id == curse && key.is_empty()));
            }
            _ => panic!("the enchanted player is asked which two"),
        }
        crate::proofs::cards::clear_prompt(&mut state);
        let effect = PendingEffect::CardEffect { source_id: curse, key: String::new() };
        for k in 0..3 {
            if k != spared {
                crate::engine::apply_pending_effect(&mut state, &Target::Object(theirs[k].expect("made")), &effect, &registry);
            }
        }
        std::mem::forget(effect);
    } else {
        assert!(state.awaiting_action.is_none());
    }
    for k in 0..cards {
        let o = state.get_object(theirs[k].expect("made")).expect("still an object");
        assert!(o.zone == if cards == 3 && k == spared { Zone::Graveyard } else { Zone::Exile } && o.owner == PlayerId(1));
    }
    assert!(state.players[1].graveyard_order.len() == usize::from(cards == 3));
    assert!(state.get_object(yours).expect("still an object").zone == Zone::Graveyard && state.players[0].graveyard_order == [yours]);
    std::mem::forget((registry, state));
}

/// Graveyard Shovel ("{2}, {T}: Target player exiles a card from their
/// graveyard. If it's a creature card, you gain 2 life.") the first
/// player's, its ability resolving at the second player, `cards` cards
/// in their graveyard, the first a creature card or not (`creature`)
/// and a second that is not one: with none nothing happens; with one it
/// is exiled with nothing asked; with two that player is asked which,
/// with no way to decline, and the effect the question holds exiles the
/// one chosen (`pick`, CR 406.1). The Shovel's controller gains 2 life
/// exactly when the card exiled is a creature card (119.3), and the
/// other card stays.
fn graveyard_shovel((cards, creature, pick): (usize, bool, usize)) {
    use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::graveyard_shovel::GraveyardShovel));
    let mut theirs = [None; 2];
    for (k, slot) in theirs.iter_mut().enumerate().take(cards) {
        *slot = Some(a_card_in(&mut state, blank, 1, Zone::Graveyard, creature && k == 0));
    }
    let shovel = a_source(&mut state, card, PlayerId(0));
    let life = [state.players[0].life, state.players[1].life];

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, shovel, 0, &[Target::Player(PlayerId(1))], &registry);

    if cards == 2 {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. }) => {
                assert!(*player == PlayerId(1) && !*optional && options.len() == 2);
                for k in 0..2 {
                    assert!(options[k] == Target::Object(theirs[k].expect("made")));
                }
                assert!(matches!(effect, PendingEffect::CardEffect { source_id, key } if *source_id == shovel && key.is_empty()));
            }
            _ => panic!("that player is asked which card"),
        }
        crate::proofs::cards::clear_prompt(&mut state);
        let effect = PendingEffect::CardEffect { source_id: shovel, key: String::new() };
        crate::engine::apply_pending_effect(&mut state, &Target::Object(theirs[pick].expect("made")), &effect, &registry);
        std::mem::forget(effect);
    } else {
        assert!(state.awaiting_action.is_none());
    }
    for k in 0..cards {
        let o = state.get_object(theirs[k].expect("made")).expect("still an object");
        assert!(o.zone == if k == pick { Zone::Exile } else { Zone::Graveyard } && o.owner == PlayerId(1));
    }
    let gained = if cards > 0 && pick == 0 && creature { 2 } else { 0 };
    assert!(state.players[0].life == life[0] + gained && state.players[1].life == life[1]);
    std::mem::forget((registry, state));
}

/// Civilized Scholar ("{T}: Draw a card, then discard a card. If a
/// creature card is discarded this way, untap this creature, then
/// transform it.") the first player's and tapped for its cost, its
/// ability resolving with an empty hand and one card in their library,
/// a creature card (`creature`: Doomed Traveler) or one that is not: the
/// card is drawn and, the only card in hand, discarded with nothing
/// asked, so that it is in their graveyard (CR 121.1, 701.8a); a
/// creature card discarded, the Scholar is untapped and shows its other
/// face, whose power and toughness the engine then reads (701.28a);
/// another, it stays tapped and as it was.
fn civilized_scholar(creature: bool) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::civilized_scholar::CivilizedScholar));
    let traveler = registry.register(Box::new(crate::cards::isd::doomed_traveler::DoomedTraveler));
    let drawn = state.create_object(if creature { traveler } else { blank }, PlayerId(0), Zone::Library, None, None);
    state.players[0].library_order.push(drawn);
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let scholar = state.create_object(card, PlayerId(0), Zone::Battlefield, power, toughness);
    state.get_object_mut(scholar).expect("just created").tapped = true;

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, scholar, 0, &[], &registry);

    assert!(state.awaiting_action.is_none());
    assert!(state.get_object(drawn).expect("still an object").zone == Zone::Graveyard);
    assert!(state.players[0].graveyard_order == [drawn] && state.players[0].library_order.is_empty() && !state.players[0].has_drawn_from_empty);
    let o = state.get_object(scholar).expect("still an object");
    assert!(o.is_transformed == creature && o.tapped != creature && o.zone == Zone::Battlefield && o.controller == PlayerId(0));
    state.get_object_mut(scholar).expect("still an object").is_transformed = creature;
    let face = registry.printed_pt(card, creature).expect("that face");
    assert!(state.effective_power(scholar, &registry) == face.0 && state.effective_toughness(scholar, &registry) == face.1);
    std::mem::forget((registry, state));
}

/// The same with a card in hand already: its controller is asked which
/// of the two to discard, one card and at once (CR 701.8a), the two
/// offered being their hand.
fn civilized_scholar_asks(_: ()) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::civilized_scholar::CivilizedScholar));
    let held = a_card_in(&mut state, blank, 0, Zone::Hand, false);
    let drawn = state.create_object(blank, PlayerId(0), Zone::Library, None, None);
    state.players[0].library_order.push(drawn);
    let scholar = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    state.get_object_mut(scholar).expect("just created").tapped = true;

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, scholar, 0, &[], &registry);

    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player: asked, source, choice: ResolutionChoiceKind::ChooseCardFromHand { player, cards, discard_immediately, remaining, .. } }) => {
            assert!(*asked == PlayerId(0) && *player == PlayerId(0) && *source == scholar);
            assert!(cards.len() == 2 && cards[0] == held && cards[1] == drawn && *discard_immediately && *remaining == 1);
        }
        _ => panic!("its controller is asked which card"),
    }
    assert!(state.get_object(drawn).expect("still an object").zone == Zone::Hand && state.get_object(held).expect("still an object").zone == Zone::Hand);
    let o = state.get_object(scholar).expect("still an object");
    assert!(!o.is_transformed && o.tapped);
    std::mem::forget((registry, state));
}

/// The Scholar told which card was discarded for it, as the engine
/// tells it once the question is answered (`engine::notify_discard`): a
/// creature card (`creature`: Doomed Traveler) and it is untapped and
/// shows its other face (CR 701.28a); another and it is as it was.
fn civilized_scholar_is_told(creature: bool) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::civilized_scholar::CivilizedScholar));
    let traveler = registry.register(Box::new(crate::cards::isd::doomed_traveler::DoomedTraveler));
    let discarded = a_card_in(&mut state, if creature { traveler } else { blank }, 0, Zone::Graveyard, false);
    let scholar = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    state.get_object_mut(scholar).expect("just created").tapped = true;

    crate::engine::notify_discard(&mut state, scholar, discarded, &registry);

    let o = state.get_object(scholar).expect("still an object");
    assert!(o.is_transformed == creature && o.tapped != creature && o.zone == Zone::Battlefield);
    std::mem::forget((registry, state));
}

/// Homicidal Brute ("At the beginning of your end step, if this creature
/// didn't attack this turn, tap this creature, then transform it."),
/// the Scholar's other face, under either player at their end step: one
/// that did not attack this turn is tapped and shows its front face
/// again, whose power and toughness the engine then reads (CR 701.28a);
/// one that attacked is as it was (603.4).
fn homicidal_brute(attacked: bool) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::civilized_scholar::CivilizedScholar));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let brute = state.create_object(card, you, Zone::Battlefield, power, toughness);
    let turn = state.turn_number;
    let o = state.get_object_mut(brute).expect("just created");
    o.is_transformed = true;
    o.tapped = attacked;
    if attacked {
        o.attacked_on_turn = Some(turn);
    }

    registry.get(card).expect("registered").on_end_step(&mut state, brute, &[], &registry);

    let o = state.get_object(brute).expect("still an object");
    assert!(o.is_transformed == attacked && o.tapped && o.zone == Zone::Battlefield && o.controller == you);
    state.get_object_mut(brute).expect("still an object").is_transformed = attacked;
    let face = registry.printed_pt(card, attacked).expect("that face");
    assert!(state.effective_power(brute, &registry) == face.0 && state.effective_toughness(brute, &registry) == face.1);
    std::mem::forget((registry, state));
}

/// Stitcher's Apprentice ("{1}{U}, {T}: Create a 2/2 blue Homunculus
/// creature token, then sacrifice a creature.") the first player's, its
/// ability resolving: a 2/2 blue Homunculus creature token is theirs
/// (CR 111.2), and they are asked which of their creatures to
/// sacrifice, the Apprentice or the token, with no way to decline; the
/// effect the question holds puts the one chosen (`the_token`, or the
/// Apprentice) into its owner's graveyard (701.21a), and the other
/// stays.
fn stitchers_apprentice(the_token: bool) {
    use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::stitchers_apprentice::StitchersApprentice));
    let apprentice = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    let name = registry.printed_name(card, false).expect("its face").to_string();

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, apprentice, 0, &[], &registry);

    let token = ObjectId(apprentice.0 + 1);
    let t = state.get_object(token).expect("made");
    assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == PlayerId(0) && t.owner == PlayerId(0));
    assert!(t.colors == [Color::Blue] && t.subtypes.len() == 1 && t.subtypes[0] == "Homunculus");
    assert!(state.is_creature(token, &registry));
    assert!(state.effective_power(token, &registry) == Some(2) && state.effective_toughness(token, &registry) == Some(2));
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. }) => {
            assert!(*player == PlayerId(0) && !*optional);
            assert!(options.len() == 2 && options[0] == Target::Object(apprentice) && options[1] == Target::Object(token));
            assert!(matches!(effect, PendingEffect::SacrificeCreature { source_name } if *source_name == name));
        }
        _ => panic!("its controller is asked which creature"),
    }
    crate::proofs::cards::clear_prompt(&mut state);
    let effect = PendingEffect::SacrificeCreature { source_name: name };
    let (gone, stays) = if the_token { (token, apprentice) } else { (apprentice, token) };
    crate::engine::apply_pending_effect(&mut state, &Target::Object(gone), &effect, &registry);

    assert!(state.get_object(gone).expect("still an object").zone == Zone::Graveyard);
    assert!(state.get_object(stays).expect("still an object").zone == Zone::Battlefield);
    std::mem::forget((registry, state, effect));
}

/// Snapcaster Mage ("When this creature enters, target instant or
/// sorcery card in your graveyard gains flashback until end of turn.
/// The flashback cost is equal to its mana cost.") entered under the
/// first player, its trigger resolving at a Lightning Bolt in their
/// graveyard, a second beside it: the engine reads one flashback cost
/// granted to the first, {R} as the card is printed (CR 702.34a), and
/// none to the second; both cards are where they were.
fn snapcaster_mage(_: ()) {
    use crate::types::{ManaCost, ManaSymbol};
    let (mut registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::snapcaster_mage::SnapcasterMage));
    let bolt = registry.register(Box::new(LightningBolt));
    let target = a_card_in(&mut state, bolt, 0, Zone::Graveyard, false);
    let beside = a_card_in(&mut state, bolt, 0, Zone::Graveyard, false);
    let mage = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, mage, &[Target::Object(target)], &registry);

    let printed = ManaCost::new(vec![ManaSymbol::Colored(Color::Red)]);
    let granted = crate::engine::legal::casting::granted_flashback_costs(&state, target);
    assert!(granted.len() == 1 && *granted[0] == printed);
    std::mem::forget(granted);
    let none = crate::engine::legal::casting::granted_flashback_costs(&state, beside);
    assert!(none.is_empty());
    std::mem::forget(none);
    assert!(state.players[0].graveyard_order == [target, beside]);
    std::mem::forget((printed, registry, state));
}

/// Skeletal Grimace ("Enchanted creature gets +1/+1 and has '{B}:
/// Regenerate this creature.'") attached to a creature under either
/// player, the ability it gives resolving for that creature: the
/// creature has one regeneration shield more than before (CR 701.19a),
/// and the Aura none.
fn skeletal_grimace_shields(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(SkeletalGrimace));
    let creature = a_permanent(&mut state, blank, Permanent::Creature);
    let aura = a_source(&mut state, card, PlayerId(0));
    state.get_object_mut(aura).expect("just created").attached_to = Some(creature);
    let had = u32::from(kani::any::<bool>());
    state.get_object_mut(creature).expect("just created").regeneration_shields = had;

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, creature, 0, &[], &registry);

    assert!(state.get_object(creature).expect("still an object").regeneration_shields == had + 1);
    assert!(state.get_object(aura).expect("still an object").regeneration_shields == 0);
    std::mem::forget((registry, state));
}

/// Moldgraf Monstrosity ("When this creature dies, exile it, then return
/// two creature cards at random from your graveyard to the
/// battlefield.") dead in the first player's graveyard beside
/// `creatures` creature cards (two at most, so that which are picked is
/// not in question) and one card that is no creature, the game's random
/// numbers at a seed: the Monstrosity is in exile (CR 406.1), each
/// creature card is on the battlefield under that player, untapped
/// (110.2), and the other card is where it was.
fn moldgraf_monstrosity(creatures: usize) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::moldgraf_monstrosity::MoldgrafMonstrosity));
    let other = a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    let mut back = [None; 2];
    for slot in back.iter_mut().take(creatures) {
        *slot = Some(a_card_in(&mut state, blank, 0, Zone::Graveyard, true));
    }
    let monstrosity = a_card_in(&mut state, card, 0, Zone::Graveyard, true);
    state.rng_state = SEED_FIRST;

    registry.get(card).expect("registered").on_dies(&mut state, monstrosity, &[], &registry);

    assert!(state.get_object(monstrosity).expect("still an object").zone == Zone::Exile);
    for k in 0..creatures {
        let o = state.get_object(back[k].expect("made")).expect("still an object");
        assert!(o.zone == Zone::Battlefield && o.controller == PlayerId(0) && o.owner == PlayerId(0) && !o.tapped);
    }
    assert!(state.get_object(other).expect("still an object").zone == Zone::Graveyard && state.players[0].graveyard_order == [other]);
    std::mem::forget((registry, state));
}

/// Blazing Torch ("Equipped creature has '{T}, Sacrifice Blazing Torch:
/// Blazing Torch deals 2 damage to any target.'") attached to the first
/// player's creature, the cost of the ability it gives paid and the
/// ability resolving at the second player or, `at_a_creature`, at their
/// creature: the Torch is in its owner's graveyard by the engine's own
/// sacrifice as the cost is paid (CR 701.21a, 602.2b), and then the
/// player has 2 life fewer or the creature 2 damage more (120.3), the
/// equipped creature as it was.
fn blazing_torch_sacrificed(at_a_creature: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::blazing_torch::BlazingTorch));
    let bearer = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let theirs = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    state.get_object_mut(theirs).expect("just created").toughness = Some(3);
    let torch = a_source(&mut state, card, PlayerId(0));
    state.get_object_mut(torch).expect("just created").attached_to = Some(bearer);
    let life = [state.players[0].life, state.players[1].life];

    let behavior = registry.get(card).expect("registered");
    behavior.pay_activation_cost(&mut state, bearer, 1, &[], &registry);
    assert!(state.get_object(torch).expect("still an object").zone == Zone::Graveyard && state.players[0].graveyard_order == [torch]);
    let at = if at_a_creature { Target::Object(theirs) } else { Target::Player(PlayerId(1)) };
    behavior.resolve_activated_ability(&mut state, bearer, 1, &[at], &registry);

    assert!(state.players[0].life == life[0] && state.players[1].life == life[1] - if at_a_creature { 0 } else { 2 });
    let o = state.get_object(theirs).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == if at_a_creature { 2 } else { 0 });
    let b = state.get_object(bearer).expect("still an object");
    assert!(b.zone == Zone::Battlefield && b.damage_marked == 0);
    std::mem::forget((registry, state));
}

/// Brain Weevil ("Sacrifice this creature: Target player discards two
/// cards. Activate only as a sorcery.") sacrificed for its cost, its
/// ability resolving at the second player with `cards` cards in hand:
/// with two or fewer, each is in their graveyard with nothing asked (CR
/// 701.8a); with three, that player is asked which of their hand to
/// discard, two to go, and nothing has left it yet.
fn brain_weevil(cards: usize) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::brain_weevil::BrainWeevil));
    let mut hand = [None; 3];
    for slot in hand.iter_mut().take(cards) {
        *slot = Some(a_card_in(&mut state, blank, 1, Zone::Hand, false));
    }
    let weevil = a_card_in(&mut state, card, 0, Zone::Graveyard, true);

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, weevil, 0, &[Target::Player(PlayerId(1))], &registry);

    if cards == 3 {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player: asked, source, choice: ResolutionChoiceKind::ChooseCardFromHand { player, cards: offered, discard_immediately, remaining, .. } }) => {
                assert!(*asked == PlayerId(1) && *player == PlayerId(1) && *source == weevil);
                assert!(offered.len() == 3 && *discard_immediately && *remaining == 2);
                for k in 0..3 {
                    assert!(Some(offered[k]) == hand[k]);
                }
            }
            _ => panic!("that player is asked which cards"),
        }
    } else {
        assert!(state.awaiting_action.is_none());
    }
    for k in 0..cards {
        let o = state.get_object(hand[k].expect("made")).expect("still an object");
        assert!(o.zone == if cards == 3 { Zone::Hand } else { Zone::Graveyard } && o.owner == PlayerId(1));
    }
    assert!(state.players[1].graveyard_order.len() == if cards == 3 { 0 } else { cards });
    std::mem::forget((registry, state));
}

// ---- Spells that take cards by what they are, or at random. ----

/// Mulch ("Reveal the top four cards of your library. Put all land cards
/// revealed this way into your hand and the rest into your graveyard.")
/// resolving for the first player, their library of `cards` cards, of
/// the top four of which `lands` names the land cards (Forest): each
/// land card of the four is in their hand, each other card of them in
/// their graveyard in the order it was revealed (CR 701.16a, 701.17a),
/// and a fifth card is still their library. Milling is not drawing
/// from an empty library (704.5b).
fn mulch((cards, lands): (usize, [bool; 4])) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::mulch::Mulch));
    let forest = registry.register(Box::new(crate::cards::forest::Forest));
    let mut library = [None; 5];
    for (k, slot) in library.iter_mut().enumerate().take(cards) {
        let of = if k < 4 && lands[k] { forest } else { blank };
        let id = state.create_object(of, PlayerId(0), Zone::Library, None, None);
        state.players[0].library_order.push(id);
        *slot = Some(id);
    }
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    let player = &state.players[0];
    let mut milled = 0;
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k >= 4 {
            assert!(o.zone == Zone::Library && player.library_order.len() == 1 && player.library_order[0] == id);
        } else if lands[k] {
            assert!(o.zone == Zone::Hand && o.owner == PlayerId(0));
        } else {
            assert!(o.zone == Zone::Graveyard && player.graveyard_order[milled] == id);
            milled += 1;
        }
    }
    assert!(player.graveyard_order.len() == milled && !player.has_drawn_from_empty);
    assert!(cards > 4 || player.library_order.is_empty());
    std::mem::forget((registry, state));
}

/// Past in Flames ("Each instant and sorcery card in your graveyard
/// gains flashback until end of turn. The flashback cost is equal to its
/// mana cost.") resolving for the first player, a Lightning Bolt and a
/// card that is neither kind in their graveyard and a Lightning Bolt in
/// the other player's: the engine reads one flashback cost granted to
/// their Bolt, {R} as the card is printed (CR 702.34a), and none to the
/// other two.
fn past_in_flames(_: ()) {
    use crate::types::{ManaCost, ManaSymbol};
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::past_in_flames::PastInFlames));
    let bolt = registry.register(Box::new(LightningBolt));
    let yours = a_card_in(&mut state, bolt, 0, Zone::Graveyard, false);
    let other = a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    let theirs = a_card_in(&mut state, bolt, 1, Zone::Graveyard, false);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    let printed = ManaCost::new(vec![ManaSymbol::Colored(Color::Red)]);
    let granted = crate::engine::legal::casting::granted_flashback_costs(&state, yours);
    assert!(granted.len() == 1 && *granted[0] == printed);
    std::mem::forget(granted);
    for id in [other, theirs] {
        let none = crate::engine::legal::casting::granted_flashback_costs(&state, id);
        assert!(none.is_empty());
        std::mem::forget(none);
    }
    assert!(state.until_end_of_turn.len() == 1);
    std::mem::forget((printed, registry, state));
}

/// Make a Wish ("Return two cards at random from your graveyard to your
/// hand.") resolving for the first player with `cards` cards in their
/// graveyard and one in the other player's, the game's random numbers
/// at a seed: two of theirs are in their hand, or all of them if there
/// are fewer, the rest still their graveyard, and the other player's
/// card is where it was. `woodland_sleuth` says why a seed.
fn make_a_wish(cards: usize) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::make_a_wish::MakeAWish));
    let mut yours = [None; 3];
    for slot in yours.iter_mut().take(cards) {
        *slot = Some(a_card_in(&mut state, blank, 0, Zone::Graveyard, false));
    }
    let theirs = a_card_in(&mut state, blank, 1, Zone::Graveyard, false);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.rng_state = SEED_FIRST;

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    let mut returned = 0;
    for k in 0..cards {
        let o = state.get_object(yours[k].expect("made")).expect("still an object");
        assert!((o.zone == Zone::Hand || o.zone == Zone::Graveyard) && o.owner == PlayerId(0));
        if o.zone == Zone::Hand {
            returned += 1;
        }
    }
    assert!(returned == cards.min(2) && state.players[0].graveyard_order.len() == cards - returned);
    assert!(state.get_object(theirs).expect("still an object").zone == Zone::Graveyard && state.players[1].graveyard_order == [theirs]);
    std::mem::forget((registry, state));
}

/// Desperate Ravings ("Draw two cards, then discard a card at random.")
/// resolving for the first player with an empty hand and a library of
/// three, the game's random numbers at `seed`: the top two cards are
/// drawn (CR 121.1), and then one of the two, the `pick`th as that seed
/// has it, is in their graveyard (701.8a) and the other in their hand,
/// the third card still their library.
fn desperate_ravings((seed, pick): (u64, usize)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::desperate_ravings::DesperateRavings));
    let library = a_library(&mut state, blank, 0, 3);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.rng_state = seed;

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    let zone = |k: usize| state.get_object(library[k].expect("made")).expect("still an object").zone;
    assert!(zone(pick) == Zone::Graveyard && zone(1 - pick) == Zone::Hand && zone(2) == Zone::Library);
    let player = &state.players[0];
    assert!(player.graveyard_order.len() == 1 && player.library_order.len() == 1 && !player.has_drawn_from_empty);
    std::mem::forget((registry, state));
}

/// Moonmist ("Transform all Humans. Prevent all combat damage that would
/// be dealt this turn by creatures other than Werewolves and Wolves.")
/// resolving beside a double-faced Human (Village Ironsmith), a
/// double-faced creature that is no Human (Ludevic's Test Subject) and
/// a Human with one face: the first shows its other face, whose power
/// and toughness the engine then reads, and the other two are as they
/// were (CR 701.28a); and the turn holds one effect, the prevention
/// the text names with the two kinds of creature it leaves out (615.1).
fn moonmist(_: ()) {
    use crate::types::CreatureFilter;
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::moonmist::Moonmist));
    let ironsmith = registry.register(Box::new(crate::cards::isd::village_ironsmith::VillageIronsmith));
    let subject = registry.register(Box::new(crate::cards::isd::ludevics_test_subject::LudevicsTestSubject));
    let (power, toughness) = registry.printed_pt(ironsmith, false).expect("its front face");
    let human = state.create_object(ironsmith, PlayerId(0), Zone::Battlefield, power, toughness);
    let (power, toughness) = registry.printed_pt(subject, false).expect("its front face");
    let lizard = state.create_object(subject, PlayerId(1), Zone::Battlefield, power, toughness);
    let one_faced = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    state.get_object_mut(one_faced).expect("just created").subtypes = vec!["Human".to_string()];
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    let shows = |id| state.get_object(id).expect("still an object").is_transformed;
    assert!(shows(human) && !shows(lizard) && !shows(one_faced));
    assert!(state.until_end_of_turn.len() == 1);
    match &state.until_end_of_turn[0] {
        TemporaryEffect::PreventCombatDamageExcept { filter: CreatureFilter::Or(kinds), .. } => {
            assert!(kinds.len() == 2);
            assert!(matches!(&kinds[0], CreatureFilter::HasSubtype(s) if s == "Wolf"));
            assert!(matches!(&kinds[1], CreatureFilter::HasSubtype(s) if s == "Werewolf"));
        }
        _ => panic!("the prevention the text names"),
    }
    state.get_object_mut(human).expect("still an object").is_transformed = true;
    let face = registry.printed_pt(ironsmith, true).expect("its other face");
    assert!(state.effective_power(human, &registry) == face.0 && state.effective_toughness(human, &registry) == face.1);
    std::mem::forget((registry, state));
}

/// Creeping Renaissance ("Choose a permanent type. Return all cards of
/// the chosen type from your graveyard to your hand.") resolving for the
/// first player, a creature card and a land card (Forest) in their
/// graveyard and a card of the type chosen in the other player's: they
/// are asked for one of the five permanent types (CR 110.4), and told
/// the answer (`land`, or creature) the spell puts their card of that
/// type into their hand and leaves the other two where they were.
fn creeping_renaissance(land: bool) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::creeping_renaissance::CreepingRenaissance));
    let forest = registry.register(Box::new(crate::cards::forest::Forest));
    let creature = a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
    let a_land = a_card_in(&mut state, forest, 0, Zone::Graveyard, false);
    let theirs = a_card_in(&mut state, if land { forest } else { blank }, 1, Zone::Graveyard, !land);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    let behavior = registry.get(card).expect("registered");
    behavior.on_resolve(&mut state, spell, &[], &registry);
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, source, choice: ResolutionChoiceKind::ChooseCardType { options, controller, .. } }) => {
            assert!(*player == PlayerId(0) && *controller == PlayerId(0) && *source == spell && options.len() == 5);
            assert!(options[0] == "Creature" && options[1] == "Artifact" && options[2] == "Enchantment");
            assert!(options[3] == "Land" && options[4] == "Planeswalker");
        }
        _ => panic!("its controller is asked for a permanent type"),
    }
    crate::proofs::cards::clear_prompt(&mut state);
    behavior.on_card_type_choice(&mut state, spell, if land { "Land" } else { "Creature" }, &registry);

    let (back, stays) = if land { (a_land, creature) } else { (creature, a_land) };
    let o = state.get_object(back).expect("still an object");
    assert!(o.zone == Zone::Hand && o.owner == PlayerId(0));
    assert!(state.get_object(stays).expect("still an object").zone == Zone::Graveyard && state.players[0].graveyard_order == [stays]);
    assert!(state.get_object(theirs).expect("still an object").zone == Zone::Graveyard && state.players[1].graveyard_order == [theirs]);
    std::mem::forget((registry, state));
}

/// Night Terrors ("Target player reveals their hand. You choose a
/// nonland card from it. Exile that card.") resolving for the first
/// player at the second, whose hand is a land card (Forest) and
/// `others` cards that are not lands: with none, nothing is exiled;
/// with one, it is exiled with nothing asked; with two, the spell's
/// controller is asked which, with no way to decline and the land not
/// offered, and the effect the question holds exiles the one chosen
/// (`pick`, CR 406.1). The land and the card not chosen stay in hand.
fn night_terrors((others, pick): (usize, usize)) {
    use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::night_terrors::NightTerrors));
    let forest = registry.register(Box::new(crate::cards::forest::Forest));
    let land = a_card_in(&mut state, forest, 1, Zone::Hand, false);
    let mut theirs = [None; 2];
    for slot in theirs.iter_mut().take(others) {
        *slot = Some(a_card_in(&mut state, blank, 1, Zone::Hand, false));
    }
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Player(PlayerId(1))], &registry);

    if others == 2 {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. }) => {
                assert!(*player == PlayerId(0) && !*optional && options.len() == 2);
                for k in 0..2 {
                    assert!(options[k] == Target::Object(theirs[k].expect("made")));
                }
                assert!(matches!(effect, PendingEffect::CardEffect { source_id, key } if *source_id == spell && key.is_empty()));
            }
            _ => panic!("the spell's controller is asked which card"),
        }
        crate::proofs::cards::clear_prompt(&mut state);
        let effect = PendingEffect::CardEffect { source_id: spell, key: String::new() };
        crate::engine::apply_pending_effect(&mut state, &Target::Object(theirs[pick].expect("made")), &effect, &registry);
        std::mem::forget(effect);
    } else {
        assert!(state.awaiting_action.is_none());
    }
    for k in 0..others {
        let o = state.get_object(theirs[k].expect("made")).expect("still an object");
        assert!(o.zone == if k == pick { Zone::Exile } else { Zone::Hand } && o.owner == PlayerId(1));
    }
    assert!(state.get_object(land).expect("still an object").zone == Zone::Hand);
    std::mem::forget((registry, state));
}

/// Forbidden Alchemy ("Look at the top four cards of your library. Put
/// one of them into your hand and the rest into your graveyard.")
/// resolving for the first player with `cards` cards in their library:
/// with none nothing happens; with one it is put into their hand with
/// nothing asked; with five they are asked which of the top four, in
/// the order they lie, the fifth not among them, and nothing has moved
/// yet (the answer's effect is the engine's, `proofs::actions`).
fn forbidden_alchemy(cards: usize) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::forbidden_alchemy::ForbiddenAlchemy));
    let mut library = [None; 5];
    for slot in library.iter_mut().take(cards) {
        let id = state.create_object(blank, PlayerId(0), Zone::Library, None, None);
        state.players[0].library_order.push(id);
        *slot = Some(id);
    }
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    if cards > 1 {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player, source, choice: ResolutionChoiceKind::ChooseFromLookedAt { looked_at, .. } }) => {
                assert!(*player == PlayerId(0) && *source == spell && looked_at.len() == cards.min(4));
                for k in 0..cards.min(4) {
                    assert!(Some(looked_at[k]) == library[k]);
                }
            }
            _ => panic!("its controller is asked which card"),
        }
        assert!(state.players[0].library_order.len() == cards && state.players[0].graveyard_order.is_empty());
    } else {
        assert!(state.awaiting_action.is_none());
        if cards == 1 {
            let o = state.get_object(library[0].expect("made")).expect("still an object");
            assert!(o.zone == Zone::Hand && o.owner == PlayerId(0) && state.players[0].library_order.is_empty());
        }
        assert!(state.players[0].graveyard_order.is_empty() && !state.players[0].has_drawn_from_empty);
    }
    std::mem::forget((registry, state));
}

/// Memory's Journey ("Target player shuffles up to three target cards
/// from their graveyard into their library.") resolving at the second
/// player and two of the three cards in their graveyard, their library
/// of one card, the game's random numbers at a seed: the two are cards
/// of their library, three cards now with the one that was there, and
/// the third card is still their graveyard.
fn memorys_journey(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::memorys_journey::MemorysJourney));
    let library = a_library(&mut state, blank, 1, 1);
    let two = [a_card_in(&mut state, blank, 1, Zone::Graveyard, false), a_card_in(&mut state, blank, 1, Zone::Graveyard, false)];
    let stays = a_card_in(&mut state, blank, 1, Zone::Graveyard, false);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.rng_state = SEED_FIRST;

    registry.get(card).expect("registered").on_resolve(&mut state, spell,
        &[Target::Player(PlayerId(1)), Target::Object(two[0]), Target::Object(two[1])], &registry);

    let player = &state.players[1];
    assert!(player.library_order.len() == 3 && player.graveyard_order.len() == 1 && player.graveyard_order[0] == stays);
    for id in [two[0], two[1], library[0].expect("made")] {
        assert!(state.get_object(id).expect("still an object").zone == Zone::Library);
        assert!(player.library_order[0] == id || player.library_order[1] == id || player.library_order[2] == id);
    }
    assert!(state.get_object(stays).expect("still an object").zone == Zone::Graveyard);
    std::mem::forget((registry, state));
}

// ---- Cards that search a library, and the last of the spells. A
// library searched is shuffled, at a seed (`woodland_sleuth`). ----

/// Three cards in the first player's library, the middle one a Forest
/// if `forest`: the three, top first.
fn a_library_to_search(state: &mut GameState, blank: CardId, land: CardId, who: u8, forest: bool) -> [ObjectId; 3] {
    let mut made = [ObjectId(0); 3];
    for (k, slot) in made.iter_mut().enumerate() {
        let id = state.create_object(if forest && k == 1 { land } else { blank }, PlayerId(who), Zone::Library, None, None);
        if who == 0 {
            state.players[0].library_order.push(id);
        } else {
            state.players[1].library_order.push(id);
        }
        *slot = id;
    }
    made
}

/// Traveler's Amulet ("{1}, Sacrifice this artifact: Search your library
/// for a basic land card, reveal it, put it into your hand, then
/// shuffle.") sacrificed for its cost, its ability resolving for the
/// first player, whose library is three cards, the middle one a Forest
/// or not (`forest`): with a Forest there they are asked which basic
/// land card, the Forest the one offered and their hand where it goes,
/// and the search finished as the engine finishes it for the card
/// chosen (`helpers::finish_library_search`) puts it in their hand, the
/// other two still their library (CR 701.19a); with none nothing is
/// asked and the three are still their library.
fn travelers_amulet(forest: bool) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::travelers_amulet::TravelersAmulet));
    let land = registry.register(Box::new(crate::cards::forest::Forest));
    let library = a_library_to_search(&mut state, blank, land, 0, forest);
    let amulet = a_card_in(&mut state, card, 0, Zone::Graveyard, false);
    state.rng_state = SEED_FIRST;

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, amulet, 0, &[], &registry);

    if forest {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseFromLibrary { options, searcher, source_id, destination, tapped, .. }, .. }) => {
                assert!(*player == PlayerId(0) && *searcher == PlayerId(0) && *source_id == amulet);
                assert!(options.len() == 1 && options[0] == library[1] && *destination == Zone::Hand && !*tapped);
            }
            _ => panic!("its controller is asked which basic land card"),
        }
        crate::proofs::cards::clear_prompt(&mut state);
        crate::cards::helpers::finish_library_search(&mut state, PlayerId(0), library[1], Zone::Hand, false, &registry);
    } else {
        assert!(state.awaiting_action.is_none());
    }
    let zone = |k: usize| state.get_object(library[k]).expect("still an object").zone;
    assert!(zone(0) == Zone::Library && zone(2) == Zone::Library && zone(1) == if forest { Zone::Hand } else { Zone::Library });
    assert!(state.players[0].library_order.len() == if forest { 2 } else { 3 });
    std::mem::forget((registry, state));
}

/// Ghost Quarter ("{T}, Sacrifice this land: Destroy target land. Its
/// controller may search their library for a basic land card, put it
/// onto the battlefield, then shuffle.") sacrificed for its cost, its
/// ability resolving at the second player's land, whose library is three
/// cards, the middle one a Forest or not (`forest`): the land is in its
/// owner's graveyard by the engine's own destruction (CR 701.7a); with
/// a Forest in the library its controller is asked, free to decline,
/// and the effect the question holds puts the Forest onto the
/// battlefield under them, untapped, the other two still their library
/// (701.19a); with none nothing is asked.
fn ghost_quarter(forest: bool) {
    use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::ghost_quarter::GhostQuarter));
    let land = registry.register(Box::new(crate::cards::forest::Forest));
    let library = a_library_to_search(&mut state, blank, land, 1, forest);
    let target = state.create_object(blank, PlayerId(1), Zone::Battlefield, None, None);
    state.get_object_mut(target).expect("just created").card_types = vec![CardType::Land];
    let quarter = a_card_in(&mut state, card, 0, Zone::Graveyard, false);
    state.rng_state = SEED_FIRST;

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, quarter, 0, &[Target::Object(target)], &registry);

    assert!(state.get_object(target).expect("still an object").zone == Zone::Graveyard && state.players[1].graveyard_order == [target]);
    if forest {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. }) => {
                assert!(*player == PlayerId(1) && *optional && options.len() == 1 && options[0] == Target::Object(library[1]));
                assert!(matches!(effect, PendingEffect::FinishLibrarySearch { searcher, destination, tapped }
                    if *searcher == PlayerId(1) && *destination == Zone::Battlefield && !*tapped));
            }
            _ => panic!("the land's controller is asked whether to search"),
        }
        crate::proofs::cards::clear_prompt(&mut state);
        let effect = PendingEffect::FinishLibrarySearch { searcher: PlayerId(1), destination: Zone::Battlefield, tapped: false };
        crate::engine::apply_pending_effect(&mut state, &Target::Object(library[1]), &effect, &registry);
        std::mem::forget(effect);
        let o = state.get_object(library[1]).expect("still an object");
        assert!(o.zone == Zone::Battlefield && o.controller == PlayerId(1) && !o.tapped);
    } else {
        assert!(state.awaiting_action.is_none() && state.get_object(library[1]).expect("still an object").zone == Zone::Library);
    }
    let zone = |k: usize| state.get_object(library[k]).expect("still an object").zone;
    assert!(zone(0) == Zone::Library && zone(2) == Zone::Library);
    assert!(state.players[1].library_order.len() == if forest { 2 } else { 3 });
    std::mem::forget((registry, state));
}

/// Caravan Vigil ("Search your library for a basic land card, reveal it,
/// put it into your hand, then shuffle. Morbid: You may put that card
/// onto the battlefield instead of putting it into your hand if a
/// creature died this turn.") resolving for the first player, whose
/// library is three cards, the middle one a Forest: they are asked which
/// basic land card, free to find none, and the effect the question holds
/// puts the Forest into their hand if no creature died this turn (CR
/// 701.19a); if one did (`morbid`), they are asked which of the two
/// places, and told the answer (`onto_the_battlefield`) the spell puts
/// it there, under them, or into their hand. The other two cards are
/// still their library. With morbid the effect is the card's own hook
/// called as the engine calls it: applied through the engine's dispatch
/// and then read by the next hook, the card it keeps went past 4 GB
/// (`fiend_hunter` says why).
fn caravan_vigil((morbid, onto_the_battlefield): (bool, bool)) {
    use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::caravan_vigil::CaravanVigil));
    let land = registry.register(Box::new(crate::cards::forest::Forest));
    let library = a_library_to_search(&mut state, blank, land, 0, true);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.creature_died_this_turn = morbid;
    state.rng_state = SEED_FIRST;

    let behavior = registry.get(card).expect("registered");
    behavior.on_resolve(&mut state, spell, &[], &registry);
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. }) => {
            assert!(*player == PlayerId(0) && *optional && options.len() == 1 && options[0] == Target::Object(library[1]));
            assert!(matches!(effect, PendingEffect::CardEffect { source_id, key } if *source_id == spell && key.is_empty()));
        }
        _ => panic!("its controller is asked which basic land card"),
    }
    crate::proofs::cards::clear_prompt(&mut state);
    if morbid {
        behavior.resolve_card_effect(&mut state, spell, "", &Target::Object(library[1]), &registry);
        asked_yes_or_no(&mut state, spell, PlayerId(0));
        behavior.on_yes_no_choice(&mut state, spell, onto_the_battlefield, &registry);
    } else {
        let effect = PendingEffect::CardEffect { source_id: spell, key: String::new() };
        crate::engine::apply_pending_effect(&mut state, &Target::Object(library[1]), &effect, &registry);
        std::mem::forget(effect);
    }

    assert!(state.awaiting_action.is_none());
    let o = state.get_object(library[1]).expect("still an object");
    assert!(o.zone == if morbid && onto_the_battlefield { Zone::Battlefield } else { Zone::Hand } && o.controller == PlayerId(0));
    let zone = |k: usize| state.get_object(library[k]).expect("still an object").zone;
    assert!(zone(0) == Zone::Library && zone(2) == Zone::Library && state.players[0].library_order.len() == 2);
    std::mem::forget((registry, state));
}

/// The same with no basic land card in their library: nothing is asked,
/// and the three cards are still their library.
fn caravan_vigil_finds_nothing(_: ()) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::caravan_vigil::CaravanVigil));
    let land = registry.register(Box::new(crate::cards::forest::Forest));
    let library = a_library_to_search(&mut state, blank, land, 0, false);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.rng_state = SEED_FIRST;

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    assert!(state.awaiting_action.is_none() && state.players[0].library_order.len() == 3);
    for id in library {
        assert!(state.get_object(id).expect("still an object").zone == Zone::Library);
    }
    std::mem::forget((registry, state));
}

/// Bitterheart Witch ("When this creature dies, you may search your
/// library for a Curse card, put it onto the battlefield attached to
/// target player, then shuffle.") dead, the first player's, its trigger
/// resolving at the second player, a Curse (Curse of the Bloody Tome)
/// the middle card of three in its controller's library: they are asked
/// whether to search; answering yes they are asked which Curse card,
/// free to find none, and the effect the question holds puts the Curse
/// onto the battlefield attached to the player targeted,
/// the other two cards still their library; answering no nothing moves.
fn bitterheart_witch(yes: bool) {
    use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::bitterheart_witch::BitterheartWitch));
    let curse = registry.register(Box::new(CurseOfTheBloodyTome));
    let library = a_library_to_search(&mut state, blank, curse, 0, true);
    let witch = a_card_in(&mut state, card, 0, Zone::Graveyard, true);
    state.get_object_mut(witch).expect("just created").card_state.reserve(2);
    state.rng_state = SEED_FIRST;

    let behavior = registry.get(card).expect("registered");
    behavior.on_dies(&mut state, witch, &[Target::Player(PlayerId(1))], &registry);
    asked_yes_or_no(&mut state, witch, PlayerId(0));
    behavior.on_yes_no_choice(&mut state, witch, yes, &registry);

    if yes {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. }) => {
                assert!(*player == PlayerId(0) && *optional && options.len() == 1 && options[0] == Target::Object(library[1]));
                assert!(matches!(effect, PendingEffect::CardEffect { source_id, key } if *source_id == witch && key == "choose"));
            }
            _ => panic!("its controller is asked which Curse card"),
        }
        crate::proofs::cards::clear_prompt(&mut state);
        let effect = PendingEffect::CardEffect { source_id: witch, key: "choose".to_string() };
        crate::engine::apply_pending_effect(&mut state, &Target::Object(library[1]), &effect, &registry);
        std::mem::forget(effect);
        let o = state.get_object(library[1]).expect("still an object");
        assert!(o.zone == Zone::Battlefield && o.attached_to_player == Some(PlayerId(1)) && o.controller == PlayerId(0));
    } else {
        assert!(state.awaiting_action.is_none() && state.get_object(library[1]).expect("still an object").zone == Zone::Library);
    }
    let zone = |k: usize| state.get_object(library[k]).expect("still an object").zone;
    assert!(zone(0) == Zone::Library && zone(2) == Zone::Library);
    assert!(state.players[0].library_order.len() == if yes { 2 } else { 3 });
    std::mem::forget((registry, state));
}

/// Mirror-Mad Phantasm ("{1}{U}: This creature's owner shuffles it into
/// their library. If that player does, they reveal cards from the top of
/// that library until a card named Mirror-Mad Phantasm is revealed. That
/// player puts that card onto the battlefield and all other cards
/// revealed this way into their graveyard.") the first player's, its
/// ability resolving with two other cards in their library, the game's
/// random numbers at `seed`: the Phantasm is on the battlefield under
/// its owner again, and each of the other two is in their graveyard, if
/// the shuffle put it above the Phantasm, or still their library.
fn mirror_mad_phantasm(seed: u64) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::mirror_mad_phantasm::MirrorMadPhantasm));
    let library = a_library(&mut state, blank, 0, 2);
    let phantasm = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    state.get_object_mut(phantasm).expect("just created").name = registry.printed_name(card, false).expect("its face").to_string();
    state.rng_state = seed;

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, phantasm, 0, &[], &registry);

    let o = state.get_object(phantasm).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.controller == PlayerId(0) && o.owner == PlayerId(0));
    let player = &state.players[0];
    assert!(player.graveyard_order.len() + player.library_order.len() == 2);
    for k in 0..2 {
        let zone = state.get_object(library[k].expect("made")).expect("still an object").zone;
        assert!(zone == Zone::Graveyard || zone == Zone::Library);
    }
    std::mem::forget((registry, state));
}

/// Frightful Delusion ("Counter target spell unless its controller pays
/// {1}. That player discards a card.") resolving for the first player at
/// the second player's spell on the stack: that spell's controller is
/// asked whether to pay {1} for it; and told their answer,
/// whichever it was, the spell makes them discard: with `cards` cards in
/// hand, none is nothing, one is discarded with nothing asked (701.8a),
/// and with two they are asked which. The countering is the engine's,
/// as it takes the answer (`proofs::actions`).
fn frightful_delusion(cards: usize) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    use crate::types::{ManaCost, ManaSymbol};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::frightful_delusion::FrightfulDelusion));
    let mut hand = [None; 2];
    for slot in hand.iter_mut().take(cards) {
        *slot = Some(a_card_in(&mut state, blank, 1, Zone::Hand, false));
    }
    let theirs = state.create_object(blank, PlayerId(1), Zone::Stack, None, None);
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    let behavior = registry.get(card).expect("registered");
    behavior.on_resolve(&mut state, spell, &[Target::Object(theirs)], &registry);
    let one = ManaCost::new(vec![ManaSymbol::Generic(1)]);
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, source, choice: ResolutionChoiceKind::PayOrNot { spell_id, source_spell_id, cost, .. } }) => {
            assert!(*player == PlayerId(1) && *source == spell && *spell_id == theirs && *source_spell_id == spell && *cost == one);
        }
        _ => panic!("the spell's controller is asked whether to pay"),
    }
    crate::proofs::cards::clear_prompt(&mut state);
    behavior.on_pay_decision(&mut state, spell, PlayerId(1), kani::any(), &registry);

    if cards == 2 {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player: asked, source, choice: ResolutionChoiceKind::ChooseCardFromHand { player, cards: offered, discard_immediately, remaining, .. } }) => {
                assert!(*asked == PlayerId(1) && *player == PlayerId(1) && *source == spell);
                assert!(offered.len() == 2 && Some(offered[0]) == hand[0] && Some(offered[1]) == hand[1] && *discard_immediately && *remaining == 1);
            }
            _ => panic!("that player is asked which card"),
        }
    } else {
        assert!(state.awaiting_action.is_none());
    }
    for k in 0..cards {
        let o = state.get_object(hand[k].expect("made")).expect("still an object");
        assert!(o.zone == if cards == 1 { Zone::Graveyard } else { Zone::Hand });
    }
    assert!(state.players[1].graveyard_order.len() == usize::from(cards == 1));
    std::mem::forget((one, registry, state));
}

/// Cackling Counterpart ("Create a token that's a copy of target
/// creature you control.") resolving for the first player at their
/// Doomed Traveler, on the battlefield or gone from it by then
/// (`there`): there, one new permanent is theirs, a token and a creature
/// with the Traveler's printed power and toughness, read back through
/// the engine's layers (CR 707.2, 111.2); gone, none is made (608.2b).
fn cackling_counterpart(there: bool) {
    let (mut registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::cackling_counterpart::CacklingCounterpart));
    let traveler = registry.register(Box::new(crate::cards::isd::doomed_traveler::DoomedTraveler));
    let (power, toughness) = registry.printed_pt(traveler, false).expect("its face");
    let target = state.create_object(traveler, PlayerId(0), Zone::Battlefield, power, toughness);
    if !there {
        state.move_object(target, Zone::Graveyard, &registry);
    }
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[Target::Object(target)], &registry);

    assert!(state.objects.len() == before + usize::from(there));
    if there {
        let id = ObjectId(spell.0 + 1);
        let t = state.get_object(id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == PlayerId(0) && t.owner == PlayerId(0) && t.card_id == traveler);
        assert!(state.is_creature(id, &registry));
        assert!(state.effective_power(id, &registry) == power && state.effective_toughness(id, &registry) == toughness);
    }
    std::mem::forget((registry, state));
}

/// Divine Reckoning ("Each player chooses a creature they control.
/// Destroy the rest.") resolving with two creatures of the first
/// player's and one of the second's: the first player, the active one,
/// is asked which of their two to keep, with no way to decline, and the
/// second, with one, is not asked; the effect the question holds, the
/// card's own hook called as the engine calls it, keeps the one chosen
/// (`pick`), and the other is in its owner's graveyard by the engine's
/// own destruction (CR 701.7a), the second player's creature where it
/// was. Applied through the engine's dispatch, what the spell keeps of
/// who has chosen ran out of memory at 7 GB in symbolic execution
/// (`fiend_hunter` says why).
fn divine_reckoning(pick: usize) {
    use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::divine_reckoning::DivineReckoning));
    let yours = [a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0)), a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0))];
    let theirs = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.get_object_mut(spell).expect("just created").card_state.reserve(4);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. }) => {
            assert!(*player == PlayerId(0) && !*optional && options.len() == 2);
            assert!(options[0] == Target::Object(yours[0]) && options[1] == Target::Object(yours[1]));
            assert!(matches!(effect, PendingEffect::CardEffect { source_id, key } if *source_id == spell && key.is_empty()));
        }
        _ => panic!("the first player is asked which creature to keep"),
    }
    assert!(state.get_object(yours[0]).expect("still an object").zone == Zone::Battlefield);
    crate::proofs::cards::clear_prompt(&mut state);
    registry.get(card).expect("registered").resolve_card_effect(&mut state, spell, "", &Target::Object(yours[pick]), &registry);

    assert!(state.awaiting_action.is_none());
    let zone = |id| state.get_object(id).expect("still an object").zone;
    assert!(zone(yours[pick]) == Zone::Battlefield && zone(yours[1 - pick]) == Zone::Graveyard && zone(theirs) == Zone::Battlefield);
    assert!(state.players[0].graveyard_order.len() == 1 && state.players[1].graveyard_order.is_empty());
    std::mem::forget((registry, state));
}

/// The same with one creature each: nobody is asked, and both stay.
fn divine_reckoning_asks_nothing(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::divine_reckoning::DivineReckoning));
    let yours = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let theirs = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    state.get_object_mut(spell).expect("just created").card_state.reserve(4);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    assert!(state.awaiting_action.is_none());
    assert!(state.get_object(yours).expect("still an object").zone == Zone::Battlefield);
    assert!(state.get_object(theirs).expect("still an object").zone == Zone::Battlefield);
    std::mem::forget((registry, state));
}

// ---- The two planeswalkers, and the cards that raise the dead. A
// loyalty ability's own hook, its loyalty cost the engine's
// (`proofs::actions`). ----

/// A planeswalker of `card` under the first player, three loyalty
/// counters on it, showing its other face if `transformed`.
fn a_walker(state: &mut GameState, card: CardId, transformed: bool) -> ObjectId {
    let id = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
    let o = state.get_object_mut(id).expect("just created");
    o.counters.insert(CounterType::Loyalty, 3);
    o.is_transformed = transformed;
    id
}

/// Garruk Relentless ("0: Garruk Relentless deals 3 damage to target
/// creature. That creature deals damage equal to its power to him.") his
/// first ability resolving at the second player's 2/3 creature: it has 3
/// damage marked on it (CR 120.3e), and Garruk two loyalty counters
/// fewer, the creature's power (120.3c).
fn garruk_relentless_fights(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::garruk_relentless::GarrukRelentless));
    let creature = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    state.get_object_mut(creature).expect("just created").toughness = Some(3);
    let garruk = a_walker(&mut state, card, false);

    registry.get(card).expect("registered").on_loyalty_ability(&mut state, garruk, 0, &[Target::Object(creature)], &registry);

    let o = state.get_object(creature).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == 3);
    let g = state.get_object(garruk).expect("still an object");
    assert!(g.zone == Zone::Battlefield && g.counters.get(&CounterType::Loyalty).copied().unwrap_or(0) == 1);
    std::mem::forget((registry, state));
}

/// Garruk's Wolves ("0: Create a 2/2 green Wolf creature token." and, of
/// Garruk, the Veil-Cursed, "+1: Create a 1/1 black Wolf creature token
/// with deathtouch.") the ability of the face he shows resolving
/// (`veil_cursed`, or Relentless): one new permanent is his
/// controller's, a token and a Wolf creature of that face's colour,
/// power and toughness, with deathtouch exactly on the black one (CR
/// 111.2).
fn garruk_makes_a_wolf(veil_cursed: bool) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::garruk_relentless::GarrukRelentless));
    let garruk = a_walker(&mut state, card, veil_cursed);
    let before = state.objects.len();

    registry.get(card).expect("registered").on_loyalty_ability(&mut state, garruk, if veil_cursed { 10 } else { 1 }, &[], &registry);

    assert!(state.objects.len() == before + 1);
    let id = ObjectId(garruk.0 + 1);
    let t = state.get_object(id).expect("made");
    assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == PlayerId(0) && t.owner == PlayerId(0) && !t.tapped);
    assert!(state.is_creature(id, &registry) && state.has_keyword(id, Keyword::Deathtouch, &registry) == veil_cursed);
    let size = if veil_cursed { 1 } else { 2 };
    assert!(state.effective_power(id, &registry) == Some(size) && state.effective_toughness(id, &registry) == Some(size));
    assert!(t.colors == [if veil_cursed { Color::Black } else { Color::Green }] && t.subtypes.len() == 1 && t.subtypes[0] == "Wolf");
    std::mem::forget((registry, state));
}

/// Garruk Relentless ("When Garruk Relentless has two or fewer loyalty
/// counters on him, transform him.") with `loyalty` counters, showing
/// his front face: the condition of his state trigger holds exactly at
/// two or fewer (CR 603.8), and the trigger resolving shows his other
/// face, his counters as they were (701.28a).
fn garruk_relentless_transforms(loyalty: u32) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::garruk_relentless::GarrukRelentless));
    let garruk = a_walker(&mut state, card, false);
    state.get_object_mut(garruk).expect("just created").counters.insert(CounterType::Loyalty, loyalty);

    let behavior = registry.get(card).expect("registered");
    assert!(behavior.state_trigger_condition(&state, garruk, &registry) == (loyalty <= 2));
    if loyalty > 2 {
        std::mem::forget((registry, state));
        return;
    }
    behavior.on_state_trigger(&mut state, garruk, &registry);

    let g = state.get_object(garruk).expect("still an object");
    assert!(g.is_transformed && g.zone == Zone::Battlefield && g.counters.get(&CounterType::Loyalty).copied().unwrap_or(0) == loyalty);
    assert!(!behavior.state_trigger_condition(&state, garruk, &registry));
    std::mem::forget((registry, state));
}

/// Garruk, the Veil-Cursed ("−3: Creatures you control gain trample and
/// get +X/+X until end of turn, where X is the number of creature cards
/// in your graveyard.") the ability resolving with two creature cards
/// and another card in his controller's graveyard and a creature card in
/// the other player's: their creature has 2 more power and toughness
/// and trample, read back through the engine's layers (CR 613.4c,
/// 613.1f), and the other player's creature is as it was.
fn garruk_the_veil_cursed_overruns(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::garruk_relentless::GarrukRelentless));
    a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
    a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
    a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    a_card_in(&mut state, blank, 1, Zone::Graveyard, true);
    let yours = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let theirs = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    let garruk = a_walker(&mut state, card, true);

    registry.get(card).expect("registered").on_loyalty_ability(&mut state, garruk, 12, &[], &registry);

    assert!(state.effective_power(yours, &registry) == Some(4) && state.effective_toughness(yours, &registry) == Some(4));
    assert!(state.has_keyword(yours, Keyword::Trample, &registry));
    assert!(state.effective_power(theirs, &registry) == Some(2) && state.effective_toughness(theirs, &registry) == Some(2));
    assert!(!state.has_keyword(theirs, Keyword::Trample, &registry));
    std::mem::forget((registry, state));
}

/// Garruk, the Veil-Cursed ("−1: Sacrifice a creature. If you do, search
/// your library for a creature card, reveal it, put it into your hand,
/// then shuffle.") the ability resolving for a controller with one
/// creature, their library three cards, the middle one a creature card:
/// the creature is in its owner's graveyard, sacrificed with nothing
/// asked (CR 701.21a); they are then asked which creature card, the one
/// in the library the one offered and their hand where it goes, and the
/// search finished as the engine finishes it puts it in their hand
/// (701.19a), the other two still their library.
fn garruk_the_veil_cursed_searches(_: ()) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::garruk_relentless::GarrukRelentless));
    let mut library = [ObjectId(0); 3];
    for (k, slot) in library.iter_mut().enumerate() {
        let pt = if k == 1 { Some(2) } else { None };
        let id = state.create_object(blank, PlayerId(0), Zone::Library, pt, pt);
        state.players[0].library_order.push(id);
        *slot = id;
    }
    let creature = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let garruk = a_walker(&mut state, card, true);
    state.rng_state = SEED_FIRST;

    registry.get(card).expect("registered").on_loyalty_ability(&mut state, garruk, 11, &[], &registry);

    assert!(state.get_object(creature).expect("still an object").zone == Zone::Graveyard && state.players[0].graveyard_order == [creature]);
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseFromLibrary { options, searcher, source_id, destination, tapped, .. }, .. }) => {
            assert!(*player == PlayerId(0) && *searcher == PlayerId(0) && *source_id == garruk);
            assert!(options.len() == 1 && options[0] == library[1] && *destination == Zone::Hand && !*tapped);
        }
        _ => panic!("his controller is asked which creature card"),
    }
    crate::proofs::cards::clear_prompt(&mut state);
    crate::cards::helpers::finish_library_search(&mut state, PlayerId(0), library[1], Zone::Hand, false, &registry);

    let zone = |k: usize| state.get_object(library[k]).expect("still an object").zone;
    assert!(zone(0) == Zone::Library && zone(1) == Zone::Hand && zone(2) == Zone::Library && state.players[0].library_order.len() == 2);
    std::mem::forget((registry, state));
}

/// Liliana of the Veil ("−2: Target player sacrifices a creature.") the
/// ability resolving at the second player, who controls `creatures`
/// creatures: with none nothing happens; with one it is in its owner's
/// graveyard with nothing asked (CR 701.21a); with two that player is
/// asked which, with no way to decline, and the effect the question
/// holds sacrifices the one chosen (`pick`), the other staying.
fn liliana_of_the_veil_edicts((creatures, pick): (usize, usize)) {
    use crate::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::liliana_of_the_veil::LilianaOfTheVeil));
    let mut theirs = [None; 2];
    for slot in theirs.iter_mut().take(creatures) {
        *slot = Some(a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1)));
    }
    let liliana = a_walker(&mut state, card, false);
    let name = registry.printed_name(card, false).expect("its face").to_string();

    registry.get(card).expect("registered").on_loyalty_ability(&mut state, liliana, 1, &[Target::Player(PlayerId(1))], &registry);

    if creatures == 2 {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, optional, effect, .. }, .. }) => {
                assert!(*player == PlayerId(1) && !*optional && options.len() == 2);
                for k in 0..2 {
                    assert!(options[k] == Target::Object(theirs[k].expect("made")));
                }
                assert!(matches!(effect, PendingEffect::SacrificeCreature { source_name } if *source_name == name));
            }
            _ => panic!("that player is asked which creature"),
        }
        crate::proofs::cards::clear_prompt(&mut state);
        let effect = PendingEffect::SacrificeCreature { source_name: name };
        crate::engine::apply_pending_effect(&mut state, &Target::Object(theirs[pick].expect("made")), &effect, &registry);
        std::mem::forget(effect);
    } else {
        assert!(state.awaiting_action.is_none());
        std::mem::forget(name);
    }
    for k in 0..creatures {
        let o = state.get_object(theirs[k].expect("made")).expect("still an object");
        assert!(o.zone == if k == pick { Zone::Graveyard } else { Zone::Battlefield });
    }
    assert!(state.players[1].graveyard_order.len() == creatures.min(1));
    std::mem::forget((registry, state));
}

/// Liliana of the Veil ("−6: Separate all permanents target player
/// controls into two piles. That player sacrifices all permanents in the
/// pile of their choice.") the ability resolving at the second player,
/// who controls two permanents or none (`permanents`): with two,
/// Liliana's controller is asked to divide exactly those two, for that
/// player to choose between; with none nothing is asked. What the
/// answers do is the engine's (`proofs::actions`).
fn liliana_of_the_veil_divides(permanents: bool) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::liliana_of_the_veil::LilianaOfTheVeil));
    let mut theirs = [None; 2];
    if permanents {
        theirs = [Some(a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1))), Some(a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1)))];
    }
    let mine = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let liliana = a_walker(&mut state, card, false);

    registry.get(card).expect("registered").on_loyalty_ability(&mut state, liliana, 2, &[Target::Player(PlayerId(1))], &registry);

    if permanents {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::DividePermanentsIntoPiles { permanents, target_player, source_id, .. }, .. }) => {
                assert!(*player == PlayerId(0) && *target_player == PlayerId(1) && *source_id == liliana);
                assert!(permanents.len() == 2 && Some(permanents[0]) == theirs[0] && Some(permanents[1]) == theirs[1]);
            }
            _ => panic!("her controller is asked to divide them"),
        }
    } else {
        assert!(state.awaiting_action.is_none());
    }
    assert!(state.get_object(mine).expect("still an object").zone == Zone::Battlefield);
    std::mem::forget((registry, state));
}

/// Liliana of the Veil ("+1: Each player discards a card.") the ability
/// resolving with one card in each player's hand: nobody has a choice,
/// and both cards are in their owners' graveyards (CR 701.8a), nothing
/// asked.
fn liliana_of_the_veil_makes_each_discard(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::liliana_of_the_veil::LilianaOfTheVeil));
    let yours = a_card_in(&mut state, blank, 0, Zone::Hand, false);
    let theirs = a_card_in(&mut state, blank, 1, Zone::Hand, false);
    let liliana = a_walker(&mut state, card, false);
    state.get_object_mut(liliana).expect("just created").card_state.reserve(8);

    registry.get(card).expect("registered").on_loyalty_ability(&mut state, liliana, 0, &[], &registry);

    assert!(state.awaiting_action.is_none());
    assert!(state.get_object(yours).expect("still an object").zone == Zone::Graveyard && state.players[0].graveyard_order == [yours]);
    assert!(state.get_object(theirs).expect("still an object").zone == Zone::Graveyard && state.players[1].graveyard_order == [theirs]);
    std::mem::forget((registry, state));
}

/// Back from the Brink ("Exile a creature card from your graveyard and
/// pay its mana cost: Create a token that's a copy of that card.
/// Activate only as a sorcery.") the first player's, its cost paid for a
/// Doomed Traveler in a graveyard, theirs (`yours`) or the other
/// player's: the other player's is no card of "your graveyard" and
/// stays where it is; theirs is in exile as the cost is paid (CR
/// 602.2b), and the ability resolving makes one new permanent, a token
/// under them that is a creature with the card's printed power and
/// toughness, read back through the engine's layers (707.2).
fn back_from_the_brink(yours: bool) {
    let (mut registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::back_from_the_brink::BackFromTheBrink));
    let traveler = registry.register(Box::new(crate::cards::isd::doomed_traveler::DoomedTraveler));
    let (power, toughness) = registry.printed_pt(traveler, false).expect("its face");
    let whose = usize::from(!yours);
    let dead = state.create_object(traveler, PlayerId(u8::from(!yours)), Zone::Graveyard, power, toughness);
    if yours {
        state.players[0].graveyard_order.push(dead);
    } else {
        state.players[1].graveyard_order.push(dead);
    }
    let brink = a_source(&mut state, card, PlayerId(0));
    let index = usize::try_from(dead.0).expect("a small id");

    let behavior = registry.get(card).expect("registered");
    behavior.pay_activation_cost(&mut state, brink, index, &[], &registry);

    if yours {
        assert!(state.get_object(dead).expect("still an object").zone == Zone::Exile && state.players[0].graveyard_order.is_empty());
        let before = state.objects.len();
        behavior.resolve_activated_ability(&mut state, brink, index, &[], &registry);
        assert!(state.objects.len() == before + 1);
        let id = ObjectId(brink.0 + 1);
        let t = state.get_object(id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == PlayerId(0) && t.owner == PlayerId(0) && t.card_id == traveler);
        assert!(state.is_creature(id, &registry));
        assert!(state.effective_power(id, &registry) == power && state.effective_toughness(id, &registry) == toughness);
    } else {
        assert!(state.get_object(dead).expect("still an object").zone == Zone::Graveyard && state.players[whose].graveyard_order.len() == 1);
    }
    std::mem::forget((registry, state));
}

/// Grimoire of the Dead ("{1}, {T}, Discard a card: Put a study counter
/// on this artifact.") the first player's, the discard of its cost paid
/// from a hand of `cards` cards and the ability resolving: with one card
/// it is discarded with nothing asked (CR 701.8a) and the Grimoire has
/// one study counter more (122.1); with two they are asked which, one
/// card and at once, and nothing has left their hand yet.
fn grimoire_of_the_dead_studies(cards: usize) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::grimoire_of_the_dead::GrimoireOfTheDead));
    let mut hand = [None; 2];
    for slot in hand.iter_mut().take(cards) {
        *slot = Some(a_card_in(&mut state, blank, 0, Zone::Hand, false));
    }
    let grimoire = a_source(&mut state, card, PlayerId(0));
    let had = u32::from(kani::any::<bool>());
    state.get_object_mut(grimoire).expect("just created").counters.insert(CounterType::Study, had);

    let behavior = registry.get(card).expect("registered");
    behavior.pay_activation_cost(&mut state, grimoire, 0, &[], &registry);

    if cards == 2 {
        match &state.awaiting_action {
            Some(AwaitingAction::ResolutionChoice { player: asked, source, choice: ResolutionChoiceKind::ChooseCardFromHand { player, cards: offered, discard_immediately, remaining, .. } }) => {
                assert!(*asked == PlayerId(0) && *player == PlayerId(0) && *source == grimoire);
                assert!(offered.len() == 2 && Some(offered[0]) == hand[0] && Some(offered[1]) == hand[1] && *discard_immediately && *remaining == 1);
            }
            _ => panic!("its controller is asked which card"),
        }
        assert!(state.players[0].graveyard_order.is_empty());
    } else {
        assert!(state.awaiting_action.is_none());
        assert!(state.get_object(hand[0].expect("made")).expect("still an object").zone == Zone::Graveyard);
        behavior.resolve_activated_ability(&mut state, grimoire, 0, &[], &registry);
        assert!(state.get_object(grimoire).expect("still an object").counters.get(&CounterType::Study).copied().unwrap_or(0) == had + 1);
    }
    std::mem::forget((registry, state));
}

/// Grimoire of the Dead ("{T}, Remove three study counters from this
/// artifact and sacrifice it: Put all creature cards from all graveyards
/// onto the battlefield under your control. They're black Zombies in
/// addition to their other colors and types.") sacrificed for its cost,
/// the first player's, its ability resolving with a creature card in
/// each player's graveyard and another card in its controller's: both
/// creature cards are on the battlefield under the first player (CR
/// 110.2), each black and a Zombie as the engine reads it (613.1d,
/// 613.1e), and the other card and the Grimoire are where they were.
fn grimoire_of_the_dead_raises(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::grimoire_of_the_dead::GrimoireOfTheDead));
    let other = a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    let raised = [a_card_in(&mut state, blank, 0, Zone::Graveyard, true), a_card_in(&mut state, blank, 1, Zone::Graveyard, true)];
    let grimoire = a_card_in(&mut state, card, 0, Zone::Graveyard, false);

    registry.get(card).expect("registered").resolve_activated_ability(&mut state, grimoire, 1, &[], &registry);

    for (k, id) in raised.into_iter().enumerate() {
        let o = state.get_object(id).expect("still an object");
        assert!(o.zone == Zone::Battlefield && o.controller == PlayerId(0) && o.owner == PlayerId(u8::try_from(k).expect("0 or 1")));
        assert!(state.has_subtype(id, "Zombie", &registry) && state.is_creature(id, &registry));
        let colours = state.colors_of(id, &registry);
        assert!(colours.len() == 1 && colours[0] == Color::Black);
        std::mem::forget(colours);
    }
    assert!(state.get_object(other).expect("still an object").zone == Zone::Graveyard);
    assert!(state.get_object(grimoire).expect("still an object").zone == Zone::Graveyard);
    assert!(state.players[0].graveyard_order.len() == 2 && state.players[1].graveyard_order.is_empty());
    std::mem::forget((registry, state));
}

// ---- Permanents that ask whether to pay, the mana already floating
// (a payment planned from lands is the engine's, `proofs::actions::
// paying`), and the one that asks for a name. ----

/// Mentor of the Meek ("Whenever another creature you control with power
/// 2 or less enters, you may pay {1}. If you do, draw a card.") the first
/// player's, its trigger resolving with {1} floating in their pool and a
/// card in their library: they are asked whether to pay, and answering
/// yes the mana is spent and the card is in their hand (CR 121.1);
/// answering no the mana floats still and nothing is drawn.
fn mentor_of_the_meek(yes: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::mentor_of_the_meek::MentorOfTheMeek));
    let library = a_library(&mut state, blank, 0, 1);
    let entered = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let mentor = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));
    state.players[0].mana_pool.add(ManaType::Colorless, 1);

    let behavior = registry.get(card).expect("registered");
    behavior.on_any_creature_enters(&mut state, mentor, entered, PlayerId(0), &registry);
    asked_yes_or_no(&mut state, mentor, PlayerId(0));
    behavior.on_yes_no_choice(&mut state, mentor, yes, &registry);

    let drawn = state.get_object(library[0].expect("made")).expect("still an object");
    assert!(drawn.zone == if yes { Zone::Hand } else { Zone::Library });
    assert!(state.players[0].mana_pool.total() == u32::from(!yes) && !state.players[0].has_drawn_from_empty);
    assert!(state.awaiting_action.is_none());
    std::mem::forget((registry, state));
}

/// The same trigger with nothing that could pay {1}: nothing is asked
/// (CR 118.3), and nothing is drawn.
fn mentor_of_the_meek_asks_nothing(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::mentor_of_the_meek::MentorOfTheMeek));
    let library = a_library(&mut state, blank, 0, 1);
    let entered = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let mentor = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));

    registry.get(card).expect("registered").on_any_creature_enters(&mut state, mentor, entered, PlayerId(0), &registry);

    assert!(state.awaiting_action.is_none());
    assert!(state.get_object(library[0].expect("made")).expect("still an object").zone == Zone::Library);
    std::mem::forget((registry, state));
}

/// Which creatures entering set Mentor of the Meek's trigger off: a
/// creature of power 2 under its controller does; the Mentor itself, a
/// creature under the other player, and one of power 3 do not ("another
/// creature you control with power 2 or less").
fn mentor_of_the_meek_watches(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::mentor_of_the_meek::MentorOfTheMeek));
    let small = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let theirs = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    let big = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    state.get_object_mut(big).expect("just created").power = Some(3);
    let mentor = a_creature_of(&mut state, card, PlayerId(0), PlayerId(0));

    let behavior = registry.get(card).expect("registered");
    assert!(behavior.should_trigger_on_creature_enters(&state, mentor, small, PlayerId(0), &registry));
    assert!(!behavior.should_trigger_on_creature_enters(&state, mentor, mentor, PlayerId(0), &registry));
    assert!(!behavior.should_trigger_on_creature_enters(&state, mentor, theirs, PlayerId(1), &registry));
    assert!(!behavior.should_trigger_on_creature_enters(&state, mentor, big, PlayerId(0), &registry));
    std::mem::forget((registry, state));
}

/// Screeching Bat ("At the beginning of your upkeep, you may pay
/// {2}{B}{B}. If you do, transform this creature.") the first player's
/// at their upkeep, showing either face (`back`), with {2}{B}{B}
/// floating in their pool: they are asked whether to pay, and answering
/// yes the mana is spent and it shows its other face, whose power and
/// toughness the engine then reads (CR 701.28a, 712.8); answering no
/// the mana floats still and it is as it was.
fn screeching_bat((back, yes): (bool, bool)) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::screeching_bat::ScreechingBat));
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let bat = state.create_object(card, PlayerId(0), Zone::Battlefield, power, toughness);
    state.get_object_mut(bat).expect("just created").is_transformed = back;
    state.players[0].mana_pool.add(ManaType::Black, 2);
    state.players[0].mana_pool.add(ManaType::Colorless, 2);

    let behavior = registry.get(card).expect("registered");
    behavior.on_upkeep(&mut state, bat, &[], &registry);
    asked_yes_or_no(&mut state, bat, PlayerId(0));
    behavior.on_yes_no_choice(&mut state, bat, yes, &registry);

    let shows_back = back != yes;
    let o = state.get_object(bat).expect("still an object");
    assert!(o.is_transformed == shows_back && o.zone == Zone::Battlefield && o.controller == PlayerId(0));
    assert!(state.players[0].mana_pool.total() == if yes { 0 } else { 4 });
    state.get_object_mut(bat).expect("still an object").is_transformed = shows_back;
    let face = registry.printed_pt(card, shows_back).expect("that face");
    assert!(state.effective_power(bat, &registry) == face.0 && state.effective_toughness(bat, &registry) == face.1);
    assert!(state.awaiting_action.is_none());
    std::mem::forget((registry, state));
}

/// The same trigger with nothing that could pay (`gone` false), or for a
/// Bat put into its graveyard since (`gone`), the mana floating: nothing
/// is asked (CR 118.3, 400.7).
fn screeching_bat_asks_nothing(gone: bool) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::screeching_bat::ScreechingBat));
    let (power, toughness) = registry.printed_pt(card, false).expect("its front face");
    let bat = state.create_object(card, PlayerId(0), Zone::Battlefield, power, toughness);
    if gone {
        state.players[0].mana_pool.add(ManaType::Black, 4);
        state.move_object(bat, Zone::Graveyard, &registry);
    }

    registry.get(card).expect("registered").on_upkeep(&mut state, bat, &[], &registry);

    assert!(state.awaiting_action.is_none());
    assert!(!state.get_object(bat).expect("still an object").is_transformed);
    std::mem::forget((registry, state));
}

/// Nevermore ("As this enchantment enters, choose a nonland card name.
/// Spells with the chosen name can't be cast.") entering under either
/// player, the registry holding it, the blank card and a land (Forest):
/// its controller is asked for a card name, on its behalf, and the
/// names offered are those of the two cards that are not lands, in
/// order, the land's not among them. What the answer does is the
/// engine's (`proofs::actions`).
fn nevermore(_: ()) {
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::nevermore::Nevermore));
    registry.register(Box::new(crate::cards::forest::Forest));
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let nevermore = a_source(&mut state, card, you);

    registry.get(card).expect("registered").on_enter_battlefield(&mut state, nevermore, &[], &registry);

    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, source, choice: ResolutionChoiceKind::ChooseCardName { options, source_id, .. } }) => {
            assert!(*player == you && *source == nevermore && *source_id == nevermore && options.len() == 2);
            assert!(options[0] == registry.printed_name(blank, false).expect("its face"));
            assert!(options[1] == registry.printed_name(card, false).expect("its face"));
        }
        _ => panic!("its controller is asked for a card name"),
    }
    std::mem::forget((registry, state));
}

// ---- Permanents the engine reads. Nothing resolves in these: the card
// is on the battlefield, and the board is read back through the engine's
// own layers and queries, as its text says the board should read. ----

/// A creature of `card` on the battlefield under `you`, at the power and
/// toughness the card prints.
fn printed_creature(state: &mut GameState, registry: &crate::cards::CardRegistry, card: CardId, you: PlayerId) -> ObjectId {
    let (power, toughness) = registry.printed_pt(card, false).expect("its face");
    state.create_object(card, you, Zone::Battlefield, power, toughness)
}

/// "Creatures you control get +N/+N." (Glorious Anthem; Heartless
/// Summoning's -1/-1) under the first player: their 2/2 creature reads
/// that much more through the engine's layers (CR 613.4c), and the other
/// player's 2/2 reads 2/2.
///
/// A filter of one question fits ("you control", "a token you
/// control"). One the card builds of several does not, read back from
/// the card: Battleground Geist's and Gallows Warden's "other Spirit
/// creatures you control" and One-Eyed Scarecrow's "creatures with
/// flying your opponents control" each ran out of memory at 7 GB in
/// symbolic execution, on a board of the card and one creature with
/// nothing open. Those three have no harness here: what each filter
/// matches is proved of the engine with the filter built where it is
/// matched (`proofs::actions::filters`), and that the card prints that
/// filter and that change is its declared harness (`proofs::cards`).
fn your_creatures_get((card, by): (Box<dyn CardBehavior>, i32)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let yours = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let theirs = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    a_source(&mut state, card, PlayerId(0));

    assert!(state.effective_power(yours, &registry) == Some(2 + by) && state.effective_toughness(yours, &registry) == Some(2 + by));
    assert!(state.effective_power(theirs, &registry) == Some(2) && state.effective_toughness(theirs, &registry) == Some(2));
    std::mem::forget((registry, state));
}

/// Boneyard Wurm ("Boneyard Wurm's power and toughness are each equal to
/// the number of creature cards in your graveyard.") under the first
/// player, `creatures` creature cards and another card in their
/// graveyard and a creature card in the other player's: it reads that
/// many and that many (CR 604.3, 613.4a).
fn boneyard_wurm(creatures: usize) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::boneyard_wurm::BoneyardWurm));
    for _ in 0..creatures {
        a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
    }
    a_card_in(&mut state, blank, 0, Zone::Graveyard, false);
    a_card_in(&mut state, blank, 1, Zone::Graveyard, true);
    let wurm = state.create_object(card, PlayerId(0), Zone::Battlefield, Some(0), Some(0));

    let n = Some(i32::try_from(creatures).expect("a few"));
    assert!(state.effective_power(wurm, &registry) == n && state.effective_toughness(wurm, &registry) == n);
    std::mem::forget((registry, state));
}

/// Scourge of Geier Reach ("This creature gets +1/+1 for each creature
/// your opponents control."), a 3/3 as printed, under the first player
/// with `theirs` creatures of the other player's and one more of its
/// controller's: it reads that many more of each (CR 613.4c).
fn scourge_of_geier_reach(theirs: usize) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::scourge_of_geier_reach::ScourgeOfGeierReach));
    for _ in 0..theirs {
        a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    }
    a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let scourge = printed_creature(&mut state, &registry, card, PlayerId(0));

    let more = i32::try_from(theirs).expect("a few");
    assert!(state.effective_power(scourge, &registry) == Some(3 + more) && state.effective_toughness(scourge, &registry) == Some(3 + more));
    std::mem::forget((registry, state));
}

/// Angelic Overseer ("Flying. As long as you control a Human, this
/// creature has hexproof and indestructible.") under the first player
/// beside a Human, theirs (`yours`) or the other player's: it has flying
/// either way, and hexproof and indestructible exactly beside a Human of
/// its controller's (CR 613.1f).
fn angelic_overseer(yours: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::angelic_overseer::AngelicOverseer));
    let human = a_permanent(&mut state, blank, Permanent::Human);
    state.get_object_mut(human).expect("just created").controller = PlayerId(u8::from(!yours));
    let overseer = printed_creature(&mut state, &registry, card, PlayerId(0));

    assert!(state.has_keyword(overseer, Keyword::Flying, &registry));
    assert!(state.has_keyword(overseer, Keyword::Hexproof, &registry) == yours);
    assert!(state.has_keyword(overseer, Keyword::Indestructible, &registry) == yours);
    std::mem::forget((registry, state));
}

/// Night Revelers ("This creature has haste as long as an opponent
/// controls a Human.") under the first player beside a Human, the other
/// player's (`theirs`) or its controller's: it has haste exactly beside
/// an opponent's (CR 613.1f).
fn night_revelers(theirs: bool) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::night_revelers::NightRevelers));
    let human = a_permanent(&mut state, blank, Permanent::Human);
    state.get_object_mut(human).expect("just created").controller = PlayerId(u8::from(theirs));
    let revelers = printed_creature(&mut state, &registry, card, PlayerId(0));

    assert!(state.has_keyword(revelers, Keyword::Haste, &registry) == theirs);
    std::mem::forget((registry, state));
}

/// Intangible Virtue ("Creature tokens you control get +1/+1 and have
/// vigilance.") under the first player: their 2/2 creature token reads
/// 3/3 with vigilance (CR 613.4c, 613.1f); their creature that is no
/// token, and the other player's token, read 2/2 without it.
fn intangible_virtue(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::intangible_virtue::IntangibleVirtue));
    let token = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    state.get_object_mut(token).expect("just created").is_token = true;
    let card_creature = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let theirs = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    state.get_object_mut(theirs).expect("just created").is_token = true;
    a_source(&mut state, card, PlayerId(0));

    assert!(state.effective_power(token, &registry) == Some(3) && state.effective_toughness(token, &registry) == Some(3));
    assert!(state.has_keyword(token, Keyword::Vigilance, &registry));
    for id in [card_creature, theirs] {
        assert!(state.effective_power(id, &registry) == Some(2) && state.effective_toughness(id, &registry) == Some(2));
        assert!(!state.has_keyword(id, Keyword::Vigilance, &registry));
    }
    std::mem::forget((registry, state));
}

/// A creature's printed keywords, as the engine reads them off the
/// permanent (CR 702): each of `has`, and none of `lacks`.
fn has_keywords((card, has, lacks): (Box<dyn CardBehavior>, &[Keyword], &[Keyword])) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let creature = printed_creature(&mut state, &registry, card, you);

    for k in has {
        assert!(state.has_keyword(creature, *k, &registry));
    }
    for k in lacks {
        assert!(!state.has_keyword(creature, *k, &registry));
    }
    std::mem::forget((registry, state));
}

/// Vampire Interloper ("Flying. This creature can't block."), untapped
/// on the battlefield: the engine says it cannot block (CR 509.1b),
/// where the blank creature beside it can.
fn vampire_interloper(_: ()) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::vampire_interloper::VampireInterloper));
    let other = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    let interloper = printed_creature(&mut state, &registry, card, PlayerId(0));

    assert!(!crate::combat::can_block_at_all(&state, interloper, &registry));
    assert!(crate::combat::can_block_at_all(&state, other, &registry));
    std::mem::forget((registry, state));
}

/// A creature that "can't be blocked" (Invisible Stalker), or "except by
/// creatures with flying or reach" (Orchard Spirit: `except`), attacked
/// with: the engine says a creature with neither cannot block it, and a
/// creature that prints flying (Chapel Geist) or reach (Somberwald
/// Spider) can exactly where the text makes the exception (CR 509.1b).
/// A creature flies by what its card prints: a keyword written onto a
/// permanent of a card that prints none is not read by the engine
/// (`GameState::scan_keywords` says why), and this harness, when it
/// gave a blank creature flying that way and claimed it could block an
/// Orchard Spirit, failed as it should.
fn evades((card, except): (Box<dyn CardBehavior>, bool)) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let geist = registry.register(Box::new(crate::cards::isd::chapel_geist::ChapelGeist));
    let spider = registry.register(Box::new(crate::cards::isd::somberwald_spider::SomberwaldSpider));
    let walker = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    let flier = printed_creature(&mut state, &registry, geist, PlayerId(1));
    let reacher = printed_creature(&mut state, &registry, spider, PlayerId(1));
    let attacker = printed_creature(&mut state, &registry, card, PlayerId(0));

    assert!(state.has_keyword(flier, Keyword::Flying, &registry) && state.has_keyword(reacher, Keyword::Reach, &registry));
    assert!(!crate::combat::can_block_attacker(&state, walker, attacker, &registry));
    assert!(crate::combat::can_block_attacker(&state, flier, attacker, &registry) == except);
    assert!(crate::combat::can_block_attacker(&state, reacher, attacker, &registry) == except);
    std::mem::forget((registry, state));
}

/// Laboratory Maniac ("If you would draw a card while your library has
/// no cards in it, you win the game instead.") under the first player,
/// the player `who` names drawing a card by the engine's own draw from a
/// library of `cards` cards: its controller drawing from an empty one,
/// the other player has lost and its controller has not drawn from an
/// empty library (CR 614.1a, 104.2b); with a card there, or the other
/// player drawing, the draw is a draw (121.1, 704.5b).
fn laboratory_maniac((cards, who): (usize, u8)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::laboratory_maniac::LaboratoryManiac));
    let library = a_library(&mut state, blank, who, cards);
    printed_creature(&mut state, &registry, card, PlayerId(0));

    let _ = crate::engine::draw_cards(&mut state, PlayerId(who), 1, &registry);

    let wins = cards == 0 && who == 0;
    assert!(state.players[1].lost == wins && !state.players[0].lost);
    assert!(state.players[who as usize].has_drawn_from_empty == (cards == 0 && who == 1));
    if cards == 1 {
        assert!(state.get_object(library[0].expect("made")).expect("still an object").zone == Zone::Hand);
    }
    std::mem::forget((registry, state));
}

/// "{T}: Add ..." (Avacyn's Pilgrim's {W}, a basic land's one mana of
/// its colour, Shimmering Grotto's {C}), the mana ability activated
/// through the engine for the first player: the permanent is tapped (CR
/// 605.3a), and its controller's pool holds exactly that mana and the
/// other player's none. For one player: with the controller left open
/// the Pilgrim and a Plains proved alone under 7 GB and the other four
/// basic lands ran out of memory there. Sol Ring's {C}{C} ran out of
/// memory either way, an artifact's ability being asked first whether
/// anything on the board forbids it, and is left to the engine's own
/// proofs of an activation (`proofs::actions::mana_abilities`).
fn taps_for((card, mana, n): (Box<dyn CardBehavior>, ManaType, u32)) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let source = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
    state.get_object_mut(source).expect("just created").summoning_sick = false;

    crate::engine::activate_mana_source(&mut state, source, 0, &registry);

    assert!(state.get_object(source).expect("still an object").tapped);
    let pool = &state.players[0].mana_pool;
    assert!(pool.get(mana) == n && pool.total() == n);
    assert!(state.players[1].mana_pool.total() == 0);
    std::mem::forget((registry, state));
}

// ---- Permanents that enter otherwise than a card does, and what
// replaces an event. The card is put onto the battlefield from its
// owner's hand by the engine's own zone change, which asks the board
// what replaces the entry (CR 614.1c, 614.12); a token is made, damage
// dealt and protection asked by the engine's own. ----

/// A card of `card` in the first player's hand, at the power and
/// toughness it prints.
fn in_hand(state: &mut GameState, registry: &crate::cards::CardRegistry, card: CardId) -> ObjectId {
    let (power, toughness) = registry.printed_pt(card, false).expect("its face");
    state.create_object(card, PlayerId(0), Zone::Hand, power, toughness)
}

/// Diregraf Ghoul ("This creature enters tapped.") put onto the
/// battlefield from its owner's hand: it is there tapped, under its
/// owner (CR 614.1c).
fn diregraf_ghoul(_: ()) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::diregraf_ghoul::DiregrafGhoul));
    let ghoul = in_hand(&mut state, &registry, card);

    state.move_object(ghoul, Zone::Battlefield, &registry);

    let o = state.get_object(ghoul).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped && o.controller == PlayerId(0));
    std::mem::forget((registry, state));
}

/// A land that "enters tapped unless you control a [type] or a [type]"
/// put onto the battlefield from its owner's hand beside a basic land of
/// one of the two types, its controller's (`yours`) or the other
/// player's: untapped beside its controller's own, tapped beside the
/// other player's (CR 614.1c).
fn check_land((card, basic, yours): (Box<dyn CardBehavior>, Box<dyn CardBehavior>, bool)) {
    let (mut registry, mut state, _, card) = crate::proofs::cards::board(card);
    let basic = registry.register(basic);
    state.create_object(basic, PlayerId(u8::from(!yours)), Zone::Battlefield, None, None);
    let land = in_hand(&mut state, &registry, card);

    state.move_object(land, Zone::Battlefield, &registry);

    let o = state.get_object(land).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.tapped != yours && o.controller == PlayerId(0));
    std::mem::forget((registry, state));
}

/// "Morbid — This creature enters with two +1/+1 counters on it if a
/// creature died this turn." (Festerhide Boar, Somberwald Spider) put
/// onto the battlefield from its owner's hand, a creature having died
/// this turn or not: with two +1/+1 counters and that much more than it
/// prints, read back through the engine's layers, or with none (CR
/// 614.1c, 122.1a).
fn morbid_counters((card, died): (Box<dyn CardBehavior>, bool)) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let creature = in_hand(&mut state, &registry, card);
    state.creature_died_this_turn = died;

    state.move_object(creature, Zone::Battlefield, &registry);

    let more = if died { 2 } else { 0 };
    let o = state.get_object(creature).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0) == more);
    let printed = registry.printed_pt(card, false).expect("its face");
    let more = i32::try_from(more).expect("0 or 2");
    assert!(state.effective_power(creature, &registry) == printed.0.map(|p| p + more));
    assert!(state.effective_toughness(creature, &registry) == printed.1.map(|t| t + more));
    std::mem::forget((registry, state));
}

/// Unbreathing Horde ("This creature enters with a +1/+1 counter on it
/// for each other Zombie you control and each Zombie card in your
/// graveyard.") put onto the battlefield from its owner's hand, a Zombie
/// of theirs and one of the other player's on the battlefield and a
/// Zombie card (Walking Corpse) and another card in their graveyard: it
/// has two +1/+1 counters, the other player's Zombie and the other card
/// not counted, and reads 2/2 (CR 614.1c, 122.1a).
fn unbreathing_horde_enters(_: ()) {
    let (mut registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::unbreathing_horde::UnbreathingHorde));
    let corpse = registry.register(Box::new(WalkingCorpse));
    for who in [0, 1] {
        let zombie = a_creature_of(&mut state, blank, PlayerId(who), PlayerId(who));
        state.get_object_mut(zombie).expect("just created").subtypes = vec!["Zombie".to_string()];
    }
    a_card_in(&mut state, corpse, 0, Zone::Graveyard, true);
    a_card_in(&mut state, blank, 0, Zone::Graveyard, true);
    let horde = in_hand(&mut state, &registry, card);

    state.move_object(horde, Zone::Battlefield, &registry);

    let o = state.get_object(horde).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0) == 2);
    assert!(state.effective_power(horde, &registry) == Some(2) && state.effective_toughness(horde, &registry) == Some(2));
    std::mem::forget((registry, state));
}

/// Unbreathing Horde ("If this creature would be dealt damage, prevent
/// that damage and remove a +1/+1 counter from it.") on the battlefield
/// with two +1/+1 counters, dealt 3 damage by the engine's own: no
/// damage is marked on it, and it has one counter (CR 615.1, 614.1a).
fn unbreathing_horde_prevents(_: ()) {
    use crate::damage::DamageKind;
    use crate::events::DamageTarget;
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::unbreathing_horde::UnbreathingHorde));
    let source = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    let horde = state.create_object(card, PlayerId(0), Zone::Battlefield, Some(0), Some(0));
    state.get_object_mut(horde).expect("just created").counters.insert(CounterType::PlusOnePlusOne, 2);

    crate::damage::deal_damage(&mut state, source, DamageTarget::Object(horde), 3, DamageKind::NonCombat, &registry);

    let o = state.get_object(horde).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.damage_marked == 0 && o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0) == 1);
    std::mem::forget((registry, state));
}

/// Dearly Departed ("As long as this creature is in your graveyard, each
/// Human creature you control enters with an additional +1/+1 counter on
/// it.") in a graveyard, the first player's (`yours`) or the other
/// player's, and a creature card of the first player's put onto the
/// battlefield from their hand, a Human or not: it has one +1/+1 counter
/// exactly when it is a Human and the Departed is in its controller's
/// graveyard (CR 614.1c, 614.12).
fn dearly_departed((yours, human): (bool, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::dearly_departed::DearlyDeparted));
    a_card_in(&mut state, card, u8::from(!yours), Zone::Graveyard, true);
    let entering = state.create_object(blank, PlayerId(0), Zone::Hand, Some(2), Some(2));
    let o = state.get_object_mut(entering).expect("just created");
    o.card_types = vec![CardType::Creature];
    if human {
        o.subtypes = vec!["Human".to_string()];
    }

    state.move_object(entering, Zone::Battlefield, &registry);

    let o = state.get_object(entering).expect("still an object");
    assert!(o.zone == Zone::Battlefield);
    assert!(o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0) == u32::from(yours && human));
    std::mem::forget((registry, state));
}

/// Parallel Lives ("If an effect would create one or more tokens under
/// your control, it creates twice that many of those tokens instead.")
/// under the first player, and the engine making one 1/1 token for the
/// player `who` names: two for its controller, one for the other player
/// (CR 614.1a, 111.2).
fn parallel_lives(who: u8) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::parallel_lives::ParallelLives));
    a_source(&mut state, card, PlayerId(0));
    let before = state.objects.len();

    let made = state.create_token_with_subtypes("", PlayerId(who), 1, 1, vec![Color::White], vec![CardType::Creature], vec![], vec!["Spirit".to_string()], &registry);

    let n = if who == 0 { 2 } else { 1 };
    assert!(state.objects.len() == before + n && made.len() == n);
    for id in &made {
        let t = state.get_object(*id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == PlayerId(who));
    }
    std::mem::forget((made, registry, state));
}

/// Stony Silence ("Activated abilities of artifacts can't be
/// activated.") on the battlefield under either player, or in a
/// graveyard (`there` false): the engine forbids an artifact's ability
/// exactly while it is on the battlefield (CR 604.1).
fn stony_silence(there: bool) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::stony_silence::StonySilence));
    if there {
        a_source(&mut state, card, PlayerId(u8::from(kani::any::<bool>())));
    } else {
        a_card_in(&mut state, card, 0, Zone::Graveyard, false);
    }

    assert!(crate::engine::prevents_artifact_abilities(&state, &registry) == there);
    std::mem::forget((registry, state));
}

/// "Protection from [subtype]" (Elite Inquisitor's from Vampires, from
/// Werewolves and from Zombies; Grave Bramble's from Zombies): the
/// engine says the creature is protected from a source of the subtype
/// `from` exactly when the text names it (CR 702.16a).
fn protected_from((card, from, named): (Box<dyn CardBehavior>, &str, bool)) {
    let (registry, mut state, blank, card) = crate::proofs::cards::board(card);
    let source = a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    state.get_object_mut(source).expect("just created").subtypes = vec![from.to_string()];
    let creature = printed_creature(&mut state, &registry, card, PlayerId(0));

    assert!(state.has_protection_from(creature, source, &registry) == named);
    std::mem::forget((registry, state));
}

// ---- Creatures that are what they print. A creature with no ability,
// or with keywords alone, has no hook to run: what there is to hold the
// card to is its printed power, toughness and keywords, read off a
// permanent of it through the engine's own layers. The numbers and
// keywords in the table are the card's Oracle text, not the card's own
// data read back. ----

/// A creature of the card on the battlefield under either player: the
/// engine reads it as a creature of `power` and `toughness`, with each
/// keyword of `has` and none of `lacks` (CR 208.1, 702).
fn reads_as_printed((card, (power, toughness), has, lacks): (Box<dyn CardBehavior>, (i32, i32), &[Keyword], &[Keyword])) {
    let (registry, mut state, _, card) = crate::proofs::cards::board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let creature = printed_creature(&mut state, &registry, card, you);

    assert!(state.is_creature(creature, &registry));
    assert!(state.effective_power(creature, &registry) == Some(power) && state.effective_toughness(creature, &registry) == Some(toughness));
    for k in has {
        assert!(state.has_keyword(creature, *k, &registry));
    }
    for k in lacks {
        assert!(!state.has_keyword(creature, *k, &registry));
    }
    std::mem::forget((registry, state));
}

/// Essence of the Wild ("Creatures you control enter as a copy of this
/// creature.") on the battlefield under the first player, a 6/6, and a
/// Doomed Traveler card put onto the battlefield from a hand, the first
/// player's (`yours`) or the other player's: its controller's enters as
/// a copy and reads 6/6 through the engine's layers (CR 614.1c, 707.2);
/// the other player's reads the 1/1 it prints.
fn essence_of_the_wild(yours: bool) {
    let (mut registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::essence_of_the_wild::EssenceOfTheWild));
    let traveler = registry.register(Box::new(crate::cards::isd::doomed_traveler::DoomedTraveler));
    printed_creature(&mut state, &registry, card, PlayerId(0));
    let (power, toughness) = registry.printed_pt(traveler, false).expect("its face");
    let entering = state.create_object(traveler, PlayerId(u8::from(!yours)), Zone::Hand, power, toughness);

    state.move_object(entering, Zone::Battlefield, &registry);

    assert!(state.get_object(entering).expect("still an object").zone == Zone::Battlefield);
    let reads = if yours { Some(6) } else { Some(1) };
    assert!(state.effective_power(entering, &registry) == reads && state.effective_toughness(entering, &registry) == reads);
    std::mem::forget((registry, state));
}

// ---- What a spell costs beside a permanent that says so, and a spell
// whose whole effect is a filter's. ----

/// Rooftop Storm ("You may pay {0} rather than pay the mana cost for
/// Zombie creature spells you cast.") under the first player: the engine
/// offers the player `caster` names, casting a creature card that is a
/// Zombie (Walking Corpse) or one that is not (Doomed Traveler), one
/// other cost, {0}, exactly for its controller's Zombie (CR 118.9), and
/// none otherwise.
fn rooftop_storm((zombie, caster): (bool, u8)) {
    use crate::types::ManaCost;
    let (mut registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::rooftop_storm::RooftopStorm));
    let corpse = registry.register(Box::new(WalkingCorpse));
    let traveler = registry.register(Box::new(crate::cards::isd::doomed_traveler::DoomedTraveler));
    a_source(&mut state, card, PlayerId(0));

    let offered = crate::engine::costs::alternative_costs(&state, &registry, if zombie { corpse } else { traveler }, PlayerId(caster));

    if zombie && caster == 0 {
        assert!(offered.len() == 1 && offered[0] == ManaCost::free());
    } else {
        assert!(offered.is_empty());
    }
    std::mem::forget((offered, registry, state));
}

/// Heartless Summoning ("Creature spells you cast cost {2} less to
/// cast.") under the first player, and what the engine says a spell
/// costs to cast (CR 601.2f): its controller's Kindercatch, {3}{G}{G}{G}
/// as printed, costs {1}{G}{G}{G}; their Walking Corpse, {1}{B}, costs
/// {B}, the reduction taking generic mana and no more than there is;
/// the other player's Kindercatch costs what it prints; and their own
/// Lightning Bolt, no creature spell, costs {R}.
fn heartless_summoning_costs(which: u8) {
    use crate::engine::costs::{cost_to_cast, CastMethod};
    use crate::types::{ManaCost, ManaSymbol};
    let (mut registry, mut state, _, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::heartless_summoning::HeartlessSummoning));
    a_source(&mut state, card, PlayerId(0));
    let green = || ManaSymbol::Colored(Color::Green);
    // The one card the row casts: the registry has room for no more.
    let (spell, caster, costs): (Box<dyn CardBehavior>, u8, Vec<ManaSymbol>) = match which {
        0 => (Box::new(Kindercatch), 0, vec![ManaSymbol::Generic(1), green(), green(), green()]),
        1 => (Box::new(WalkingCorpse), 0, vec![ManaSymbol::Colored(Color::Black)]),
        2 => (Box::new(Kindercatch), 1, vec![ManaSymbol::Generic(3), green(), green(), green()]),
        _ => (Box::new(LightningBolt), 0, vec![ManaSymbol::Colored(Color::Red)]),
    };
    let spell = registry.register(spell);

    let cost = cost_to_cast(&state, &registry, spell, PlayerId(caster), &CastMethod::Normal);

    let want = ManaCost::new(costs);
    assert!(cost.mana == want && cost.x_discount == 0);
    std::mem::forget((cost, want, registry, state));
}

/// Spare from Evil ("Creatures you control gain protection from
/// non-Human creatures until end of turn.") resolving for the first
/// player with a creature of theirs and one of the other player's: the
/// turn holds one effect, protection for their creature, from whatever
/// is a creature and not a Human, and none for the other player's.
/// What the effect is, not what the engine then reads of it: read back
/// through the engine's protection it ran out of memory at 8 GB, the
/// filter being one the card builds of two questions. What that filter
/// matches is proved of the engine (`proofs::actions::filters`).
fn spare_from_evil(_: ()) {
    use crate::types::CreatureFilter;
    let (registry, mut state, blank, card) = crate::proofs::cards::board(Box::new(crate::cards::isd::spare_from_evil::SpareFromEvil));
    let yours = a_creature_of(&mut state, blank, PlayerId(0), PlayerId(0));
    a_creature_of(&mut state, blank, PlayerId(1), PlayerId(1));
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);

    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);

    assert!(state.until_end_of_turn.len() == 1);
    match &state.until_end_of_turn[0] {
        TemporaryEffect::GrantProtection { target, filter: CreatureFilter::And(parts) } => {
            assert!(*target == yours && parts.len() == 2);
            assert!(matches!(&parts[0], CreatureFilter::HasCardType(CardType::Creature)));
            assert!(matches!(&parts[1], CreatureFilter::Not(inner) if matches!(&**inner, CreatureFilter::HasSubtype(s) if s == "Human")));
        }
        _ => panic!("protection for their creature from non-Human creatures"),
    }
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

// The Werewolves either of whose faces prints a static ability: the
// spells cast last turn fixed by each harness (`werewolf_as_cast`), and
// for Instigator Gang the transform alone (`werewolf_flips`).
engine_under_card! {
    hanweir_watchkeep_transforms_with_no_spell_cast: werewolf_as_cast((Box::new(HanweirWatchkeep), false, [0, 0]));
    hanweir_watchkeep_stays_with_a_spell_cast: werewolf_as_cast((Box::new(HanweirWatchkeep), false, [0, 1]));
    bane_of_hanweir_transforms_back_with_two_spells_cast_by_a_player: werewolf_as_cast((Box::new(HanweirWatchkeep), true, [0, 2]));
    bane_of_hanweir_stays_with_one_spell_cast_by_each_player: werewolf_as_cast((Box::new(HanweirWatchkeep), true, [1, 1]));
    instigator_gang_transforms_with_no_spell_cast: werewolf_flips((Box::new(InstigatorGang), false, [0, 0]));
    instigator_gang_stays_with_a_spell_cast: werewolf_flips((Box::new(InstigatorGang), false, [0, 1]));
    wildblood_pack_transforms_back_with_two_spells_cast_by_a_player: werewolf_flips((Box::new(InstigatorGang), true, [0, 2]));
    wildblood_pack_stays_with_one_spell_cast_by_each_player: werewolf_flips((Box::new(InstigatorGang), true, [1, 1]));
    kruin_outlaw_transforms_with_no_spell_cast: werewolf_as_cast((Box::new(KruinOutlaw), false, [0, 0]));
    kruin_outlaw_stays_with_a_spell_cast: werewolf_as_cast((Box::new(KruinOutlaw), false, [0, 1]));
    terror_of_kruin_pass_transforms_back_with_two_spells_cast_by_a_player: werewolf_as_cast((Box::new(KruinOutlaw), true, [0, 2]));
    terror_of_kruin_pass_stays_with_one_spell_cast_by_each_player: werewolf_as_cast((Box::new(KruinOutlaw), true, [1, 1]));
    mayor_of_avabruck_transforms_with_no_spell_cast: werewolf_as_cast((Box::new(MayorOfAvabruck), false, [0, 0]));
    mayor_of_avabruck_stays_with_a_spell_cast: werewolf_as_cast((Box::new(MayorOfAvabruck), false, [0, 1]));
    howlpack_alpha_transforms_back_with_two_spells_cast_by_a_player: werewolf_as_cast((Box::new(MayorOfAvabruck), true, [0, 2]));
    howlpack_alpha_stays_with_one_spell_cast_by_each_player: werewolf_as_cast((Box::new(MayorOfAvabruck), true, [1, 1]));
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
    curse_of_the_pierced_heart_deals_one_to_you_enchanted: curse_of_the_pierced_heart(0);
    curse_of_the_pierced_heart_deals_one_to_them_enchanted: curse_of_the_pierced_heart(1);
    curse_of_the_pierced_heart_asks_and_deals_one_to_the_player: curse_of_the_pierced_heart_chooses(false);
    curse_of_the_pierced_heart_asks_and_deals_one_to_their_planeswalker: curse_of_the_pierced_heart_chooses(true);
    endless_ranks_of_the_dead_makes_you_nothing_with_no_zombie: endless_ranks_of_the_dead((0, true, 0));
    endless_ranks_of_the_dead_makes_you_nothing_with_one_zombie_beside_one_of_theirs: endless_ranks_of_the_dead((1, true, 0));
    endless_ranks_of_the_dead_makes_you_a_zombie_with_two: endless_ranks_of_the_dead((2, false, 0));
    endless_ranks_of_the_dead_makes_them_a_zombie_with_two: endless_ranks_of_the_dead((2, false, 1));
}

// Permanents: end step triggers, and a cast trigger.
engine_under_card! {
    howlpack_alpha_gone_still_makes_a_wolf: howlpack_alpha(());
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
    kessig_cagebreakers_attack_alone_with_no_creature_card: kessig_cagebreakers(0);
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

use crate::cards::isd::{cloistered_youth::CloisteredYouth, thraben_sentry::ThrabenSentry};

// Permanents: triggers that ask "you may".
engine_under_card! {
    thraben_sentry_asked_and_answered_yes_transforms: may_transform((Box::new(ThrabenSentry), false, true));
    thraben_sentry_asked_and_answered_no_does_not: may_transform((Box::new(ThrabenSentry), false, false));
    thraben_militia_is_asked_nothing_by_a_second_trigger: may_transform_asks_nothing((Box::new(ThrabenSentry), false, false));
    thraben_sentry_gone_is_asked_nothing: may_transform_asks_nothing((Box::new(ThrabenSentry), false, true));
    cloistered_youth_asked_and_answered_yes_transforms: may_transform((Box::new(CloisteredYouth), true, true));
    cloistered_youth_asked_and_answered_no_does_not: may_transform((Box::new(CloisteredYouth), true, false));
    unholy_fiend_is_asked_nothing_at_upkeep: may_transform_asks_nothing((Box::new(CloisteredYouth), true, false));
    cloistered_youth_gone_is_asked_nothing: may_transform_asks_nothing((Box::new(CloisteredYouth), true, true));
    curiosity_asked_and_answered_yes_draws_your_top_card: curiosity((2, 0, true));
    curiosity_asked_and_answered_yes_draws_their_top_card: curiosity((2, 1, true));
    curiosity_asked_and_answered_no_draws_nothing_for_you: curiosity((2, 0, false));
    curiosity_asked_and_answered_no_draws_nothing_for_them: curiosity((2, 1, false));
    curiosity_asked_and_answered_yes_draws_from_your_empty_library: curiosity((0, 0, true));
    murder_of_crows_asked_and_answered_yes_draws_and_discards_your_card: murder_of_crows((2, 0, true));
    murder_of_crows_asked_and_answered_yes_draws_and_discards_their_card: murder_of_crows((2, 1, true));
    murder_of_crows_asked_and_answered_no_moves_nothing: murder_of_crows((2, 0, false));
    murder_of_crows_with_no_card_to_draw_discards_nothing: murder_of_crows((0, 1, true));
}

// Permanents: more enters triggers, watchers and abilities.
engine_under_card! {
    village_bell_ringer_untaps_your_creatures: village_bell_ringer(());
    undead_alchemist_exiles_the_card_and_makes_a_zombie: undead_alchemist(true);
    undead_alchemist_makes_a_zombie_for_a_card_gone_since: undead_alchemist(false);
    deranged_assistant_mills_nothing_from_your_empty_library: deranged_assistant((0, 0));
    deranged_assistant_mills_the_top_of_your_two: deranged_assistant((2, 0));
    deranged_assistant_mills_the_top_of_their_two: deranged_assistant((2, 1));
    cellar_door_mills_nothing_from_your_empty_library: cellar_door((0, 0, false));
    cellar_door_mills_the_bottom_of_your_two: cellar_door((2, 0, false));
    cellar_door_mills_the_bottom_of_their_two: cellar_door((2, 1, false));
    cellar_door_mills_your_creature_card_and_makes_a_zombie: cellar_door((2, 0, true));
    cellar_door_mills_their_creature_card_and_makes_you_a_zombie: cellar_door((2, 1, true));
    ludevics_test_subject_gets_a_first_hatchling_counter: ludevics_test_subject(0);
    ludevics_test_subject_gets_a_fourth_hatchling_counter: ludevics_test_subject(3);
    ludevics_test_subject_hatches_at_five: ludevics_test_subject(4);
    full_moons_rise_shields_your_werewolves: full_moons_rise(());
    grimgrin_untaps_and_gets_a_counter: grimgrin_untaps(true);
    grimgrin_gone_gets_no_counter: grimgrin_untaps(false);
    moorland_haunt_makes_a_spirit: moorland_haunt(());
    evil_twin_destroys_its_target: evil_twin_destroys(());
}

/// Two states of the game's random numbers (`GameState::rng_state`): from
/// the first, the next number is odd and a choice of one card of two at
/// random is the first; from the second it is even, and the second.
const SEED_FIRST: u64 = 0;
const SEED_SECOND: u64 = 2;

// Permanents: more damage, a coin, a random choice.
engine_under_card! {
    balefire_dragon_deals_that_much_to_the_creature_of_the_player_it_damaged: balefire_dragon(true);
    balefire_dragon_deals_nothing_to_its_own_controllers_creature: balefire_dragon(false);
    burning_vengeance_deals_two_to_a_creature: burning_vengeance(Some(Permanent::Creature));
    burning_vengeance_deals_two_to_a_planeswalker: burning_vengeance(Some(Permanent::Planeswalker));
    burning_vengeance_deals_two_to_a_player: burning_vengeance(None);
    nightfall_predator_fights_its_target: nightfall_predator_fights(());
    olivia_voldaren_bites_and_gets_a_counter: olivia_voldaren_bites(());
    creepy_doll_destroys_what_it_damaged_or_not_as_the_coin_falls: creepy_doll(());
    woodland_sleuth_returns_nothing_with_no_death: woodland_sleuth((false, 0, 0));
    woodland_sleuth_with_morbid_returns_the_first_creature_card_at_one_seed: woodland_sleuth((true, SEED_FIRST, 0));
    woodland_sleuth_with_morbid_returns_the_second_creature_card_at_another: woodland_sleuth((true, SEED_SECOND, 1));
}

use crate::cards::isd::{charmbreaker_devils::CharmbreakerDevils, ghoulraiser::Ghoulraiser, walking_corpse::WalkingCorpse};

// Permanents: exile and return, control, and cards read by what they are.
engine_under_card! {
    fiend_hunter_asks_and_exiles_its_target: fiend_hunter(());
    fiend_hunter_holds_the_card_it_exiled: fiend_hunter_holds(());
    fiend_hunter_exiles_and_returns_on_a_board_with_nothing_open: fiend_hunter_exiles_and_returns(());
    fiend_hunter_leaving_returns_what_it_exiled: fiend_hunter_leaves(true);
    fiend_hunter_leaving_returns_nothing_it_did_not_exile: fiend_hunter_leaves(false);
    olivia_voldaren_takes_a_vampire: olivia_voldaren_takes(true);
    olivia_voldaren_takes_nothing_that_is_no_vampire: olivia_voldaren_takes(false);
    witchbane_orb_destroys_the_curse_attached_to_you: witchbane_orb(());
    trepanation_blade_reveals_nothing_from_an_empty_library: trepanation_blade((0, None));
    trepanation_blade_reveals_a_land_on_top_for_one_power: trepanation_blade((2, Some(0)));
    trepanation_blade_reveals_down_to_the_land_for_two_power: trepanation_blade((3, Some(1)));
    trepanation_blade_reveals_all_of_a_library_with_no_land: trepanation_blade((2, None));
    ghoulraiser_returns_the_first_zombie_card_at_one_seed: returns_one_at_random((Box::new(Ghoulraiser), Box::new(WalkingCorpse), false, SEED_FIRST, 0));
    ghoulraiser_returns_the_second_zombie_card_at_another: returns_one_at_random((Box::new(Ghoulraiser), Box::new(WalkingCorpse), false, SEED_SECOND, 1));
    charmbreaker_devils_return_the_first_instant_at_one_seed: returns_one_at_random((Box::new(CharmbreakerDevils), Box::new(LightningBolt), true, SEED_FIRST, 0));
    charmbreaker_devils_return_the_second_instant_at_another: returns_one_at_random((Box::new(CharmbreakerDevils), Box::new(LightningBolt), true, SEED_SECOND, 1));
    mindshrieker_mills_a_card_and_grows_by_its_mana_value: mindshrieker(());
    heretics_punishment_mills_three_and_deals_the_greatest_mana_value: heretics_punishment(());
}

// Permanents: a card looked at, picked or discarded as the hook resolves.
engine_under_card! {
    delver_of_secrets_reveals_an_instant_and_transforms: delver_of_secrets((true, true));
    delver_of_secrets_keeps_an_instant_to_itself_and_stays: delver_of_secrets((true, false));
    delver_of_secrets_reveals_another_card_and_stays: delver_of_secrets((false, true));
    delver_of_secrets_asks_nothing_of_an_empty_library: delver_of_secrets_asks_nothing(false);
    delver_of_secrets_gone_asks_nothing: delver_of_secrets_asks_nothing(true);
    curse_of_oblivion_exiles_nothing_from_an_empty_graveyard: curse_of_oblivion((0, 0));
    curse_of_oblivion_exiles_the_one_card_there: curse_of_oblivion((1, 0));
    curse_of_oblivion_exiles_both_of_two_cards: curse_of_oblivion((2, 0));
    curse_of_oblivion_asks_for_two_of_three_and_spares_the_first: curse_of_oblivion((3, 0));
    curse_of_oblivion_asks_for_two_of_three_and_spares_the_last: curse_of_oblivion((3, 2));
    graveyard_shovel_exiles_nothing_from_an_empty_graveyard: graveyard_shovel((0, false, 0));
    graveyard_shovel_exiles_the_one_creature_card_for_two_life: graveyard_shovel((1, true, 0));
    graveyard_shovel_exiles_the_one_other_card_for_no_life: graveyard_shovel((1, false, 0));
    graveyard_shovel_asks_and_exiles_the_creature_card_for_two_life: graveyard_shovel((2, true, 0));
    graveyard_shovel_asks_and_exiles_the_other_card_for_no_life: graveyard_shovel((2, true, 1));
    civilized_scholar_discards_the_creature_card_it_drew_and_transforms: civilized_scholar(true);
    civilized_scholar_discards_another_card_it_drew_and_stays: civilized_scholar(false);
    civilized_scholar_asks_which_of_two_cards_to_discard: civilized_scholar_asks(());
    civilized_scholar_told_of_a_creature_card_untaps_and_transforms: civilized_scholar_is_told(true);
    civilized_scholar_told_of_another_card_stays: civilized_scholar_is_told(false);
    homicidal_brute_that_did_not_attack_taps_and_transforms: homicidal_brute(false);
    homicidal_brute_that_attacked_stays: homicidal_brute(true);
    stitchers_apprentice_makes_a_homunculus_and_sacrifices_it: stitchers_apprentice(true);
    stitchers_apprentice_makes_a_homunculus_and_sacrifices_itself: stitchers_apprentice(false);
    snapcaster_mage_grants_flashback_at_the_cards_own_cost: snapcaster_mage(());
    skeletal_grimace_shields_the_creature_it_enchants: skeletal_grimace_shields(());
    moldgraf_monstrosity_is_exiled_with_no_creature_card_to_return: moldgraf_monstrosity(0);
    moldgraf_monstrosity_returns_the_one_creature_card: moldgraf_monstrosity(1);
    moldgraf_monstrosity_returns_both_creature_cards: moldgraf_monstrosity(2);
    blazing_torch_sacrificed_deals_two_to_a_player: blazing_torch_sacrificed(false);
    blazing_torch_sacrificed_deals_two_to_a_creature: blazing_torch_sacrificed(true);
    brain_weevil_discards_nothing_from_an_empty_hand: brain_weevil(0);
    brain_weevil_discards_the_one_card_in_hand: brain_weevil(1);
    brain_weevil_discards_both_of_two_cards: brain_weevil(2);
    brain_weevil_asks_which_two_of_three: brain_weevil(3);
}

// Spells that take cards by what they are, or at random.
engine_under_card! {
    mulch_reveals_nothing_from_an_empty_library: mulch((0, [false; 4]));
    mulch_puts_the_one_land_in_hand_and_the_other_card_in_the_graveyard: mulch((2, [false, true, false, false]));
    mulch_sorts_the_top_four_and_leaves_the_fifth: mulch((5, [true, false, true, false]));
    past_in_flames_grants_flashback_to_your_instant_alone: past_in_flames(());
    make_a_wish_returns_nothing_from_an_empty_graveyard: make_a_wish(0);
    make_a_wish_returns_the_one_card_there: make_a_wish(1);
    make_a_wish_returns_both_of_two_cards: make_a_wish(2);
    make_a_wish_returns_two_of_three_cards: make_a_wish(3);
    desperate_ravings_draws_two_and_discards_the_first_at_one_seed: desperate_ravings((SEED_FIRST, 0));
    desperate_ravings_draws_two_and_discards_the_second_at_another: desperate_ravings((SEED_SECOND, 1));
    moonmist_transforms_the_human_with_two_faces_and_prevents: moonmist(());
    creeping_renaissance_asks_and_returns_the_creature_card: creeping_renaissance(false);
    creeping_renaissance_asks_and_returns_the_land_card: creeping_renaissance(true);
    night_terrors_exiles_nothing_from_a_hand_of_lands: night_terrors((0, 0));
    night_terrors_exiles_the_one_nonland_card: night_terrors((1, 0));
    night_terrors_asks_and_exiles_the_first_nonland_card: night_terrors((2, 0));
    night_terrors_asks_and_exiles_the_second_nonland_card: night_terrors((2, 1));
    forbidden_alchemy_looks_at_nothing_in_an_empty_library: forbidden_alchemy(0);
    forbidden_alchemy_puts_the_one_card_in_hand: forbidden_alchemy(1);
    forbidden_alchemy_asks_which_of_the_top_four: forbidden_alchemy(5);
    memorys_journey_shuffles_two_cards_into_their_library: memorys_journey(());
}

// Cards that search a library, and the last of the spells.
engine_under_card! {
    travelers_amulet_asks_for_the_forest_and_puts_it_in_hand: travelers_amulet(true);
    travelers_amulet_finds_no_basic_land_and_asks_nothing: travelers_amulet(false);
    ghost_quarter_destroys_the_land_and_its_controller_finds_a_forest: ghost_quarter(true);
    ghost_quarter_destroys_the_land_with_no_basic_land_to_find: ghost_quarter(false);
    caravan_vigil_puts_the_forest_in_hand: caravan_vigil((false, false));
    caravan_vigil_with_morbid_puts_the_forest_onto_the_battlefield: caravan_vigil((true, true));
    caravan_vigil_with_morbid_declined_puts_the_forest_in_hand: caravan_vigil((true, false));
    caravan_vigil_finds_no_basic_land_and_asks_nothing: caravan_vigil_finds_nothing(());
    bitterheart_witch_asked_and_answered_yes_attaches_a_curse_to_the_player: bitterheart_witch(true);
    bitterheart_witch_asked_and_answered_no_searches_nothing: bitterheart_witch(false);
    mirror_mad_phantasm_returns_at_one_seed: mirror_mad_phantasm(SEED_FIRST);
    mirror_mad_phantasm_returns_at_another_seed: mirror_mad_phantasm(SEED_SECOND);
    frightful_delusion_asks_to_pay_and_discards_nothing_from_an_empty_hand: frightful_delusion(0);
    frightful_delusion_asks_to_pay_and_discards_the_one_card: frightful_delusion(1);
    frightful_delusion_asks_to_pay_and_asks_which_of_two_cards: frightful_delusion(2);
    cackling_counterpart_makes_a_token_copy_of_your_creature: cackling_counterpart(true);
    cackling_counterpart_makes_nothing_of_a_creature_gone: cackling_counterpart(false);
    divine_reckoning_keeps_the_first_of_two_and_destroys_the_other: divine_reckoning(0);
    divine_reckoning_keeps_the_second_of_two_and_destroys_the_other: divine_reckoning(1);
    divine_reckoning_with_one_creature_each_asks_nothing: divine_reckoning_asks_nothing(());
}

// The two planeswalkers, and the cards that raise the dead.
engine_under_card! {
    garruk_relentless_deals_three_and_is_dealt_the_creatures_power: garruk_relentless_fights(());
    garruk_relentless_makes_a_green_wolf: garruk_makes_a_wolf(false);
    garruk_the_veil_cursed_makes_a_black_wolf_with_deathtouch: garruk_makes_a_wolf(true);
    garruk_relentless_at_two_loyalty_transforms: garruk_relentless_transforms(2);
    garruk_relentless_at_one_loyalty_transforms: garruk_relentless_transforms(1);
    garruk_relentless_at_three_loyalty_does_not_trigger: garruk_relentless_transforms(3);
    garruk_the_veil_cursed_sacrifices_the_one_creature_and_searches: garruk_the_veil_cursed_searches(());
    liliana_of_the_veil_edicts_a_player_with_no_creature: liliana_of_the_veil_edicts((0, 0));
    liliana_of_the_veil_edicts_a_player_with_one_creature: liliana_of_the_veil_edicts((1, 0));
    liliana_of_the_veil_edict_asks_and_sacrifices_the_first: liliana_of_the_veil_edicts((2, 0));
    liliana_of_the_veil_edict_asks_and_sacrifices_the_second: liliana_of_the_veil_edicts((2, 1));
    liliana_of_the_veil_asks_her_controller_to_divide_their_permanents: liliana_of_the_veil_divides(true);
    liliana_of_the_veil_divides_nothing_of_a_player_with_no_permanent: liliana_of_the_veil_divides(false);
    liliana_of_the_veil_makes_each_player_discard_their_one_card: liliana_of_the_veil_makes_each_discard(());
    back_from_the_brink_exiles_your_creature_card_and_makes_a_copy: back_from_the_brink(true);
    back_from_the_brink_leaves_their_creature_card_alone: back_from_the_brink(false);
    grimoire_of_the_dead_discards_the_one_card_and_studies: grimoire_of_the_dead_studies(1);
    grimoire_of_the_dead_asks_which_of_two_cards_to_discard: grimoire_of_the_dead_studies(2);
    grimoire_of_the_dead_raises_every_creature_card_as_a_black_zombie: grimoire_of_the_dead_raises(());
}

// Seven objects on the board, so its loops are unwound further.
engine_under_card! {
    unwind(9)
    garruk_the_veil_cursed_gives_your_creatures_trample_and_the_graveyards_count: garruk_the_veil_cursed_overruns(());
}

// Permanents that ask whether to pay, and the one that asks for a name.
engine_under_card! {
    mentor_of_the_meek_asked_and_paid_draws_a_card: mentor_of_the_meek(true);
    mentor_of_the_meek_asked_and_declined_draws_nothing: mentor_of_the_meek(false);
    mentor_of_the_meek_asks_nothing_with_nothing_to_pay: mentor_of_the_meek_asks_nothing(());
    mentor_of_the_meek_watches_small_creatures_of_yours_that_are_not_itself: mentor_of_the_meek_watches(());
    screeching_bat_asked_and_paid_transforms: screeching_bat((false, true));
    screeching_bat_asked_and_declined_stays: screeching_bat((false, false));
    stalking_vampire_asked_and_paid_transforms_back: screeching_bat((true, true));
    stalking_vampire_asked_and_declined_stays: screeching_bat((true, false));
    screeching_bat_asks_nothing_with_nothing_to_pay: screeching_bat_asks_nothing(false);
    screeching_bat_gone_asks_nothing: screeching_bat_asks_nothing(true);
    nevermore_asks_its_controller_for_a_nonland_card_name: nevermore(());
}

// Permanents the engine reads.
engine_under_card! {
    glorious_anthem_gives_your_creatures_one_and_one: your_creatures_get((Box::new(crate::cards::glorious_anthem::GloriousAnthem), 1));
    heartless_summoning_takes_one_and_one_from_your_creatures: your_creatures_get((Box::new(crate::cards::isd::heartless_summoning::HeartlessSummoning), -1));
    boneyard_wurm_with_no_creature_card_is_nothing_by_nothing: boneyard_wurm(0);
    boneyard_wurm_with_two_creature_cards_is_two_by_two: boneyard_wurm(2);
    scourge_of_geier_reach_alone_is_as_printed: scourge_of_geier_reach(0);
    scourge_of_geier_reach_grows_by_their_two_creatures: scourge_of_geier_reach(2);
    angelic_overseer_beside_your_human_has_hexproof_and_indestructible: angelic_overseer(true);
    angelic_overseer_beside_their_human_has_neither: angelic_overseer(false);
    night_revelers_beside_their_human_has_haste: night_revelers(true);
    night_revelers_beside_your_human_has_none: night_revelers(false);
    intangible_virtue_gives_your_tokens_one_and_one_and_vigilance: intangible_virtue(());
    elite_inquisitor_has_first_strike_and_vigilance: has_keywords((Box::new(crate::cards::isd::elite_inquisitor::EliteInquisitor), &[Keyword::FirstStrike, Keyword::Vigilance], &[Keyword::Flying, Keyword::Hexproof]));
    invisible_stalker_has_hexproof: has_keywords((Box::new(crate::cards::isd::invisible_stalker::InvisibleStalker), &[Keyword::Hexproof], &[Keyword::Flying]));
    grave_bramble_has_defender: has_keywords((Box::new(crate::cards::isd::grave_bramble::GraveBramble), &[Keyword::Defender], &[Keyword::Reach]));
    vampire_interloper_cannot_block: vampire_interloper(());
    invisible_stalker_is_blocked_by_nothing: evades((Box::new(crate::cards::isd::invisible_stalker::InvisibleStalker), false));
    orchard_spirit_is_blocked_only_by_flying_or_reach: evades((Box::new(crate::cards::isd::orchard_spirit::OrchardSpirit), true));
    laboratory_maniac_wins_for_its_controller_drawing_from_an_empty_library: laboratory_maniac((0, 0));
    laboratory_maniac_leaves_the_other_player_to_draw_from_an_empty_library: laboratory_maniac((0, 1));
    laboratory_maniac_leaves_a_draw_from_a_library_with_a_card_alone: laboratory_maniac((1, 0));
    avacyns_pilgrim_taps_for_white: taps_for((Box::new(crate::cards::isd::avacyns_pilgrim::AvacynsPilgrim), ManaType::White, 1));
    sol_ring_taps_for_two_colorless: taps_for((Box::new(crate::cards::sol_ring::SolRing), ManaType::Colorless, 2));
}

use crate::cards::isd::{clifftop_retreat::ClifftopRetreat, hinterland_harbor::HinterlandHarbor, isolated_chapel::IsolatedChapel,
    sulfur_falls::SulfurFalls, woodland_cemetery::WoodlandCemetery};
use crate::cards::{forest::Forest, island::Island, mountain::Mountain, plains::Plains, swamp::Swamp};

// Permanents that enter otherwise than a card does, and what replaces an event.
engine_under_card! {
    diregraf_ghoul_enters_tapped: diregraf_ghoul(());
    clifftop_retreat_enters_untapped_beside_your_mountain: check_land((Box::new(ClifftopRetreat), Box::new(Mountain), true));
    clifftop_retreat_enters_untapped_beside_your_plains: check_land((Box::new(ClifftopRetreat), Box::new(Plains), true));
    clifftop_retreat_enters_tapped_beside_their_mountain: check_land((Box::new(ClifftopRetreat), Box::new(Mountain), false));
    hinterland_harbor_enters_untapped_beside_your_forest: check_land((Box::new(HinterlandHarbor), Box::new(Forest), true));
    hinterland_harbor_enters_untapped_beside_your_island: check_land((Box::new(HinterlandHarbor), Box::new(Island), true));
    hinterland_harbor_enters_tapped_beside_their_forest: check_land((Box::new(HinterlandHarbor), Box::new(Forest), false));
    isolated_chapel_enters_untapped_beside_your_plains: check_land((Box::new(IsolatedChapel), Box::new(Plains), true));
    isolated_chapel_enters_untapped_beside_your_swamp: check_land((Box::new(IsolatedChapel), Box::new(Swamp), true));
    isolated_chapel_enters_tapped_beside_their_plains: check_land((Box::new(IsolatedChapel), Box::new(Plains), false));
    sulfur_falls_enters_untapped_beside_your_island: check_land((Box::new(SulfurFalls), Box::new(Island), true));
    sulfur_falls_enters_untapped_beside_your_mountain: check_land((Box::new(SulfurFalls), Box::new(Mountain), true));
    sulfur_falls_enters_tapped_beside_their_island: check_land((Box::new(SulfurFalls), Box::new(Island), false));
    woodland_cemetery_enters_untapped_beside_your_swamp: check_land((Box::new(WoodlandCemetery), Box::new(Swamp), true));
    woodland_cemetery_enters_untapped_beside_your_forest: check_land((Box::new(WoodlandCemetery), Box::new(Forest), true));
    woodland_cemetery_enters_tapped_beside_their_swamp: check_land((Box::new(WoodlandCemetery), Box::new(Swamp), false));
    festerhide_boar_enters_with_two_counters_after_a_death: morbid_counters((Box::new(crate::cards::isd::festerhide_boar::FesterhideBoar), true));
    festerhide_boar_enters_with_none_without_one: morbid_counters((Box::new(crate::cards::isd::festerhide_boar::FesterhideBoar), false));
    somberwald_spider_enters_with_two_counters_after_a_death: morbid_counters((Box::new(crate::cards::isd::somberwald_spider::SomberwaldSpider), true));
    somberwald_spider_enters_with_none_without_one: morbid_counters((Box::new(crate::cards::isd::somberwald_spider::SomberwaldSpider), false));
    unbreathing_horde_enters_with_a_counter_for_each_of_your_zombies: unbreathing_horde_enters(());
    unbreathing_horde_loses_a_counter_in_place_of_damage: unbreathing_horde_prevents(());
    dearly_departed_in_your_graveyard_gives_your_human_a_counter: dearly_departed((true, true));
    dearly_departed_in_your_graveyard_gives_another_creature_none: dearly_departed((true, false));
    dearly_departed_in_their_graveyard_gives_your_human_none: dearly_departed((false, true));
    parallel_lives_makes_two_tokens_for_its_controller: parallel_lives(0);
    parallel_lives_makes_one_token_for_the_other_player: parallel_lives(1);
    stony_silence_on_the_battlefield_forbids_artifact_abilities: stony_silence(true);
    stony_silence_in_a_graveyard_forbids_nothing: stony_silence(false);
    elite_inquisitor_is_protected_from_vampires: protected_from((Box::new(crate::cards::isd::elite_inquisitor::EliteInquisitor), "Vampire", true));
    elite_inquisitor_is_protected_from_werewolves: protected_from((Box::new(crate::cards::isd::elite_inquisitor::EliteInquisitor), "Werewolf", true));
    elite_inquisitor_is_protected_from_zombies: protected_from((Box::new(crate::cards::isd::elite_inquisitor::EliteInquisitor), "Zombie", true));
    elite_inquisitor_is_not_protected_from_humans: protected_from((Box::new(crate::cards::isd::elite_inquisitor::EliteInquisitor), "Human", false));
    grave_bramble_is_protected_from_zombies: protected_from((Box::new(crate::cards::isd::grave_bramble::GraveBramble), "Zombie", true));
    grave_bramble_is_not_protected_from_vampires: protected_from((Box::new(crate::cards::isd::grave_bramble::GraveBramble), "Vampire", false));
    forest_taps_for_green: taps_for((Box::new(Forest), ManaType::Green, 1));
    island_taps_for_blue: taps_for((Box::new(Island), ManaType::Blue, 1));
    mountain_taps_for_red: taps_for((Box::new(Mountain), ManaType::Red, 1));
    plains_taps_for_white: taps_for((Box::new(Plains), ManaType::White, 1));
    swamp_taps_for_black: taps_for((Box::new(Swamp), ManaType::Black, 1));
}

use crate::cards::isd::{abbey_griffin::AbbeyGriffin, ambush_viper::AmbushViper, chapel_geist::ChapelGeist, fortress_crab::FortressCrab,
    kindercatch::Kindercatch, makeshift_mauler::MakeshiftMauler, markov_patrician::MarkovPatrician, moon_heron::MoonHeron,
    riot_devils::RiotDevils, rotting_fensnake::RottingFensnake, skaab_goliath::SkaabGoliath, spectral_rider::SpectralRider,
    stitched_drake::StitchedDrake, thraben_purebloods::ThrabenPurebloods, typhoid_rats::TyphoidRats, voiceless_spirit::VoicelessSpirit};
use crate::cards::{coral_merfolk::CoralMerfolk, goblin_piker::GoblinPiker, grizzly_bears::GrizzlyBears, kalonian_tusker::KalonianTusker,
    savannah_lions::SavannahLions};

// Creatures that are what they print, and the last two that replace or tap.
engine_under_card! {
    abbey_griffin_is_a_two_two_with_flying_and_vigilance: reads_as_printed((Box::new(AbbeyGriffin), (2, 2), &[Keyword::Flying, Keyword::Vigilance], &[Keyword::Deathtouch]));
    ambush_viper_is_a_two_one_with_flash_and_deathtouch: reads_as_printed((Box::new(AmbushViper), (2, 1), &[Keyword::Flash, Keyword::Deathtouch], &[Keyword::Flying]));
    chapel_geist_is_a_two_three_with_flying: reads_as_printed((Box::new(ChapelGeist), (2, 3), &[Keyword::Flying], &[Keyword::Vigilance]));
    coral_merfolk_is_a_two_one: reads_as_printed((Box::new(CoralMerfolk), (2, 1), &[], &[Keyword::Flying]));
    fortress_crab_is_a_one_six: reads_as_printed((Box::new(FortressCrab), (1, 6), &[], &[Keyword::Defender]));
    goblin_piker_is_a_two_one: reads_as_printed((Box::new(GoblinPiker), (2, 1), &[], &[Keyword::Haste]));
    grizzly_bears_is_a_two_two: reads_as_printed((Box::new(GrizzlyBears), (2, 2), &[], &[Keyword::Trample]));
    kalonian_tusker_is_a_three_three: reads_as_printed((Box::new(KalonianTusker), (3, 3), &[], &[Keyword::Trample]));
    kindercatch_is_a_six_six: reads_as_printed((Box::new(Kindercatch), (6, 6), &[], &[Keyword::Flying]));
    markov_patrician_is_a_three_one_with_lifelink: reads_as_printed((Box::new(MarkovPatrician), (3, 1), &[Keyword::Lifelink], &[Keyword::Flying]));
    moon_heron_is_a_three_two_with_flying: reads_as_printed((Box::new(MoonHeron), (3, 2), &[Keyword::Flying], &[Keyword::Vigilance]));
    riot_devils_is_a_two_three: reads_as_printed((Box::new(RiotDevils), (2, 3), &[], &[Keyword::Haste]));
    rotting_fensnake_is_a_five_one: reads_as_printed((Box::new(RottingFensnake), (5, 1), &[], &[Keyword::Deathtouch]));
    savannah_lions_is_a_two_one: reads_as_printed((Box::new(SavannahLions), (2, 1), &[], &[Keyword::FirstStrike]));
    spectral_rider_is_a_two_two_with_intimidate: reads_as_printed((Box::new(SpectralRider), (2, 2), &[Keyword::Intimidate], &[Keyword::Flying]));
    thraben_purebloods_is_a_three_five: reads_as_printed((Box::new(ThrabenPurebloods), (3, 5), &[], &[Keyword::Vigilance]));
    typhoid_rats_is_a_one_one_with_deathtouch: reads_as_printed((Box::new(TyphoidRats), (1, 1), &[Keyword::Deathtouch], &[Keyword::Flying]));
    voiceless_spirit_is_a_two_one_with_flying_and_first_strike: reads_as_printed((Box::new(VoicelessSpirit), (2, 1), &[Keyword::Flying, Keyword::FirstStrike], &[Keyword::Vigilance]));
    walking_corpse_is_a_two_two: reads_as_printed((Box::new(WalkingCorpse), (2, 2), &[], &[Keyword::Deathtouch]));
    skaab_goliath_is_a_six_nine_with_trample: reads_as_printed((Box::new(SkaabGoliath), (6, 9), &[Keyword::Trample], &[Keyword::Flying]));
    makeshift_mauler_is_a_four_five: reads_as_printed((Box::new(MakeshiftMauler), (4, 5), &[], &[Keyword::Trample]));
    stitched_drake_is_a_three_four_with_flying: reads_as_printed((Box::new(StitchedDrake), (3, 4), &[Keyword::Flying], &[Keyword::Trample]));
    // The body of each of four cards whose own ability is not proved
    // here: the two Spirit lords (their filter, above), Skaab Ruinator (a
    // cost, and a cast from a graveyard, which are the engine's) and
    // Geist of Saint Traft (its Angel enters attacking). One-Eyed
    // Scarecrow has not even this: its filter asks of each creature
    // whether it has flying, the Scarecrow itself as its own power is
    // read, and alone on the board that ran out of memory at 7 GB.
    battleground_geist_is_a_three_three_with_flying: reads_as_printed((Box::new(crate::cards::isd::battleground_geist::BattlegroundGeist), (3, 3), &[Keyword::Flying], &[Keyword::Vigilance]));
    gallows_warden_is_a_three_three_with_flying: reads_as_printed((Box::new(crate::cards::isd::gallows_warden::GallowsWarden), (3, 3), &[Keyword::Flying], &[Keyword::Vigilance]));
    skaab_ruinator_is_a_five_six_with_flying: reads_as_printed((Box::new(crate::cards::isd::skaab_ruinator::SkaabRuinator), (5, 6), &[Keyword::Flying], &[Keyword::Trample]));
    geist_of_saint_traft_is_a_two_two_with_hexproof: reads_as_printed((Box::new(crate::cards::isd::geist_of_saint_traft::GeistOfSaintTraft), (2, 2), &[Keyword::Hexproof], &[Keyword::Flying]));
    essence_of_the_wild_makes_your_creature_enter_as_a_copy: essence_of_the_wild(true);
    essence_of_the_wild_leaves_their_creature_as_it_is: essence_of_the_wild(false);
    shimmering_grotto_taps_for_colorless: taps_for((Box::new(crate::cards::isd::shimmering_grotto::ShimmeringGrotto), ManaType::Colorless, 1));
}

// What a spell costs beside a permanent that says so, and a spell whose effect is a filter's.
engine_under_card! {
    rooftop_storm_offers_its_controller_a_zombie_for_nothing: rooftop_storm((true, 0));
    rooftop_storm_offers_nothing_for_a_creature_that_is_no_zombie: rooftop_storm((false, 0));
    rooftop_storm_offers_the_other_player_nothing: rooftop_storm((true, 1));
    heartless_summoning_takes_two_generic_from_your_creature_spell: heartless_summoning_costs(0);
    heartless_summoning_takes_no_more_generic_than_there_is: heartless_summoning_costs(1);
    heartless_summoning_leaves_the_other_players_creature_spell: heartless_summoning_costs(2);
    heartless_summoning_leaves_a_spell_that_is_no_creature: heartless_summoning_costs(3);
    spare_from_evil_gives_your_creature_protection_from_non_human_creatures: spare_from_evil(());
}
