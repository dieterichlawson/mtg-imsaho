//! What the engine calls a group of creature types when it has to say it out
//! loud.
//!
//! `protections_of` is the one reader that turns a `ProtectionFromSubtype`
//! back into something a player is shown, and the same string reaches an LLM
//! seat through `view.rs`. It built the plural by appending an `s`, so every
//! Elite Inquisitor on the battlefield read "protection from Werewolfs" —
//! contradicting the oracle text the CARDS pane printed on the same screen
//! (issue #324). The fix is the rule, not the word: `-f`/`-fe` is a family
//! Magic's creature types keep landing in (Wolf, Werewolf, Elf, Dwarf).

mod common;
use common::*;
use mtg_engine::types::*;

#[test]
fn protection_from_a_subtype_reads_as_the_oracle_text_does() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let inquisitor = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);

    let shown = state.protections_of(inquisitor, &reg);

    assert!(shown.contains(&"protection from Werewolves".to_string()),
        "the card's own oracle text says \"from Werewolves\": {shown:?}");
    assert!(shown.contains(&"protection from Vampires".to_string()), "{shown:?}");
    assert!(shown.contains(&"protection from Zombies".to_string()), "{shown:?}");
}

/// The `-f` family is the one an `+s` rule gets wrong, and it is the one
/// Innistrad is made of. The rest are here so the rule is pinned as a rule.
#[test]
fn a_creature_types_plural_follows_english_not_a_bare_s() {
    for (one, many) in [
        ("Werewolf", "Werewolves"),
        ("Wolf", "Wolves"),
        ("Elf", "Elves"),
        ("Dwarf", "Dwarves"),
        ("Zombie", "Zombies"),
        ("Human", "Humans"),
        ("Spirit", "Spirits"),
        ("Ooze", "Oozes"),
        ("Fox", "Foxes"),
        ("Horror", "Horrors"),
        ("Ally", "Allies"),
        ("Monkey", "Monkeys"),
    ] {
        assert_eq!(plural_of(one), many, "plural of {one}");
    }
}

/// The same rule, through the other reader: a filter that names a subtype
/// describes itself with the plural too.
#[test]
fn a_subtype_filter_describes_itself_with_the_same_plural() {
    assert_eq!(CreatureFilter::HasSubtype("Werewolf".into()).describe(), "Werewolves");
}
