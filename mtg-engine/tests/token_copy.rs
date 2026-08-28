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

    let zombies = count_tokens_named_by(&state, "Zombie Token", P0);
    assert!(zombies >= 26,
        "test precondition: 13 tokens doubled is 26, got {zombies}");
    // "Zombie Token", not "Zombie" — this loop ran over nothing, which is
    // exactly the claim the test exists to make.
    let tokens: Vec<_> = state.objects.values()
        .filter(|o| o.is_token && o.name == "Zombie Token" && o.controller == P0)
        .collect();
    assert_eq!(tokens.len(), zombies, "the loop below has to run over something");
    for z in tokens {
        assert!(z.tapped,
            "token {:?} is untapped: 'create thirteen tapped Zombies' has to \
             mean all of them, doubled ones included", z.id);
    }
}
