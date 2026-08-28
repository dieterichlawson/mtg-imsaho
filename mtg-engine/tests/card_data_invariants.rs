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
/// The reach is the whole crate, because every layer of it has an
/// order-sensitive scan: which creature the player is offered first, which of
/// two simultaneous triggers goes on the stack first, which of two creatures
/// dying together is reported first, which state trigger fires when several
/// are ready at once. Everything goes through the accessors that sort by id —
/// `objects_in_zone`, `all_objects_in_zone`, `objects_in_id_order`.
///
/// `state.rs` is exempt: it is where those accessors are built, and where the
/// genuinely order-free walks live (summing continuous effects over every
/// source reaches the same total in any order).
#[test]
fn nothing_iterates_the_object_map_in_map_order() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    // The whole crate, except `state.rs` — that is where the sorted accessors
    // live, and where the order-free walks (summing continuous effects) belong.
    let mut stack = vec![root.join("src")];
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
        if name == "state.rs" {
            continue;
        }
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

/// Counters are game objects' state, not a card's private bookkeeping, so
/// every card reaches them through the engine: `state.add_counters`,
/// `state.remove_counters`, `state.get_counter_count`.
///
/// Four cards used to reach into `obj.counters` directly, and the shortcuts
/// were not free. `add_counters` refuses to put a counter on a permanent that
/// has left the battlefield (CR 121.1) — a hand-rolled `entry().or_insert(0)`
/// does not, and it also leaves a zero-valued entry behind where the pipeline
/// drops the key. Worse, a card that removes a counter by hand at resolution
/// is almost always removing it on the wrong side of the priority window: if
/// the removal is a cost it belongs in `ActivatedAbilityDef::counter_cost`,
/// which the engine pays on activation and checks for payability first
/// (CR 601.2h, CR 602.2b). Mikaeus, the Lunarch did exactly that.
///
/// `enters_with_counters` builds a list of counters for an ETB replacement
/// effect rather than touching an object, so `e.counters` is not this.
#[test]
fn no_card_reaches_into_the_counter_map_by_hand() {
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

    let ops = ["get(", "entry(", "remove(", "insert(", "contains_key(", "get_mut("];
    let mut offenders = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") || code.starts_with("///") {
                continue;
            }
            let Some(rest) = code.split_once(".counters.").map(|(_, r)| r) else { continue };
            if ops.iter().any(|op| rest.starts_with(op)) {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} site(s) reach into an object's counter map by hand:\n  {}\n\n\
         Use `state.add_counters` / `state.remove_counters` / \
         `state.get_counter_count`, and declare a counter that is part of an \
         activation cost as `ActivatedAbilityDef::counter_cost`.",
        offenders.len(), offenders.join("\n  "));
}

