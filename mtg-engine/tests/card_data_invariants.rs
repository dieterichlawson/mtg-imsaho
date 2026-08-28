//! Cross-cutting invariants over every card in the registry.
//!
//! These replace the per-card `*_card_data` / `*_has_correct_stats` tests that
//! used to sit at the top of each card file. Those read a `CardData` literal
//! and asserted its fields straight back — `power: Some(1)` in the card,
//! `assert_eq!(data.power, Some(1))` in the test. A restatement cannot fail
//! unless somebody edits the card, and then it fails without telling anyone
//! anything they did not already know from the diff. The card file is the
//! source of truth for what a card says; there is no second, independent
//! source here to check it against.
//!
//! What *is* worth asserting is consistency — the relationships between the
//! fields that a typo or a half-finished card breaks, checked across all of
//! them at once so a new card is covered the moment it is registered.

mod common;
use common::*;
use mtg_engine::cards::{CardData, CardRegistry};
use mtg_engine::types::{CardType, Color, Keyword, Step, Supertype};
use std::collections::HashSet;

/// Every card in the registry, by name.
fn all_cards(reg: &CardRegistry) -> Vec<CardData> {
    let mut names: Vec<String> = reg.all_names().iter().map(|s| (*s).to_string()).collect();
    names.sort();
    names
        .iter()
        .map(|n| {
            let id = reg
                .get_id_by_name(n)
                .unwrap_or_else(|| panic!("{n} is in all_names but has no id"));
            reg.card_data(id).unwrap_or_else(|| panic!("{n} has no card data"))
        })
        .collect()
}

/// Guard against a vacuous invariant: an assertion that no card in the set
/// exercises passes for the wrong reason. Each test below states how many
/// cards it actually looked at.
fn assert_covers(n: usize, floor: usize, what: &str) {
    assert!(n >= floor, "only {n} card(s) {what} — this invariant has stopped covering anything");
}

/// Report every offender at once — one failing card should not hide the rest.
fn assert_none(offenders: &[String], what: &str) {
    assert!(
        offenders.is_empty(),
        "{} card(s) {what}:\n  {}",
        offenders.len(),
        offenders.join("\n  ")
    );
}

#[test]
fn every_card_round_trips_through_its_name() {
    let reg = registry();
    let mut offenders = Vec::new();
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else {
            offenders.push(format!("{name}: not findable by its own name"));
            continue;
        };
        match reg.card_data(id) {
            None => offenders.push(format!("{name}: no card data")),
            Some(d) if d.name != name => {
                offenders.push(format!("{name}: registered under a different name ({})", d.name));
            }
            Some(_) => {}
        }
    }
    assert_none(&offenders, "do not round-trip through the registry");
}

#[test]
fn card_names_are_unique() {
    let reg = registry();
    let mut seen = HashSet::new();
    let dupes: Vec<String> = reg
        .all_names()
        .iter()
        .filter(|n| !seen.insert((*n).to_string()))
        .map(|n| (*n).to_string())
        .collect();
    assert_none(&dupes, "are registered twice");
}

#[test]
fn a_card_has_power_and_toughness_exactly_when_it_is_a_creature() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut creatures = 0;
    for d in all_cards(&reg) {
        let creature = d.card_types.contains(&CardType::Creature);
        let has_pt = d.power.is_some() && d.toughness.is_some();
        if creature && !has_pt {
            offenders.push(format!("{}: creature with power {:?} / toughness {:?}", d.name, d.power, d.toughness));
        }
        if !creature && (d.power.is_some() || d.toughness.is_some()) {
            offenders.push(format!("{}: not a creature but has P/T {:?}/{:?}", d.name, d.power, d.toughness));
        }
        if creature {
            creatures += 1;
        }
    }
    assert_covers(creatures, 100, "are creatures");
    assert_none(&offenders, "disagree about being a creature");
}

#[test]
fn lands_have_no_mana_cost_and_everything_else_has_one() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut lands = 0;
    for d in all_cards(&reg) {
        let land = d.card_types.contains(&CardType::Land);
        match (land, d.cost.is_some()) {
            (true, true) => offenders.push(format!("{}: a land with a mana cost", d.name)),
            (false, false) => offenders.push(format!("{}: a nonland with no mana cost", d.name)),
            _ => {}
        }
        if land {
            lands += 1;
        }
    }
    assert_covers(lands, 10, "are lands");
    assert_none(&offenders, "have the wrong kind of mana cost");
}

