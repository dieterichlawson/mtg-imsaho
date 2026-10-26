//! Cards against their Oracle text, read mechanically: tier 3 of
//! `docs/plans/formal-verification.md`.
//!
//! The acceptance tests (`cards_*.rs`) check that a card does what its text
//! says in the situations someone wrote down. This file checks the same
//! thing from the other end. A small grammar reads each instant's and
//! sorcery's Oracle text, as Scryfall prints it (`data/oracle_cache.json`),
//! into a list of effects ("deals 3 damage to any target", "destroy target
//! nonblack creature", "create two 1/1 white Spirit creature tokens with
//! flying"). A reference semantics, written from the Comprehensive Rules
//! and sharing no code with `src/cards/`, says what those effects do to a
//! board described by the Oracle data of the cards on it. Then every card
//! the grammar reads is cast at every set of targets the engine offers on a
//! board of a dozen permanents, and the engine is held to two things:
//!
//! - **the targets**: the engine offers exactly the target sets the text
//!   allows (CR 115.1, 601.2c) — a Doom Blade that offers a black creature,
//!   or a Smite the Monstrous that leaves out a 6/6, fails here;
//! - **the outcome**: after the spell resolves and state-based actions are
//!   checked, every player's life, hand, graveyard, exile and library, and
//!   every permanent's controller, power, toughness, damage, keywords,
//!   colours, subtypes and tapped state are what the reference says.
//!
//! The grammar is the reviewed part: each template is one sentence shape
//! with its CR reference, and a card is covered only when every sentence of
//! its text matches one. A card it cannot read is not checked here, and is
//! counted: `the_grammar_reads_at_least_as_many_cards_as_it_did` keeps the
//! coverage from shrinking silently. Reading the text rather than
//! transcribing each card keeps the specification independent of whoever
//! wrote the card, which is the point: the engine's card says what its
//! author understood, and this says what the printed words say.

mod common;

use std::collections::BTreeMap;

use common::*;
use mtg_engine::actions::{Action, CastTargetSpec, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
use mtg_engine::types::*;

// ---- the Oracle data ----

/// One card as Scryfall prints it.
#[derive(Debug, Clone)]
struct Printed {
    name: String,
    mana_cost: String,
    type_line: String,
    oracle_text: String,
    power: Option<i32>,
    toughness: Option<i32>,
}

fn oracle_cache() -> BTreeMap<String, Printed> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/oracle_cache.json");
    let raw = std::fs::read_to_string(path).expect("data/oracle_cache.json is readable");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("the oracle cache is JSON");
    let int = |v: &serde_json::Value| v.as_str().and_then(|s| s.parse().ok());
    json["cards"].as_object().expect("a cards map").iter()
        .map(|(name, c)| (name.clone(), Printed {
            name: name.clone(),
            mana_cost: c["mana_cost"].as_str().unwrap_or("").to_string(),
            type_line: c["type_line"].as_str().unwrap_or("").to_string(),
            oracle_text: c["oracle_text"].as_str().unwrap_or("").to_string(),
            power: int(&c["power"]),
            toughness: int(&c["toughness"]),
        }))
        .collect()
}

impl Printed {
    /// The card types and subtypes on its type line (CR 205).
    fn types(&self) -> (Vec<String>, Vec<String>) {
        let (front, back) = self.type_line.split_once(" — ").unwrap_or((&self.type_line, ""));
        (front.split_whitespace().map(str::to_string).collect(),
         back.split_whitespace().map(str::to_string).collect())
    }

    fn has_type(&self, t: &str) -> bool {
        self.types().0.iter().any(|x| x == t)
    }

    /// Its colours: the colours of the mana symbols in its cost (CR 202.2),
    /// which is all there is for the cards on the board here (none has a
    /// colour indicator).
    fn colors(&self) -> Vec<Color> {
        let mut colors: Vec<Color> = Color::ALL.into_iter()
            .filter(|c| self.mana_cost.contains(&format!("{{{}}}", color_letter(*c))))
            .collect();
        colors.dedup();
        colors
    }

    /// The keywords the engine models, as the Oracle text prints them on a
    /// line of their own.
    fn keywords(&self) -> Vec<Keyword> {
        let mut out = Vec::new();
        for line in without_reminders(&self.oracle_text).lines() {
            let words: Vec<&str> = line.split(',').map(str::trim).collect();
            let all: Option<Vec<Keyword>> = words.iter().map(|w| keyword_named(w)).collect();
            if let Some(ks) = all {
                out.extend(ks);
            }
        }
        out.sort_by_key(|k| k.label());
        out
    }
}

fn color_letter(c: Color) -> char {
    match c {
        Color::White => 'W',
        Color::Blue => 'U',
        Color::Black => 'B',
        Color::Red => 'R',
        Color::Green => 'G',
    }
}

fn color_named(word: &str) -> Option<Color> {
    Color::ALL.into_iter().find(|c| c.label().eq_ignore_ascii_case(word))
}

fn keyword_named(word: &str) -> Option<Keyword> {
    Keyword::ALL.into_iter().find(|k| k.label().eq_ignore_ascii_case(word))
}

/// The text with reminder text (in parentheses, CR 207.2a) removed.
fn without_reminders(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for ch in text.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out.lines().map(str::trim_end).collect::<Vec<_>>().join("\n")
}

/// A printed mana cost, `{2}{R}`, as symbols.
fn mana_cost(printed: &str) -> Option<ManaCost> {
    let mut symbols = Vec::new();
    for sym in printed.split('}').filter(|s| !s.is_empty()) {
        let sym = sym.strip_prefix('{')?;
        if let Ok(n) = sym.parse::<u32>() {
            if n > 0 {
                symbols.push(ManaSymbol::Generic(n));
            }
        } else if let Some(c) = Color::ALL.into_iter().find(|c| sym == color_letter(*c).to_string()) {
            symbols.push(ManaSymbol::Colored(c));
        } else {
            return None;
        }
    }
    Some(ManaCost::new(symbols))
}

// ---- the grammar ----

/// What a target slot accepts, as the text words it (CR 115.1).
#[derive(Debug, Clone, PartialEq)]
enum Slot {
    /// "any target": a creature, a player or a planeswalker (CR 115.4).
    Any,
    /// "target player or planeswalker".
    PlayerOrPlaneswalker,
    /// "target player".
    Player,
    /// "target opponent".
    Opponent,
    /// "target <permanents>".
    Permanent(Filter),
}

