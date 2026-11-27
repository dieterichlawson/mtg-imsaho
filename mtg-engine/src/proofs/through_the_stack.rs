//! A card taken through the game as the engine plays it, nothing stood in
//! for: cast from where its owner casts it (CR 601.2: its targets checked,
//! its costs paid, the spell put on the stack), each player passing in
//! turn (117.3-117.4), resolved off the stack (608.2: its targets checked
//! again, what its text says done, the card put where a resolved spell
//! goes), and the state-based actions and triggers that come before each
//! player would receive priority (117.5, 704.3).
//!
//! `proofs::engine_under_card` calls a card's own hook with the engine's
//! actions under it. That shows what the hook does; it does not show that
//! the engine, handed the card in a game, calls that hook with those
//! targets on that state, or what the engine does around it. Here each
//! harness plays the actions the game loop would
//! (`engine::run_game_loop_inner`), each as the loop plays it ([`play`]):
//! `before_priority`, the action itself by `submit_action_on` (all of
//! `submit_action` but its copy of the game, which is more than symbolic
//! execution gets through, and which `submit_action` is defined as), and
//! `hand_on_priority`, which resolves the top of the stack when every
//! player has passed.
//!
//! What a harness here leaves to other proofs: which actions the engine
//! offers a player. The loop asks `legal_actions` and plays only an
//! action it offered; a harness names the cast and the passes itself,
//! because `legal_actions` alone, on a board of three objects, ran out of
//! memory at 11 GB (`docs/plans/verification-goals.md` has this and every
//! other measurement these were shaped by). What it offers is proved of
//! it in `proofs::actions::offering` and `priority`. Only `format!`, an
//! object's name for the log, the log and the allocator's growing and
//! freeing are taken out, as in `engine_under_card`.
//!
//! Each row is one case of one card, nothing on its board left to the
//! solver: which player casts, whose the target is, what the target is, a
//! target the card's text does not allow (the engine refuses the cast,
//! 601.2c), the target destroyed in response by a spell of the other
//! player's (608.2b), a flashback cast from the graveyard (702.34a), an
//! amount at none, one and more. What a card's rows cover can be read off
//! their names.
//!
//! Not every row has been proved: `scripts/kani/times.txt` records each
//! one's last result, NOT_RUN for one written and not yet run to a result,
//! and `reports/verification-scoreboard.md` counts them by card. On a
//! machine of 15 GB a spell goes through every pass whether a creature
//! dies of it or not, and so does a cast the engine refuses.
//!
//! These need `scripts/precise-enums.py --wide-pointer-free`, which
//! `scripts/kani-bounded.py` gives every harness of this module: without
//! it the entry a cast puts on the stack is read back as an entry of any
//! kind, and the resolution does not fit in memory.

use crate::actions::{Action, Target};
use crate::cards::{CardBehavior, CardRegistry};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{CardType, ManaSymbol, ManaType, Step, Zone};

/// One action as a pass of the game loop plays it: what is checked before
/// a player receives priority (CR 117.5), the action, and priority handed
/// on (117.3-117.4), the top of the stack resolved if that was the last
/// player's pass.
fn play(state: &mut GameState, registry: &CardRegistry, action: &Action) {
    if IN_THE_GAME_LOOP.load(core::sync::atomic::Ordering::Relaxed) {
        return play_in_the_game_loop(state, registry, action);
    }
    crate::engine::before_priority(state, registry);
    let cast_prompt_player = crate::engine::cast_time_prompt_player(state);
    crate::engine::submit_action_on(state, action, registry);
    crate::engine::hand_on_priority(state, action, cast_prompt_player, 2, registry);
}

/// Whether [`play`] hands its action to the engine's own game loop
/// ([`play_in_the_game_loop`]): set by a harness of [`in_the_game_loop`]
/// before it does anything else, and by no other.
static IN_THE_GAME_LOOP: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// One action played by the engine's own game loop, the harness in the
/// seat and nowhere else: `engine::resume_game_loop`, the loop every
/// runner of a game calls, is given the state and a seat that answers
/// `action` the first time it is asked for a decision and stops the
/// harness the second (`AbandonGame`, as a runner stops a game it has
/// seen enough of). Everything between is the engine's and none of it the
/// harness's: what is checked before a player receives priority, the
/// actions it offers (`legal_actions`), its refusal to play an answer it
/// did not offer, `submit_action` with its copy of the game, and priority
/// handed on.
///
/// The loop passes for a player it offers nothing worth a decision. With
/// `observe_every_submit`, the engine's own setting for a runner that
/// wants to see every action, it asks the seat for those passes too:
/// without it a harness could not stop the game after one decision, and
/// the loop would play on through the turn.
fn play_in_the_game_loop(state: &mut GameState, registry: &CardRegistry, action: &Action) {
    let mut answered = false;
    state.observe_every_submit = true;
    crate::engine::resume_game_loop(state, registry, |_, _, _| {
        if answered {
            Action::AbandonGame
        } else {
            answered = true;
            action.clone()
        }
    });
    assert!(answered);
}

/// A registry of the blank card and `card`, and a two-player game: the
/// blank card's id and `card`'s. As `proofs::cards::board`, with room for
/// the events of a spell's whole life: a list that outgrows what it was
/// given is a failure (`proofs::cards::no_growing`).
fn board(card: Box<dyn CardBehavior>) -> (CardRegistry, GameState, CardId, CardId) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(crate::proofs::cards::Blank));
    let card = registry.register(card);
    (registry, a_game(), blank, card)
}

fn a_game() -> GameState {
    let mut state = GameState::new(2);
    state.objects.reserve(24);
    state.until_end_of_turn.reserve(8);
    // Room for 32 events and no more. The list is an action's own (each
    // submitted action clears it), so that is room enough; and with room
    // for 40, an event read back from the list is of a kind symbolic
    // execution does not know, the drop of the list and the collecting of
    // triggers follow every kind for each event, and Doom Blade's cast,
    // resolution and the check before priority after ran out of memory at
    // 11 GB. Measured with one event in an otherwise empty list
    // (2026-11-27): its kind is known with room for 8, 10, 16, 21, 22 and
    // 32, and not with room for 40, 42 or 43.
    state.events.reserve(32);
    state.pending_damage.reserve(4);
    state.end_of_combat_exiles.reserve(2);
    state.stack.reserve(4);
    for p in &mut state.players {
        // Twenty life each: a new game's players start at Commander's forty.
        p.life = 20;
        p.library_order.reserve(6);
        p.graveyard_order.reserve(6);
    }
    state
}

/// How a spell is cast: by which player, and from their hand for its
/// printed cost or from their graveyard for its flashback cost (CR
/// 702.34a).
#[derive(Clone, Copy)]
struct Cast {
    by: u8,
    flashback: bool,
}

const FIRST: Cast = Cast { by: 0, flashback: false };
const SECOND: Cast = Cast { by: 1, flashback: false };
const FLASHBACK: Cast = Cast { by: 0, flashback: true };

/// The mana a cost of these symbols asks for, in `player`'s pool: each
/// coloured pip as that colour, generic mana as colourless.
fn mana_for_symbols(state: &mut GameState, player: PlayerId, symbols: &[ManaSymbol]) {
    let pool = &mut state.players[usize::from(player.0)].mana_pool;
    for symbol in symbols {
        match *symbol {
            ManaSymbol::Colored(c) => pool.add(ManaType::from(c), 1),
            ManaSymbol::Generic(n) => pool.add(ManaType::Colorless, n),
            _ => panic!("a cost of coloured and generic mana"),
        }
    }
}

/// `card` in `player`'s hand, and the mana its printed cost asks for in
/// their pool: the cost the Oracle prints for it ([`oracle::cost`]), not
/// the one the engine's data gives it. A cast then goes through, and
/// leaves the pool empty, only if the two are the same cost.
fn holds(state: &mut GameState, registry: &CardRegistry, card: CardId, player: PlayerId) -> ObjectId {
    let spell = state.create_object(card, player, Zone::Hand, None, None);
    let name = registry.get(card).expect("registered").card_data().name;
    mana_for_symbols(state, player, oracle::cost(&name));
    std::mem::forget(name);
    spell
}

/// `card` where `cast` casts it from, its caster's own first main phase,
/// the stack empty and priority theirs, and the mana for that cast in
/// their pool: in their hand with its printed cost, or in their graveyard
/// with its printed flashback cost, each as the Oracle has it.
fn a_spell(state: &mut GameState, registry: &CardRegistry, card: CardId, cast: Cast) -> ObjectId {
    let caster = PlayerId(cast.by);
    state.active_player = caster;
    state.step = Step::PrecombatMain;
    state.priority_player = Some(caster);
    if !cast.flashback {
        return holds(state, registry, card, caster);
    }
    let spell = state.create_object(card, caster, Zone::Graveyard, None, None);
    if cast.by == 0 {
        state.players[0].graveyard_order.push(spell);
    } else {
        state.players[1].graveyard_order.push(spell);
    }
    let name = registry.get(card).expect("registered").card_data().name;
    mana_for_symbols(state, caster, oracle::flashback_cost(&name));
    std::mem::forget(name);
    spell
}

/// Whether `spell` is where a spell cast as `cast` says rests once it has
/// left the stack: its owner's graveyard (CR 608.2n), or exile and nowhere
/// else if it was cast with flashback (702.34a).
fn rests(state: &GameState, spell: ObjectId, cast: Cast) -> bool {
    let zone = state.get_object(spell).expect("still an object").zone;
    let buried = state.players[usize::from(cast.by)].graveyard_order.contains(&spell);
    if cast.flashback { zone == Zone::Exile && !buried } else { zone == Zone::Graveyard && buried }
}

/// The cast of `spell` at `targets`, with no additional cost named.
fn the_cast(spell: ObjectId, targets: Vec<Target>) -> Action {
    Action::CastSpell { object_id: spell, targets, sacrifice: None, exile_count: None, exile_ids: Vec::new(), alternative_cost: None, tap_plan: Vec::new() }
}

/// `cast` (a `CastSpell`) played by the player with priority: the spell is
/// the top of the stack, one entry more than before, its caster's pool is
/// empty, and its caster still has priority (CR 601.2i, 117.3c).
fn casts_as(state: &mut GameState, registry: &CardRegistry, cast: Action) {
    let Action::CastSpell { object_id: spell, .. } = cast else { panic!("a cast") };
    let caster = state.priority_player.expect("the caster has priority");
    let depth = state.stack.len();
    play(state, registry, &cast);
    assert!(state.get_object(spell).expect("still an object").zone == Zone::Stack && state.stack.len() == depth + 1);
    assert!(state.stack[depth].as_spell() == Some(spell));
    assert!(state.players[usize::from(caster.0)].mana_pool.total() == 0 && state.priority_player == Some(caster));
    std::mem::forget(cast);
}

/// `spell` cast at `targets` by the player with priority ([`casts_as`]).
fn casts(state: &mut GameState, registry: &CardRegistry, spell: ObjectId, targets: Vec<Target>) {
    casts_as(state, registry, the_cast(spell, targets));
}

/// Each player passes in turn, the one with priority first (CR 117.4): the
/// top of the stack resolves, and what is checked before the next player
/// would receive priority is checked (117.5). The stack stays as it is
/// through the first pass, and priority is the active player's after
/// (117.3b).
fn all_pass(state: &mut GameState, registry: &CardRegistry) {
    let first = state.priority_player.expect("someone has priority");
    let depth = state.stack.len();
    play(state, registry, &Action::PassPriority);
    assert!(state.priority_player == Some(PlayerId(1 - first.0)) && state.stack.len() == depth);
    play(state, registry, &Action::PassPriority);
    crate::engine::before_priority(state, registry);
    assert!(state.priority_player == Some(state.active_player));
}

/// `spell` cast at `targets` by the player with priority, each player's
/// pass in turn, and what is checked before the next player would receive
/// priority: the whole of a spell's life from a hand to where it rests.
fn cast_and_resolve(state: &mut GameState, registry: &CardRegistry, spell: ObjectId, targets: Vec<Target>) {
    cast_and_resolve_as(state, registry, the_cast(spell, targets));
}

/// The same of a cast with its additional costs named.
fn cast_and_resolve_as(state: &mut GameState, registry: &CardRegistry, cast: Action) {
    casts_as(state, registry, cast);
    assert!(state.stack.len() == 1);
    all_pass(state, registry);
    assert!(state.stack.is_empty());
}

/// "Draw N cards." cast and resolved by `caster`, whose library holds
/// `cards` cards and the other player's one: the top N of the caster's
/// are in their hand in order, the rest of their library as it was, the
/// other player's untouched, the spell in its owner's graveyard and
/// nowhere else, and a caster who drew from a library with fewer than N
/// has lost (CR 121.4, 704.5b), the state-based actions having been
/// checked.
fn draws((card, n, cards, cast): (Box<dyn CardBehavior>, usize, usize, Cast)) {
    let caster = cast.by;
    let (registry, mut state, blank, card) = board(card);
    let mut libraries = [[None; 3]; 2];
    for p in 0..2u8 {
        // The other player's library is one card, to be found untouched.
        for k in 0..(if p == caster { cards } else { 1 }) {
            let id = state.create_object(blank, PlayerId(p), Zone::Library, None, None);
            if p == 0 {
                state.players[0].library_order.push(id);
            } else {
                state.players[1].library_order.push(id);
            }
            libraries[p as usize][k] = Some(id);
        }
    }
    let (mine, theirs) = (usize::from(caster), usize::from(1 - caster));
    let spell = a_spell(&mut state, &registry, card, cast);

    if cards >= n {
        cast_and_resolve(&mut state, &registry, spell, Vec::new());
    } else {
        // A draw from a library too small ends the game as the state-based
        // actions are checked, before anyone has priority again.
        let action = the_cast(spell, Vec::new());
        play(&mut state, &registry, &action);
        play(&mut state, &registry, &Action::PassPriority);
        play(&mut state, &registry, &Action::PassPriority);
        crate::engine::before_priority(&mut state, &registry);
        std::mem::forget(action);
    }

    assert!(state.get_object(libraries[theirs][0].expect("made")).expect("still an object").zone == Zone::Library);
    for k in 0..cards {
        let o = state.get_object(libraries[mine][k].expect("made")).expect("still an object");
        assert!(o.zone == if k < n { Zone::Hand } else { Zone::Library });
        assert!(o.controller == PlayerId(caster));
    }
    assert!(state.players[theirs].library_order.len() == 1);
    assert!(state.players[mine].library_order.len() == cards.saturating_sub(n));
    if cards > n {
        assert!(state.players[mine].library_order[0] == libraries[mine][n].expect("made"));
    }
    assert!(rests(&state, spell, cast) && state.get_object(spell).expect("still an object").owner == PlayerId(caster));
    assert!(state.players[mine].graveyard_order.len() == usize::from(!cast.flashback));
    assert!(state.players[mine].lost == (cards < n) && !state.players[theirs].lost);
    std::mem::forget((registry, state));
}

/// `spell` cast at `targets` the engine does not allow (CR 601.2c): the
/// cast is refused and nothing is done. The card is still where it was,
/// the mana still in the pool, the stack empty and priority where it was.
fn cast_is_refused(state: &mut GameState, registry: &CardRegistry, spell: ObjectId, targets: Vec<Target>) {
    cast_is_refused_as(state, registry, the_cast(spell, targets));
}

/// The same of a cast with its additional costs named (601.2h).
fn cast_is_refused_as(state: &mut GameState, registry: &CardRegistry, cast: Action) {
    let Action::CastSpell { object_id: spell, .. } = cast else { panic!("a cast") };
    let caster = state.priority_player.expect("the caster has priority");
    let (pool, zone) = (state.players[usize::from(caster.0)].mana_pool.total(), state.get_object(spell).expect("an object").zone);
    let before = everything(state);
    play(state, registry, &cast);
    assert!(state.get_object(spell).expect("still an object").zone == zone && state.stack.is_empty() && state.awaiting_action.is_none());
    assert!(state.players[usize::from(caster.0)].mana_pool.total() == pool && state.priority_player == Some(caster));
    // And nothing else of the game is other than it was.
    assert!(everything(state) == before);
    std::mem::forget(cast);
}

/// What a game can see of one object: where it is and whose, and what a
/// permanent carries. Not what the engine keeps for its own use (how
/// often it has changed zone, what it last was): a rule reads none of it.
#[derive(Clone, Copy, PartialEq)]
struct Seen {
    zone: Zone,
    owner: PlayerId,
    controller: PlayerId,
    tapped: bool,
    damage: u32,
    /// Came under its controller's control this turn (CR 302.6): asked
    /// only of a permanent.
    sick: bool,
    power: Option<i32>,
    toughness: Option<i32>,
    counters: u32,
    shields: u32,
    attached_to: Option<ObjectId>,
    attached_to_player: Option<PlayerId>,
    transformed: bool,
    token: bool,
}

/// What a game can see of one player.
#[derive(Clone, Copy, PartialEq)]
struct SeenOfAPlayer {
    life: i32,
    lost: bool,
    mana: u32,
    land_plays: u32,
    library: [Option<ObjectId>; 6],
    graveyard: [Option<ObjectId>; 6],
}