#[test]
fn subtypes_imply_their_card_type() {
    let reg = registry();
    // (subtype, the card type it can only appear on)
    const REQUIRED: &[(&str, CardType)] = &[
        ("Equipment", CardType::Artifact),
        ("Aura", CardType::Enchantment),
        ("Curse", CardType::Enchantment),
    ];
    let mut offenders = Vec::new();
    let mut matched = 0;
    for d in all_cards(&reg) {
        for (sub, ty) in REQUIRED {
            if d.subtypes.iter().any(|s| s == sub) {
                matched += 1;
                if !d.card_types.contains(ty) {
                    offenders.push(format!("{}: {sub} but not {ty:?} ({:?})", d.name, d.card_types));
                }
            }
        }
        // A Curse is a kind of Aura (CR 205.3h) and must say so, or the
        // attachment code that looks for Auras will not see it.
        if d.subtypes.iter().any(|s| s == "Curse") && !d.subtypes.iter().any(|s| s == "Aura") {
            offenders.push(format!("{}: a Curse that is not also an Aura", d.name));
        }
    }
    assert_covers(matched, 20, "carry one of these subtypes");
    assert_none(&offenders, "carry a subtype their card type cannot have");
}

#[test]
fn basic_and_legendary_land_on_the_right_card_types() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut legendary = 0;
    for d in all_cards(&reg) {
        if d.supertypes.contains(&Supertype::Basic) && !d.card_types.contains(&CardType::Land) {
            offenders.push(format!("{}: Basic but not a land", d.name));
        }
        // CR 205.4a: only permanents (and, in other formats, instants and
        // sorceries we do not have) are legendary.
        if d.supertypes.contains(&Supertype::Legendary) {
            legendary += 1;
            if !d.card_types.iter().any(CardType::is_permanent) {
                offenders.push(format!("{}: Legendary but not a permanent", d.name));
            }
        }
    }
    assert_covers(legendary, 5, "are legendary");
    assert_none(&offenders, "carry a supertype their card type cannot have");
}

#[test]
fn flashback_is_only_on_instants_and_sorceries_and_says_so() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut with_flashback = 0;
    for d in all_cards(&reg) {
        let Some(cost) = &d.flashback_cost else { continue };
        with_flashback += 1;
        if !d.card_types.iter().any(|t| matches!(t, CardType::Instant | CardType::Sorcery)) {
            offenders.push(format!("{}: flashback on a {:?}", d.name, d.card_types));
        }
        if !d.oracle_text.to_lowercase().contains("flashback") {
            offenders.push(format!("{}: has a flashback cost but its text never mentions it", d.name));
        }
        // CR 702.33a: flashback is an alternative cost, so there has to be one
        // to pay. A free flashback is the "no mana cost" bug in disguise.
        if cost.mana_value() == 0 && cost.symbols.is_empty() {
            offenders.push(format!("{}: flashback for nothing", d.name));
        }
    }
    assert_covers(with_flashback, 10, "have flashback");
    assert_none(&offenders, "declare flashback inconsistently");
}

/// The word a keyword is printed as, for checking it against the oracle text.
fn keyword_word(k: Keyword) -> &'static str {
    match k {
        Keyword::Flying => "flying",
        Keyword::FirstStrike => "first strike",
        Keyword::DoubleStrike => "double strike",
        Keyword::Trample => "trample",
        Keyword::Deathtouch => "deathtouch",
        Keyword::Lifelink => "lifelink",
        Keyword::Vigilance => "vigilance",
        Keyword::Flash => "flash",
        Keyword::Reach => "reach",
        Keyword::Haste => "haste",
        Keyword::Defender => "defender",
        Keyword::Hexproof => "hexproof",
        Keyword::Intimidate => "intimidate",
        Keyword::Menace => "menace",
        Keyword::Indestructible => "indestructible",
    }
}

#[test]
fn every_declared_keyword_is_printed_on_the_card() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut declared = 0;
    for d in all_cards(&reg) {
        let text = d.oracle_text.to_lowercase();
        for k in &d.keywords {
            declared += 1;
            if !text.contains(keyword_word(*k)) {
                offenders.push(format!("{}: declares {k:?}, which its text never prints", d.name));
            }
        }
    }
    assert_covers(declared, 50, "declare a keyword");
    assert_none(&offenders, "declare a keyword their oracle text does not print");
}

#[test]
fn no_card_declares_the_same_thing_twice() {
    let reg = registry();
    let mut offenders = Vec::new();
    for d in all_cards(&reg) {
        let mut seen = HashSet::new();
        for k in &d.keywords {
            if !seen.insert(*k) {
                offenders.push(format!("{}: keyword {k:?} twice", d.name));
            }
        }
        let mut seen = HashSet::new();
        for s in &d.subtypes {
            if !seen.insert(s.clone()) {
                offenders.push(format!("{}: subtype {s} twice", d.name));
            }
        }
        let mut seen = HashSet::new();
        for t in &d.card_types {
            if !seen.insert(*t) {
                offenders.push(format!("{}: card type {t:?} twice", d.name));
            }
        }
    }
    assert_none(&offenders, "declare something twice");
}