/// "Who controls my source" and "is my source still on the battlefield" are
/// two questions, and cards kept asking them with one expression:
///
/// ```ignore
/// let controller = match state.get_object(self_id) {
///     Some(o) if o.zone == Zone::Battlefield => o.controller,
///     _ => return,
/// };
/// ```
///
/// It reads as the first and behaves as the second, so an ability whose effect
/// has nothing to do with its source silently did nothing when the source was
/// removed in response to it — against CR 113.7a, which is the whole point of
/// an ability existing on the stack independently of the object it came from.
/// Hamlet Captain stopped pumping the rest of the team; Ghoulraiser stopped
/// returning a Zombie, so removal in response ate the card advantage as well
/// as the body. And once the source *has* left, `o.controller` is reset to
/// `o.owner`, so the read is wrong on its own terms too (CR 608.2g).
///
/// Ask them separately: `helpers::controller_of` for the first,
/// `helpers::still_on_battlefield` for the second. Most effects need neither
/// guard — `add_counters` and `apply_transform` already decline on a permanent
/// that is not there.
///
/// The methods listed in `FUNCTIONS_ON_THE_BATTLEFIELD` are exempt, because
/// for them the battlefield check is the correct question rather than a
/// smuggled one: a static ability, a replacement effect and the list of
/// abilities a permanent offers all function only while the permanent is on
/// the battlefield (CR 113.6), unlike an ability already on the stack.
#[test]
fn no_card_conflates_its_controller_with_still_being_on_the_battlefield() {
    /// Hooks that answer "what is true of this permanent right now", not
    /// "resolve this ability".
    const FUNCTIONS_ON_THE_BATTLEFIELD: &[&str] = &[
        "replace_event", "activated_abilities", "continuous_effects",
        "is_valid_target", "dynamic_pt", "should_trigger",
        "should_trigger_on_blocks", "should_trigger_on_becomes_blocked",
        "state_trigger_condition", "should_transform",
    ];

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
    let mut scanned = 0usize;
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "helpers.rs" {
            continue; // where both helpers live, and the doc comment shows the idiom
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let mut current_fn = String::new();
        for (n, line) in lines.iter().enumerate() {
            let code = line.trim_start();
            if let Some(rest) = code.strip_prefix("fn ") {
                current_fn = rest.split('(').next().unwrap_or("").to_string();
            }
            if code.starts_with("//") {
                continue;
            }
            if !code.contains("o.zone == Zone::Battlefield") || !code.contains("=>") {
                continue;
            }
            scanned += 1;
            if FUNCTIONS_ON_THE_BATTLEFIELD.contains(&current_fn.as_str()) {
                continue;
            }
            // Whose object is being matched on? Only the ability's own source
            // counts; a guard on a *target* is CR 608.2b and belongs there.
            let subject_is_source = lines[n.saturating_sub(1)].contains("state.get_object(self_id)")
                || lines[n.saturating_sub(1)].contains("state.get_object(object_id)");
            if !subject_is_source {
                continue;
            }
            // Comparing one object's controller against another's is a
            // different question, so only a bare `o.controller` counts.
            let yields_controller = code.match_indices("o.controller").any(|(i, _)| {
                let after = code[i + "o.controller".len()..].trim_start();
                let before = code[..i].trim_end();
                !after.starts_with("==") && !before.ends_with("==")
            });
            if yields_controller {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(scanned >= 5,
        "only {scanned} battlefield-guarded match arm(s) in src/cards — this invariant has stopped covering anything");
    assert!(offenders.is_empty(),
        "{} site(s) read a controller through a battlefield guard:\n  {}\n\n\
         Split them: `helpers::controller_of` for who \"you\" is (CR 608.2g), \
         and `helpers::still_on_battlefield` only if the effect genuinely \
         needs the permanent to be there.",
        offenders.len(), offenders.join("\n  "));
}

/// A card resolving one of its own abilities must not read `o.controller` off
/// its source.
///
/// Two rules say so, and they agree. CR 608.2g: an ability that resolves after
/// its source has left the battlefield uses the source's *last known*
/// controller — and leaving the battlefield resets `controller` to `owner`, so
/// the field being read is not that. CR 602.2a: an *activated* ability's
/// controller is the player who activated it, which is not the source's
/// controller either if someone took the permanent in response.
/// `helpers::controller_of` answers the first, `helpers::ability_controller`
/// the second (and falls through to the first).
///
/// Twenty-five sites read the raw field. Most carried a comment saying exactly
/// the rule they were breaking — "triggered ability resolves even if source
/// has left the battlefield", "'your' means last-known controller, not owner"
/// — above a `match` that returned the owner. Moldgraf Monstrosity's is a dies
/// trigger, so its source is *always* in the graveyard by the time it reads
/// the field, and "return two creature cards from your graveyard" looked in
/// the owner's. And every one of them paired the read with `None => return`,
/// throwing the whole effect away if the source had gone, against CR 113.7a.
///
/// Exempt are the hooks that answer "what is true of this permanent right
/// now" rather than resolving anything: a static or replacement effect and a
/// trigger *condition* are evaluated while the source is on the battlefield
/// (CR 113.6), where the two answers coincide, and the enters-tapped check on
/// the dual lands runs on the land as it enters.
#[test]
fn no_card_reads_its_controller_off_its_own_source_while_resolving() {
    const FUNCTIONS_ON_THE_BATTLEFIELD: &[&str] = &[
        "replace_event", "activated_abilities", "continuous_effects",
        "is_valid_target", "dynamic_pt", "should_trigger",
        "should_trigger_on_blocks", "should_trigger_on_becomes_blocked",
        "should_trigger_on_spell_cast", "state_trigger_condition",
        "should_transform", "pay_activation_cost", "mana_abilities",
        "card_data", "back_face_data", "step_trigger_scope", "loyalty_abilities",
        // The dual lands' "unless you control a Mountain or a Plains" check,
        // run as the land itself enters (CR 614.1c).
        "controller_has_matching_land",
    ];

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
    let mut scanned = 0usize;
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "helpers.rs" {
            continue; // where both helpers live
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let mut current_fn = String::new();
        for (n, line) in lines.iter().enumerate() {
            let code = line.trim_start();
            if let Some(rest) = code.strip_prefix("fn ") {
                current_fn = rest.split('(').next().unwrap_or("").to_string();
            }
            if code.starts_with("//") || !code.contains("o.controller") {
                continue;
            }
            scanned += 1;
            if FUNCTIONS_ON_THE_BATTLEFIELD.contains(&current_fn.as_str()) {
                continue;
            }
            // Whose controller? Only the ability's own source is this rule; a
            // read off a *target* is that target's business, and Ghost Quarter
            // makes one.
            let prev = lines[n.saturating_sub(1)];
            let subject_is_source = prev.contains("state.get_object(object_id)")
                || prev.contains("state.get_object(self_id)");
            // A comparison against a controller already in hand is a different
            // question again — Olivia Voldaren checks one against the recorded
            // activator.
            let is_comparison = code.contains("o.controller ==") || code.contains("== o.controller");
            if subject_is_source && !is_comparison {
                offenders.push(format!("{name}:{}: fn {current_fn}: {code}", n + 1));
            }
        }
    }
    assert!(scanned >= 20,
        "only {scanned} controller read(s) in src/cards — this invariant has \
         stopped covering anything");
    assert!(offenders.is_empty(),
        "{} site(s) read the source's own `controller` field while resolving:\n  {}\n\n\
         Use `helpers::ability_controller` for an activated ability (CR 602.2a) \
         and `helpers::controller_of` everywhere else (CR 608.2g). Neither \
         needs a `None => return`, which CR 113.7a forbids anyway.",
        offenders.len(), offenders.join("\n  "));
}

/// Equip is one rules action (CR 702.6b) and it was written out eleven times,
/// once per Equipment in the set: four identical lines to set `attached_to`,
/// and above them a byte-identical `is_valid_target` in ten of them.
///
/// The duplication was not free. The engine's CR 608.2b re-check runs
/// `is_target_legal` plus the card's own `is_valid_target`, and for
/// `CreatureWithFilter` the former only re-runs the *filter* — it accepts a
/// target in the Stack zone and asks nothing about creature-ness. So the
/// legality check at the moment of attaching was each card's to remember, with
/// no shared place for it or for CR 301.5c (an Equipment that is also a
/// creature does not become attached) to live.
///
/// `helpers::resolve_equip` is that place. Auras have had `helpers::resolve_aura`
/// all along; this is its counterpart.
#[test]
fn no_equipment_attaches_itself_by_hand() {
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
            continue; // where resolve_aura and resolve_equip live
        }
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            // Player attachment is a different question: Bitterheart Witch
            // puts a Curse onto the battlefield attached to a player straight
            // out of a library, which is CR 303.4h rather than a curse spell
            // resolving, and has its own "can this player be enchanted" check.
            if code.contains("attached_to = Some") {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} card(s) attach a permanent to a creature by hand:\n  {}\n\n\
         Use `helpers::resolve_equip` (CR 702.6b) or `helpers::resolve_aura`, \
         which check the target is still legal where the attachment happens.",
        offenders.len(), offenders.join("\n  "));
}

/// Paying a mana cost is the engine's: `pay_cost_with_sources` (or
/// `plan_autotap_for_cost` and `execute_tap_plan_and_pay`, which are the same
/// thing in two steps). Both tap lands for the mana, which CR 601.2g requires
/// and a player expects — "you may pay {1}" with an empty pool and four
/// untapped Plains has to be payable.
///
/// Mentor of the Meek walked the mana pool by hand instead — colorless first,
/// then WUBRG — spending a floating unit if it found one and quietly doing
/// nothing if it did not. Saying "yes" with lands untapped paid nothing and
/// drew nothing. Screeching Bat, the set's other "you may pay", has always
/// gone through the engine.
#[test]
fn no_card_spends_mana_out_of_the_pool_by_hand() {
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
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            // Reaching past the pool's API into its map. `mana_pool.add` is
            // how a mana ability produces mana and is not this; reading the
            // pool (`mana_pool.get`) to decide whether to offer something is
            // not this either.
            if code.contains("mana_pool.mana") {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} site(s) reach into a mana pool's map by hand:\n  {}\n\n\
         Use `engine::pay_cost_with_sources`, which taps lands for the mana \
         (CR 601.2g) rather than only spending what happens to be floating.",
        offenders.len(), offenders.join("\n  "));
}