/// The whole game as a rule can see it: every object, each player, and
/// whose turn and priority it is. A proof takes it before a spell is
/// cast, changes in it what the card's text says is done, and asks that
/// the game after is that and nothing else ([`everything`]): "and
/// nothing else is done" of the whole game, where a neighbour and the
/// life totals were looked at.
#[derive(Clone, Copy, PartialEq)]
struct Everything {
    objects: [Option<Seen>; 24],
    players: [SeenOfAPlayer; 2],
    stack: usize,
    effects_until_end_of_turn: usize,
    damage_waiting: usize,
    turn: u32,
    step: Step,
    active: PlayerId,
    priority: Option<PlayerId>,
    asked: bool,
    a_creature_died: bool,
}

fn everything(state: &GameState) -> Everything {
    let mut objects = [None; 24];
    for o in state.objects.values() {
        let at = usize::try_from(o.id.0).expect("a small id");
        assert!(at < 24);
        objects[at] = Some(Seen {
            zone: o.zone,
            owner: o.owner,
            controller: o.controller,
            tapped: o.tapped,
            damage: o.damage_marked,
            sick: o.zone == Zone::Battlefield && o.summoning_sick,
            power: o.power,
            toughness: o.toughness,
            counters: o.counters.values().sum(),
            shields: o.regeneration_shields,
            attached_to: o.attached_to,
            attached_to_player: o.attached_to_player,
            transformed: o.is_transformed,
            token: o.is_token,
        });
    }
    let player = |p: usize| {
        let (mut library, mut graveyard) = ([None; 6], [None; 6]);
        let of = &state.players[p];
        assert!(of.library_order.len() <= 6 && of.graveyard_order.len() <= 6);
        for (i, id) in of.library_order.iter().enumerate() {
            library[i] = Some(*id);
        }
        for (i, id) in of.graveyard_order.iter().enumerate() {
            graveyard[i] = Some(*id);
        }
        SeenOfAPlayer { life: of.life, lost: of.lost, mana: of.mana_pool.total(), land_plays: of.land_plays_remaining, library, graveyard }
    };
    Everything {
        objects,
        players: [player(0), player(1)],
        stack: state.stack.len(),
        effects_until_end_of_turn: state.until_end_of_turn.len(),
        damage_waiting: state.pending_damage.len(),
        turn: state.turn_number,
        step: state.step,
        active: state.active_player,
        priority: state.priority_player,
        asked: state.awaiting_action.is_some(),
        a_creature_died: state.creature_died_this_turn,
    }
}

impl Everything {
    /// What is seen of `id`, to be changed to what a card's text says.
    fn of(&mut self, id: ObjectId) -> &mut Seen {
        self.objects[usize::try_from(id.0).expect("a small id")].as_mut().expect("an object of the game")
    }
}

/// A harness for each row, as in `engine_under_card`.
macro_rules! through_the_stack {
    ($($name:ident: $proof:ident($open:expr);)*) => {
        through_the_stack! { unwind(16) $($name: $proof($open);)* }
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

/// The same, each action played by the engine's own game loop
/// ([`play_in_the_game_loop`]).
macro_rules! in_the_game_loop {
    ($($name:ident: $proof:ident($open:expr);)*) => {$(
        #[kani::proof]
        #[kani::unwind(16)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        fn $name() {
            IN_THE_GAME_LOOP.store(true, core::sync::atomic::Ordering::Relaxed);
            $proof($open);
        }
    )*};
}
use crate::cards::divination::Divination;

// "Draw two cards.": each player casting, a library of none, one, two and three.
through_the_stack! {
    divination_cast_by_the_first_player_draws_the_top_two_of_three: draws((Box::new(Divination), 2, 3, FIRST));
    divination_cast_by_the_second_player_draws_the_top_two_of_three: draws((Box::new(Divination), 2, 3, SECOND));
    divination_cast_by_the_first_player_draws_their_whole_library_of_two: draws((Box::new(Divination), 2, 2, FIRST));
    divination_cast_by_the_first_player_with_one_card_draws_it_and_loses: draws((Box::new(Divination), 2, 1, FIRST));
}

use crate::cards::{lava_axe::LavaAxe, lightning_bolt::LightningBolt};

/// What a permanent on a board here is: a permanent of the blank card,
/// made each of these.
#[derive(Clone, Copy, PartialEq)]
enum Permanent {
    /// A creature, a 2/2.
    Creature,
    /// A black creature (Doom Blade).
    Black,
    /// A creature of power 4, a 4/2 (Smite the Monstrous).
    Monstrous,
    /// A creature of toughness 6, a 2/6: one the pool's damage spells do
    /// not destroy.
    Stout,
    /// A Spirit creature (Urgent Exorcism).
    Spirit,
    /// A Human creature.
    Human,
    /// A Vampire, a Werewolf and a Zombie creature (Victim of Night).
    Vampire,
    Werewolf,
    Zombie,
    /// A creature attacking the player who does not control it (Rebuke).
    Attacker,
    Artifact,
    Enchantment,
    Land,
    /// A planeswalker with three loyalty counters.
    Planeswalker,
}

impl Permanent {
    fn is_creature(self) -> bool {
        !matches!(self, Permanent::Artifact | Permanent::Enchantment | Permanent::Land | Permanent::Planeswalker)
    }
}

/// A permanent of the blank card on the battlefield, `owner`'s card under
/// `controller`, `what` it is, untapped and undamaged; a creature is a 2/2
/// (a 4/2 if `Monstrous`, a 2/6 if `Stout`). An attacker is attacking the
/// player who does not control it. Nothing about it is left to the
/// solver: what varies is a harness for each value.
fn a_permanent(state: &mut GameState, blank: CardId, what: Permanent, (owner, controller): (u8, u8)) -> ObjectId {
    use crate::types::{Color, CounterType};
    let (power, toughness) = match what {
        Permanent::Monstrous => (Some(4), Some(2)),
        Permanent::Stout => (Some(2), Some(6)),
        _ if what.is_creature() => (Some(2), Some(2)),
        _ => (None, None),
    };
    let id = state.create_object(blank, PlayerId(owner), Zone::Battlefield, power, toughness);
    let o = state.get_object_mut(id).expect("just created");
    o.card_types = vec![match what {
        Permanent::Artifact => CardType::Artifact,
        Permanent::Enchantment => CardType::Enchantment,
        Permanent::Land => CardType::Land,
        Permanent::Planeswalker => CardType::Planeswalker,
        _ => CardType::Creature,
    }];
    o.controller = PlayerId(controller);
    match what {
        Permanent::Black => o.colors = vec![Color::Black],
        Permanent::Spirit => o.subtypes = vec!["Spirit".to_string()],
        Permanent::Human => o.subtypes = vec!["Human".to_string()],
        Permanent::Vampire => o.subtypes = vec!["Vampire".to_string()],
        Permanent::Werewolf => o.subtypes = vec!["Werewolf".to_string()],
        Permanent::Zombie => o.subtypes = vec!["Zombie".to_string()],
        Permanent::Planeswalker => {
            o.counters.insert(CounterType::Loyalty, 3);
        }
        _ => {}
    }
    if what == Permanent::Attacker {
        let mut combat = state.combat.take().unwrap_or_else(crate::state::CombatState::new);
        combat.attackers.insert(id, PlayerId(1 - controller));
        state.combat = Some(combat);
    }
    id
}

/// The declare attackers step of `attacking`'s turn, the player who has
/// priority keeping it: the attacking player has it first (CR 508.2), so
/// if the other player has it now, the attacking player has passed.
fn in_the_declare_attackers_step(state: &mut GameState, attacking: u8) {
    state.active_player = PlayerId(attacking);
    state.step = Step::DeclareAttackers;
    state.consecutive_passes = u32::from(state.priority_player != Some(PlayerId(attacking)));
}

/// What a permanent is that a proof reads back to say nothing was done to it.
#[derive(Clone, Copy, PartialEq)]
struct AsItWas {
    tapped: bool,
    damage: u32,
    controller: PlayerId,
    shields: u32,
}

fn as_it_is(state: &GameState, id: ObjectId) -> Option<AsItWas> {
    let o = state.get_object(id).expect("still an object");
    (o.zone == Zone::Battlefield).then_some(AsItWas { tapped: o.tapped, damage: o.damage_marked, controller: o.controller, shields: o.regeneration_shields })
}

/// What stands between a permanent and its destruction.
#[derive(Clone, Copy, PartialEq)]
enum Guard {
    Nothing,
    /// A regeneration shield (CR 701.15a).
    Shield,
    /// Indestructible until end of turn (702.12b).
    Indestructible,
}

/// "Destroy target <permanent>." cast by `cast` at a permanent its text
/// names, `owner`'s under `controller`, and resolved: the permanent is in
/// its owner's graveyard (CR 701.7a, 400.3) and no longer in combat
/// (506.4); or, indestructible, nothing happens to it (702.12b); or, with
/// a regeneration shield, it is tapped, out of combat and the shield used
/// up instead (701.15a). A second permanent of the same kind beside it is
/// as it was, no player's life has changed, and the spell rests where a
/// spell cast so rests.
fn destroys((card, what, cast, (owner, controller), guard): (Box<dyn CardBehavior>, Permanent, Cast, (u8, u8), Guard)) {
    use crate::state::TemporaryEffect;
    let (registry, mut state, blank, card) = board(card);
    let target = a_permanent(&mut state, blank, what, (owner, controller));
    let beside = a_permanent(&mut state, blank, what, (owner, controller));
    match guard {
        Guard::Nothing => {}
        Guard::Shield => state.get_object_mut(target).expect("just created").regeneration_shields = 1,
        // As an effect until end of turn: the blank card prints no keyword.
        Guard::Indestructible => state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target, keyword: Keyword::Indestructible }),
    }
    let spell = a_spell(&mut state, &registry, card, cast);
    if what == Permanent::Attacker {
        in_the_declare_attackers_step(&mut state, controller);
    }
    let (before, beside_before) = (as_it_is(&state, target).expect("there"), as_it_is(&state, beside));

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(target)]);

    let o = state.get_object(target).expect("still an object");
    let attacking = state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&target));
    match guard {
        Guard::Indestructible => {
            assert!(as_it_is(&state, target) == Some(before) && attacking == (what == Permanent::Attacker));
        }
        Guard::Shield => {
            assert!(as_it_is(&state, target) == Some(AsItWas { tapped: true, shields: 0, ..before }) && !attacking);
        }
        Guard::Nothing => {
            assert!(o.zone == Zone::Graveyard && o.controller == PlayerId(owner) && !o.tapped && o.damage_marked == 0);
            assert!(state.players[usize::from(owner)].graveyard_order.contains(&target) && !attacking);
        }
    }
    assert!(as_it_is(&state, beside) == beside_before && beside_before.is_some());
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// A spell with one target cast by `caster` at a permanent its text does
/// not let it target, `controller`'s: the engine refuses the cast (CR
/// 601.2c) and nothing is done, to the permanent or to anything else.
fn is_not_cast_at((card, what, cast, controller): (Box<dyn CardBehavior>, Permanent, Cast, u8)) {
    let (registry, mut state, blank, card) = board(card);
    let target = a_permanent(&mut state, blank, what, (controller, controller));
    let spell = a_spell(&mut state, &registry, card, cast);
    if what == Permanent::Attacker {
        in_the_declare_attackers_step(&mut state, controller);
    }
    let before = as_it_is(&state, target);

    cast_is_refused(&mut state, &registry, spell, vec![Target::Object(target)]);

    assert!(as_it_is(&state, target) == before && before.is_some());
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    std::mem::forget((registry, state));
}

/// A spell with one target, cast by `caster` at the other player's
/// permanent, which that player destroys in response with `answer` (a
/// spell of their own, cast with the first on the stack and resolved
/// first, CR 608.1): its only target gone, the first spell does nothing
/// as it resolves (608.2b) and is put into its caster's graveyard. Nothing
/// is made, no effect is left, no player's life has changed, and a second
/// permanent beside the target is as it was.
fn does_nothing_once_its_target_is_gone((card, what, answer, cast): (Box<dyn CardBehavior>, Permanent, Box<dyn CardBehavior>, Cast)) {
    let (mut registry, mut state, blank, card) = board(card);
    let answer = registry.register(answer);
    let other = 1 - cast.by;
    let target = a_permanent(&mut state, blank, what, (other, other));
    let beside = a_permanent(&mut state, blank, what, (other, other));
    let spell = a_spell(&mut state, &registry, card, cast);
    let response = holds(&mut state, &registry, answer, PlayerId(other));
    if what == Permanent::Attacker {
        in_the_declare_attackers_step(&mut state, other);
    }
    let (objects, beside_before) = (state.objects.len(), as_it_is(&state, beside));

    casts(&mut state, &registry, spell, vec![Target::Object(target)]);
    play(&mut state, &registry, &Action::PassPriority);
    assert!(state.priority_player == Some(PlayerId(other)));
    casts(&mut state, &registry, response, vec![Target::Object(target)]);
    all_pass(&mut state, &registry);
    // The answer has resolved and the first spell is still on the stack.
    assert!(state.stack.len() == 1 && state.stack[0].as_spell() == Some(spell));
    assert!(state.get_object(target).expect("still an object").zone == Zone::Graveyard);
    all_pass(&mut state, &registry);

    assert!(state.stack.is_empty());
    assert!(state.get_object(target).expect("still an object").zone == Zone::Graveyard);
    assert!(rests(&state, spell, cast));
    assert!(state.get_object(response).expect("still an object").zone == Zone::Graveyard);
    assert!(as_it_is(&state, beside) == beside_before);
    assert!(state.objects.len() == objects && state.until_end_of_turn.is_empty());
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    std::mem::forget((registry, state));
}

use crate::cards::doom_blade::DoomBlade;
use crate::cards::isd::{ancient_grudge::AncientGrudge, bramblecrush::Bramblecrush, naturalize::Naturalize, rebuke::Rebuke};
use crate::cards::isd::{smite_the_monstrous::SmiteTheMonstrous, urgent_exorcism::UrgentExorcism, victim_of_night::VictimOfNight};

// Doom Blade: "Destroy target nonblack creature."
through_the_stack! {
    doom_blade_destroys_the_second_players_creature: destroys((Box::new(DoomBlade), Permanent::Creature, FIRST, (1, 1), Guard::Nothing));
    doom_blade_cast_by_the_second_player_destroys_the_first_players: destroys((Box::new(DoomBlade), Permanent::Creature, SECOND, (0, 0), Guard::Nothing));
    doom_blade_destroys_your_own_creature: destroys((Box::new(DoomBlade), Permanent::Creature, FIRST, (0, 0), Guard::Nothing));
    doom_blade_destroys_a_creature_you_control_and_the_other_player_owns: destroys((Box::new(DoomBlade), Permanent::Creature, FIRST, (1, 0), Guard::Nothing));
    doom_blade_at_an_indestructible_creature_does_nothing_to_it: destroys((Box::new(DoomBlade), Permanent::Creature, FIRST, (1, 1), Guard::Indestructible));
    doom_blade_at_a_creature_with_a_regeneration_shield_taps_it_and_uses_the_shield: destroys((Box::new(DoomBlade), Permanent::Creature, FIRST, (1, 1), Guard::Shield));
    doom_blade_is_not_cast_at_the_other_players_black_creature: is_not_cast_at((Box::new(DoomBlade), Permanent::Black, FIRST, 1));
    doom_blade_is_not_cast_at_your_own_black_creature: is_not_cast_at((Box::new(DoomBlade), Permanent::Black, SECOND, 1));
    doom_blade_is_not_cast_at_an_artifact: is_not_cast_at((Box::new(DoomBlade), Permanent::Artifact, FIRST, 1));
    doom_blade_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(DoomBlade), Permanent::Creature, Box::new(LightningBolt), FIRST));
}

// Bramblecrush: "Destroy target noncreature permanent."
through_the_stack! {
    bramblecrush_destroys_the_second_players_artifact: destroys((Box::new(Bramblecrush), Permanent::Artifact, FIRST, (1, 1), Guard::Nothing));
    bramblecrush_cast_by_the_second_player_destroys_the_first_players_artifact: destroys((Box::new(Bramblecrush), Permanent::Artifact, SECOND, (0, 0), Guard::Nothing));
    bramblecrush_destroys_your_own_artifact: destroys((Box::new(Bramblecrush), Permanent::Artifact, FIRST, (0, 0), Guard::Nothing));
    bramblecrush_destroys_an_enchantment: destroys((Box::new(Bramblecrush), Permanent::Enchantment, FIRST, (1, 1), Guard::Nothing));
    bramblecrush_destroys_a_land: destroys((Box::new(Bramblecrush), Permanent::Land, FIRST, (1, 1), Guard::Nothing));
    bramblecrush_destroys_a_planeswalker: destroys((Box::new(Bramblecrush), Permanent::Planeswalker, FIRST, (1, 1), Guard::Nothing));
    bramblecrush_is_not_cast_at_a_creature: is_not_cast_at((Box::new(Bramblecrush), Permanent::Creature, FIRST, 1));
    bramblecrush_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(Bramblecrush), Permanent::Artifact, Box::new(Naturalize), FIRST));
}