/// A description of permanents: any one of the alternatives ("artifact or
/// enchantment").
#[derive(Debug, Clone, PartialEq)]
struct Filter(Vec<Clause>);

/// One alternative: a head noun and what qualifies it.
#[derive(Debug, Clone, PartialEq, Default)]
struct Clause {
    /// A card type ("creature"), a subtype ("Spirit"), or none for
    /// "permanent".
    head: Option<String>,
    /// "nonblack": colours it must not have.
    not_colors: Vec<Color>,
    /// "noncreature", "non-Vampire": types and subtypes it must not have.
    not_types: Vec<String>,
    /// "with power 4 or greater".
    min_power: Option<i32>,
    /// "without flying".
    without: Vec<Keyword>,
    /// "you control" (Some(true)), "you don't control" / "your opponents
    /// control" (Some(false)).
    yours: Option<bool>,
    /// "attacking".
    attacking: bool,
}

/// One thing a spell does, in the order its text says it (CR 608.2c).
#[derive(Debug, Clone, PartialEq)]
enum Effect {
    /// "deals N damage to <slot>" (CR 120.3).
    Damage { amount: u32, slot: usize },
    /// "deals N damage to each <filter>".
    DamageEach { amount: u32, filter: Filter },
    /// "<slot> gets +P/+T [and gains K] until end of turn" (CR 611.2).
    Pump { slot: usize, power: i32, toughness: i32, gains: Vec<Keyword> },
    /// "<filter> get +P/+T [and gain K] until end of turn".
    PumpEach { filter: Filter, power: i32, toughness: i32, gains: Vec<Keyword> },
    /// "Destroy target ..." (CR 701.8).
    Destroy { slot: usize },
    /// "Exile target ..." (CR 701.13 then 406).
    Exile { slot: usize },
    /// "Return target ... to its owner's hand".
    ToHand { slot: usize },
    /// "Put target ... on top of its owner's library".
    ToLibraryTop { slot: usize },
    /// "Tap target ..." / "Untap target ...".
    Tap { slot: usize, tapped: bool },
    /// "Draw N cards" (CR 121.1).
    Draw { n: usize },
    /// "You gain N life" (CR 119.3).
    GainLife { n: i32 },
    /// "<player slot> loses N life".
    LoseLife { slot: usize, n: i32 },
    /// "<player slot> mills N cards" (CR 701.17).
    Mill { slot: usize, n: usize },
    /// "Create N [tapped] P/T <colour> <subtypes> creature tokens [with K]"
    /// (CR 111.2).
    Tokens { n: usize, power: i32, toughness: i32, colors: Vec<Color>, subtypes: Vec<String>, keywords: Vec<Keyword>, tapped: bool },
    /// "Its controller gains life equal to its power": the power the
    /// object last had on the battlefield (CR 608.2h).
    ControllerGainsLifeEqualToPower { slot: usize },
}

/// A card's text, read.
#[derive(Debug, Clone, PartialEq, Default)]
struct Spec {
    slots: Vec<Slot>,
    effects: Vec<Effect>,
    /// "This spell costs {1} less to cast for each creature on the
    /// battlefield" (CR 601.2f).
    cheaper_per_creature: bool,
}

fn number(word: &str) -> Option<u32> {
    let n = match word {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        "thirteen" => 13,
        _ => return word.parse().ok(),
    };
    Some(n)
}

/// "+3/+3", "-4/-0".
fn pt_change(word: &str) -> Option<(i32, i32)> {
    let (p, t) = word.split_once('/')?;
    Some((p.parse().ok()?, t.parse().ok()?))
}

const CARD_TYPES: [&str; 5] = ["creature", "artifact", "enchantment", "land", "planeswalker"];