/// Creating a regeneration shield is `state.add_regeneration_shield`, which
/// refuses a permanent that is not on the battlefield (CR 701.15 — the shield
/// replaces a destruction, and a permanent that has left is a different object
/// that cannot be destroyed).
///
/// Four cards wrote `obj.regeneration_shields += 1` by hand with no such
/// check, and the cleanup step only clears unused shields from permanents on
/// the battlefield — so a creature destroyed in response to its own
/// "{B}: Regenerate this creature" kept the shield through the graveyard and
/// came back from a reanimation with a free regeneration.
#[test]
fn no_card_creates_a_regeneration_shield_by_hand() {
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
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            if code.contains("regeneration_shields") {
                offenders.push(format!("{name}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} card(s) touch `regeneration_shields` directly:\n  {}\n\n\
         Use `state.add_regeneration_shield`, which refuses a permanent that \
         is no longer on the battlefield.",
        offenders.len(), offenders.join("\n  "));
}

/// Strip parenthesised reminder text and collapse the leftover whitespace.
///
/// Reminder text is printed on the card but says nothing the rules do not
/// already say, and the set is inconsistent about carrying it — Scryfall gives
/// Gatstaf Howler's intimidate with reminder text and the code writes it
/// without. That difference is not drift worth failing a build over; a changed
/// *rule* is.
fn without_reminder_text(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every card's oracle text — both faces — says what Scryfall says.
///
/// `data/oracle_cache.json` is fetched, not written alongside the card, so this
/// is a real cross-check. Cards are errata'd, and a card whose text has drifted
/// is a card being audited against the wrong words: seven back faces still read
/// "transform Ironfang" long after the front faces were updated to "transform
/// this creature", and Ulvenwald Primordials still regenerated itself by name.
/// Nothing behavioural depended on those strings, which is exactly why they sat
/// there — the text is what a reader, a log line, and an audit compare against.
#[test]
fn oracle_text_says_what_scryfall_says() {
    let raw = std::fs::read_to_string("../data/oracle_cache.json")
        .expect("oracle cache is checked in at data/oracle_cache.json");

    // The cache is pretty-printed: cards are keyed at four-space indent, a back
    // face is a nested object, and both carry an "oracle_text" line.
    let mut front: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut back: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut current: Option<String> = None;
    let mut in_back = false;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("    \"") {
            if let Some(end) = rest.find("\": {") {
                current = Some(rest[..end].to_string());
                in_back = false;
            }
        }
        let t = line.trim_start();
        if t.starts_with("\"back_face\": {") {
            in_back = true;
        }
        if let Some(rest) = t.strip_prefix("\"oracle_text\": \"") {
            // The value ends at the closing quote, and exactly one — text that
            // itself ends in an escaped quote (`... this creature.\""`) loses
            // its last character to a greedy trim.
            let raw_text = rest.trim_end().strip_suffix(',').unwrap_or(rest.trim_end());
            let raw_text = raw_text.strip_suffix('"').unwrap_or(raw_text);
            let text = raw_text
                .replace("\\n", "\n")
                .replace("\\\"", "\"")
                .replace("\\u2014", "\u{2014}")
                .replace("\\u2019", "\u{2019}");
            if let Some(name) = current.clone() {
                if in_back { back.insert(name, text); } else { front.insert(name, text); }
            }
        }
    }
    assert!(front.len() > 200, "parsed only {} front texts from the cache", front.len());

    // A basic land's printed text *is* its reminder text — Scryfall gives
    // "({T}: Add {U}.)" and nothing else, because the mana ability is intrinsic
    // (CR 305.6) rather than printed. The cards state it as the ability it is.
    const INTRINSIC_MANA: &[&str] = &["Plains", "Island", "Swamp", "Mountain", "Forest"];

    let reg = registry();
    let mut offenders = Vec::new();
    let mut checked = 0;
    for name in reg.all_names() {
        if INTRINSIC_MANA.contains(&name) {
            continue;
        }
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(data) = reg.card_data(id) else { continue };
        if let Some(want) = front.get(name) {
            checked += 1;
            if without_reminder_text(want) != without_reminder_text(&data.oracle_text) {
                offenders.push(format!(
                    "{name}\n    Scryfall: {:?}\n    card    : {:?}", want, data.oracle_text));
            }
        }
        if let (Some(want), Some(face)) = (back.get(name), reg.get(id).and_then(|b| b.back_face_data())) {
            checked += 1;
            if without_reminder_text(want) != without_reminder_text(&face.oracle_text) {
                offenders.push(format!(
                    "{name} // {}\n    Scryfall: {:?}\n    card    : {:?}",
                    face.name, want, face.oracle_text));
            }
        }
    }
    assert!(checked > 200, "only cross-checked {checked} faces");
    assert!(offenders.is_empty(),
        "{} card face(s) state oracle text the fetched cache disagrees with:\n\n{}\n",
        offenders.len(), offenders.join("\n\n"));
}