// Naturalize: "Destroy target artifact or enchantment."
through_the_stack! {
    naturalize_destroys_the_second_players_artifact: destroys((Box::new(Naturalize), Permanent::Artifact, FIRST, (1, 1), Guard::Nothing));
    naturalize_cast_by_the_second_player_destroys_the_first_players_artifact: destroys((Box::new(Naturalize), Permanent::Artifact, SECOND, (0, 0), Guard::Nothing));
    naturalize_destroys_your_own_enchantment: destroys((Box::new(Naturalize), Permanent::Enchantment, FIRST, (0, 0), Guard::Nothing));
    naturalize_destroys_the_second_players_enchantment: destroys((Box::new(Naturalize), Permanent::Enchantment, FIRST, (1, 1), Guard::Nothing));
    naturalize_at_an_indestructible_artifact_does_nothing_to_it: destroys((Box::new(Naturalize), Permanent::Artifact, FIRST, (1, 1), Guard::Indestructible));
    naturalize_is_not_cast_at_a_creature: is_not_cast_at((Box::new(Naturalize), Permanent::Creature, FIRST, 1));
    naturalize_is_not_cast_at_a_land: is_not_cast_at((Box::new(Naturalize), Permanent::Land, FIRST, 1));
    naturalize_is_not_cast_at_a_planeswalker: is_not_cast_at((Box::new(Naturalize), Permanent::Planeswalker, FIRST, 1));
    naturalize_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(Naturalize), Permanent::Artifact, Box::new(AncientGrudge), FIRST));
}

// Ancient Grudge: "Destroy target artifact."
through_the_stack! {
    ancient_grudge_destroys_the_second_players_artifact: destroys((Box::new(AncientGrudge), Permanent::Artifact, FIRST, (1, 1), Guard::Nothing));
    ancient_grudge_cast_by_the_second_player_destroys_the_first_players_artifact: destroys((Box::new(AncientGrudge), Permanent::Artifact, SECOND, (0, 0), Guard::Nothing));
    ancient_grudge_destroys_your_own_artifact: destroys((Box::new(AncientGrudge), Permanent::Artifact, FIRST, (0, 0), Guard::Nothing));
    ancient_grudge_flashed_back_destroys_an_artifact_and_is_exiled: destroys((Box::new(AncientGrudge), Permanent::Artifact, FLASHBACK, (1, 1), Guard::Nothing));
    ancient_grudge_is_not_cast_at_an_enchantment: is_not_cast_at((Box::new(AncientGrudge), Permanent::Enchantment, FIRST, 1));
    ancient_grudge_is_not_cast_at_a_creature: is_not_cast_at((Box::new(AncientGrudge), Permanent::Creature, FIRST, 1));
    ancient_grudge_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(AncientGrudge), Permanent::Artifact, Box::new(Naturalize), FIRST));
}

// Urgent Exorcism: "Destroy target Spirit or enchantment."
through_the_stack! {
    urgent_exorcism_destroys_the_second_players_spirit: destroys((Box::new(UrgentExorcism), Permanent::Spirit, FIRST, (1, 1), Guard::Nothing));
    urgent_exorcism_cast_by_the_second_player_destroys_the_first_players_spirit: destroys((Box::new(UrgentExorcism), Permanent::Spirit, SECOND, (0, 0), Guard::Nothing));
    urgent_exorcism_destroys_your_own_spirit: destroys((Box::new(UrgentExorcism), Permanent::Spirit, FIRST, (0, 0), Guard::Nothing));
    urgent_exorcism_destroys_an_enchantment: destroys((Box::new(UrgentExorcism), Permanent::Enchantment, FIRST, (1, 1), Guard::Nothing));
    urgent_exorcism_is_not_cast_at_a_creature_that_is_no_spirit: is_not_cast_at((Box::new(UrgentExorcism), Permanent::Creature, FIRST, 1));
    urgent_exorcism_is_not_cast_at_an_artifact: is_not_cast_at((Box::new(UrgentExorcism), Permanent::Artifact, FIRST, 1));
    urgent_exorcism_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(UrgentExorcism), Permanent::Spirit, Box::new(LightningBolt), FIRST));
}

// Smite the Monstrous: "Destroy target creature with power 4 or greater."
through_the_stack! {
    smite_the_monstrous_destroys_the_second_players_four_power: destroys((Box::new(SmiteTheMonstrous), Permanent::Monstrous, FIRST, (1, 1), Guard::Nothing));
    smite_the_monstrous_cast_by_the_second_player_destroys_the_first_players: destroys((Box::new(SmiteTheMonstrous), Permanent::Monstrous, SECOND, (0, 0), Guard::Nothing));
    smite_the_monstrous_destroys_your_own_four_power: destroys((Box::new(SmiteTheMonstrous), Permanent::Monstrous, FIRST, (0, 0), Guard::Nothing));
    smite_the_monstrous_is_not_cast_at_a_creature_of_power_two: is_not_cast_at((Box::new(SmiteTheMonstrous), Permanent::Creature, FIRST, 1));
    smite_the_monstrous_is_not_cast_at_an_artifact: is_not_cast_at((Box::new(SmiteTheMonstrous), Permanent::Artifact, FIRST, 1));
    smite_the_monstrous_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(SmiteTheMonstrous), Permanent::Monstrous, Box::new(LightningBolt), FIRST));
}

// Victim of Night: "Destroy target non-Vampire, non-Werewolf, non-Zombie creature."
through_the_stack! {
    victim_of_night_destroys_the_second_players_creature: destroys((Box::new(VictimOfNight), Permanent::Creature, FIRST, (1, 1), Guard::Nothing));
    victim_of_night_cast_by_the_second_player_destroys_the_first_players: destroys((Box::new(VictimOfNight), Permanent::Creature, SECOND, (0, 0), Guard::Nothing));
    victim_of_night_destroys_your_own_spirit: destroys((Box::new(VictimOfNight), Permanent::Spirit, FIRST, (0, 0), Guard::Nothing));
    victim_of_night_is_not_cast_at_a_vampire: is_not_cast_at((Box::new(VictimOfNight), Permanent::Vampire, FIRST, 1));
    victim_of_night_is_not_cast_at_a_werewolf: is_not_cast_at((Box::new(VictimOfNight), Permanent::Werewolf, FIRST, 1));
    victim_of_night_is_not_cast_at_a_zombie: is_not_cast_at((Box::new(VictimOfNight), Permanent::Zombie, FIRST, 1));
    victim_of_night_is_not_cast_at_an_enchantment: is_not_cast_at((Box::new(VictimOfNight), Permanent::Enchantment, FIRST, 1));
    victim_of_night_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(VictimOfNight), Permanent::Creature, Box::new(LightningBolt), FIRST));
}

// Rebuke: "Destroy target attacking creature."
through_the_stack! {
    rebuke_destroys_a_creature_attacking_you: destroys((Box::new(Rebuke), Permanent::Attacker, FIRST, (1, 1), Guard::Nothing));
    rebuke_cast_by_the_second_player_destroys_a_creature_attacking_them: destroys((Box::new(Rebuke), Permanent::Attacker, SECOND, (0, 0), Guard::Nothing));
    rebuke_destroys_your_own_attacking_creature: destroys((Box::new(Rebuke), Permanent::Attacker, FIRST, (0, 0), Guard::Nothing));
    rebuke_at_an_attacker_with_a_regeneration_shield_takes_it_out_of_combat: destroys((Box::new(Rebuke), Permanent::Attacker, FIRST, (1, 1), Guard::Shield));
    rebuke_is_not_cast_at_a_creature_that_is_not_attacking: is_not_cast_at((Box::new(Rebuke), Permanent::Creature, FIRST, 1));
    rebuke_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(Rebuke), Permanent::Attacker, Box::new(LightningBolt), FIRST));
}

/// "Target creature gets +P/+T [and gains a keyword] until end of turn."
/// cast by `caster` at `controller`'s creature and resolved, read back
/// through the engine's own power, toughness and keywords (CR 613.4c,
/// 613.1f): that much more of each than before, the keyword if the text
/// gives one, untapped if the text untaps it (701.21a) and otherwise
/// tapped as it was, and nothing else about the creature changed. A
/// second creature beside it reads what it read before, and the spell is
/// in its caster's graveyard.
fn pumps((card, (power_up, toughness_up), keyword, untaps, cast, controller): (Box<dyn CardBehavior>, (i32, i32), Option<crate::types::Keyword>, bool, Cast, u8)) {
    let (registry, mut state, blank, card) = board(card);
    let target = a_permanent(&mut state, blank, Permanent::Creature, (controller, controller));
    let beside = a_permanent(&mut state, blank, Permanent::Creature, (controller, controller));
    // Tapped, both, where the text untaps its target: the one is untapped and the other not.
    state.get_object_mut(target).expect("just created").tapped = untaps;
    state.get_object_mut(beside).expect("just created").tapped = untaps;
    let spell = a_spell(&mut state, &registry, card, cast);
    let read = |state: &GameState, id| (state.effective_power(id, &registry), state.effective_toughness(id, &registry));
    let (pt, beside_pt) = (read(&state, target), read(&state, beside));
    let (before, beside_before) = (as_it_is(&state, target).expect("there"), as_it_is(&state, beside));

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(target)]);

    assert!(read(&state, target) == (pt.0.map(|p| p + power_up), pt.1.map(|t| t + toughness_up)));
    if let Some(keyword) = keyword {
        assert!(state.has_keyword(target, keyword, &registry) && !state.has_keyword(beside, keyword, &registry));
    }
    assert!(as_it_is(&state, target) == Some(AsItWas { tapped: before.tapped && !untaps, ..before }));
    assert!(read(&state, beside) == beside_pt && as_it_is(&state, beside) == beside_before);
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

use crate::cards::giant_growth::GiantGrowth;
use crate::cards::isd::{moment_of_heroism::MomentOfHeroism, rangers_guile::RangersGuile, spidery_grasp::SpideryGrasp};
use crate::types::Keyword;

// Giant Growth: "Target creature gets +3/+3 until end of turn."
through_the_stack! {
    giant_growth_gives_your_creature_three_and_three: pumps((Box::new(GiantGrowth), (3, 3), None, false, FIRST, 0));
    giant_growth_gives_the_other_players_creature_three_and_three: pumps((Box::new(GiantGrowth), (3, 3), None, false, FIRST, 1));
    giant_growth_cast_by_the_second_player_gives_their_creature_three_and_three: pumps((Box::new(GiantGrowth), (3, 3), None, false, SECOND, 1));
    giant_growth_is_not_cast_at_an_artifact: is_not_cast_at((Box::new(GiantGrowth), Permanent::Artifact, FIRST, 0));
    giant_growth_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(GiantGrowth), Permanent::Creature, Box::new(DoomBlade), FIRST));
}

// Moment of Heroism: "Target creature gets +2/+2 and gains lifelink until end of turn."
through_the_stack! {
    moment_of_heroism_gives_your_creature_two_and_two_and_lifelink: pumps((Box::new(MomentOfHeroism), (2, 2), Some(Keyword::Lifelink), false, FIRST, 0));
    moment_of_heroism_gives_the_other_players_creature_two_and_two_and_lifelink: pumps((Box::new(MomentOfHeroism), (2, 2), Some(Keyword::Lifelink), false, FIRST, 1));
    moment_of_heroism_cast_by_the_second_player_gives_their_creature_two_and_two: pumps((Box::new(MomentOfHeroism), (2, 2), Some(Keyword::Lifelink), false, SECOND, 1));
    moment_of_heroism_is_not_cast_at_a_land: is_not_cast_at((Box::new(MomentOfHeroism), Permanent::Land, FIRST, 0));
    moment_of_heroism_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(MomentOfHeroism), Permanent::Creature, Box::new(LightningBolt), FIRST));
}

// Ranger's Guile: "Target creature you control gets +1/+1 and gains hexproof until end of turn."
through_the_stack! {
    rangers_guile_gives_your_creature_one_and_one_and_hexproof: pumps((Box::new(RangersGuile), (1, 1), Some(Keyword::Hexproof), false, FIRST, 0));
    rangers_guile_cast_by_the_second_player_gives_their_creature_one_and_one_and_hexproof: pumps((Box::new(RangersGuile), (1, 1), Some(Keyword::Hexproof), false, SECOND, 1));
    rangers_guile_is_not_cast_at_the_other_players_creature: is_not_cast_at((Box::new(RangersGuile), Permanent::Creature, FIRST, 1));
    rangers_guile_cast_by_the_second_player_is_not_cast_at_the_first_players_creature: is_not_cast_at((Box::new(RangersGuile), Permanent::Creature, SECOND, 0));
    rangers_guile_is_not_cast_at_your_enchantment: is_not_cast_at((Box::new(RangersGuile), Permanent::Enchantment, FIRST, 0));
}

// Spidery Grasp: "Untap target creature. It gets +2/+4 and gains reach until end of turn."
through_the_stack! {
    spidery_grasp_untaps_your_creature_and_gives_it_two_and_four_and_reach: pumps((Box::new(SpideryGrasp), (2, 4), Some(Keyword::Reach), true, FIRST, 0));
    spidery_grasp_untaps_the_other_players_creature_and_gives_it_two_and_four: pumps((Box::new(SpideryGrasp), (2, 4), Some(Keyword::Reach), true, FIRST, 1));
    spidery_grasp_cast_by_the_second_player_untaps_their_creature: pumps((Box::new(SpideryGrasp), (2, 4), Some(Keyword::Reach), true, SECOND, 1));
    spidery_grasp_is_not_cast_at_a_planeswalker: is_not_cast_at((Box::new(SpideryGrasp), Permanent::Planeswalker, FIRST, 0));
    spidery_grasp_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(SpideryGrasp), Permanent::Creature, Box::new(LightningBolt), FIRST));
}

/// How much a damage spell deals as it resolves: a constant of each
/// harness.
#[derive(Clone, Copy)]
enum Amount {
    Fixed(u32),
    /// 5 if a creature died this turn, else 3 (Brimstone Volley's morbid,
    /// CR 700.4): which, the harness says.
    Morbid { died: bool },
}

impl Amount {
    /// The amount, the board set up as it says.
    fn on(self, state: &mut GameState) -> u32 {
        match self {
            Amount::Fixed(n) => n,
            Amount::Morbid { died } => {
                state.creature_died_this_turn = died;
                if died { 5 } else { 3 }
            }
        }
    }
}

/// "~ deals N damage to <target>." cast by `cast` at `controller`'s
/// creature (`what` it is: a 2/2, or a 2/6), `marked` damage on it
/// already, and resolved, the state-based actions checked: with N and
/// what was marked at least its toughness it has been destroyed and is in
/// its owner's graveyard (CR 704.5g), and with less it is on the
/// battlefield with N more damage marked than before and otherwise as it
/// was (120.3e). A second creature beside it is as it was, no player's
/// life has changed, and the spell rests where a spell cast so rests.
fn damages_a_creature((card, amount, (what, marked), cast, controller): (Box<dyn CardBehavior>, Amount, (Permanent, u32), Cast, u8)) {
    let (registry, mut state, blank, card) = board(card);
    let target = a_permanent(&mut state, blank, what, (controller, controller));
    let beside = a_permanent(&mut state, blank, what, (controller, controller));
    state.get_object_mut(target).expect("just created").damage_marked = marked;
    let n = amount.on(&mut state);
    let spell = a_spell(&mut state, &registry, card, cast);
    let toughness = state.get_object(target).expect("just created").toughness.expect("a creature");
    let (before, beside_before) = (as_it_is(&state, target).expect("there"), as_it_is(&state, beside));

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(target)]);

    let o = state.get_object(target).expect("still an object");
    if i64::from(marked + n) >= i64::from(toughness) {
        assert!(o.zone == Zone::Graveyard && o.owner == PlayerId(controller) && o.damage_marked == 0);
        assert!(state.players[usize::from(controller)].graveyard_order.contains(&target));
    } else {
        assert!(as_it_is(&state, target) == Some(AsItWas { damage: marked + n, ..before }));
    }
    assert!(as_it_is(&state, beside) == beside_before && beside_before.is_some());
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// The same at a player, `at`, from 20 life: that player has N life fewer
/// (CR 120.3a) and has not lost, the other has as many as before, a
/// creature of each player's is as it was, and the spell is in its
/// caster's graveyard.
fn damages_a_player((card, amount, cast, at): (Box<dyn CardBehavior>, Amount, Cast, u8)) {
    let (registry, mut state, blank, card) = board(card);
    let creatures = [a_permanent(&mut state, blank, Permanent::Creature, (0, 0)), a_permanent(&mut state, blank, Permanent::Creature, (1, 1))];
    let n = amount.on(&mut state);
    let spell = a_spell(&mut state, &registry, card, cast);
    let before = [as_it_is(&state, creatures[0]), as_it_is(&state, creatures[1])];

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Player(PlayerId(at))]);

    for p in 0..2u8 {
        let lost = if p == at { i32::try_from(n).expect("small") } else { 0 };
        assert!(state.players[usize::from(p)].life == 20 - lost && !state.players[usize::from(p)].lost);
        assert!(as_it_is(&state, creatures[usize::from(p)]) == before[usize::from(p)]);
    }
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// The same at `controller`'s planeswalker with `loyalty` counters: N of
/// them are removed (CR 120.3c) and no damage is marked on it, and with
/// none left it has been put into its owner's graveyard (704.5i). No
/// player's life has changed, and the spell is in its caster's graveyard.
fn damages_a_planeswalker((card, amount, loyalty, cast, controller): (Box<dyn CardBehavior>, Amount, u32, Cast, u8)) {
    use crate::types::CounterType;
    let (registry, mut state, blank, card) = board(card);
    let target = a_permanent(&mut state, blank, Permanent::Planeswalker, (controller, controller));
    state.get_object_mut(target).expect("just created").counters.insert(CounterType::Loyalty, loyalty);
    let n = amount.on(&mut state);
    let spell = a_spell(&mut state, &registry, card, cast);
    let before = as_it_is(&state, target);

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(target)]);

    let o = state.get_object(target).expect("still an object");
    if n >= loyalty {
        assert!(o.zone == Zone::Graveyard && o.owner == PlayerId(controller));
    } else {
        assert!(as_it_is(&state, target) == before && o.damage_marked == 0);
        assert!(o.counters.get(&CounterType::Loyalty).copied() == Some(loyalty - n));
    }
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