#[test]
fn every_card_has_a_name_a_type_and_rules_text() {
    let reg = registry();
    let mut offenders = Vec::new();
    for d in all_cards(&reg) {
        if d.name.trim().is_empty() {
            offenders.push("<unnamed card>".to_string());
        }
        if d.card_types.is_empty() {
            offenders.push(format!("{}: no card type", d.name));
        }
        // A vanilla creature is the only thing allowed to say nothing.
        let vanilla = d.card_types == vec![CardType::Creature] && d.keywords.is_empty();
        if d.oracle_text.trim().is_empty() && !vanilla {
            offenders.push(format!("{}: no oracle text", d.name));
        }
    }
    assert_none(&offenders, "are missing something every card has");
}

#[test]
fn every_triggered_ability_describes_itself() {
    let reg = registry();
    let mut offenders = Vec::new();
    let mut triggers = 0;
    for d in all_cards(&reg) {
        for a in &d.triggered_abilities {
            triggers += 1;
            if a.description.trim().is_empty() {
                offenders.push(format!("{}: a {:?} trigger with no description", d.name, a.kind));
            }
        }
    }
    assert_covers(triggers, 80, "declare a triggered ability");
    assert_none(&offenders, "have an undescribed triggered ability");
}

/// A triggered ability that targets must say so.
///
/// The engine chooses a trigger's targets as it goes on the stack (CR 603.3b).
/// If the ability's `target_requirement` is `None` the engine pushes it
/// untargeted, and the card is left to pick something at resolution — which is
/// both the wrong time and invisible to the "no legal target, no trigger" rule
/// (CR 603.3c). Was four hand-listed cards; the declaration itself says which
/// abilities target, so ask all of them.
#[test]
fn a_triggered_ability_whose_text_targets_declares_a_target_requirement() {
    let reg = registry();
    let mut targeting = 0;
    let mut offenders = Vec::new();

    for d in all_cards(&reg) {
        for ability in &d.triggered_abilities {
            let text = ability.description.to_lowercase();
            // "target" in the ability's own description is the declaration that
            // it targets. "that creature" / "enchanted player" do not target.
            if !text.contains("target") {
                continue;
            }
            targeting += 1;
            if ability.target_requirement.is_none() {
                offenders.push(format!(
                    "{}: its {:?} ability says {:?} but declares no target_requirement",
                    d.name, ability.kind, ability.description));
            }
        }
    }
    assert_covers(targeting, 8, "declare a targeting trigger");
    assert_none(&offenders, "have a targeting trigger that does not declare its target");
}

/// A card's declared trigger kinds must match what its text says it watches.
///
/// Creepy Doll had a per-card test asserting it declares
/// `DealsCombatDamageToCreature` and NOT `Blocks` / `BecomesBlocked` — a real
/// constraint, written out for one card. The oracle text says which event a
/// trigger watches, so the constraint generalises: if the text says "deals
/// combat damage to a creature", the declaration has to say so too.
#[test]
fn a_triggers_declared_kind_matches_what_its_text_watches() {
    use mtg_engine::cards::TriggerKind;

    // (phrase in the ability's own description, the kind it must be declared as)
    const SAYS: &[(&str, TriggerKind)] = &[
        ("deals combat damage to a creature", TriggerKind::DealsCombatDamageToCreature),
        ("at the beginning of your upkeep", TriggerKind::Upkeep),
        ("at the beginning of each upkeep", TriggerKind::Upkeep),
        ("at the beginning of your end step", TriggerKind::EndStep),
        ("when this creature dies", TriggerKind::SelfDies),
        ("when this creature enters", TriggerKind::EntersBattlefield),
        ("when this permanent enters", TriggerKind::EntersBattlefield),
    ];

    let reg = registry();
    let mut matched = 0;
    let mut offenders = Vec::new();

    for d in all_cards(&reg) {
        let text = d.oracle_text.to_lowercase();
        for (phrase, kind) in SAYS {
            if !text.contains(phrase) {
                continue;
            }
            matched += 1;
            let front = d.triggered_abilities.iter().any(|a| a.kind == *kind);
            let back = reg
                .get_id_by_name(&d.name)
                .and_then(|id| reg.get(id))
                .and_then(|b| b.back_face_data())
                .is_some_and(|back| back.triggered_abilities.iter().any(|a| a.kind == *kind));
            if !front && !back {
                offenders.push(format!(
                    "{}: text says {phrase:?} but no {kind:?} trigger is declared",
                    d.name));
            }
        }
    }
    assert_covers(matched, 25, "have text naming one of these trigger events");
    assert_none(&offenders, "declare a trigger kind that does not match their text");
}

