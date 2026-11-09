//! What the log every seat is sent says of a card no one has seen: nothing.
//!
//! Every line above `Private` is in the log every seat is sent
//! (`GameView::full_log`), so a line written between two decisions that names
//! a card in a hand or a library at both (drawn, put back, looked at,
//! searched for) tells the other seat what it is (CR 400.2, 401.2, 402.3),
//! unless the card is known: the line says it was revealed (701.16a), it is
//! the card being cast, which announcing put on the stack in view (601.2a,
//! though the engine keeps a cast waiting on its X in its hand), or it has
//! been seen in a public zone, as a creature returned to its owner's hand
//! has. The runner's view checks cannot see this: a view and its copy with
//! the hands dealt anew share one log.
//!
//! A line names a card by its id (`Name (#12)`, as `GameState::obj_name`
//! writes it), or by its name alone (Delver's look is written so). A name is
//! flagged only if no card bearing it has been in view (kept across the
//! game, with the names a line says were revealed), after the line's seen
//! names are taken out, longest first ("Kessig Wolf Run" is not "Kessig
//! Wolf"), and what it quotes, a name chosen aloud (Nevermore's) being a name
//! and not a card. So a hidden card that shares its name with one in view is
//! not seen by name, only by id.

use std::collections::BTreeSet;

use crate::ids::ObjectId;
use crate::state::{GameObject, GameState, LogLevel};
use crate::types::Zone;

/// What has been in view so far in one game, for [`SharedLog::check`].
#[derive(Debug, Default)]
pub struct SharedLog {
    /// Every object found in a public zone at a decision.
    seen: BTreeSet<ObjectId>,
    /// Every name a card in view has borne, or a line said was revealed.
    seen_names: BTreeSet<String>,
}

/// The ids a log line names, as `obj_name` writes them (`Name (#12)`) and as
/// the stack's lines do (`spell #12`): every run of digits after a `#`.
pub fn ids_named(line: &str) -> impl Iterator<Item = ObjectId> + '_ {
    line.split('#').skip(1).filter_map(|rest| {
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok().map(ObjectId)
    })
}

fn hidden(s: &GameState, id: ObjectId) -> Option<&GameObject> {
    s.get_object(id).filter(|o| matches!(o.zone, Zone::Hand | Zone::Library))
}

fn being_cast(s: &GameState, id: ObjectId) -> bool {
    s.pending_spell_cast.as_ref().is_some_and(|c| c.object_id == id)
}

impl SharedLog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The lines written between `prev`, the state at one decision, and
    /// `cur`, the state at the next, that name a card no one has seen; then
    /// what is in view at `cur` is remembered. With no `prev` (the first
    /// decision) only the remembering is done. One message per card a line
    /// names.
    pub fn check(&mut self, prev: Option<&GameState>, cur: &GameState) -> Vec<String> {
        let out = prev.map(|prev| self.lines(prev, cur)).unwrap_or_default();
        for obj in cur.objects.values() {
            if !matches!(obj.zone, Zone::Hand | Zone::Library) {
                self.seen.insert(obj.id);
                self.seen_names.insert(obj.name.clone());
            }
        }
        out
    }

    fn lines(&mut self, prev: &GameState, cur: &GameState) -> Vec<String> {
        // The cards no one has seen, at both decisions.
        let unseen: Vec<&GameObject> = cur.objects.values()
            .filter(|o| !self.seen.contains(&o.id) && !being_cast(prev, o.id) && !being_cast(cur, o.id))
            .filter(|o| hidden(cur, o.id).is_some() && hidden(prev, o.id).is_some())
            .collect();
        // A name some card in view bears at either decision is seen: the line
        // may be about that card ("p0 played Mountain (#12)"). So is the name
        // of the card being cast, announced as it was: "Corpse Lunge (#12):
        // cast cancelled" is about it, not about another Corpse Lunge in a
        // library.
        for s in [prev, cur] {
            for o in s.objects.values() {
                if hidden(s, o.id).is_none() || being_cast(s, o.id) {
                    self.seen_names.insert(o.name.clone());
                }
            }
        }
        let mut out = Vec::new();
        for entry in cur.game_log.iter().skip(prev.game_log.len()) {
            if entry.level <= LogLevel::Private {
                continue;
            }
            if entry.message.contains("reveal") {
                for o in &unseen {
                    if !o.name.is_empty() && entry.message.contains(o.name.as_str()) {
                        self.seen_names.insert(o.name.clone());
                    }
                }
                continue;
            }
            for id in ids_named(&entry.message) {
                if let (Some(is), Some(was)) = (unseen.iter().find(|o| o.id == id), hidden(prev, id)) {
                    out.push(format!("the shared log names {} (#{}), in p{}'s {:?} before and {:?} after and never seen (CR 400.2): {:?}",
                        is.name, id.0, is.owner.0, was.zone, is.zone, entry.message));
                }
            }
            // The line with every seen name taken out, longest first, and
            // with what it quotes taken out.
            let mut rest: String = entry.message.split('"').step_by(2).collect::<Vec<_>>().join(" ");
            let mut names: Vec<&String> = self.seen_names.iter().collect();
            names.sort_by_key(|n| std::cmp::Reverse(n.len()));
            for n in names {
                rest = rest.replace(n.as_str(), " ");
            }
            for o in &unseen {
                if !o.name.is_empty() && !self.seen_names.contains(&o.name) && rest.contains(o.name.as_str()) {
                    out.push(format!("the shared log names {}, a name no one has seen, in p{}'s {:?} (CR 400.2): {:?}",
                        o.name, o.owner.0, o.zone, entry.message));
                }
            }
        }
        out
    }
}