use crate::cards::isd::{brimstone_volley::BrimstoneVolley, geistflame::Geistflame};

// Lightning Bolt: "Lightning Bolt deals 3 damage to any target."
through_the_stack! {
    lightning_bolt_destroys_the_other_players_small_creature: damages_a_creature((Box::new(LightningBolt), Amount::Fixed(3), (Permanent::Creature, 0), FIRST, 1));
    lightning_bolt_cast_by_the_second_player_destroys_the_first_players_small_creature: damages_a_creature((Box::new(LightningBolt), Amount::Fixed(3), (Permanent::Creature, 0), SECOND, 0));
    lightning_bolt_destroys_your_own_small_creature: damages_a_creature((Box::new(LightningBolt), Amount::Fixed(3), (Permanent::Creature, 0), FIRST, 0));
    lightning_bolt_marks_three_on_a_stout_creature: damages_a_creature((Box::new(LightningBolt), Amount::Fixed(3), (Permanent::Stout, 0), FIRST, 1));
    lightning_bolt_takes_three_life_from_the_other_player: damages_a_player((Box::new(LightningBolt), Amount::Fixed(3), FIRST, 1));
    lightning_bolt_takes_three_life_from_its_caster: damages_a_player((Box::new(LightningBolt), Amount::Fixed(3), FIRST, 0));
    lightning_bolt_cast_by_the_second_player_takes_three_life_from_the_first: damages_a_player((Box::new(LightningBolt), Amount::Fixed(3), SECOND, 0));
    lightning_bolt_takes_three_loyalty_of_five: damages_a_planeswalker((Box::new(LightningBolt), Amount::Fixed(3), 5, FIRST, 1));
    lightning_bolt_takes_the_last_three_loyalty: damages_a_planeswalker((Box::new(LightningBolt), Amount::Fixed(3), 3, FIRST, 1));
    lightning_bolt_is_not_cast_at_an_artifact: is_not_cast_at((Box::new(LightningBolt), Permanent::Artifact, FIRST, 1));
    lightning_bolt_is_not_cast_at_a_land: is_not_cast_at((Box::new(LightningBolt), Permanent::Land, FIRST, 1));
    lightning_bolt_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(LightningBolt), Permanent::Creature, Box::new(DoomBlade), FIRST));
}

// Lava Axe: "Lava Axe deals 5 damage to target player or planeswalker."
through_the_stack! {
    lava_axe_takes_five_life_from_the_other_player: damages_a_player((Box::new(LavaAxe), Amount::Fixed(5), FIRST, 1));
    lava_axe_takes_five_life_from_its_caster: damages_a_player((Box::new(LavaAxe), Amount::Fixed(5), FIRST, 0));
    lava_axe_cast_by_the_second_player_takes_five_life_from_the_first: damages_a_player((Box::new(LavaAxe), Amount::Fixed(5), SECOND, 0));
    lava_axe_takes_five_loyalty_of_six: damages_a_planeswalker((Box::new(LavaAxe), Amount::Fixed(5), 6, FIRST, 1));
    lava_axe_takes_the_last_five_loyalty: damages_a_planeswalker((Box::new(LavaAxe), Amount::Fixed(5), 5, FIRST, 1));
    lava_axe_takes_all_of_three_loyalty: damages_a_planeswalker((Box::new(LavaAxe), Amount::Fixed(5), 3, SECOND, 0));
    lava_axe_is_not_cast_at_a_creature: is_not_cast_at((Box::new(LavaAxe), Permanent::Creature, FIRST, 1));
    lava_axe_is_not_cast_at_an_enchantment: is_not_cast_at((Box::new(LavaAxe), Permanent::Enchantment, FIRST, 1));
}

// Geistflame: "Geistflame deals 1 damage to any target."
through_the_stack! {
    geistflame_marks_one_on_the_other_players_creature: damages_a_creature((Box::new(Geistflame), Amount::Fixed(1), (Permanent::Creature, 0), FIRST, 1));
    geistflame_cast_by_the_second_player_marks_one_on_the_first_players: damages_a_creature((Box::new(Geistflame), Amount::Fixed(1), (Permanent::Creature, 0), SECOND, 0));
    geistflame_marks_one_on_your_own_creature: damages_a_creature((Box::new(Geistflame), Amount::Fixed(1), (Permanent::Creature, 0), FIRST, 0));
    geistflame_destroys_a_creature_with_a_point_of_damage_already: damages_a_creature((Box::new(Geistflame), Amount::Fixed(1), (Permanent::Creature, 1), FIRST, 1));
    geistflame_takes_one_life_from_the_other_player: damages_a_player((Box::new(Geistflame), Amount::Fixed(1), FIRST, 1));
    geistflame_flashed_back_takes_one_life_and_is_exiled: damages_a_player((Box::new(Geistflame), Amount::Fixed(1), FLASHBACK, 1));
    geistflame_flashed_back_marks_one_on_a_creature_and_is_exiled: damages_a_creature((Box::new(Geistflame), Amount::Fixed(1), (Permanent::Creature, 0), FLASHBACK, 1));
    geistflame_cast_by_the_second_player_takes_one_life_from_the_first: damages_a_player((Box::new(Geistflame), Amount::Fixed(1), SECOND, 0));
    geistflame_takes_one_loyalty_of_three: damages_a_planeswalker((Box::new(Geistflame), Amount::Fixed(1), 3, FIRST, 1));
    geistflame_takes_the_last_loyalty: damages_a_planeswalker((Box::new(Geistflame), Amount::Fixed(1), 1, FIRST, 1));
    geistflame_is_not_cast_at_an_artifact: is_not_cast_at((Box::new(Geistflame), Permanent::Artifact, FIRST, 1));
    geistflame_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(Geistflame), Permanent::Creature, Box::new(DoomBlade), FIRST));
}

// Brimstone Volley: "Brimstone Volley deals 3 damage to any target. Morbid:
// 5 damage instead if a creature died this turn."
through_the_stack! {
    brimstone_volley_destroys_the_other_players_small_creature: damages_a_creature((Box::new(BrimstoneVolley), Amount::Morbid { died: false }, (Permanent::Creature, 0), FIRST, 1));
    brimstone_volley_marks_three_on_a_stout_creature: damages_a_creature((Box::new(BrimstoneVolley), Amount::Morbid { died: false }, (Permanent::Stout, 0), FIRST, 1));
    brimstone_volley_after_a_death_marks_five_on_a_stout_creature: damages_a_creature((Box::new(BrimstoneVolley), Amount::Morbid { died: true }, (Permanent::Stout, 0), FIRST, 1));
    brimstone_volley_cast_by_the_second_player_after_a_death_marks_five: damages_a_creature((Box::new(BrimstoneVolley), Amount::Morbid { died: true }, (Permanent::Stout, 0), SECOND, 0));
    brimstone_volley_after_a_death_destroys_a_stout_creature_with_a_point_of_damage: damages_a_creature((Box::new(BrimstoneVolley), Amount::Morbid { died: true }, (Permanent::Stout, 1), FIRST, 1));
    brimstone_volley_takes_three_life_from_the_other_player: damages_a_player((Box::new(BrimstoneVolley), Amount::Morbid { died: false }, FIRST, 1));
    brimstone_volley_after_a_death_takes_five_life_from_the_other_player: damages_a_player((Box::new(BrimstoneVolley), Amount::Morbid { died: true }, FIRST, 1));
    brimstone_volley_after_a_death_takes_five_life_from_its_caster: damages_a_player((Box::new(BrimstoneVolley), Amount::Morbid { died: true }, SECOND, 1));
    brimstone_volley_takes_three_loyalty_of_five: damages_a_planeswalker((Box::new(BrimstoneVolley), Amount::Morbid { died: false }, 5, FIRST, 1));
    brimstone_volley_after_a_death_takes_all_five_loyalty: damages_a_planeswalker((Box::new(BrimstoneVolley), Amount::Morbid { died: true }, 5, FIRST, 1));
    brimstone_volley_is_not_cast_at_a_land: is_not_cast_at((Box::new(BrimstoneVolley), Permanent::Land, FIRST, 1));
    brimstone_volley_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(BrimstoneVolley), Permanent::Creature, Box::new(DoomBlade), FIRST));
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

/// A card of the blank card in `zone`, the player `whose` names its owner;
/// a creature card (a 2/2) if `creature`.
fn a_card_in(state: &mut GameState, blank: CardId, whose: u8, zone: Zone, creature: bool) -> ObjectId {
    let pt = if creature { Some(2) } else { None };
    let id = state.create_object(blank, PlayerId(whose), zone, pt, pt);
    if creature {
        state.get_object_mut(id).expect("just created").card_types = vec![CardType::Creature];
    }
    if zone == Zone::Graveyard {
        if whose == 0 {
            state.players[0].graveyard_order.push(id);
        } else {
            state.players[1].graveyard_order.push(id);
        }
    }
    id
}

/// "Target player mills N cards." cast at the player `who` names, their
/// library of `cards` cards, and resolved: the top N of it, or all of it
/// if there are fewer, are in their graveyard, top first (CR 701.17a), the
/// rest are their library in the order they were, and a player who milled
/// an empty library has not drawn from one and has not lost (701.17b,
/// 704.5b). The other player's library is untouched.
fn mills((card, n, cards, cast, who): (Box<dyn CardBehavior>, usize, usize, Cast, u8)) {
    let (registry, mut state, blank, card) = board(card);
    let library = a_library(&mut state, blank, who, cards);
    let other = a_library(&mut state, blank, 1 - who, 1);
    let spell = a_spell(&mut state, &registry, card, cast);

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Player(PlayerId(who))]);

    let milled = n.min(cards);
    let player = &state.players[usize::from(who)];
    // The caster's graveyard holds the spell too, on top, if it rests there.
    let theirs = player.graveyard_order.len() - usize::from(cast.by == who && !cast.flashback);
    assert!(player.library_order.len() == cards - milled && theirs == milled && !player.has_drawn_from_empty && !player.lost);
    for k in 0..cards {
        let id = library[k].expect("made");
        let o = state.get_object(id).expect("still an object");
        if k < milled {
            assert!(o.zone == Zone::Graveyard && player.graveyard_order[k] == id);
        } else {
            assert!(o.zone == Zone::Library && player.library_order[k - milled] == id);
        }
    }
    assert!(state.get_object(other[0].expect("made")).expect("still an object").zone == Zone::Library);
    assert!(state.players[usize::from(1 - who)].library_order.len() == 1 && !state.players[usize::from(1 - who)].lost);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// "Create N P/T <colour> <subtype> creature tokens [with a keyword]."
/// cast and resolved on a board with a creature of each player's: N new
/// permanents, each a token, a creature of that power, toughness, colour
/// and subtype (and keyword), untapped under the spell's caster (CR 111.1,
/// 111.2), read back through the engine's own characteristics; the two
/// creatures are as they were, and nothing else was made.
fn makes_tokens((card, n, (power, toughness), colour, subtype, keyword, cast): (Box<dyn CardBehavior>, u64, (i32, i32), crate::types::Color, &str, Option<crate::types::Keyword>, Cast)) {
    let (registry, mut state, blank, card) = board(card);
    let creatures = [a_permanent(&mut state, blank, Permanent::Creature, (0, 0)), a_permanent(&mut state, blank, Permanent::Creature, (1, 1))];
    let spell = a_spell(&mut state, &registry, card, cast);
    let (objects, before) = (state.objects.len(), [as_it_is(&state, creatures[0]), as_it_is(&state, creatures[1])]);
    let you = PlayerId(cast.by);

    cast_and_resolve(&mut state, &registry, spell, Vec::new());

    assert!(state.objects.len() == objects + usize::try_from(n).expect("a few"));
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
    assert!(as_it_is(&state, creatures[0]) == before[0] && as_it_is(&state, creatures[1]) == before[1]);
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// Gnaw to the Bone ("You gain 2 life for each creature card in your
/// graveyard.") cast and resolved, `creatures` creature cards and one
/// other card in its caster's graveyard and a creature card in the other
/// player's: its caster has twice that many life more (CR 119.3), the
/// other player as much as before, and every card is where it was.
fn gnaw_to_the_bone((creatures, cast): (usize, Cast)) {
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::isd::gnaw_to_the_bone::GnawToTheBone));
    let (you, other) = (cast.by, 1 - cast.by);
    for _ in 0..creatures {
        a_card_in(&mut state, blank, you, Zone::Graveyard, true);
    }
    a_card_in(&mut state, blank, you, Zone::Graveyard, false);
    a_card_in(&mut state, blank, other, Zone::Graveyard, true);
    let spell = a_spell(&mut state, &registry, card, cast);
    let buried = [state.players[0].graveyard_order.len(), state.players[1].graveyard_order.len()];

    cast_and_resolve(&mut state, &registry, spell, Vec::new());

    assert!(state.players[usize::from(you)].life == 20 + 2 * i32::try_from(creatures).expect("a few"));
    assert!(state.players[usize::from(other)].life == 20);
    // Cast from the hand it joins its caster's graveyard; flashed back it leaves it.
    let now = if cast.flashback { buried[usize::from(you)] - 1 } else { buried[usize::from(you)] + 1 };
    assert!(state.players[usize::from(you)].graveyard_order.len() == now);
    assert!(state.players[usize::from(other)].graveyard_order.len() == buried[usize::from(other)]);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// Bump in the Night ("Target opponent loses 3 life.") cast at the other
/// player and resolved: that player has 3 life fewer (CR 119.3) and has
/// not lost, its caster as many as before, and a creature of each
/// player's is as it was.
fn bump_in_the_night(cast: Cast) {
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::isd::bump_in_the_night::BumpInTheNight));
    let creatures = [a_permanent(&mut state, blank, Permanent::Creature, (0, 0)), a_permanent(&mut state, blank, Permanent::Creature, (1, 1))];
    let spell = a_spell(&mut state, &registry, card, cast);
    let before = [as_it_is(&state, creatures[0]), as_it_is(&state, creatures[1])];
    let at = 1 - cast.by;

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Player(PlayerId(at))]);

    assert!(state.players[usize::from(at)].life == 17 && state.players[usize::from(cast.by)].life == 20);
    assert!(!state.players[0].lost && !state.players[1].lost);
    assert!(as_it_is(&state, creatures[0]) == before[0] && as_it_is(&state, creatures[1]) == before[1]);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// A spell with one target cast by `cast` at a player its text does not
/// let it target (`at`): the engine refuses the cast (CR 601.2c) and
/// nothing is done.
fn is_not_cast_at_the_player((card, cast, at): (Box<dyn CardBehavior>, Cast, u8)) {
    let (registry, mut state, _, card) = board(card);
    let spell = a_spell(&mut state, &registry, card, cast);

    cast_is_refused(&mut state, &registry, spell, vec![Target::Player(PlayerId(at))]);

    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    std::mem::forget((registry, state));
}

use crate::cards::isd::{bump_in_the_night::BumpInTheNight, dream_twist::DreamTwist, midnight_haunting::MidnightHaunting};
use crate::cards::isd::{moan_of_the_unhallowed::MoanOfTheUnhallowed, think_twice::ThinkTwice};
use crate::types::Color;

// Think Twice: "Draw a card." Flashback {2}{U}.
through_the_stack! {
    think_twice_cast_by_the_first_player_draws_the_top_of_two: draws((Box::new(ThinkTwice), 1, 2, FIRST));
    think_twice_cast_by_the_second_player_draws_the_top_of_two: draws((Box::new(ThinkTwice), 1, 2, SECOND));
    think_twice_draws_the_last_card_of_a_library: draws((Box::new(ThinkTwice), 1, 1, FIRST));
    think_twice_flashed_back_draws_a_card_and_is_exiled: draws((Box::new(ThinkTwice), 1, 2, FLASHBACK));
}

// Dream Twist: "Target player mills three cards." Flashback {1}{U}.
through_the_stack! {
    dream_twist_mills_the_top_three_of_the_other_players_four: mills((Box::new(DreamTwist), 3, 4, FIRST, 1));
    dream_twist_mills_the_top_three_of_its_casters_four: mills((Box::new(DreamTwist), 3, 4, FIRST, 0));
    dream_twist_cast_by_the_second_player_mills_the_first_players_three: mills((Box::new(DreamTwist), 3, 3, SECOND, 0));
    dream_twist_mills_all_of_a_library_of_two: mills((Box::new(DreamTwist), 3, 2, FIRST, 1));
    dream_twist_mills_nothing_from_an_empty_library: mills((Box::new(DreamTwist), 3, 0, FIRST, 1));
    dream_twist_flashed_back_mills_three_and_is_exiled: mills((Box::new(DreamTwist), 3, 4, FLASHBACK, 1));
    dream_twist_is_not_cast_at_a_creature: is_not_cast_at((Box::new(DreamTwist), Permanent::Creature, FIRST, 1));
}