/// Every card that has a back face declares it.
///
/// `data/oracle_cache.json` is the independent source here — it is fetched from
/// Scryfall, not written alongside the card — so this is a real cross-check
/// rather than a restatement.
///
/// A card that skips `back_face_data()` and models its second face by branching
/// on `is_transformed` still *behaves* like the back face, which is why this is
/// easy to miss: what breaks is every characteristics read. `face_data` falls
/// through to the front face, so `name_of` gives the front face's name and the
/// oracle text stays the front face's — reaching the legend rule (CR 704.5j),
/// the log, and anything matching on names. Garruk Relentless was written that
/// way, with the back face's name hand-written into `obj.name` on transform,
/// which covered the displays that read the cache and nothing that read the
/// card.
#[test]
fn every_card_with_a_back_face_declares_it() {
    let raw = std::fs::read_to_string("../data/oracle_cache.json")
        .expect("oracle cache is checked in at data/oracle_cache.json");

    // Cards are keyed at four-space indent; a back face is a six-space
    // `"back_face": {` whose first entry is that face's name.
    let mut expected: Vec<(String, String)> = Vec::new();
    let mut current: Option<String> = None;
    let mut lines = raw.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix("    \"") {
            if let Some(end) = rest.find("\": {") {
                current = Some(rest[..end].to_string());
            }
        }
        if line.trim_start().starts_with("\"back_face\": {") {
            if let (Some(front), Some(name_line)) = (current.clone(), lines.peek()) {
                let t = name_line.trim();
                if let Some(rest) = t.strip_prefix("\"name\": \"") {
                    if let Some(end) = rest.find('"') {
                        expected.push((front, rest[..end].to_string()));
                    }
                }
            }
        }
    }

    let reg = registry();
    let mut offenders = Vec::new();
    let mut checked = 0;
    for (front, back_name) in &expected {
        let Some(behavior) = reg.get_id_by_name(front).and_then(|id| reg.get(id)) else {
            continue; // not implemented in this pool
        };
        checked += 1;
        match behavior.back_face_data() {
            None => offenders.push(format!(
                "{front}: Scryfall gives it a back face ({back_name}) but the card \
                 declares no back_face_data()")),
            Some(back) if back.name != *back_name => offenders.push(format!(
                "{front}: declares back face {:?}, Scryfall says {back_name:?}", back.name)),
            Some(_) => {}
        }
    }

    assert_covers(checked, 15, "have a back face in the oracle cache");
    assert_none(&offenders, "declare the back face the oracle cache gives them");
}

/// CR 111.4: "If the spell or ability doesn't specify the name of the token,
/// its name is the same as its subtype(s) plus the word 'Token.'"
///
/// No card in this set names a token, so every token it makes should be named
/// that way. This mattered because five cards make a 1/1 white flying Spirit
/// and they did not agree what to call it — four said `Spirit`, Moorland Haunt
/// said `Spirit Token`. Two cards match creatures *by name* (Sever the
/// Bloodline, Evil Twin's granted ability), so a Sever aimed at one kind of
/// Spirit token would have missed the other.
#[test]
fn tokens_are_named_after_their_subtypes() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cases: &[(&str, &[&str])] = &[
        ("Spirit Token", &["Spirit"]),
        ("Zombie Token", &["Zombie"]),
        ("Wolf Token", &["Wolf"]),
        ("Human Soldier Token", &["Human", "Soldier"]),
    ];

    for (expected, subtypes) in cases {
        let ids = state.create_token_with_subtypes(
            "", P0, 1, 1, vec![Color::White], vec![CardType::Creature], vec![],
            subtypes.iter().map(|s| (*s).to_string()).collect(), &reg);
        assert_eq!(state.get_object(ids[0]).unwrap().name, *expected,
            "a token with subtypes {subtypes:?} is named {expected}");
    }

    // A token the effect *does* name keeps that name (CR 111.4's other half).
    let ids = state.create_token_with_subtypes(
        "Boo", P0, 1, 1, vec![Color::Red], vec![CardType::Creature], vec![],
        vec!["Hamster".into()], &reg);
    assert_eq!(state.get_object(ids[0]).unwrap().name, "Boo",
        "a named token keeps its given name");
}

