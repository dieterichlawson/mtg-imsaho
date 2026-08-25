//! Enforcement for the characteristics rule in `state.rs`.
//!
//!     an object's characteristics = its active face  UNION  its runtime grants
//!
//! This file is a guard, not a behaviour test. The rule was documented in
//! `state.rs` long before this existed and was violated in roughly twenty
//! places anyway, because nothing failed when it was: card code that read
//! `obj.card_types` directly worked in a real game (where `setup_game` used to
//! copy each card's data onto its object) and silently did nothing under test
//! (where `create_object` left the same fields empty). The same bug was found
//! and re-reported about fifteen times by successive audits.
//!
//! So the rule is now checked mechanically. If you are here because this test
//! failed, the fix is almost never to add your file to the allowlist — it is
//! to call the accessor:
//!
//!     obj.card_types.contains(&CardType::Creature)  ->  state.has_card_type(id, CardType::Creature, registry)
//!     obj.subtypes.iter().any(|s| s == "Human")     ->  state.has_subtype(id, "Human", registry)
//!     obj.colors.contains(&Color::Black)            ->  state.colors_of(id, registry).contains(&Color::Black)
//!     obj.keywords.contains(&Keyword::Flying)       ->  state.has_keyword(id, Keyword::Flying, registry)
//!     registry.card_data(obj.card_id)               ->  state.face_data(id, registry)   // card_data is ALWAYS the front face
//!
//! Writing to these fields is still fine — that is how a runtime grant is
//! recorded (Olivia Voldaren's "Vampire", Grimoire of the Dead's "Zombie").
//! Only reads are policed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Files permitted to read the raw fields, with the reason each is exempt.
/// Every entry is a place that legitimately implements the layer rather than
/// consuming it. Adding to this list needs the same kind of justification.
const ALLOWED: &[(&str, &str)] = &[
    ("state.rs", "defines the characteristics layer itself"),
    ("view.rs", "renders the object for display, not a rules decision"),
    ("engine.rs", "setup_game builds library objects from a decklist; no game object exists yet"),
    ("engine/costs.rs", "matches a CARD being cast against a SpellFilter by CardId — a \
      DFC is always cast as its front face (CR 712.6a), so the registry face is the right read"),
    ("cards/isd/olivia_voldaren.rs", "grants the Vampire subtype at runtime"),
    ("cards/isd/grimoire_of_the_dead.rs", "grants Zombie and black at runtime"),
    ("cards/isd/nevermore.rs", "names a CARD from the registry; no game object involved"),
    ("triggers.rs", "face_name implements front/back resolution for trigger display"),
];

/// Reads that indicate a rules decision made off the raw field.
const BANNED_READS: &[&str] = &[
    ".card_types.contains",
    ".card_types.iter",
    ".card_types.is_empty",
    ".subtypes.contains",
    ".subtypes.iter",
    ".subtypes.is_empty",
    ".colors.contains",
    ".colors.iter",
    ".keywords.contains",
    ".keywords.iter",
    "registry.card_data(",
];

fn src_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rel(path: &Path) -> String {
    path.strip_prefix(src_root()).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

fn is_allowed(rel_path: &str) -> bool {
    ALLOWED.iter().any(|(f, _)| *f == rel_path)
}

/// A `CardData` literal declares a card's printed characteristics — those are
/// `subtypes: vec![...]` field initialisers, not reads of a game object.
fn is_declaration(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("subtypes:") || t.starts_with("card_types:")
        || t.starts_with("colors:") || t.starts_with("keywords:")
}

/// Reads of a `CardData` binding (`d.subtypes`, `back.keywords`) are reads of a
/// face, which is exactly what the layer is built on.
fn is_face_binding(line: &str, pat: &str) -> bool {
    let Some(idx) = line.find(pat) else { return false };
    let before = &line[..idx];
    ["d", "data", "card_data", "front", "back", "face", "self"]
        .iter()
        .any(|b| before.ends_with(b))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("src dir should be readable").flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn card_code_reads_characteristics_through_the_accessors() {
    let mut files = Vec::new();
    walk(&src_root(), &mut files);
    files.sort();

    let mut violations: Vec<String> = Vec::new();
    for path in &files {
        let rel_path = rel(path);
        if is_allowed(&rel_path) {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("source file should be readable");
        for (i, line) in text.lines().enumerate() {
            if is_declaration(line) {
                continue;
            }
            for pat in BANNED_READS {
                if line.contains(pat) && !is_face_binding(line, pat) {
                    violations.push(format!("{rel_path}:{}: {}", i + 1, line.trim()));
                }
            }
        }
    }

    assert!(violations.is_empty(),
        "{} place(s) read an object's characteristics off the raw field instead \
         of through the characteristics layer.\n\n{}\n\nUse \
         `state.has_card_type` / `has_subtype` / `colors_of` / `has_keyword` / \
         `face_data`. See the header of this file for the mapping.",
        violations.len(), violations.join("\n"));
}

/// The allowlist is a liability, so keep it honest: every entry must name a
/// file that exists and must still actually need the exemption.
#[test]
fn the_allowlist_has_no_dead_entries() {
    let mut stale = Vec::new();
    for (file, reason) in ALLOWED {
        let path = src_root().join(file);
        if !path.exists() {
            stale.push(format!("{file} (no such file) — {reason}"));
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("allowlisted file should be readable");
        let still_needed = text.lines()
            .filter(|l| !is_declaration(l))
            .any(|l| BANNED_READS.iter().any(|p| l.contains(p) && !is_face_binding(l, p)));
        if !still_needed {
            stale.push(format!("{file} no longer reads raw characteristics — {reason}"));
        }
    }
    let names: BTreeSet<&str> = ALLOWED.iter().map(|(f, _)| *f).collect();
    assert_eq!(names.len(), ALLOWED.len(), "duplicate entry in the allowlist");
    assert!(stale.is_empty(),
        "stale allowlist entries — drop them:\n{}", stale.join("\n"));
}