// Midnight Haunting: "Create two 1/1 white Spirit creature tokens with flying."
through_the_stack! {
    midnight_haunting_makes_its_caster_two_flying_spirits: makes_tokens((Box::new(MidnightHaunting), 2, (1, 1), Color::White, "Spirit", Some(Keyword::Flying), FIRST));
    midnight_haunting_cast_by_the_second_player_makes_them_two_flying_spirits: makes_tokens((Box::new(MidnightHaunting), 2, (1, 1), Color::White, "Spirit", Some(Keyword::Flying), SECOND));
}

// Moan of the Unhallowed: "Create two 2/2 black Zombie creature tokens." Flashback {5}{B}{B}.
through_the_stack! {
    moan_of_the_unhallowed_makes_its_caster_two_zombies: makes_tokens((Box::new(MoanOfTheUnhallowed), 2, (2, 2), Color::Black, "Zombie", None, FIRST));
    moan_of_the_unhallowed_cast_by_the_second_player_makes_them_two_zombies: makes_tokens((Box::new(MoanOfTheUnhallowed), 2, (2, 2), Color::Black, "Zombie", None, SECOND));
    moan_of_the_unhallowed_flashed_back_makes_two_zombies_and_is_exiled: makes_tokens((Box::new(MoanOfTheUnhallowed), 2, (2, 2), Color::Black, "Zombie", None, FLASHBACK));
}

// Gnaw to the Bone: "You gain 2 life for each creature card in your graveyard." Flashback {2}{G}.
through_the_stack! {
    gnaw_to_the_bone_gains_nothing_with_no_creature_card: gnaw_to_the_bone((0, FIRST));
    gnaw_to_the_bone_gains_two_for_one_creature_card: gnaw_to_the_bone((1, FIRST));
    gnaw_to_the_bone_gains_four_for_two_creature_cards: gnaw_to_the_bone((2, FIRST));
    gnaw_to_the_bone_cast_by_the_second_player_gains_them_four: gnaw_to_the_bone((2, SECOND));
    gnaw_to_the_bone_flashed_back_gains_two_and_is_exiled: gnaw_to_the_bone((1, FLASHBACK));
}

// Bump in the Night: "Target opponent loses 3 life." Flashback {5}{R}.
through_the_stack! {
    bump_in_the_night_takes_three_life_from_the_second_player: bump_in_the_night(FIRST);
    bump_in_the_night_cast_by_the_second_player_takes_three_life_from_the_first: bump_in_the_night(SECOND);
    bump_in_the_night_flashed_back_takes_three_life_and_is_exiled: bump_in_the_night(FLASHBACK);
    bump_in_the_night_is_not_cast_at_its_caster: is_not_cast_at_the_player((Box::new(BumpInTheNight), FIRST, 0));
    bump_in_the_night_cast_by_the_second_player_is_not_cast_at_themselves: is_not_cast_at_the_player((Box::new(BumpInTheNight), SECOND, 1));
    bump_in_the_night_is_not_cast_at_a_creature: is_not_cast_at((Box::new(BumpInTheNight), Permanent::Creature, FIRST, 1));
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
    /// Creatures you control gain protection from non-Human creatures
    /// (Spare from Evil).
    YoursProtectionFromNonHumans,
}

/// A spell that gives every creature of one side something until end of
/// turn, cast and resolved on a board of its caster's creature, its
/// caster's Vampire, the other player's Vampire and the other player's
/// Human: each creature the text names has it, read back through the
/// engine's own power, toughness, keywords and protection (CR 611.2c: the
/// creatures there as it resolves), and each other creature reads what it
/// read before; nothing is tapped, damaged or moved.
fn gives_each((card, does, cast): (Box<dyn CardBehavior>, ToAll, Cast)) {
    let (registry, mut state, blank, card) = board(card);
    let (you, other) = (cast.by, 1 - cast.by);
    let creatures = [
        a_permanent(&mut state, blank, Permanent::Creature, (you, you)),
        a_permanent(&mut state, blank, Permanent::Vampire, (you, you)),
        a_permanent(&mut state, blank, Permanent::Vampire, (other, other)),
        a_permanent(&mut state, blank, Permanent::Human, (other, other)),
    ];
    let spell = a_spell(&mut state, &registry, card, cast);
    let read = |state: &GameState, id| (state.effective_power(id, &registry), state.effective_toughness(id, &registry));
    let mut before = [((None, None), None); 4];
    for (i, &c) in creatures.iter().enumerate() {
        before[i] = (read(&state, c), as_it_is(&state, c));
    }

    cast_and_resolve(&mut state, &registry, spell, Vec::new());

    for (i, &c) in creatures.iter().enumerate() {
        let (yours, vampire) = (i < 2, i == 1 || i == 2);
        let (up, first_strike, protected) = match does {
            ToAll::YoursPlusTwo => (if yours { 2 } else { 0 }, false, false),
            ToAll::YourVampiresPlusTwoAndFirstStrike => (if yours && vampire { 2 } else { 0 }, yours && vampire, false),
            ToAll::TheirsMinusFour => (if yours { 0 } else { -4 }, false, false),
            ToAll::YoursProtectionFromNonHumans => (0, false, yours),
        };
        let ((power, toughness), was) = before[i];
        assert!(read(&state, c) == (power.map(|p: i32| p + up), toughness) && as_it_is(&state, c) == was);
        assert!(state.has_keyword(c, Keyword::FirstStrike, &registry) == first_strike);
        // From the other player's Vampire, which is no Human, and not from their Human.
        if i < 2 {
            assert!(state.has_protection_from(c, creatures[2], &registry) == protected);
            assert!(!state.has_protection_from(c, creatures[3], &registry));
        }
    }
    assert!(rests(&state, spell, cast));
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

/// "... each of up to two target creatures" cast at the first `targets`
/// of two creatures of the other player's and resolved, a third creature
/// beside them: each one targeted has had `does` done to it, read back
/// through the engine (a counter and the +1/+1 it gives, CR 122.1a,
/// 613.4c; tapped, 701.21a; unable to block), and each not targeted is as
/// it was. With no target the engine asks for the set (601.2c), and the
/// cast goes on from the answer that names none.
fn does_to_each_of_up_to_two((card, does, targets, cast): (Box<dyn CardBehavior>, ToEach, usize, Cast)) {
    use crate::actions::ResolvedChoice;
    use crate::state::{AwaitingAction, ResolutionChoiceKind};
    use crate::types::CounterType;
    let (registry, mut state, blank, card) = board(card);
    let other = 1 - cast.by;
    let creatures = [
        a_permanent(&mut state, blank, Permanent::Creature, (other, other)),
        a_permanent(&mut state, blank, Permanent::Creature, (other, other)),
        a_permanent(&mut state, blank, Permanent::Creature, (other, other)),
    ];
    let spell = a_spell(&mut state, &registry, card, cast);
    let chosen = [Target::Object(creatures[0]), Target::Object(creatures[1])];
    let read = |state: &GameState, id| (state.effective_power(id, &registry), state.effective_toughness(id, &registry));
    let mut before = [((None, None), None); 3];
    for (i, &c) in creatures.iter().enumerate() {
        before[i] = (read(&state, c), as_it_is(&state, c));
    }

    if targets == 0 {
        let caster = PlayerId(cast.by);
        let action = the_cast(spell, Vec::new());
        play(&mut state, &registry, &action);
        // Asked which, with nothing paid and the card where it was.
        assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTargetSet { min: 0, max: 2, .. }, .. }) if *player == caster));
        assert!(state.stack.is_empty() && state.players[usize::from(cast.by)].mana_pool.total() > 0);
        let answer = Action::ResolveChoice { choice: ResolvedChoice::ChosenTargetSet(Vec::new()) };
        play(&mut state, &registry, &answer);
        assert!(state.stack.len() == 1 && state.stack[0].as_spell() == Some(spell) && state.awaiting_action.is_none());
        assert!(state.players[usize::from(cast.by)].mana_pool.total() == 0 && state.priority_player == Some(caster));
        all_pass(&mut state, &registry);
        assert!(state.stack.is_empty());
        std::mem::forget((action, answer));
    } else {
        cast_and_resolve(&mut state, &registry, spell, chosen[..targets].to_vec());
    }

    for (i, &c) in creatures.iter().enumerate() {
        let ((power, toughness), was) = before[i];
        let was = was.expect("there");
        let hit = i < targets;
        let grown = i32::from(hit && matches!(does, ToEach::Counter));
        let o = state.get_object(c).expect("still an object");
        assert!(o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0) == u32::from(hit && matches!(does, ToEach::Counter)));
        assert!(as_it_is(&state, c) == Some(AsItWas { tapped: was.tapped || (hit && matches!(does, ToEach::Tap)), ..was }));
        assert!(read(&state, c) == (power.map(|p: i32| p + grown), toughness.map(|t: i32| t + grown)));
        assert!(state.can_block(c, &registry) != (hit && matches!(does, ToEach::CantBlock)));
    }
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

use crate::cards::isd::{feeling_of_dread::FeelingOfDread, hysterical_blindness::HystericalBlindness, nightbirds_clutches::NightbirdsClutches};
use crate::cards::isd::{rally_the_peasants::RallyThePeasants, spare_from_evil::SpareFromEvil, travel_preparations::TravelPreparations, vampiric_fury::VampiricFury};

// Rally the Peasants: "Creatures you control get +2/+0 until end of turn." Flashback {2}{R}.
through_the_stack! {
    rally_the_peasants_gives_its_casters_creatures_two_power: gives_each((Box::new(RallyThePeasants), ToAll::YoursPlusTwo, FIRST));
    rally_the_peasants_cast_by_the_second_player_gives_theirs_two_power: gives_each((Box::new(RallyThePeasants), ToAll::YoursPlusTwo, SECOND));
    rally_the_peasants_flashed_back_gives_two_power_and_is_exiled: gives_each((Box::new(RallyThePeasants), ToAll::YoursPlusTwo, FLASHBACK));
}

// Vampiric Fury: "Vampire creatures you control get +2/+0 and gain first strike until end of turn."
through_the_stack! {
    vampiric_fury_gives_its_casters_vampires_two_power_and_first_strike: gives_each((Box::new(VampiricFury), ToAll::YourVampiresPlusTwoAndFirstStrike, FIRST));
    vampiric_fury_cast_by_the_second_player_gives_their_vampires_two_power: gives_each((Box::new(VampiricFury), ToAll::YourVampiresPlusTwoAndFirstStrike, SECOND));
}

// Hysterical Blindness: "Creatures your opponents control get -4/-0 until end of turn."
through_the_stack! {
    hysterical_blindness_takes_four_power_from_the_other_players_creatures: gives_each((Box::new(HystericalBlindness), ToAll::TheirsMinusFour, FIRST));
    hysterical_blindness_cast_by_the_second_player_takes_four_power_from_the_firsts: gives_each((Box::new(HystericalBlindness), ToAll::TheirsMinusFour, SECOND));
}

// Spare from Evil: "Creatures you control gain protection from non-Human creatures until end of turn."
through_the_stack! {
    spare_from_evil_protects_its_casters_creatures_from_what_is_no_human: gives_each((Box::new(SpareFromEvil), ToAll::YoursProtectionFromNonHumans, FIRST));
    spare_from_evil_cast_by_the_second_player_protects_theirs: gives_each((Box::new(SpareFromEvil), ToAll::YoursProtectionFromNonHumans, SECOND));
}

// Feeling of Dread: "Tap up to two target creatures." Flashback {1}{U}.
through_the_stack! {
    feeling_of_dread_at_no_creature_taps_none: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 0, FIRST));
    feeling_of_dread_at_one_creature_taps_it: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 1, FIRST));
    feeling_of_dread_at_two_creatures_taps_both: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 2, FIRST));
    feeling_of_dread_cast_by_the_second_player_taps_two_of_the_first_players: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 2, SECOND));
    feeling_of_dread_flashed_back_taps_two_and_is_exiled: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 2, FLASHBACK));
    feeling_of_dread_is_not_cast_at_an_artifact: is_not_cast_at((Box::new(FeelingOfDread), Permanent::Artifact, FIRST, 1));
}

// Nightbird's Clutches: "Up to two target creatures can't block this turn." Flashback {3}{R}.
through_the_stack! {
    nightbirds_clutches_at_no_creature_stops_none_blocking: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 0, FIRST));
    nightbirds_clutches_at_one_creature_stops_it_blocking: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 1, FIRST));
    nightbirds_clutches_at_two_creatures_stops_both_blocking: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 2, FIRST));
    nightbirds_clutches_cast_by_the_second_player_stops_two_blocking: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 2, SECOND));
    nightbirds_clutches_flashed_back_stops_two_blocking_and_is_exiled: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 2, FLASHBACK));
    nightbirds_clutches_is_not_cast_at_a_land: is_not_cast_at((Box::new(NightbirdsClutches), Permanent::Land, FIRST, 1));
}

// Travel Preparations: "Put a +1/+1 counter on each of up to two target creatures." Flashback {1}{W}.
through_the_stack! {
    travel_preparations_at_no_creature_puts_no_counter: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 0, FIRST));
    travel_preparations_at_one_creature_puts_a_counter_on_it: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 1, FIRST));
    travel_preparations_at_two_creatures_puts_a_counter_on_each: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 2, FIRST));
    travel_preparations_cast_by_the_second_player_puts_a_counter_on_each_of_two: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 2, SECOND));
    travel_preparations_flashed_back_puts_two_counters_and_is_exiled: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 2, FLASHBACK));
    travel_preparations_is_not_cast_at_an_enchantment: is_not_cast_at((Box::new(TravelPreparations), Permanent::Enchantment, FIRST, 1));
}

/// Silent Departure ("Return target creature to its owner's hand.") cast
/// at `owner`'s creature under `controller` and resolved: the creature is
/// in its owner's hand, under its owner, as a new object with no damage
/// and untapped (CR 400.7), whoever controlled it; a second creature
/// beside it is as it was.
fn silent_departure((cast, (owner, controller)): (Cast, (u8, u8))) {
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::isd::silent_departure::SilentDeparture));
    let target = a_permanent(&mut state, blank, Permanent::Creature, (owner, controller));
    let beside = a_permanent(&mut state, blank, Permanent::Creature, (owner, controller));
    let spell = a_spell(&mut state, &registry, card, cast);
    let beside_before = as_it_is(&state, beside);

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(target)]);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Hand && o.controller == PlayerId(owner) && o.owner == PlayerId(owner) && !o.tapped && o.damage_marked == 0);
    assert!(as_it_is(&state, beside) == beside_before && beside_before.is_some());
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// Grasp of Phantoms ("Put target creature on top of its owner's
/// library.") cast at `owner`'s creature under `controller`, their library
/// of `cards` cards, and resolved: the creature is the top card of its
/// owner's library (CR 401.4), a card there and no longer a permanent, and
/// the cards that were there are under it in the order they were.
fn grasp_of_phantoms((cards, cast, (owner, controller)): (usize, Cast, (u8, u8))) {
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::isd::grasp_of_phantoms::GraspOfPhantoms));
    let library = a_library(&mut state, blank, owner, cards);
    let target = a_permanent(&mut state, blank, Permanent::Creature, (owner, controller));
    let spell = a_spell(&mut state, &registry, card, cast);

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(target)]);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Library && o.controller == PlayerId(owner) && !o.tapped && o.damage_marked == 0);
    let order = &state.players[usize::from(owner)].library_order;
    assert!(order.len() == cards + 1 && order[0] == target);
    for k in 0..cards {
        assert!(order[k + 1] == library[k].expect("made"));
    }
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// Swords to Plowshares ("Exile target creature. Its controller gains
/// life equal to its power.") cast at `owner`'s creature under
/// `controller`, of power 2 or (`Monstrous`) 4, and resolved: the creature
/// is in exile, its owner's (CR 406.1), and the player who controlled it
/// has that much life more (119.3), the other as much as before; a second
/// creature beside it is as it was.
fn swords_to_plowshares((what, cast, (owner, controller)): (Permanent, Cast, (u8, u8))) {
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::swords_to_plowshares::SwordsToPlowshares));
    let target = a_permanent(&mut state, blank, what, (owner, controller));
    let beside = a_permanent(&mut state, blank, what, (owner, controller));
    let spell = a_spell(&mut state, &registry, card, cast);
    let power = if what == Permanent::Monstrous { 4 } else { 2 };
    let beside_before = as_it_is(&state, beside);

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(target)]);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == Zone::Exile && o.controller == PlayerId(owner) && o.owner == PlayerId(owner));
    for p in 0..2u8 {
        assert!(state.players[usize::from(p)].life == 20 + if p == controller { power } else { 0 });
    }
    assert!(as_it_is(&state, beside) == beside_before && beside_before.is_some());
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// Traitorous Blood ("Gain control of target creature until end of turn.
/// Untap it. It gains trample and haste until end of turn.") cast at
/// `controller`'s tapped creature and resolved: its caster
/// controls it (CR 613.1b), its owner unchanged, it is untapped (701.21a)
/// and has trample and haste, read back through the engine's own
/// keywords, its damage is as it was, and a second creature beside it is
/// as it was and has neither keyword.
fn traitorous_blood((cast, controller): (Cast, u8)) {
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::isd::traitorous_blood::TraitorousBlood));
    let target = a_permanent(&mut state, blank, Permanent::Creature, (controller, controller));
    let beside = a_permanent(&mut state, blank, Permanent::Creature, (controller, controller));
    state.get_object_mut(target).expect("just created").tapped = true;
    state.get_object_mut(beside).expect("just created").tapped = true;
    let spell = a_spell(&mut state, &registry, card, cast);
    let (before, beside_before) = (as_it_is(&state, target).expect("there"), as_it_is(&state, beside));

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(target)]);

    assert!(as_it_is(&state, target) == Some(AsItWas { tapped: false, controller: PlayerId(cast.by), ..before }));
    assert!(state.get_object(target).expect("still an object").owner == PlayerId(controller));
    assert!(state.has_keyword(target, Keyword::Trample, &registry) && state.has_keyword(target, Keyword::Haste, &registry));
    assert!(as_it_is(&state, beside) == beside_before);
    assert!(!state.has_keyword(beside, Keyword::Trample, &registry) && !state.has_keyword(beside, Keyword::Haste, &registry));
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// A spell that moves its one target card from a zone that is not the
/// battlefield to another, cast at a creature card of the player `whose`
/// names and resolved: the card is in the zone the text names, under its
/// owner (CR 400.3, 108.3), no longer in the graveyard's order if it came
/// from there, and on the battlefield untapped, a creature and summoning
/// sick (302.6); a second card beside it where it was is where it was.
fn moves_a_card((card, from, to, cast, whose): (Box<dyn CardBehavior>, Zone, Zone, Cast, u8)) {
    let (registry, mut state, blank, card) = board(card);
    let target = a_card_in(&mut state, blank, whose, from, true);
    let beside = a_card_in(&mut state, blank, whose, from, true);
    let spell = a_spell(&mut state, &registry, card, cast);

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(target)]);

    let o = state.get_object(target).expect("still an object");
    assert!(o.zone == to && o.owner == PlayerId(whose) && o.controller == PlayerId(whose) && !o.tapped);
    if to == Zone::Battlefield {
        assert!(o.summoning_sick && state.is_creature(target, &registry));
    }
    assert!(!state.players[usize::from(whose)].graveyard_order.contains(&target));
    assert!(state.get_object(beside).expect("still an object").zone == from);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// A spell with one target cast at a card (a creature card or not) in a