/// No card hardcodes a token name that the engine would derive anyway — that
/// duplication is what let the set disagree with itself about Spirit tokens.
#[test]
fn no_card_hardcodes_a_derivable_token_name() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut offenders = Vec::new();
    let mut files = Vec::new();
    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        for (i, window) in text.split("create_token_with_subtypes(").skip(1).enumerate() {
            let head: String = window.chars().take(60).collect();
            let first_arg = head.trim_start().trim_start_matches('\n').trim_start();
            if first_arg.starts_with('"') && !first_arg.starts_with("\"\"") {
                offenders.push(format!("{name}: token call #{} passes a literal name: {}",
                    i + 1, first_arg.lines().next().unwrap_or("")));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} card(s) name a token the engine derives from its subtypes:\n  {}\n\n\
         Pass \"\" and let CR 111.4 name it.",
        offenders.len(), offenders.join("\n  "));
}

/// `GameState::objects` is a `HashMap`, and its iteration order is seeded per
/// process. Card code that scans it directly gets a different order on every
/// run of the same game, which shows up in three ways: a list of options
/// offered to a player by position (Curse of the Pierced Heart's planeswalkers,
/// Divine Reckoning's creatures), a `find`/`any` that stops at the first match,
/// and a log that reports the same board in a different order.
///
/// Cards go through the accessors that sort by id instead —
/// `objects_in_zone`, `all_objects_in_zone`, `objects_in_id_order`.
#[test]
fn nothing_iterates_the_object_map_in_map_order() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    // Trigger collection is held to the same rule: the order the watchers are
    // scanned is the order simultaneous triggers go on the stack. So is the
    // engine's action generation, where the scan order is the order targets
    // and abilities are offered to the player — who picks by position.
    let mut stack = vec![
        root.join("src/cards"),
        root.join("src/triggers"),
        root.join("src/engine"),
    ];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") || code.starts_with("///") {
                continue; // the accessors' own doc comments name the pattern
            }
            if code.contains("objects.values()")
                || code.contains("objects.iter()")
                || code.contains("objects.keys()")
            {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} site(s) iterate the raw object map, whose order is \
         seeded per process:\n  {}\n\n\
         Use objects_in_zone / all_objects_in_zone / objects_in_id_order, \
         which sort by id.",
        offenders.len(), offenders.join("\n  "));
}

/// "X transforms into Y" is written once, by `helpers::apply_transform`, which
/// is where the flip happens and where both names are known.
///
/// Nineteen cards used to write it themselves around that call. They said it
/// on the paths where `apply_transform` refuses to flip (a token copy of a
/// double-faced card, CR 111.7), several hardcoded both face names so a rename
/// would leave the log lying, and one said only "Transforms into Stalking
/// Vampire" without naming the permanent at all.
#[test]
fn no_card_announces_its_own_transform() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "helpers.rs" {
            continue; // this is the one place it is written
        }
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            if code.contains("transforms into") || code.contains("transforms back") {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} card(s) announce their own transform:\n  {}\n\n\
         `helpers::apply_transform` logs it, and only when the flip happens.",
        offenders.len(), offenders.join("\n  "));
}

/// An ability's "you" is its source's controller, and CR 608.2g says that is
/// the *last known* controller once the source has left the battlefield —
/// which `helpers::controller_of` answers.
///
/// Reading `o.controller` off the source instead is wrong in exactly the case
/// cards keep commenting about (CR 113.7a, "the ability still resolves if the
/// source is destroyed in response"), because leaving the battlefield resets
/// `controller` to `owner`. Curse of the Pierced Heart handed the choice to
/// the owner; Curiosity offered the draw to the owner. The `PlayerId(0)`
/// fallback these sites carried also silently named a real player.
///
/// Comparisons of one object's controller against another's are a different
/// question and are not what this looks for.
#[test]
fn no_card_reads_its_sources_controller_by_hand() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
    let mut files = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() { stack.push(p); }
            else if p.extension().is_some_and(|e| e == "rs") { files.push(p); }
        }
    }
    files.sort();

    let mut offenders = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "helpers.rs" {
            continue; // where controller_of lives
        }
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            // The two idioms that mean "the controller of this ability's
            // source": a PlayerId(0) fallback, or an unwrap.
            let hand_rolled = code.contains("|o| o.controller")
                && (code.contains("PlayerId(0)") || code.contains(".unwrap()"));
            if hand_rolled {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} site(s) read a source's controller by hand:\n  {}\n\n\
         Use `helpers::controller_of`, which answers CR 608.2g.",
        offenders.len(), offenders.join("\n  "));
}