/// A description of permanents: "nonblack creature", "artifact or
/// enchantment", "non-Vampire, non-Werewolf, non-Zombie creature",
/// "creature with power 4 or greater", "creature you control".
fn filter(text: &str) -> Option<Filter> {
    let mut clauses = Vec::new();
    // "power 4 or greater" is one phrase, not two alternatives.
    let text = text.replace(" or greater", " or_greater");
    for alt in text.split(" or ") {
        let mut clause = Clause::default();
        let mut rest = alt.trim();
        for (suffix, yours) in [(" you control", Some(true)), (" you don't control", Some(false)), (" your opponents control", Some(false))] {
            if let Some(r) = rest.strip_suffix(suffix) {
                rest = r;
                clause.yours = yours;
            }
        }
        if let Some((r, n)) = rest.split_once(" with power ") {
            clause.min_power = Some(n.strip_suffix(" or_greater")?.parse().ok()?);
            rest = r;
        }
        if let Some((r, k)) = rest.split_once(" without ") {
            clause.without.push(keyword_named(k)?);
            rest = r;
        }
        let words: Vec<&str> = rest.split_whitespace().map(|w| w.trim_end_matches(',')).collect();
        let (head, quals) = words.split_last()?;
        for q in quals {
            if *q == "attacking" {
                clause.attacking = true;
            } else if let Some(sub) = q.strip_prefix("non-") {
                clause.not_types.push(sub.to_string());
            } else if let Some(neg) = q.strip_prefix("non") {
                if let Some(c) = color_named(neg) {
                    clause.not_colors.push(c);
                } else if CARD_TYPES.contains(&neg) {
                    clause.not_types.push(neg.to_string());
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        // A card type may open the sentence ("Creatures you control"), and
        // is plural for a group; a subtype keeps its capital.
        let lower = head.to_lowercase();
        let singular = lower.strip_suffix('s').unwrap_or(&lower);
        let head = if CARD_TYPES.contains(&singular) || singular == "permanent" { singular } else { head };
        clause.head = match head {
            "permanent" => None,
            h if CARD_TYPES.contains(&h) => Some(h.to_string()),
            // A subtype is capitalised in Oracle text.
            h if h.chars().next().is_some_and(char::is_uppercase) => Some(h.to_string()),
            _ => return None,
        };
        clauses.push(clause);
    }
    Some(Filter(clauses))
}

/// "gets +3/+3 until end of turn", "gets +2/+2 and gains lifelink until end
/// of turn": the change, and the keywords gained.
fn pump(text: &str) -> Option<((i32, i32), Vec<Keyword>)> {
    let body = text.strip_suffix(" until end of turn")?;
    let (change, gains) = match body.split_once(" and ") {
        Some((c, g)) => (c, Some(g)),
        None => (body, None),
    };
    let change = pt_change(change.split_whitespace().nth(1)?)?;
    let mut keywords = Vec::new();
    if let Some(g) = gains {
        let g = g.strip_prefix("gains ").or_else(|| g.strip_prefix("gain "))?;
        for k in g.split(" and ") {
            keywords.push(keyword_named(k)?);
        }
    }
    Some((change, keywords))
}

impl Spec {
    fn slot(&mut self, slot: Slot) -> usize {
        self.slots.push(slot);
        self.slots.len() - 1
    }

    /// Read one sentence, `name` already replaced by `~`. None if no
    /// template matches.
    fn sentence(&mut self, s: &str) -> Option<()> {
        let s = s.trim().strip_suffix('.')?;
        // CR 120.3: "~ deals N damage to ...".
        if let Some(rest) = s.strip_prefix("~ deals ") {
            let (n, to) = rest.split_once(" damage to ")?;
            let amount = number(n)?;
            let effect = match to {
                "any target" => Effect::Damage { amount, slot: self.slot(Slot::Any) },
                "target player or planeswalker" => Effect::Damage { amount, slot: self.slot(Slot::PlayerOrPlaneswalker) },
                _ => {
                    if let Some(f) = to.strip_prefix("target ") {
                        let f = filter(f)?;
                        Effect::Damage { amount, slot: self.slot(Slot::Permanent(f)) }
                    } else {
                        Effect::DamageEach { amount, filter: filter(to.strip_prefix("each ")?)? }
                    }
                }
            };
            self.effects.push(effect);
            return Some(());
        }
        // A pump of one target or of a group (CR 611.2).
        if let Some(rest) = s.strip_prefix("Target ") {
            if let Some((who, change)) = rest.split_once(" gets ") {
                let ((power, toughness), gains) = pump(&format!("gets {change}"))?;
                let slot = self.slot(Slot::Permanent(filter(who)?));
                self.effects.push(Effect::Pump { slot, power, toughness, gains });
                return Some(());
            }
            if let Some((who, n)) = rest.split_once(" loses ") {
                let n: i32 = number(n.strip_suffix(" life")?)?.try_into().ok()?;
                let slot = self.player_slot(who)?;
                self.effects.push(Effect::LoseLife { slot, n });
                return Some(());
            }
            if let Some((who, n)) = rest.split_once(" mills ") {
                let n = number(n.strip_suffix(" cards").or_else(|| n.strip_suffix(" card"))?)? as usize;
                let slot = self.player_slot(who)?;
                self.effects.push(Effect::Mill { slot, n });
                return Some(());
            }
            return None;
        }
        if let Some((who, change)) = s.split_once(" get ") {
            let ((power, toughness), gains) = pump(&format!("get {change}"))?;
            self.effects.push(Effect::PumpEach { filter: filter(who)?, power, toughness, gains });
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("It gets ") {
            let ((power, toughness), gains) = pump(&format!("gets {rest}"))?;
            let slot = self.slots.len().checked_sub(1)?;
            self.effects.push(Effect::Pump { slot, power, toughness, gains });
            return Some(());
        }
        // Moving or changing one target permanent.
        for (verb, make) in [
            ("Destroy target ", (|slot| Effect::Destroy { slot }) as fn(usize) -> Effect),
            ("Exile target ", |slot| Effect::Exile { slot }),
            ("Tap target ", |slot| Effect::Tap { slot, tapped: true }),
            ("Untap target ", |slot| Effect::Tap { slot, tapped: false }),
        ] {
            if let Some(f) = s.strip_prefix(verb) {
                let slot = self.slot(Slot::Permanent(filter(f)?));
                self.effects.push(make(slot));
                return Some(());
            }
        }
        if let Some(f) = s.strip_prefix("Return target ").and_then(|r| r.strip_suffix(" to its owner's hand")) {
            let slot = self.slot(Slot::Permanent(filter(f)?));
            self.effects.push(Effect::ToHand { slot });
            return Some(());
        }
        if let Some(f) = s.strip_prefix("Put target ").and_then(|r| r.strip_suffix(" on top of its owner's library")) {
            let slot = self.slot(Slot::Permanent(filter(f)?));
            self.effects.push(Effect::ToLibraryTop { slot });
            return Some(());
        }
        if s == "Its controller gains life equal to its power" {
            let slot = self.slots.len().checked_sub(1)?;
            self.effects.push(Effect::ControllerGainsLifeEqualToPower { slot });
            return Some(());
        }
        if let Some(n) = s.strip_prefix("Draw ") {
            let n = number(n.strip_suffix(" cards").or_else(|| n.strip_suffix(" card"))?)? as usize;
            self.effects.push(Effect::Draw { n });
            return Some(());
        }
        if let Some(n) = s.strip_prefix("You gain ").and_then(|r| r.strip_suffix(" life")) {
            self.effects.push(Effect::GainLife { n: number(n)?.try_into().ok()? });
            return Some(());
        }
        if let Some(rest) = s.strip_prefix("Create ") {
            return self.tokens(rest);
        }
        None
    }

    fn player_slot(&mut self, who: &str) -> Option<usize> {
        match who {
            "player" => Some(self.slot(Slot::Player)),
            "opponent" => Some(self.slot(Slot::Opponent)),
            _ => None,
        }
    }

    /// "two 1/1 white Spirit creature tokens with flying", "thirteen tapped
    /// 2/2 black Zombie creature tokens", "a 1/1 ... token".
    fn tokens(&mut self, rest: &str) -> Option<()> {
        let (rest, keywords) = match rest.split_once(" with ") {
            Some((r, k)) => (r, k.split(" and ").map(keyword_named).collect::<Option<Vec<_>>>()?),
            None => (rest, Vec::new()),
        };
        let rest = rest.strip_suffix(" creature tokens").or_else(|| rest.strip_suffix(" creature token"))?;
        let mut words = rest.split_whitespace();
        let n = number(words.next()?)? as usize;
        let mut next = words.next()?;
        let tapped = next == "tapped";
        if tapped {
            next = words.next()?;
        }
        let (power, toughness) = {
            let (p, t) = next.split_once('/')?;
            (p.parse().ok()?, t.parse().ok()?)
        };
        let mut colors = Vec::new();
        let mut subtypes = Vec::new();
        for w in words {
            if let Some(c) = color_named(w) {
                colors.push(c);
            } else if w != "and" && w.chars().next().is_some_and(char::is_uppercase) {
                subtypes.push(w.to_string());
            } else if w != "and" {
                return None;
            }
        }
        let mut keywords = keywords;
        keywords.sort_by_key(|k| k.label());
        self.effects.push(Effect::Tokens { n, power, toughness, colors, subtypes, keywords, tapped });
        Some(())
    }
}

/// Read a card's whole text, or say which sentence the grammar cannot read.
fn read(card: &Printed) -> Result<Spec, String> {
    let text = without_reminders(&card.oracle_text).replace(&card.name, "~");
    let mut spec = Spec::default();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        // Flashback is a second way to cast the card (CR 702.34a); the cast
        // from hand is what is checked here, and `flashback.rs` reaches the
        // other one.
        if line.starts_with("Flashback {") {
            continue;
        }
        if line == "This spell costs {1} less to cast for each creature on the battlefield." {
            spec.cheaper_per_creature = true;
            continue;
        }
        for sentence in line.split_inclusive(". ") {
            spec.sentence(sentence).ok_or_else(|| format!("cannot read {sentence:?}"))?;
        }
    }
    if spec.effects.is_empty() {
        return Err("no effects".to_string());
    }
    Ok(spec)
}

// ---- the board ----

/// The board every card is cast on: P0 casts, in their precombat main
/// phase. Cards whose text is keywords or nothing but a trigger that cannot
/// fire mid-turn, so nothing on the board changes what a spell does except
/// through what it is: colour, type, subtype, power, flying, controller.
const BOARD: [(&str, PlayerId); 14] = [
    ("Grizzly Bears", P0),
    ("Chapel Geist", P0),
    ("Markov Patrician", P0),
    ("Gatstaf Shepherd", P0),
    ("Plains", P0),
    ("Sol Ring", P0),
    ("Walking Corpse", P1),
    ("Kindercatch", P1),
    ("Moon Heron", P1),
    ("Goblin Piker", P1),
    ("Rotting Fensnake", P1),
    ("Villagers of Estwald", P1),
    ("Forest", P1),
    ("Burning Vengeance", P1),
];

/// Each player's library, top first: distinct names, so a card put on top
/// or drawn is told apart from the rest.
const LIBRARY: [&str; 5] = ["Island", "Swamp", "Mountain", "Plains", "Forest"];

/// One permanent as the reference sees it: its printed characteristics and
/// what has happened to it.
#[derive(Debug, Clone)]
struct Model {
    /// None for a token.
    name: Option<String>,
    owner: PlayerId,
    controller: PlayerId,
    types: Vec<String>,
    subtypes: Vec<String>,
    colors: Vec<Color>,
    keywords: Vec<Keyword>,
    power: Option<i32>,
    toughness: Option<i32>,
    damage: u32,
    tapped: bool,
}

impl Model {
    fn printed(p: &Printed, owner: PlayerId) -> Self {
        let (types, subtypes) = p.types();
        Model {
            name: Some(p.name.clone()),
            owner,
            controller: owner,
            types: types.iter().map(|t| t.to_lowercase()).collect(),
            subtypes,
            colors: p.colors(),
            keywords: p.keywords(),
            power: p.power,
            toughness: p.toughness,
            damage: 0,
            tapped: false,
        }
    }

    fn is_creature(&self) -> bool {
        self.types.iter().any(|t| t == "creature")
    }

    fn matches(&self, filter: &Filter, caster: PlayerId) -> bool {
        filter.0.iter().any(|c| {
            let head = match &c.head {
                None => true,
                Some(h) if CARD_TYPES.contains(&h.as_str()) => self.types.contains(h),
                Some(sub) => self.subtypes.contains(sub),
            };
            head
                && !c.not_colors.iter().any(|col| self.colors.contains(col))
                && !c.not_types.iter().any(|t| self.types.contains(t) || self.subtypes.contains(t))
                && c.min_power.is_none_or(|n| self.power.is_some_and(|p| p >= n))
                && !c.without.iter().any(|k| self.keywords.contains(k))
                && c.yours.is_none_or(|y| (self.controller == caster) == y)
                // Nothing on the board is attacking: it is the precombat
                // main phase.
                && !c.attacking
        })
    }
}

/// The reference's whole board: permanents by the id the engine gave them,
/// and each player's zones by name.
#[derive(Debug, Clone)]
struct World {
    permanents: BTreeMap<ObjectId, Model>,
    tokens: Vec<Model>,
    life: [i32; 2],
    hand: [Vec<String>; 2],
    graveyard: [Vec<String>; 2],
    exile: [Vec<String>; 2],
    library: [Vec<String>; 2],
}

/// What is observable about a game at rest, in a form both the engine's
/// state and the reference's world produce: every zone by name, every
/// permanent by its characteristics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Observed {
    life: [i32; 2],
    hand: [Vec<String>; 2],
    graveyard: [Vec<String>; 2],
    exile: [Vec<String>; 2],
    library: [Vec<String>; 2],
    battlefield: Vec<PermanentSeen>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PermanentSeen {
    name: Option<String>,
    controller: u8,
    power: Option<i32>,
    toughness: Option<i32>,
    damage: u32,
    tapped: bool,
    keywords: Vec<&'static str>,
    colors: Vec<char>,
    subtypes: Vec<String>,
}

fn seen(m: &Model) -> PermanentSeen {
    let mut keywords: Vec<&'static str> = m.keywords.iter().map(|k| k.label()).collect();
    keywords.sort_unstable();
    keywords.dedup();
    let mut colors: Vec<char> = m.colors.iter().map(|c| color_letter(*c)).collect();
    colors.sort_unstable();
    let mut subtypes = m.subtypes.clone();
    subtypes.sort();
    PermanentSeen {
        name: m.name.clone(),
        controller: m.controller.0,
        power: m.power,
        toughness: m.toughness,
        damage: m.damage,
        tapped: m.tapped,
        keywords,
        colors,
        subtypes,
    }
}

impl World {
    fn observed(&self) -> Observed {
        let mut battlefield: Vec<PermanentSeen> = self.permanents.values().chain(&self.tokens).map(seen).collect();
        battlefield.sort();
        let sorted = |zones: &[Vec<String>; 2]| zones.clone().map(|mut z| { z.sort(); z });
        Observed {
            life: self.life,
            hand: sorted(&self.hand),
            graveyard: sorted(&self.graveyard),
            exile: sorted(&self.exile),
            library: self.library.clone(),
            battlefield,
        }
    }

    /// A permanent leaves the battlefield for one of its owner's zones; a
    /// token leaves the game (CR 111.7, 704.5d).
    fn leave(&mut self, id: ObjectId, to: fn(&mut World) -> &mut [Vec<String>; 2], top: bool) {
        let Some(m) = self.permanents.remove(&id) else { return };
        let Some(name) = m.name else { return };
        let owner = m.owner.0 as usize;
        let zone = &mut to(self)[owner];
        if top {
            zone.insert(0, name);
        } else {
            zone.push(name);
        }
    }

    /// State-based actions, as CR 704.5f-g state them: a creature with
    /// toughness 0 or less, or with damage at least its toughness, is put
    /// into its owner's graveyard. Repeated until none applies (CR 704.3).
    fn state_based_actions(&mut self) {
        loop {
            let dying: Vec<ObjectId> = self.permanents.iter()
                .filter(|(_, m)| m.is_creature() && m.toughness.is_some_and(|t| t <= 0 || m.damage >= t as u32))
                .map(|(id, _)| *id)
                .collect();
            let before = self.tokens.len();
            self.tokens.retain(|m| m.toughness.is_none_or(|t| t > 0 && m.damage < t as u32));
            if dying.is_empty() && before == self.tokens.len() {
                return;
            }
            for id in dying {
                self.leave(id, |w| &mut w.graveyard, false);
            }
        }
    }
}

/// The board, built in the engine, and the same board as the reference
/// sees it.
fn board(registry: &CardRegistry, cache: &BTreeMap<String, Printed>) -> (GameState, World) {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mut world = World {
        permanents: BTreeMap::new(),
        tokens: Vec::new(),
        life: [20, 20],
        hand: [Vec::new(), Vec::new()],
        graveyard: [Vec::new(), Vec::new()],
        exile: [Vec::new(), Vec::new()],
        library: [Vec::new(), Vec::new()],
    };
    for (name, owner) in BOARD {
        let id = named_permanent(&mut state, registry, name, owner);
        world.permanents.insert(id, Model::printed(&cache[name], owner));
    }
    // A land the caster could play, so that once the spell has resolved the
    // game loop has something to ask them: it passes priority on its own
    // for a player with nothing to do, and would play on into the next
    // turn after a spell that left the caster nothing.
    spell_in_hand(&mut state, registry, "Island", P0);
    world.hand[0].push("Island".to_string());
    for p in [P0, P1] {
        for name in LIBRARY {
            stock_library_with(&mut state, registry, p, name, 1);
            world.library[p.0 as usize].push(name.to_string());
        }
    }
    (state, world)
}

/// The engine's state, observed the way [`World::observed`] observes the
/// reference.
fn observe(state: &GameState, registry: &CardRegistry) -> Observed {
    let name_of = |id: ObjectId| -> String {
        let obj = state.get_object(id).expect("a zone names an object that exists");
        registry.card_data(obj.card_id).map_or_else(|| obj.name.clone(), |d| d.name)
    };
    // By owner: a hand, a graveyard and a library are each a player's
    // own, and exile is one zone for everyone (CR 400.1), its cards told
    // apart here by whose they are.
    let zone = |zone: Zone| -> [Vec<String>; 2] {
        [P0, P1].map(|p| {
            let mut names: Vec<String> = state.objects.values()
                .filter(|o| o.zone == zone && o.owner == p)
                .map(|o| name_of(o.id))
                .collect();
            names.sort();
            names
        })
    };
    let mut battlefield: Vec<PermanentSeen> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield)
        .map(|o| {
            let creature = state.is_creature(o.id, registry);
            let mut keywords: Vec<&'static str> = Keyword::ALL.into_iter()
                .filter(|k| state.has_keyword(o.id, *k, registry))
                .map(|k| k.label())
                .collect();
            keywords.sort_unstable();
            let mut colors: Vec<char> = state.colors_of(o.id, registry).into_iter().map(color_letter).collect();
            colors.sort_unstable();
            let mut subtypes = state.subtypes_of(o.id, registry);
            subtypes.sort();
            PermanentSeen {
                name: if o.is_token { None } else { Some(name_of(o.id)) },
                controller: o.controller.0,
                power: if creature { state.effective_power(o.id, registry) } else { None },
                toughness: if creature { state.effective_toughness(o.id, registry) } else { None },
                damage: o.damage_marked,
                tapped: o.tapped,
                keywords,
                colors,
                subtypes,
            }
        })
        .collect();
    battlefield.sort();
    Observed {
        life: [state.players[0].life, state.players[1].life],
        hand: zone(Zone::Hand),
        graveyard: zone(Zone::Graveyard),
        exile: zone(Zone::Exile),
        library: [P0, P1].map(|p| state.players[p.0 as usize].library_order.iter().map(|&id| name_of(id)).collect()),
        battlefield,
    }
}

/// What differs between the engine's game and the text's, field by field:
/// the zones that disagree, and the permanents only one side has.
fn differences(engine: &Observed, text: &Observed) -> String {
    let mut out = Vec::new();
    if engine.life != text.life {
        out.push(format!("life: engine {:?}, the text {:?}", engine.life, text.life));
    }
    for (what, e, t) in [
        ("hand", &engine.hand, &text.hand),
        ("graveyard", &engine.graveyard, &text.graveyard),
        ("exile", &engine.exile, &text.exile),
        ("library", &engine.library, &text.library),
    ] {
        if e != t {
            out.push(format!("{what}: engine {e:?}, the text {t:?}"));
        }
    }
    let only = |a: &[PermanentSeen], b: &[PermanentSeen]| -> Vec<String> {
        let mut rest = b.to_vec();
        a.iter()
            .filter(|p| match rest.iter().position(|q| q == *p) {
                Some(i) => { rest.remove(i); false }
                None => true,
            })
            .map(|p| format!("{p:?}"))
            .collect()
    };
    let e = only(&engine.battlefield, &text.battlefield);
    let t = only(&text.battlefield, &engine.battlefield);
    if !e.is_empty() || !t.is_empty() {
        out.push(format!("battlefield: only the engine has\n        {}\n      only the text has\n        {}",
            e.join("\n        "), t.join("\n        ")));
    }
    out.join("\n      ")
}

// ---- the reference semantics ----

/// Every target a slot accepts on `world`, for a spell `caster` casts. No
/// permanent on the board has hexproof, shroud or protection, so "can be
/// targeted" is "matches the words" (CR 115.1, 702.11, 702.16).
fn candidates(slot: &Slot, world: &World, caster: PlayerId) -> Vec<Target> {
    let players = |which: &dyn Fn(PlayerId) -> bool| -> Vec<Target> {
        [P0, P1].into_iter().filter(|p| which(*p)).map(Target::Player).collect()
    };
    let creatures = || world.permanents.iter()
        .filter(|(_, m)| m.is_creature())
        .map(|(id, _)| Target::Object(*id))
        .collect::<Vec<_>>();
    let mut out = match slot {
        Slot::Any => [creatures(), players(&|_| true)].concat(),
        Slot::PlayerOrPlaneswalker | Slot::Player => players(&|_| true),
        Slot::Opponent => players(&|p| p != caster),
        Slot::Permanent(f) => world.permanents.iter()
            .filter(|(_, m)| m.matches(f, caster))
            .map(|(id, _)| Target::Object(*id))
            .collect(),
    };
    out.sort_by_key(target_key);
    out
}

fn target_key(t: &Target) -> (u8, u64) {
    match t {
        Target::Object(id) => (0, id.0),
        Target::Player(p) => (1, u64::from(p.0)),
        other => panic!("a cast names an object or a player, not {other:?}"),
    }
}

/// Every set of targets the text allows: one from each slot, the same
/// object at most once per slot (CR 115.3) and allowed across slots,
/// which is what the spells here would need (none shares a candidate
/// between slots).
fn target_sets(spec: &Spec, world: &World, caster: PlayerId) -> Vec<Vec<Target>> {
    let mut sets: Vec<Vec<Target>> = vec![Vec::new()];
    for slot in &spec.slots {
        let options = candidates(slot, world, caster);
        sets = sets.into_iter()
            .flat_map(|set| options.iter().map(move |t| {
                let mut s = set.clone();
                s.push(t.clone());
                s
            }))
            .collect();
    }
    sets
}

/// What resolving `spec` with `targets` does to `world`, then state-based
/// actions (CR 608.2, 704.3), with the spell itself put into its owner's
/// graveyard last (CR 608.2n).
fn resolve(spec: &Spec, world: &World, targets: &[Target], caster: PlayerId, name: &str) -> World {
    let mut w = world.clone();
    let c = caster.0 as usize;
    let object = |slot: usize| match &targets[slot] {
        Target::Object(id) => Some(*id),
        _ => None,
    };
    let player = |slot: usize| match &targets[slot] {
        Target::Player(p) => Some(p.0 as usize),
        _ => None,
    };
    for effect in &spec.effects {
        match effect {
            Effect::Damage { amount, slot } => {
                if let Some(id) = object(*slot) {
                    w.permanents.get_mut(&id).expect("a target on the board").damage += amount;
                } else if let Some(p) = player(*slot) {
                    w.life[p] -= *amount as i32;
                }
            }
            Effect::DamageEach { amount, filter } => {
                for m in w.permanents.values_mut().chain(w.tokens.iter_mut()) {
                    if m.matches(filter, caster) {
                        m.damage += amount;
                    }
                }
            }
            Effect::Pump { slot, power, toughness, gains } => {
                if let Some(m) = object(*slot).and_then(|id| w.permanents.get_mut(&id)) {
                    m.power = m.power.map(|p| p + power);
                    m.toughness = m.toughness.map(|t| t + toughness);
                    m.keywords.extend(gains);
                }
            }
            Effect::PumpEach { filter, power, toughness, gains } => {
                for m in w.permanents.values_mut().chain(w.tokens.iter_mut()) {
                    if m.is_creature() && m.matches(filter, caster) {
                        m.power = m.power.map(|p| p + power);
                        m.toughness = m.toughness.map(|t| t + toughness);
                        m.keywords.extend(gains);
                    }
                }
            }
            Effect::Destroy { slot } => {
                if let Some(id) = object(*slot) {
                    w.leave(id, |w| &mut w.graveyard, false);
                }
            }
            Effect::Exile { slot } => {
                if let Some(id) = object(*slot) {
                    w.leave(id, |w| &mut w.exile, false);
                }
            }
            Effect::ToHand { slot } => {
                if let Some(id) = object(*slot) {
                    w.leave(id, |w| &mut w.hand, false);
                }
            }
            Effect::ToLibraryTop { slot } => {
                if let Some(id) = object(*slot) {
                    w.leave(id, |w| &mut w.library, true);
                }
            }
            Effect::Tap { slot, tapped } => {
                if let Some(m) = object(*slot).and_then(|id| w.permanents.get_mut(&id)) {
                    m.tapped = *tapped;
                }
            }
            Effect::Draw { n } => {
                for _ in 0..*n {
                    let card = w.library[c].remove(0);
                    w.hand[c].push(card);
                }
            }
            Effect::GainLife { n } => w.life[c] += n,
            Effect::LoseLife { slot, n } => {
                if let Some(p) = player(*slot) {
                    w.life[p] -= n;
                }
            }
            Effect::Mill { slot, n } => {
                if let Some(p) = player(*slot) {
                    for _ in 0..*n {
                        let card = w.library[p].remove(0);
                        w.graveyard[p].push(card);
                    }
                }
            }
            Effect::Tokens { n, power, toughness, colors, subtypes, keywords, tapped } => {
                for _ in 0..*n {
                    w.tokens.push(Model {
                        name: None,
                        owner: caster,
                        controller: caster,
                        types: vec!["creature".to_string()],
                        subtypes: subtypes.clone(),
                        colors: colors.clone(),
                        keywords: keywords.clone(),
                        power: Some(*power),
                        toughness: Some(*toughness),
                        damage: 0,
                        tapped: *tapped,
                    });
                }
            }
            Effect::ControllerGainsLifeEqualToPower { slot } => {
                // The object is gone by now; its last known power and
                // controller are those it had on `world`, modified by
                // anything earlier in this resolution.
                let id = object(*slot).expect("an object slot");
                let last = w.permanents.get(&id).or_else(|| world.permanents.get(&id)).expect("on the board");
                let gain = last.power.unwrap_or(0);
                w.life[last.controller.0 as usize] += gain;
            }
        }
    }
    w.state_based_actions();
    w.graveyard[c].push(name.to_string());
    w
}

// ---- the engine side ----

/// Mana for exactly `cost` in `player`'s pool: each coloured pip of its
/// colour, generic as colorless, less `cheaper` generic.
fn fund(state: &mut GameState, player: PlayerId, cost: &ManaCost, cheaper: u32) {
    let pool = &mut state.get_player_mut(player).mana_pool;
    for sym in &cost.symbols {
        match sym {
            ManaSymbol::Colored(c) => pool.add(ManaType::from(*c), 1),
            ManaSymbol::Generic(n) => pool.add(ManaType::Colorless, n.saturating_sub(cheaper)),
            other => panic!("no card read here costs {other:?}"),
        }
    }
}

/// The target sets the engine offers for casting `spell`, sorted: what
/// its row in the legal actions says (CR 601.2c), and for a spell that
/// asks at cast time, every path through the questions it asks.
fn offered(state: &GameState, registry: &CardRegistry, spell: ObjectId) -> Result<Vec<Vec<Target>>, String> {
    let legal = engine::legal_actions(state, registry);
    let Some(row) = legal.castable_spells.iter().find(|c| c.object_id == spell && !c.is_flashback) else {
        return Ok(Vec::new());
    };
    let mut sets = match &row.target_spec {
        CastTargetSpec::NoTargets => vec![Vec::new()],
        CastTargetSpec::SingleTarget(options) => options.iter().map(|t| vec![t.clone()]).collect(),
        CastTargetSpec::TwoTargets { first, second, second_min: 1, second_max: 1 } => first.iter().zip(second)
            .flat_map(|(a, bs)| bs.iter().map(move |b| vec![a.clone(), b.clone()]))
            .collect(),
        CastTargetSpec::TwoTargets { .. } => return Err("an \"up to\" second slot, which no template reads".to_string()),
        CastTargetSpec::ChosenAtCast => {
            let asked = engine::submit_action(state, &cast_action(spell, Vec::new()), registry);
            asked_at_cast(&asked, registry)?
        }
    };
    sets.sort_by_key(|s| s.iter().map(target_key).collect::<Vec<_>>());
    sets.dedup();
    Ok(sets)
}

/// Every target set reachable by answering the cast-time target questions
/// one target at a time, as a player would.
fn asked_at_cast(state: &GameState, registry: &CardRegistry) -> Result<Vec<Vec<Target>>, String> {
    match &state.awaiting_action {
        None => Ok(vec![Vec::new()]),
        Some(AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::ChooseTargetSet { options, min: 1, max: 1, .. }, .. }) => {
            let mut sets = Vec::new();
            for t in options {
                let answer = Action::ResolveChoice { choice: ResolvedChoice::ChosenTargetSet(vec![t.clone()]) };
                let next = engine::submit_action(state, &answer, registry);
                for rest in asked_at_cast(&next, registry)? {
                    sets.push([vec![t.clone()], rest].concat());
                }
            }
            Ok(sets)
        }
        Some(other) => Err(format!("the cast asks {other:?}, which no template reads")),
    }
}

/// Cast `spell` at `targets` and play on until the stack is empty, both
/// players passing priority (CR 117.4, 608.1): the engine's own game loop,
/// state-based actions and triggers and all.
fn cast_and_let_resolve(state: &GameState, registry: &CardRegistry, spell: ObjectId, targets: Vec<Target>) -> Result<GameState, String> {
    let mut state = state.clone();
    // A spell that asks for its targets at cast time is cast bare, and each
    // question answered in turn, one target each, as a player answers them.
    let asks = engine::legal_actions(&state, registry).castable_spells.iter()
        .any(|c| c.object_id == spell && matches!(c.target_spec, CastTargetSpec::ChosenAtCast));
    let mut answers: Vec<Action> = if asks {
        targets.iter().rev()
            .map(|t| Action::ResolveChoice { choice: ResolvedChoice::ChosenTargetSet(vec![t.clone()]) })
            .collect()
    } else {
        Vec::new()
    };
    let mut cast = Some(cast_action(spell, if asks { Vec::new() } else { targets }));
    let mut trouble = None;
    let mut decisions = 0;
    engine::run_game_loop(&mut state, registry, |s: &GameState, _acting: PlayerId, legal: &engine::LegalActions| {
        decisions += 1;
        if let Some(c) = cast.take() {
            return c;
        }
        if matches!(legal.resolution_prompt, Some(ResolutionChoiceKind::ChooseTargetSet { .. })) {
            if let Some(a) = answers.pop() {
                return a;
            }
        }
        if s.stack.is_empty() || decisions > 20 {
            if decisions > 20 {
                trouble = Some("the stack did not empty".to_string());
            }
            return Action::AbandonGame;
        }
        if let Some(prompt) = &legal.resolution_prompt {
            trouble = Some(format!("the engine asked {prompt:?}, which the text does not"));
            return Action::AbandonGame;
        }
        Action::PassPriority
    });
    match trouble {
        Some(why) => Err(why),
        None if state.objects.get(&spell).is_some_and(|o| o.zone == Zone::Hand) => Err("the cast was refused".to_string()),
        None => Ok(state),
    }
}

/// Every card the grammar reads, with what it read.
fn readable(registry: &CardRegistry, cache: &BTreeMap<String, Printed>) -> (Vec<(Printed, Spec)>, Vec<(String, String)>) {
    let mut read_ok = Vec::new();
    let mut unread = Vec::new();
    for name in registry.all_names() {
        let Some(card) = cache.get(name) else { continue };
        if !(card.has_type("Instant") || card.has_type("Sorcery")) {
            continue;
        }
        match read(card) {
            Ok(spec) if mana_cost(&card.mana_cost).is_some() => read_ok.push((card.clone(), spec)),
            Ok(_) => unread.push((name.to_string(), format!("cannot read the cost {}", card.mana_cost))),
            Err(why) => unread.push((name.to_string(), why)),
        }
    }
    read_ok.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    unread.sort();
    (read_ok, unread)
}

/// How many cards the grammar read when this was last raised. A change
/// that reads more raises it; one that reads fewer fails here, so coverage
/// is never lost without someone deciding to lose it.
const CARDS_READ: usize = 31;

// ---- the properties ----

/// The board as the engine builds it is the board the Oracle data
/// describes, before anything is cast: otherwise every comparison below
/// would be against the wrong starting point.
#[test]
fn the_board_is_what_its_cards_print() {
    let registry = registry();
    let cache = oracle_cache();
    let (state, world) = board(&registry, &cache);
    let (got, want) = (observe(&state, &registry), world.observed());
    assert!(got == want, "{}", differences(&got, &want));
    assert!(world.permanents.values().all(|m| !m.keywords.contains(&Keyword::Hexproof)),
        "the reference's targeting assumes nothing on the board has hexproof");
}

/// Every card the grammar reads offers exactly the targets its text allows,
/// and at each of them does what its text says.
#[test]
fn every_readable_card_does_what_its_text_says() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, _) = readable(&registry, &cache);
    let mut failures = Vec::new();
    let mut casts = 0;
    for (card, spec) in &cards {
        let (mut state, world) = board(&registry, &cache);
        let spell = spell_in_hand(&mut state, &registry, &card.name, P0);
        let cost = mana_cost(&card.mana_cost).expect("read above");
        let creatures = u32::try_from(world.permanents.values().filter(|m| m.is_creature()).count()).expect("a few");
        fund(&mut state, P0, &cost, if spec.cheaper_per_creature { creatures } else { 0 });
        let mut before = world.clone();
        before.hand[0].push(card.name.clone());

        let expected_sets = target_sets(spec, &before, P0);
        let offered_sets = match offered(&state, &registry, spell) {
            Ok(sets) => sets,
            Err(why) => {
                failures.push(format!("{}: {why}", card.name));
                continue;
            }
        };
        if offered_sets != expected_sets {
            failures.push(format!("{}: offers targets {offered_sets:?}, the text allows {expected_sets:?} ({spec:?})", card.name));
            continue;
        }
        for targets in expected_sets {
            casts += 1;
            let mut cast_from = before.clone();
            cast_from.hand[0].retain(|n| *n != card.name);
            let want = resolve(spec, &cast_from, &targets, P0, &card.name).observed();
            match cast_and_let_resolve(&state, &registry, spell, targets.clone()) {
                Ok(after) => {
                    let got = observe(&after, &registry);
                    if got != want {
                        failures.push(format!("{} at {targets:?}:\n      {}", card.name, differences(&got, &want)));
                    }
                }
                Err(why) => failures.push(format!("{} at {targets:?}: {why}", card.name)),
            }
        }
    }
    assert!(casts > 0);
    assert!(failures.is_empty(), "{} of {} cards' casts disagree with their text:\n  {}",
        failures.len(), cards.len(), failures.join("\n  "));
}