/// zone that is not the battlefield, the player `whose` names its owner,
/// where its text does not let it target that card: the engine refuses
/// the cast (CR 601.2c), and the card is where it was.
fn is_not_cast_at_the_card((card, zone, creature, cast, whose): (Box<dyn CardBehavior>, Zone, bool, Cast, u8)) {
    let (registry, mut state, blank, card) = board(card);
    let target = a_card_in(&mut state, blank, whose, zone, creature);
    let spell = a_spell(&mut state, &registry, card, cast);

    cast_is_refused(&mut state, &registry, spell, vec![Target::Object(target)]);

    assert!(state.get_object(target).expect("still an object").zone == zone);
    std::mem::forget((registry, state));
}

/// Runic Repetition ("Return target exiled card with flashback you own to
/// your hand.") cast at a card with flashback in exile (Think Twice) its
/// caster owns, and resolved: the card is in its owner's hand, and an
/// exiled card beside it is still in exile. At one the other player owns
/// (`yours` false), the engine refuses the cast and nothing moves.
fn runic_repetition((cast, yours): (Cast, bool)) {
    let (mut registry, mut state, blank, card) = board(Box::new(crate::cards::isd::runic_repetition::RunicRepetition));
    let flashback = registry.register(Box::new(ThinkTwice));
    let owner = if yours { cast.by } else { 1 - cast.by };
    let target = state.create_object(flashback, PlayerId(owner), Zone::Exile, None, None);
    let beside = a_card_in(&mut state, blank, cast.by, Zone::Exile, false);
    let spell = a_spell(&mut state, &registry, card, cast);

    if yours {
        cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(target)]);
        let o = state.get_object(target).expect("still an object");
        assert!(o.zone == Zone::Hand && o.owner == PlayerId(owner) && o.controller == PlayerId(owner));
        assert!(rests(&state, spell, cast));
    } else {
        cast_is_refused(&mut state, &registry, spell, vec![Target::Object(target)]);
        assert!(state.get_object(target).expect("still an object").zone == Zone::Exile);
    }
    assert!(state.get_object(beside).expect("still an object").zone == Zone::Exile);
    std::mem::forget((registry, state));
}

use crate::cards::isd::{grasp_of_phantoms::GraspOfPhantoms, purify_the_grave::PurifyTheGrave, runic_repetition::RunicRepetition};
use crate::cards::isd::{silent_departure::SilentDeparture, traitorous_blood::TraitorousBlood, unburial_rites::UnburialRites};
use crate::cards::swords_to_plowshares::SwordsToPlowshares;

// Silent Departure: "Return target creature to its owner's hand." Flashback {4}{U}.
through_the_stack! {
    silent_departure_returns_the_other_players_creature: silent_departure((FIRST, (1, 1)));
    silent_departure_cast_by_the_second_player_returns_the_first_players: silent_departure((SECOND, (0, 0)));
    silent_departure_returns_your_own_creature: silent_departure((FIRST, (0, 0)));
    silent_departure_returns_a_creature_to_its_owner_not_its_controller: silent_departure((FIRST, (1, 0)));
    silent_departure_flashed_back_returns_a_creature_and_is_exiled: silent_departure((FLASHBACK, (1, 1)));
    silent_departure_is_not_cast_at_an_artifact: is_not_cast_at((Box::new(SilentDeparture), Permanent::Artifact, FIRST, 1));
    silent_departure_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(SilentDeparture), Permanent::Creature, Box::new(LightningBolt), FIRST));
}

// Grasp of Phantoms: "Put target creature on top of its owner's library." Flashback {7}{U}.
through_the_stack! {
    grasp_of_phantoms_puts_the_other_players_creature_on_their_library_of_two: grasp_of_phantoms((2, FIRST, (1, 1)));
    grasp_of_phantoms_puts_a_creature_on_an_empty_library: grasp_of_phantoms((0, FIRST, (1, 1)));
    grasp_of_phantoms_cast_by_the_second_player_puts_the_first_players_on_their_library: grasp_of_phantoms((2, SECOND, (0, 0)));
    grasp_of_phantoms_puts_a_creature_on_its_owners_library_not_its_controllers: grasp_of_phantoms((1, FIRST, (1, 0)));
    grasp_of_phantoms_flashed_back_puts_a_creature_on_its_library_and_is_exiled: grasp_of_phantoms((1, FLASHBACK, (1, 1)));
    grasp_of_phantoms_is_not_cast_at_a_land: is_not_cast_at((Box::new(GraspOfPhantoms), Permanent::Land, FIRST, 1));
    grasp_of_phantoms_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(GraspOfPhantoms), Permanent::Creature, Box::new(LightningBolt), FIRST));
}

// Swords to Plowshares: "Exile target creature. Its controller gains life equal to its power."
through_the_stack! {
    swords_to_plowshares_exiles_the_other_players_creature_and_they_gain_two: swords_to_plowshares((Permanent::Creature, FIRST, (1, 1)));
    swords_to_plowshares_cast_by_the_second_player_exiles_the_first_players: swords_to_plowshares((Permanent::Creature, SECOND, (0, 0)));
    swords_to_plowshares_exiles_your_own_creature_and_you_gain_two: swords_to_plowshares((Permanent::Creature, FIRST, (0, 0)));
    swords_to_plowshares_exiles_a_four_power_creature_and_its_controller_gains_four: swords_to_plowshares((Permanent::Monstrous, FIRST, (1, 1)));
    swords_to_plowshares_gives_the_life_to_its_controller_not_its_owner: swords_to_plowshares((Permanent::Creature, FIRST, (1, 0)));
    swords_to_plowshares_is_not_cast_at_an_enchantment: is_not_cast_at((Box::new(SwordsToPlowshares), Permanent::Enchantment, FIRST, 1));
    swords_to_plowshares_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(SwordsToPlowshares), Permanent::Creature, Box::new(LightningBolt), FIRST));
}

// Traitorous Blood: "Gain control of target creature until end of turn. Untap it. It gains trample and haste until end of turn."
through_the_stack! {
    traitorous_blood_takes_the_other_players_creature: traitorous_blood((FIRST, 1));
    traitorous_blood_cast_by_the_second_player_takes_the_first_players_creature: traitorous_blood((SECOND, 0));
    traitorous_blood_on_your_own_creature_untaps_it_and_gives_it_the_keywords: traitorous_blood((FIRST, 0));
    traitorous_blood_is_not_cast_at_a_planeswalker: is_not_cast_at((Box::new(TraitorousBlood), Permanent::Planeswalker, FIRST, 1));
    traitorous_blood_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(TraitorousBlood), Permanent::Creature, Box::new(LightningBolt), FIRST));
}

// Unburial Rites: "Return target creature card from your graveyard to the battlefield." Flashback {3}{W}.
through_the_stack! {
    unburial_rites_returns_a_creature_card_from_your_graveyard: moves_a_card((Box::new(UnburialRites), Zone::Graveyard, Zone::Battlefield, FIRST, 0));
    unburial_rites_cast_by_the_second_player_returns_one_from_theirs: moves_a_card((Box::new(UnburialRites), Zone::Graveyard, Zone::Battlefield, SECOND, 1));
    unburial_rites_flashed_back_returns_a_creature_card_and_is_exiled: moves_a_card((Box::new(UnburialRites), Zone::Graveyard, Zone::Battlefield, FLASHBACK, 0));
    unburial_rites_is_not_cast_at_a_card_in_the_other_players_graveyard: is_not_cast_at_the_card((Box::new(UnburialRites), Zone::Graveyard, true, FIRST, 1));
    unburial_rites_is_not_cast_at_a_card_that_is_no_creature: is_not_cast_at_the_card((Box::new(UnburialRites), Zone::Graveyard, false, FIRST, 0));
    unburial_rites_is_not_cast_at_a_creature_on_the_battlefield: is_not_cast_at((Box::new(UnburialRites), Permanent::Creature, FIRST, 0));
}

// Purify the Grave: "Exile target card from a graveyard." Flashback {W}.
through_the_stack! {
    purify_the_grave_exiles_a_card_from_the_other_players_graveyard: moves_a_card((Box::new(PurifyTheGrave), Zone::Graveyard, Zone::Exile, FIRST, 1));
    purify_the_grave_exiles_a_card_from_your_own_graveyard: moves_a_card((Box::new(PurifyTheGrave), Zone::Graveyard, Zone::Exile, FIRST, 0));
    purify_the_grave_cast_by_the_second_player_exiles_a_card_from_the_first_players: moves_a_card((Box::new(PurifyTheGrave), Zone::Graveyard, Zone::Exile, SECOND, 0));
    purify_the_grave_flashed_back_exiles_a_card_and_is_exiled: moves_a_card((Box::new(PurifyTheGrave), Zone::Graveyard, Zone::Exile, FLASHBACK, 1));
    purify_the_grave_is_not_cast_at_a_card_in_exile: is_not_cast_at_the_card((Box::new(PurifyTheGrave), Zone::Exile, true, FIRST, 1));
    purify_the_grave_is_not_cast_at_a_creature_on_the_battlefield: is_not_cast_at((Box::new(PurifyTheGrave), Permanent::Creature, FIRST, 1));
}

// Runic Repetition: "Return target exiled card with flashback you own to your hand."
through_the_stack! {
    runic_repetition_returns_your_exiled_card_with_flashback: runic_repetition((FIRST, true));
    runic_repetition_cast_by_the_second_player_returns_theirs: runic_repetition((SECOND, true));
    runic_repetition_is_not_cast_at_the_other_players_exiled_card: runic_repetition((FIRST, false));
    runic_repetition_is_not_cast_at_an_exiled_card_without_flashback: is_not_cast_at_the_card((Box::new(RunicRepetition), Zone::Exile, false, FIRST, 0));
    runic_repetition_is_not_cast_at_a_card_in_a_graveyard: is_not_cast_at_the_card((Box::new(RunicRepetition), Zone::Graveyard, false, FIRST, 0));
}

/// What "counter target spell" does to the spell it counters.
#[derive(Clone, Copy, PartialEq)]
enum Countered {
    /// Put into its owner's graveyard (CR 701.5a).
    Buried,
    /// Exiled instead (Dissipate, 614.1a).
    Exiled,
}

/// "Counter target spell." The player `first` names casts Divination with
/// two cards in their library, and `by` (the other player in response, or
/// the same player with their own spell on the stack) casts the counter at
/// it; each player passes and the counter resolves: Divination is off the
/// stack, in its owner's graveyard or (`Countered::Exiled`) in exile
/// instead, and has done nothing, the two cards still in the library; the
/// counter is in its caster's graveyard, the stack is empty and priority
/// is the active player's. `flashback`: Divination's place is taken by
/// Think Twice cast with flashback, which is exiled whichever counter it
/// was (702.34a).
fn counters((card, fate, first, by, flashback): (Box<dyn CardBehavior>, Countered, u8, u8, bool)) {
    let (mut registry, mut state, blank, card) = board(card);
    let victim_card = registry.register(if flashback { Box::new(ThinkTwice) as Box<dyn CardBehavior> } else { Box::new(Divination) });
    let library = a_library(&mut state, blank, first, 2);
    let victim = a_spell(&mut state, &registry, victim_card, Cast { by: first, flashback });
    let counter = holds(&mut state, &registry, card, PlayerId(by));

    // The spell to be countered, cast first; mana for both is in the pool
    // of a player who casts both, so the pool is not empty between them.
    let action = the_cast(victim, Vec::new());
    play(&mut state, &registry, &action);
    assert!(state.stack.len() == 1 && state.stack[0].as_spell() == Some(victim) && state.priority_player == Some(PlayerId(first)));
    if by != first {
        play(&mut state, &registry, &Action::PassPriority);
        assert!(state.priority_player == Some(PlayerId(by)));
    }
    casts(&mut state, &registry, counter, vec![Target::Object(victim)]);
    assert!(state.stack.len() == 2);
    all_pass(&mut state, &registry);

    assert!(state.stack.is_empty());
    let v = state.get_object(victim).expect("still an object");
    let exiled = fate == Countered::Exiled || flashback;
    assert!(v.zone == if exiled { Zone::Exile } else { Zone::Graveyard } && v.owner == PlayerId(first));
    assert!(state.players[usize::from(first)].graveyard_order.contains(&victim) != exiled);
    for card in library.iter().take(2) {
        assert!(state.get_object(card.expect("made")).expect("still an object").zone == Zone::Library);
    }
    assert!(state.players[usize::from(first)].library_order.len() == 2);
    assert!(rests(&state, counter, Cast { by, flashback: false }));
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    std::mem::forget((registry, state, action));
}

/// Lost in the Mist ("Counter target spell. Return target permanent to
/// its owner's hand.") cast by the second player at the first player's
/// Divination on the stack and at `controller`'s creature: Divination is
/// countered, in its owner's graveyard having drawn nothing, and the
/// creature is in its owner's hand (CR 701.5a, 400.3); a second creature
/// is as it was.
fn lost_in_the_mist(controller: u8) {
    let (mut registry, mut state, blank, card) = board(Box::new(crate::cards::isd::lost_in_the_mist::LostInTheMist));
    let victim_card = registry.register(Box::new(Divination));
    let library = a_library(&mut state, blank, 0, 2);
    let permanent = a_permanent(&mut state, blank, Permanent::Creature, (controller, controller));
    let beside = a_permanent(&mut state, blank, Permanent::Creature, (controller, controller));
    let victim = a_spell(&mut state, &registry, victim_card, FIRST);
    let counter = holds(&mut state, &registry, card, PlayerId(1));
    let beside_before = as_it_is(&state, beside);

    casts(&mut state, &registry, victim, Vec::new());
    play(&mut state, &registry, &Action::PassPriority);
    casts(&mut state, &registry, counter, vec![Target::Object(victim), Target::Object(permanent)]);
    all_pass(&mut state, &registry);

    assert!(state.stack.is_empty());
    let v = state.get_object(victim).expect("still an object");
    assert!(v.zone == Zone::Graveyard && state.players[0].graveyard_order.contains(&victim));
    assert!(state.players[0].library_order.len() == 2 && state.get_object(library[0].expect("made")).expect("still an object").zone == Zone::Library);
    let o = state.get_object(permanent).expect("still an object");
    assert!(o.zone == Zone::Hand && o.controller == PlayerId(controller) && !o.tapped && o.damage_marked == 0);
    assert!(as_it_is(&state, beside) == beside_before && beside_before.is_some());
    assert!(rests(&state, counter, SECOND));
    std::mem::forget((registry, state));
}

