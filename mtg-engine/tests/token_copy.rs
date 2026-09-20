//! `create_token_*` has to carry across everything the token needs.
//!
//! A token is built from scratch rather than from a card, so every
//! characteristic and flag the caller depends on is one the helper must copy
//! or set explicitly — and each one it drops fails silently. The legend rule
//! stops noticing a legendary token; a registry lookup on `CardId(0)` returns
//! `None`, so a copy has no `CardBehavior` and misses every trigger its source
//! has; a caller that taps the id it was handed taps only half the tokens
//! Parallel Lives made.
//!
//! Each of these failed when it was written and passes now; they stay to
//! protect against the flag being dropped again.

mod common;
use common::*;

use mtg_engine::types::*;

/// CR 704.5j keys the legend rule on the object's legendary flag, and a token
/// is not built from a card — so a token copy of a legendary creature used to
/// be non-legendary, and the two coexisted indefinitely.
#[test]
fn a_token_copy_of_a_legendary_creature_is_itself_legendary() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    assert!(state.get_object(olivia).unwrap().is_legendary,
        "test precondition: the original is flagged legendary");

    let token = state.create_token_copy(olivia, P0, &reg);

    assert!(state.get_object(token).unwrap().is_legendary,
        "a token copy of Olivia Voldaren must be legendary too, or the legend \
         rule finds no pair and lets both stay (CR 704.5j)");
}

/// Parallel Lives makes two tokens where the effect asked for one. Both are
/// copies, so both need the source's `card_id` — a token left at `CardId(0)`
/// has no registry entry and therefore no `CardBehavior`, losing every trigger,
/// static ability and characteristic-defining P/T its source has.
#[test]
fn every_doubled_token_copy_carries_the_sources_card_id() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    named_permanent(&mut state, &reg, "Parallel Lives", P0);
    // Splinterfright's P/T counts creature cards in the graveyard; give it some
    // so the copies are not 0/0 and swept away before they can be examined.
    for _ in 0..3 {
        named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    }

    let splinter = named_permanent(&mut state, &reg, "Splinterfright", P0);
    let source_card_id = state.get_object(splinter).unwrap().card_id;
    state.create_token_copy(splinter, P0, &reg);

    let copies: Vec<_> = state.objects.values()
        .filter(|o| o.is_token && o.name == "Splinterfright")
        .map(|o| (o.id, o.card_id))
        .collect();

    assert!(copies.len() >= 2,
        "test precondition: Parallel Lives should have doubled the copy, got {}",
        copies.len());
    for (id, card_id) in &copies {
        assert_eq!(*card_id, source_card_id,
            "token {id:?} has card_id {card_id:?}, so a registry lookup finds \
             nothing and it behaves like a vanilla token");
    }
}

/// The doubled tokens also have to reach the caller. Army of the Damned makes
/// its thirteen Zombies *tapped* by setting the flag on the id the helper
/// returned — so a helper that returns only the primary leaves half the tokens
/// untapped.
#[test]
fn a_caller_that_mutates_the_returned_tokens_reaches_the_doubled_ones() {
    let reg = registry();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    named_permanent(&mut state, &reg, "Parallel Lives", P0);

    let card_id = reg.get_id_by_name("Army of the Damned").unwrap();
    let army = state.create_object(card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(army).unwrap().name = "Army of the Damned".into();
    reg.get(card_id).unwrap().on_resolve(&mut state, army, &[], &reg);

    let zombies = count_tokens_named_by(&state, "Zombie", P0);
    assert!(zombies >= 26,
        "test precondition: 13 tokens doubled is 26, got {zombies}");
    // The name is "Zombie" (CR 111.4). A loop filtering on any other string
    // runs over nothing, which is exactly the claim the test exists to make.
    let tokens: Vec<_> = state.objects.values()
        .filter(|o| o.is_token && o.name == "Zombie" && o.controller == P0)
        .collect();
    assert_eq!(tokens.len(), zombies, "the loop below has to run over something");
    for z in tokens {
        assert!(z.tapped,
            "token {:?} is untapped: 'create thirteen tapped Zombies' has to \
             mean all of them, doubled ones included", z.id);
    }
}

/// CR 111.4: "If the spell or ability doesn't specify the name of the token,
/// its name is the same as its subtype(s)." The rule's worked example, a
/// "Goblin Scout creature token", is named `Goblin Scout` — there is no
/// literal "Token" in a token's name.
///
/// This is a characteristic, not a label. "Creatures with the same name"
/// (Sever the Bloodline) and "cards named" (Nevermore) compare it, and the
/// engine used to derive `"<subtypes> Token"`, a string no printed card can
/// ever be named — so a token could not match a card that shares its name,
/// silently and with no error (issues #331, #334). The word "Token" belongs
/// to the renderer, which is where the CLI and the LLM board now say it.
#[test]
fn an_unnamed_tokens_name_is_its_subtypes_alone() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let spirit = state.create_token_with_subtypes("", P0, 1, 1, vec![Color::White],
        vec![CardType::Creature], vec![Keyword::Flying], vec!["Spirit".into()], &reg)[0];
    assert_eq!(state.get_object(spirit).unwrap().name, "Spirit");
    assert_eq!(state.name_of(spirit, &reg), "Spirit",
        "the name every same-name comparison reads is the subtype (CR 111.4)");

    let scout = state.create_token_with_subtypes("", P0, 1, 1, vec![Color::Red],
        vec![CardType::Creature], vec![], vec!["Goblin".into(), "Scout".into()], &reg)[0];
    assert_eq!(state.name_of(scout, &reg), "Goblin Scout",
        "two subtypes make a two-word name, the rule's own example");

    // The other half of CR 111.4: an effect that *does* name its token keeps
    // that name, subtypes or not.
    let named = state.create_token_with_subtypes("Ashaya, the Awoken World", P0, 4, 4,
        vec![Color::Green], vec![CardType::Creature], vec![], vec!["Elemental".into()], &reg)[0];
    assert_eq!(state.name_of(named, &reg), "Ashaya, the Awoken World",
        "a name the effect gave is not overwritten by the subtypes");
}