#[test]
fn the_grammar_reads_at_least_as_many_cards_as_it_did() {
    let registry = registry();
    let cache = oracle_cache();
    let (cards, unread) = readable(&registry, &cache);
    let names: Vec<&str> = cards.iter().map(|(c, _)| c.name.as_str()).collect();
    assert!(cards.len() >= CARDS_READ,
        "the grammar reads {} instants and sorceries, fewer than the {CARDS_READ} it did: {names:?}\nunread: {unread:#?}",
        cards.len());
    println!("read {} of {}: {names:?}", cards.len(), cards.len() + unread.len());
    for (name, why) in &unread {
        println!("  unread {name}: {why}");
    }
}

/// The grammar reads what it is meant to, on text written out here, so a
/// template that silently stopped matching is caught even if no card in the
/// pool uses it any more.
#[test]
fn the_grammar_reads_its_templates() {
    let card = |name: &str, text: &str| Printed {
        name: name.to_string(),
        mana_cost: "{R}".to_string(),
        type_line: "Instant".to_string(),
        oracle_text: text.to_string(),
        power: None,
        toughness: None,
    };
    let spec = read(&card("Bolt", "Bolt deals 3 damage to any target.")).unwrap();
    assert_eq!(spec.slots, vec![Slot::Any]);
    assert_eq!(spec.effects, vec![Effect::Damage { amount: 3, slot: 0 }]);

    let spec = read(&card("X", "Destroy target non-Vampire, non-Werewolf, non-Zombie creature.")).unwrap();
    let Slot::Permanent(Filter(clauses)) = &spec.slots[0] else { panic!("a permanent slot") };
    assert_eq!(clauses[0].head.as_deref(), Some("creature"));
    assert_eq!(clauses[0].not_types, vec!["Vampire", "Werewolf", "Zombie"]);

    let spec = read(&card("X", "Create thirteen tapped 2/2 black Zombie creature tokens.")).unwrap();
    assert_eq!(spec.effects, vec![Effect::Tokens {
        n: 13, power: 2, toughness: 2, colors: vec![Color::Black], subtypes: vec!["Zombie".to_string()],
        keywords: vec![], tapped: true,
    }]);

    let spec = read(&card("X", "Exile target creature. Its controller gains life equal to its power.")).unwrap();
    assert_eq!(spec.effects, vec![Effect::Exile { slot: 0 }, Effect::ControllerGainsLifeEqualToPower { slot: 0 }]);

    assert!(read(&card("X", "Counter target spell.")).is_err(), "a spell target is not a template here yet");
    assert!(read(&card("X", "Draw two cards, then discard a card at random.")).is_err());
}