/// A spell whose text has "As an additional cost to cast this spell,
/// sacrifice a creature", cast by `cast` naming one of its caster's two
/// creatures: that creature is in its owner's graveyard as soon as the
/// spell is cast (CR 601.2h), before anything resolves, and the other is
/// as it was throughout. Then what the spell does: Altar's Reap's caster
/// has drawn the top two of their library of three, or Infernal Plunge's
/// has three red mana and no other in their pool (106.4).
fn sacrifices_a_creature_to((card, draws, cast): (Box<dyn CardBehavior>, bool, Cast)) {
    let (registry, mut state, blank, card) = board(card);
    let you = cast.by;
    let library = a_library(&mut state, blank, you, 3);
    let paid = a_permanent(&mut state, blank, Permanent::Creature, (you, you));
    let kept = a_permanent(&mut state, blank, Permanent::Creature, (you, you));
    let theirs = a_permanent(&mut state, blank, Permanent::Creature, (1 - you, 1 - you));
    let spell = a_spell(&mut state, &registry, card, cast);
    let before = [as_it_is(&state, kept), as_it_is(&state, theirs)];
    let Action::CastSpell { object_id, targets, exile_count, exile_ids, alternative_cost, tap_plan, .. } = the_cast(spell, Vec::new()) else { panic!("a cast") };
    let action = Action::CastSpell { object_id, targets, sacrifice: Some(paid), exile_count, exile_ids, alternative_cost, tap_plan };

    casts_as(&mut state, &registry, action);
    // Paid as the spell was cast.
    let p = state.get_object(paid).expect("still an object");
    assert!(p.zone == Zone::Graveyard && state.players[usize::from(you)].graveyard_order.contains(&paid));
    assert!(as_it_is(&state, kept) == before[0] && as_it_is(&state, theirs) == before[1]);
    all_pass(&mut state, &registry);

    assert!(state.stack.is_empty());
    assert!(as_it_is(&state, kept) == before[0] && as_it_is(&state, theirs) == before[1]);
    let pool = &state.players[usize::from(you)].mana_pool;
    if draws {
        for (k, card) in library.iter().take(3).enumerate() {
            assert!(state.get_object(card.expect("made")).expect("still an object").zone == if k < 2 { Zone::Hand } else { Zone::Library });
        }
        assert!(pool.total() == 0);
    } else {
        assert!(pool.get(ManaType::Red) == 3 && pool.total() == 3);
        assert!(state.players[usize::from(you)].library_order.len() == 3);
    }
    assert!(state.players[usize::from(1 - you)].mana_pool.total() == 0);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// The same spell cast naming a creature its caster may not sacrifice
/// (the other player's), or naming none with no creature of its caster's
/// to take: the engine refuses the cast (CR 601.2h) and nothing is paid
/// or sacrificed.
fn is_not_cast_without_its_sacrifice((card, names_theirs, cast): (Box<dyn CardBehavior>, bool, Cast)) {
    let (registry, mut state, blank, card) = board(card);
    let theirs = a_permanent(&mut state, blank, Permanent::Creature, (1 - cast.by, 1 - cast.by));
    let spell = a_spell(&mut state, &registry, card, cast);
    let before = as_it_is(&state, theirs);
    let Action::CastSpell { object_id, targets, exile_count, exile_ids, alternative_cost, tap_plan, .. } = the_cast(spell, Vec::new()) else { panic!("a cast") };
    let action = Action::CastSpell { object_id, targets, sacrifice: names_theirs.then_some(theirs), exile_count, exile_ids, alternative_cost, tap_plan };

    cast_is_refused_as(&mut state, &registry, action);

    assert!(as_it_is(&state, theirs) == before && before.is_some());
    std::mem::forget((registry, state));
}

/// Prey Upon ("Target creature you control fights target creature you
/// don't control.") cast at its caster's 2/2 and the other player's
/// creature (`theirs`: a 2/2 or a 2/6), and resolved, the state-based actions checked: each has been
/// dealt 2 damage by the other (CR 701.14a), and one whose damage is then
/// at least its toughness has been destroyed (704.5g), the other on the
/// battlefield with 2 more marked than before; a third creature is as it
/// was and no player's life has changed.
fn prey_upon((cast, theirs): (Cast, Permanent)) {
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::isd::prey_upon::PreyUpon));
    let (you, other) = (cast.by, 1 - cast.by);
    let fighters = [a_permanent(&mut state, blank, Permanent::Creature, (you, you)), a_permanent(&mut state, blank, theirs, (other, other))];
    let beside = a_permanent(&mut state, blank, Permanent::Creature, (other, other));
    let spell = a_spell(&mut state, &registry, card, cast);
    let mut before = [(None, 0); 2];
    for (i, &f) in fighters.iter().enumerate() {
        before[i] = (as_it_is(&state, f), state.get_object(f).expect("just created").toughness.expect("a creature"));
    }
    let beside_before = as_it_is(&state, beside);

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(fighters[0]), Target::Object(fighters[1])]);

    for (i, &f) in fighters.iter().enumerate() {
        let (was, toughness) = before[i];
        let was = was.expect("there");
        if i64::from(was.damage + 2) >= i64::from(toughness) {
            assert!(state.get_object(f).expect("still an object").zone == Zone::Graveyard);
        } else {
            assert!(as_it_is(&state, f) == Some(AsItWas { damage: was.damage + 2, ..was }));
        }
    }
    assert!(as_it_is(&state, beside) == beside_before);
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// Prey Upon cast at two creatures its text does not pair: both its
/// caster's, or both the other player's, or the two the wrong way round.
/// The engine refuses the cast (CR 601.2c).
fn prey_upon_is_not_cast_at((first, second): (u8, u8)) {
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::isd::prey_upon::PreyUpon));
    let fighters = [a_permanent(&mut state, blank, Permanent::Creature, (first, first)), a_permanent(&mut state, blank, Permanent::Creature, (second, second))];
    let spell = a_spell(&mut state, &registry, card, FIRST);
    let before = [as_it_is(&state, fighters[0]), as_it_is(&state, fighters[1])];

    cast_is_refused(&mut state, &registry, spell, vec![Target::Object(fighters[0]), Target::Object(fighters[1])]);

    assert!(as_it_is(&state, fighters[0]) == before[0] && as_it_is(&state, fighters[1]) == before[1]);
    std::mem::forget((registry, state));
}

/// "Destroy target land. ..." cast at `controller`'s land and resolved:
/// the land is in its owner's graveyard (CR 701.7a) and a second land is
/// as it was. Maw of the Mire's caster has 4 life more (119.3) and the
/// other player as much as before; Into the Maw of Hell also deals 13
/// damage to its second target, a creature of the other player's, which
/// is destroyed (704.5g), a second creature as it was.
fn destroys_a_land_and((card, thirteen, cast, controller): (Box<dyn CardBehavior>, bool, Cast, u8)) {
    let (registry, mut state, blank, card) = board(card);
    let land = a_permanent(&mut state, blank, Permanent::Land, (controller, controller));
    let other_land = a_permanent(&mut state, blank, Permanent::Land, (controller, controller));
    let creatures = [a_permanent(&mut state, blank, Permanent::Stout, (1 - cast.by, 1 - cast.by)), a_permanent(&mut state, blank, Permanent::Stout, (1 - cast.by, 1 - cast.by))];
    let spell = a_spell(&mut state, &registry, card, cast);
    let before = [as_it_is(&state, other_land), as_it_is(&state, creatures[0]), as_it_is(&state, creatures[1])];
    let targets = if thirteen { vec![Target::Object(land), Target::Object(creatures[0])] } else { vec![Target::Object(land)] };

    cast_and_resolve(&mut state, &registry, spell, targets);

    let l = state.get_object(land).expect("still an object");
    assert!(l.zone == Zone::Graveyard && l.owner == PlayerId(controller));
    assert!(as_it_is(&state, other_land) == before[0]);
    if thirteen {
        assert!(state.get_object(creatures[0]).expect("still an object").zone == Zone::Graveyard);
        assert!(state.players[0].life == 20 && state.players[1].life == 20);
    } else {
        assert!(as_it_is(&state, creatures[0]) == before[1]);
        assert!(state.players[usize::from(cast.by)].life == 24 && state.players[usize::from(1 - cast.by)].life == 20);
    }
    assert!(as_it_is(&state, creatures[1]) == before[2]);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

use crate::cards::counterspell::Counterspell;
use crate::cards::isd::{altars_reap::AltarsReap, dissipate::Dissipate, infernal_plunge::InfernalPlunge, into_the_maw_of_hell::IntoTheMawOfHell, maw_of_the_mire::MawOfTheMire};

// Counterspell: "Counter target spell."
through_the_stack! {
    counterspell_counters_the_first_players_spell: counters((Box::new(Counterspell), Countered::Buried, 0, 1, false));
    counterspell_cast_by_the_first_player_counters_the_second_players_spell: counters((Box::new(Counterspell), Countered::Buried, 1, 0, false));
    counterspell_counters_its_casters_own_spell: counters((Box::new(Counterspell), Countered::Buried, 0, 0, false));
    counterspell_counters_a_spell_cast_with_flashback_which_is_exiled: counters((Box::new(Counterspell), Countered::Buried, 0, 1, true));
    counterspell_is_not_cast_at_a_creature: is_not_cast_at((Box::new(Counterspell), Permanent::Creature, FIRST, 1));
    counterspell_is_not_cast_at_a_card_in_a_graveyard: is_not_cast_at_the_card((Box::new(Counterspell), Zone::Graveyard, false, FIRST, 1));
}

// Dissipate: "Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard."
through_the_stack! {
    dissipate_counters_the_first_players_spell_and_exiles_it: counters((Box::new(Dissipate), Countered::Exiled, 0, 1, false));
    dissipate_cast_by_the_first_player_counters_the_second_players_spell_and_exiles_it: counters((Box::new(Dissipate), Countered::Exiled, 1, 0, false));
    dissipate_counters_its_casters_own_spell_and_exiles_it: counters((Box::new(Dissipate), Countered::Exiled, 0, 0, false));
    dissipate_is_not_cast_at_a_creature: is_not_cast_at((Box::new(Dissipate), Permanent::Creature, FIRST, 1));
}

// Lost in the Mist: "Counter target spell. Return target permanent to its owner's hand."
through_the_stack! {
    lost_in_the_mist_counters_a_spell_and_returns_the_first_players_creature: lost_in_the_mist(0);
    lost_in_the_mist_counters_a_spell_and_returns_its_casters_own_creature: lost_in_the_mist(1);
}

// Altar's Reap: "As an additional cost to cast this spell, sacrifice a creature. Draw two cards."
through_the_stack! {
    altars_reap_sacrifices_a_creature_as_it_is_cast_and_draws_two: sacrifices_a_creature_to((Box::new(AltarsReap), true, FIRST));
    altars_reap_cast_by_the_second_player_sacrifices_theirs_and_draws_two: sacrifices_a_creature_to((Box::new(AltarsReap), true, SECOND));
    altars_reap_is_not_cast_sacrificing_the_other_players_creature: is_not_cast_without_its_sacrifice((Box::new(AltarsReap), true, FIRST));
    altars_reap_is_not_cast_with_no_creature_to_sacrifice: is_not_cast_without_its_sacrifice((Box::new(AltarsReap), false, FIRST));
}

// Infernal Plunge: "As an additional cost to cast this spell, sacrifice a creature. Add {R}{R}{R}."
through_the_stack! {
    infernal_plunge_sacrifices_a_creature_as_it_is_cast_and_adds_three_red: sacrifices_a_creature_to((Box::new(InfernalPlunge), false, FIRST));
    infernal_plunge_cast_by_the_second_player_sacrifices_theirs_and_adds_three_red: sacrifices_a_creature_to((Box::new(InfernalPlunge), false, SECOND));
    infernal_plunge_is_not_cast_sacrificing_the_other_players_creature: is_not_cast_without_its_sacrifice((Box::new(InfernalPlunge), true, FIRST));
    infernal_plunge_is_not_cast_with_no_creature_to_sacrifice: is_not_cast_without_its_sacrifice((Box::new(InfernalPlunge), false, FIRST));
}

// Prey Upon: "Target creature you control fights target creature you don't control."
through_the_stack! {
    prey_upon_has_your_creature_fight_the_other_players_and_both_die: prey_upon((FIRST, Permanent::Creature));
    prey_upon_cast_by_the_second_player_has_theirs_fight_the_first_players: prey_upon((SECOND, Permanent::Creature));
    prey_upon_against_a_stout_creature_kills_yours_and_marks_two_on_theirs: prey_upon((FIRST, Permanent::Stout));
    prey_upon_is_not_cast_at_two_creatures_of_your_own: prey_upon_is_not_cast_at((0, 0));
    prey_upon_is_not_cast_at_two_creatures_of_the_other_players: prey_upon_is_not_cast_at((1, 1));
    prey_upon_is_not_cast_with_the_other_players_creature_named_first: prey_upon_is_not_cast_at((1, 0));
}

// Maw of the Mire: "Destroy target land. You gain 4 life."
through_the_stack! {
    maw_of_the_mire_destroys_the_other_players_land_and_gains_four: destroys_a_land_and((Box::new(MawOfTheMire), false, FIRST, 1));
    maw_of_the_mire_cast_by_the_second_player_destroys_the_first_players_land: destroys_a_land_and((Box::new(MawOfTheMire), false, SECOND, 0));
    maw_of_the_mire_destroys_your_own_land_and_gains_four: destroys_a_land_and((Box::new(MawOfTheMire), false, FIRST, 0));
    maw_of_the_mire_is_not_cast_at_a_creature: is_not_cast_at((Box::new(MawOfTheMire), Permanent::Creature, FIRST, 1));
    maw_of_the_mire_is_not_cast_at_an_artifact: is_not_cast_at((Box::new(MawOfTheMire), Permanent::Artifact, FIRST, 1));
}

// Into the Maw of Hell: "Destroy target land. Into the Maw of Hell deals 13 damage to target creature."
through_the_stack! {
    into_the_maw_of_hell_destroys_a_land_and_a_stout_creature: destroys_a_land_and((Box::new(IntoTheMawOfHell), true, FIRST, 1));
    into_the_maw_of_hell_cast_by_the_second_player_destroys_a_land_and_a_creature: destroys_a_land_and((Box::new(IntoTheMawOfHell), true, SECOND, 0));
    into_the_maw_of_hell_destroys_your_own_land_and_their_creature: destroys_a_land_and((Box::new(IntoTheMawOfHell), true, FIRST, 0));
}

/// Paraselene ("Destroy all enchantments. You gain 1 life for each
/// enchantment destroyed this way.") cast and resolved on a board of an
/// enchantment of each player's and a creature, the second player's
/// enchantment indestructible or not: each enchantment that is not
/// indestructible is in its owner's graveyard (CR 701.7a, 702.12b), the
/// creature is as it was, and its caster has 1 life more for each one
/// destroyed (119.3), the other player as much as before.
fn paraselene((cast, indestructible): (Cast, bool)) {
    use crate::state::TemporaryEffect;
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::isd::paraselene::Paraselene));
    let enchantments = [a_permanent(&mut state, blank, Permanent::Enchantment, (0, 0)), a_permanent(&mut state, blank, Permanent::Enchantment, (1, 1))];
    let creature = a_permanent(&mut state, blank, Permanent::Creature, (1 - cast.by, 1 - cast.by));
    if indestructible {
        state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target: enchantments[1], keyword: Keyword::Indestructible });
    }
    let spell = a_spell(&mut state, &registry, card, cast);
    let before = as_it_is(&state, creature);

    cast_and_resolve(&mut state, &registry, spell, Vec::new());

    assert!(state.get_object(enchantments[0]).expect("still an object").zone == Zone::Graveyard);
    assert!(state.get_object(enchantments[1]).expect("still an object").zone == if indestructible { Zone::Battlefield } else { Zone::Graveyard });
    assert!(as_it_is(&state, creature) == before && before.is_some());
    assert!(state.players[usize::from(cast.by)].life == 20 + if indestructible { 1 } else { 2 });
    assert!(state.players[usize::from(1 - cast.by)].life == 20);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// Rolling Temblor ("Rolling Temblor deals 2 damage to each creature