/// CR 707.2: the copiable values are the printed text "as modified by other
/// copy effects", so an Evil Twin clone's "except it has `{U}{B}, {T}:
/// Destroy target creature with the same name`" is part of what a later copy
/// of that clone copies. The whole of that ability hangs off `copy_grantor`,
/// and `create_token_copy` used to stamp the token with the copied card and
/// leave the grantor `None` — a Cackling Counterpart token of an Evil Twin
/// clone came out as a plain copy of the creature the Twin had copied, with
/// the ability gone and nothing able to put it back (#554).
#[test]
fn a_token_copy_of_a_clone_keeps_the_ability_the_clone_was_granted() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let twin = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(bears));
    assert!(mtg_engine::cards::ability_granting_grantor(&state, twin, &reg).is_some(),
        "test precondition: the clone itself has the granted ability");

    let token = state.create_token_copy(twin, P0, &reg);

    assert_eq!(
        mtg_engine::cards::ability_granting_grantor(&state, token, &reg),
        mtg_engine::cards::ability_granting_grantor(&state, twin, &reg),
        "a token copy of an Evil Twin clone has the clause the clone has \
         (CR 707.2) — it is a copiable value, not something the Twin kept");
    assert_eq!(state.get_object(token).unwrap().name, "Grizzly Bears",
        "and it is still a copy of what the clone copied");
}

/// The other side of the same rule: a token copy of a permanent that is NOT a
/// copy carries no grantor. `copy_grantor` doubles as "the card I am printed
/// as, to be given back on the way off the battlefield", and a token that
/// claimed one would be claiming an ability it has no source for.
#[test]
fn a_token_copy_of_an_ordinary_creature_is_granted_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let token = state.create_token_copy(bears, P0, &reg);

    assert_eq!(state.get_object(token).unwrap().copy_grantor, None,
        "nothing granted this token an ability, so nothing may be looked up as \
         having granted it one");
}

/// A clone that enters as a copy of another clone. The ability survives —
/// through the entering permanent's own copy effect rather than through the
/// source's copiable values, because Evil Twin is the only card in the pool
/// with an "except it has ..." clause and so is always its own grantor. Here
/// to record which of the two routes is doing the work: a clone card without
/// a clause of its own would need `copy_grantor`'s two jobs split apart,
/// which is worth doing when such a card arrives and not before.
#[test]
fn a_clone_of_a_clone_still_has_the_granted_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let first = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(bears));
    let second = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(first));

    assert!(mtg_engine::cards::ability_granting_grantor(&state, second, &reg).is_some(),
        "a copy of an Evil Twin clone has the granted ability (CR 707.2)");
}

/// A token on its way off the battlefield is not rewritten into the card it
/// copied.
///
/// CR 400.7 makes a permanent that changes zones a new object printed as its
/// front face, and `move_object` writes that printed name and P/T back from
/// the registry — for a CARD, which has one. A token does not (CR 111.1):
/// what it is, it is on the object. A token copy of an Evil Twin clone
/// carries the Twin's id in `copy_grantor` (CR 707.2, #554), so a reset that
/// did not exclude tokens would put a 3/2 Grizzly Bears token into the
/// graveyard renamed "Evil Twin" with Evil Twin's printed 0/0.
///
/// The window is short — the next state-based action pass removes it
/// (CR 111.7) — but the death line and every LTB trigger read the object
/// inside it. Written after a mutation sweep of `move_object_inner` found
/// this the one live difference in the guard (#548).
#[test]
fn a_token_copy_leaving_the_battlefield_is_still_what_it_was() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let twin = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(bears));
    let token = state.create_token_copy(twin, P0, &reg);

    let before = state.get_object(token).unwrap().clone();
    assert_eq!(before.name, "Grizzly Bears", "test setup: the token is a copy of the Bears");

    state.move_object(token, Zone::Graveyard, &reg);

    let after = state.get_object(token).unwrap();
    assert_eq!(after.name, before.name,
        "a token keeps its name on the way out — there is no printed card to restore it from");
    assert_eq!((after.power, after.toughness), (before.power, before.toughness),
        "and its power and toughness, which are equally its own (CR 111.1)");
    assert_eq!(after.card_id, before.card_id,
        "and the card it is a copy of, which is not the card that granted it an ability");
}