/// without flying.") cast and resolved on a board of a 2/6 of each
/// player's and an artifact, the second player's creature with flying or
/// not: each creature without flying has 2 damage marked on it (CR
/// 120.3e) and one with flying none; the artifact has none, nothing has
/// left the battlefield, and no player's life has changed.
fn rolling_temblor((cast, flying): (Cast, bool)) {
    use crate::state::TemporaryEffect;
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::isd::rolling_temblor::RollingTemblor));
    let creatures = [a_permanent(&mut state, blank, Permanent::Stout, (0, 0)), a_permanent(&mut state, blank, Permanent::Stout, (1, 1))];
    let artifact = a_permanent(&mut state, blank, Permanent::Artifact, (1, 1));
    if flying {
        state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target: creatures[1], keyword: Keyword::Flying });
    }
    let spell = a_spell(&mut state, &registry, card, cast);
    let before = [as_it_is(&state, creatures[0]).expect("there"), as_it_is(&state, creatures[1]).expect("there")];
    let artifact_before = as_it_is(&state, artifact);

    cast_and_resolve(&mut state, &registry, spell, Vec::new());

    assert!(as_it_is(&state, creatures[0]) == Some(AsItWas { damage: 2, ..before[0] }));
    assert!(as_it_is(&state, creatures[1]) == Some(AsItWas { damage: if flying { 0 } else { 2 }, ..before[1] }));
    assert!(as_it_is(&state, artifact) == artifact_before && artifact_before.is_some());
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// Blasphemous Act ("This spell costs {1} less to cast for each creature
/// on the battlefield. Blasphemous Act deals 13 damage to each creature.")
/// cast with `creatures` 2/6 creatures on the battlefield (the first its
/// caster's, a second the other player's) and an artifact, the nine mana
/// of its printed cost in its caster's pool: one mana for each creature is
/// left unspent as it is cast (CR 601.2f), and once it has resolved every
/// creature has been destroyed (704.5g), the artifact is as it was and no
/// player's life has changed.
fn blasphemous_act((creatures, cast): (usize, Cast)) {
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::isd::blasphemous_act::BlasphemousAct));
    let mut made = [None; 2];
    for (k, slot) in made.iter_mut().enumerate().take(creatures) {
        let whose = if k == 0 { cast.by } else { 1 - cast.by };
        *slot = Some(a_permanent(&mut state, blank, Permanent::Stout, (whose, whose)));
    }
    let artifact = a_permanent(&mut state, blank, Permanent::Artifact, (1 - cast.by, 1 - cast.by));
    let spell = a_spell(&mut state, &registry, card, cast);
    let artifact_before = as_it_is(&state, artifact);
    let action = the_cast(spell, Vec::new());

    play(&mut state, &registry, &action);
    assert!(state.stack.len() == 1 && state.stack[0].as_spell() == Some(spell));
    assert!(state.players[usize::from(cast.by)].mana_pool.total() == u32::try_from(creatures).expect("a few"));
    all_pass(&mut state, &registry);

    assert!(state.stack.is_empty());
    for creature in made.iter().take(creatures) {
        assert!(state.get_object(creature.expect("made")).expect("still an object").zone == Zone::Graveyard);
    }
    assert!(as_it_is(&state, artifact) == artifact_before && artifact_before.is_some());
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state, action));
}

/// Sever the Bloodline ("Exile target creature and all other creatures
/// with the same name as that creature.") cast at the other player's
/// creature on a board with another creature of its name, its caster's,
/// and one of another name: the two of that name are in exile, each its
/// owner's (CR 406.1), and the third is as it was.
fn sever_the_bloodline(cast: Cast) {
    let (mut registry, mut state, blank, card) = board(Box::new(crate::cards::isd::sever_the_bloodline::SeverTheBloodline));
    let unlike = registry.register(Box::new(crate::proofs::cards::OtherBlank));
    let (you, other) = (cast.by, 1 - cast.by);
    let target = a_permanent(&mut state, blank, Permanent::Creature, (other, other));
    let namesake = a_permanent(&mut state, blank, Permanent::Creature, (you, you));
    let stranger = a_permanent(&mut state, unlike, Permanent::Creature, (other, other));
    let spell = a_spell(&mut state, &registry, card, cast);
    let before = as_it_is(&state, stranger);

    cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(target)]);

    let (t, n) = (state.get_object(target).expect("still an object"), state.get_object(namesake).expect("still an object"));
    assert!(t.zone == Zone::Exile && t.owner == PlayerId(other) && n.zone == Zone::Exile && n.owner == PlayerId(you));
    assert!(as_it_is(&state, stranger) == before && before.is_some());
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

/// Spider Spawning ("Create a 1/2 green Spider creature token with reach
/// for each creature card in your graveyard.") cast and resolved,
/// `creatures` creature cards and one other card in its caster's
/// graveyard and a creature card in the other player's: that many new
/// permanents, each a token, a 1/2 green Spider creature with reach under
/// its caster (CR 111.1, 111.2), read back through the engine's own
/// characteristics, and nothing else made.
fn spider_spawning((creatures, cast): (u64, Cast)) {
    let (registry, mut state, blank, card) = board(Box::new(crate::cards::isd::spider_spawning::SpiderSpawning));
    for _ in 0..creatures {
        a_card_in(&mut state, blank, cast.by, Zone::Graveyard, true);
    }
    a_card_in(&mut state, blank, cast.by, Zone::Graveyard, false);
    a_card_in(&mut state, blank, 1 - cast.by, Zone::Graveyard, true);
    let you = PlayerId(cast.by);
    let spell = a_spell(&mut state, &registry, card, cast);
    let objects = state.objects.len();

    cast_and_resolve(&mut state, &registry, spell, Vec::new());

    assert!(state.objects.len() == objects + usize::try_from(creatures).expect("a few"));
    for k in 1..=creatures {
        let id = ObjectId(spell.0 + k);
        let t = state.get_object(id).expect("made");
        assert!(t.is_token && t.zone == Zone::Battlefield && t.controller == you && t.owner == you && !t.tapped);
        assert!(state.is_creature(id, &registry) && state.has_keyword(id, Keyword::Reach, &registry));
        assert!(state.effective_power(id, &registry) == Some(1) && state.effective_toughness(id, &registry) == Some(2));
        assert!(t.colors == [Color::Green] && t.subtypes.len() == 1 && t.subtypes[0] == "Spider");
    }
    assert!(rests(&state, spell, cast));
    std::mem::forget((registry, state));
}

use crate::cards::isd::sever_the_bloodline::SeverTheBloodline;

// Paraselene: "Destroy all enchantments. You gain 1 life for each enchantment destroyed this way."
through_the_stack! {
    paraselene_destroys_both_enchantments_and_gains_two: paraselene((FIRST, false));
    paraselene_cast_by_the_second_player_destroys_both_and_gains_them_two: paraselene((SECOND, false));
    paraselene_leaves_an_indestructible_enchantment_and_gains_one: paraselene((FIRST, true));
}

// Rolling Temblor: "Rolling Temblor deals 2 damage to each creature without flying." Flashback {4}{R}{R}.
through_the_stack! {
    rolling_temblor_marks_two_on_each_creature: rolling_temblor((FIRST, false));
    rolling_temblor_cast_by_the_second_player_marks_two_on_each_creature: rolling_temblor((SECOND, false));
    rolling_temblor_spares_a_creature_with_flying: rolling_temblor((FIRST, true));
    rolling_temblor_flashed_back_marks_two_on_each_creature_and_is_exiled: rolling_temblor((FLASHBACK, false));
}

// Blasphemous Act: "This spell costs {1} less to cast for each creature on the battlefield. Blasphemous Act deals 13 damage to each creature."
through_the_stack! {
    blasphemous_act_with_no_creature_costs_all_nine: blasphemous_act((0, FIRST));
    blasphemous_act_with_one_creature_costs_one_less_and_destroys_it: blasphemous_act((1, FIRST));
    blasphemous_act_with_two_creatures_costs_two_less_and_destroys_both: blasphemous_act((2, FIRST));
    blasphemous_act_cast_by_the_second_player_costs_two_less_and_destroys_both: blasphemous_act((2, SECOND));
}

// Sever the Bloodline: "Exile target creature and all other creatures with the same name as that creature." Flashback {5}{B}{B}.
through_the_stack! {
    sever_the_bloodline_exiles_a_creature_and_its_namesake_and_no_other: sever_the_bloodline(FIRST);
    sever_the_bloodline_cast_by_the_second_player_exiles_a_creature_and_its_namesake: sever_the_bloodline(SECOND);
    sever_the_bloodline_flashed_back_exiles_both_and_is_exiled: sever_the_bloodline(FLASHBACK);
    sever_the_bloodline_is_not_cast_at_an_artifact: is_not_cast_at((Box::new(SeverTheBloodline), Permanent::Artifact, FIRST, 1));
    sever_the_bloodline_does_nothing_once_its_target_is_gone: does_nothing_once_its_target_is_gone((Box::new(SeverTheBloodline), Permanent::Creature, Box::new(LightningBolt), FIRST));
}

// Spider Spawning: "Create a 1/2 green Spider creature token with reach for each creature card in your graveyard." Flashback {6}{B}.
through_the_stack! {
    spider_spawning_makes_no_spider_with_no_creature_card: spider_spawning((0, FIRST));
    spider_spawning_makes_one_spider_for_one_creature_card: spider_spawning((1, FIRST));
    spider_spawning_makes_two_spiders_for_two_creature_cards: spider_spawning((2, FIRST));
    spider_spawning_cast_by_the_second_player_makes_them_two_spiders: spider_spawning((2, SECOND));
    spider_spawning_flashed_back_makes_a_spider_and_is_exiled: spider_spawning((1, FLASHBACK));
}

/// What a permanent card's printed text says of it, as the Oracle has it
/// (`data/oracle_cache.json`, from Scryfall): its mana cost, the card
/// types and subtypes of its type line, its power and toughness, and the
/// keyword abilities it has on lines of their own. Nothing here is read
/// from the engine's own data for the card, which is what it is held
/// against.
#[derive(Clone, Copy)]
struct Printed {
    cost: &'static [ManaSymbol],
    types: &'static [CardType],
    subtypes: &'static [&'static str],
    power: i32,
    toughness: i32,
    keywords: &'static [Keyword],
}

/// A creature card cast by `cast` for exactly the mana its printed cost
/// asks for, and resolved: the cast is accepted and leaves the pool
/// empty, so the engine's cost for it is the printed one and neither more
/// nor less (CR 601.2f-h); the card is on the battlefield under its
/// caster's control (608.3), untapped and undamaged; its power and
/// toughness, each card type, its subtypes and each of the engine's
/// keywords are the printed ones and no others, as the engine's layers
/// report them (208, 205, 702); and it is summoning-sick unless it has
/// haste (302.6). A creature of the other player's beside it is as it
/// was, no life has changed, and nothing is left on the stack or asked of
/// anyone.
///
/// A row is written for each creature card whose text has nothing happen
/// as it enters: `scripts/kani-printed-rows.py` writes them from the
/// Oracle's text, and says which cards it leaves and why.
fn enters_as_printed((card, printed, cast): (Box<dyn CardBehavior>, Printed, Cast)) {
    let (registry, mut state, blank, card) = board(card);
    let caster = PlayerId(cast.by);
    let beside = a_permanent(&mut state, blank, Permanent::Creature, (1 - cast.by, 1 - cast.by));
    state.active_player = caster;
    state.step = Step::PrecombatMain;
    state.priority_player = Some(caster);
    let spell = state.create_object(card, caster, Zone::Hand, None, None);
    mana_for_symbols(&mut state, caster, printed.cost);
    let before = as_it_is(&state, beside);
    let mut expected = everything(&state);

    cast_and_resolve(&mut state, &registry, spell, Vec::new());

    is_there_as_printed(&state, &registry, spell, printed, caster);
    assert!(as_it_is(&state, beside) == before && before.is_some());
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    assert!(state.awaiting_action.is_none());
    assert!(state.players[0].graveyard_order.is_empty() && state.players[1].graveyard_order.is_empty());
    // The whole game after is the game before with the card on the
    // battlefield, new there this turn, and the mana spent: nothing else.
    (expected.of(spell).zone, expected.of(spell).sick) = (Zone::Battlefield, true);
    expected.players[usize::from(cast.by)].mana = 0;
    assert!(everything(&state) == expected);
    std::mem::forget((registry, state));
}

/// `spell`, a creature card just resolved, is on the battlefield under
/// `caster` as `printed` says ([`enters_as_printed`] says what that is).
fn is_there_as_printed(state: &GameState, registry: &CardRegistry, spell: ObjectId, printed: Printed, caster: PlayerId) {
    let o = state.get_object(spell).expect("still an object");
    assert!(o.zone == Zone::Battlefield && o.controller == caster && o.owner == caster);
    assert!(!o.tapped && o.damage_marked == 0);
    assert!(state.effective_power(spell, registry) == Some(printed.power));
    assert!(state.effective_toughness(spell, registry) == Some(printed.toughness));
    for t in [CardType::Land, CardType::Creature, CardType::Instant, CardType::Sorcery, CardType::Enchantment, CardType::Artifact, CardType::Planeswalker] {
        assert!(state.has_card_type(spell, t, registry) == printed.types.contains(&t));
    }
    let subtypes = state.subtypes_of(spell, registry);
    assert!(subtypes.len() == printed.subtypes.len());
    for subtype in printed.subtypes {
        assert!(subtypes.iter().any(|s| s == subtype));
    }
    for k in Keyword::ALL {
        assert!(state.has_keyword(spell, k, registry) == printed.keywords.contains(&k));
    }
    assert!(state.has_summoning_sickness(spell, registry) != printed.keywords.contains(&Keyword::Haste));
    std::mem::forget(subtypes);
}

/// A creature card cast for its printed cost less one mana, a generic one
/// if the cost has any and a coloured one if not: the cast is refused and
/// nothing is done (CR 601.2h). With [`enters_as_printed`], the engine's
/// cost for the card is the printed cost from both sides.
fn is_not_cast_for_one_mana_less((card, printed): (Box<dyn CardBehavior>, Printed)) {
    let (registry, mut state, _, card) = board(card);
    let caster = PlayerId(0);
    state.active_player = caster;
    state.step = Step::PrecombatMain;
    state.priority_player = Some(caster);
    let spell = state.create_object(card, caster, Zone::Hand, None, None);
    // The one mana left out: of the first generic symbol, or else the
    // first symbol.
    let generic = printed.cost.iter().position(|s| matches!(s, ManaSymbol::Generic(_)));
    let pool = &mut state.players[0].mana_pool;
    for (i, symbol) in printed.cost.iter().enumerate() {
        match *symbol {
            ManaSymbol::Generic(n) if generic == Some(i) => pool.add(ManaType::Colorless, n - 1),
            ManaSymbol::Generic(n) => pool.add(ManaType::Colorless, n),
            ManaSymbol::Colored(_) if generic.is_none() && i == 0 => {}
            ManaSymbol::Colored(c) => pool.add(ManaType::from(c), 1),
            _ => panic!("a cost of coloured and generic mana"),
        }
    }

    cast_is_refused(&mut state, &registry, spell, Vec::new());

    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    std::mem::forget((registry, state));
}

/// When a creature card is cast other than in its caster's own main
/// phase with the stack empty.
#[derive(Clone, Copy, PartialEq)]
enum At {
    /// The other player's first main phase, the caster holding priority.
    TheOtherPlayersTurn,
    /// The caster's own upkeep.
    ItsCastersUpkeep,
}

/// A creature card cast, for its printed cost, at a time only a card
/// with flash may be (CR 307.1, 702.8a): with flash it is cast, resolves
/// and is there as printed; without, the cast is refused and nothing is
/// done.
fn is_cast_only_at_its_time((card, printed, at): (Box<dyn CardBehavior>, Printed, At)) {
    let (registry, mut state, _, card) = board(card);
    let caster = PlayerId(0);
    (state.active_player, state.step) = match at {
        At::TheOtherPlayersTurn => (PlayerId(1), Step::PrecombatMain),
        At::ItsCastersUpkeep => (caster, Step::Upkeep),
    };
    state.priority_player = Some(caster);
    let spell = state.create_object(card, caster, Zone::Hand, None, None);
    mana_for_symbols(&mut state, caster, printed.cost);
    let mut expected = everything(&state);

    if printed.keywords.contains(&Keyword::Flash) {
        cast_and_resolve(&mut state, &registry, spell, Vec::new());
        is_there_as_printed(&state, &registry, spell, printed, caster);
        // As in its caster's own main phase, and priority is the active
        // player's once it has resolved (CR 117.3b).
        (expected.of(spell).zone, expected.of(spell).sick) = (Zone::Battlefield, true);
        expected.players[0].mana = 0;
        expected.priority = Some(state.active_player);
        assert!(everything(&state) == expected);
    } else {
        cast_is_refused(&mut state, &registry, spell, Vec::new());
    }
    assert!(state.players[0].life == 20 && state.players[1].life == 20);
    std::mem::forget((registry, state));
}

mod oracle;
mod printed;

// The engine's own game loop playing each action, where the rows above
// name the action to the engine themselves. Four rows to begin with, one
// of each kind of thing a spell does here (it resolves and nothing dies,
// it is refused, a permanent is destroyed, a creature dies), to find out
// what the loop takes: `legal_actions` alone ran out of memory at 11 GB
// and `submit_action`'s copy at 6 GB on a machine of 15 GB. If the loop
// fits a larger machine, every row above is to be played this way and
// `play`'s other half goes.
in_the_game_loop! {
    divination_in_the_game_loop_draws_the_top_two_of_three: draws((Box::new(Divination), 2, 3, FIRST));
    doom_blade_in_the_game_loop_is_not_cast_at_an_artifact: is_not_cast_at((Box::new(DoomBlade), Permanent::Artifact, FIRST, 1));
    bramblecrush_in_the_game_loop_destroys_the_second_players_artifact: destroys((Box::new(Bramblecrush), Permanent::Artifact, FIRST, (1, 1), Guard::Nothing));
    doom_blade_in_the_game_loop_destroys_the_second_players_creature: destroys((Box::new(DoomBlade), Permanent::Creature, FIRST, (1, 1), Guard::Nothing));
}
