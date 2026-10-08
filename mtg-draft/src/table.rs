//! The asynchronous draft table.
//!
//! `DraftState` (`draft.rs`) drafts in lockstep: every seat picks, then
//! every pack moves. That is the right shape for a pod of LLM seats that
//! all answer in about the same time, and the wrong one for a table with a
//! person at it — a person takes thirty seconds, an AI takes ten, and
//! nobody should wait on anybody who is not holding their next pack.
//!
//! Here each seat has a queue of packs in front of it. A pick takes one
//! card off the head pack and passes the rest to the neighbour's queue —
//! left for rounds 1 and 3, right for round 2, the rotation
//! `DraftState::rotate_packs` performs — so a slow seat accumulates packs
//! and a fast one waits only when its queue is empty. A round ends when
//! every pack of the round is empty; the next round's packs are then dealt
//! to every seat at once.

use std::collections::VecDeque;

use serde::Serialize;

use crate::pack::BoosterPack;

/// Where a pack goes after a pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Seat N passes to seat N+1.
    Left,
    /// Seat N passes to seat N-1.
    Right,
}

impl Direction {
    /// The direction of a round, 0-based: rounds 1 and 3 pass left, round 2
    /// passes right.
    #[must_use]
    pub fn for_round(round: usize) -> Self {
        if round.is_multiple_of(2) {
            Direction::Left
        } else {
            Direction::Right
        }
    }
}

/// Which seat a pack goes to from `seat`, in a pod of `pod_size`.
#[must_use]
pub fn neighbour(seat: usize, pod_size: usize, direction: Direction) -> usize {
    match direction {
        Direction::Left => (seat + 1) % pod_size,
        Direction::Right => (seat + pod_size - 1) % pod_size,
    }
}

/// One booster on the table.
#[derive(Debug, Clone)]
struct Pack {
    /// 0-based.
    round: usize,
    /// What is left in it.
    cards: Vec<String>,
    /// How many it was dealt with.
    size: usize,
}

/// One pick a seat made.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TablePick {
    /// 1-based.
    pub round: usize,
    /// 1-based, within the pack: pick `size - remaining + 1`.
    pub pick: usize,
    pub card: String,
    /// The table picked for the seat (a timer ran out, or the seat was
    /// kicked) rather than the seat choosing.
    pub auto: bool,
    /// What the pack held when the pick was made, the chosen card
    /// included — what the lockstep draft's log records as `available`.
    pub available: Vec<String>,
}

/// The pack a seat is looking at.
#[derive(Debug, Clone, Copy)]
pub struct PackInFront<'a> {
    pub id: usize,
    /// 1-based.
    pub round: usize,
    /// 1-based: the pick this seat is making from this pack.
    pub pick: usize,
    /// How many cards the pack was dealt with.
    pub size: usize,
    pub cards: &'a [String],
    /// Packs queued behind this one.
    pub waiting: usize,
}

/// What one pick did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picked {
    pub card: String,
    /// 1-based.
    pub round: usize,
    /// 1-based.
    pub pick: usize,
    /// The seat the rest of the pack went to, or `None` when the pick
    /// emptied it.
    pub passed_to: Option<usize>,
    /// Every pack of the round is now empty. The next round, if any, has
    /// been dealt.
    pub round_over: bool,
    /// The whole draft is over.
    pub draft_over: bool,
}

/// The draft table: every pack, every queue, every pick.
#[derive(Debug, Clone)]
pub struct Table {
    pod_size: usize,
    pack_size: usize,
    packs: Vec<Pack>,
    queues: Vec<VecDeque<usize>>,
    picks: Vec<Vec<TablePick>>,
    pools: Vec<Vec<String>>,
    /// 0-based; `ROUNDS` once the draft is over.
    round: usize,
    /// `[seat][round]`, as dealt.
    original: Vec<Vec<Vec<String>>>,
}

/// A booster draft is three packs a seat.
pub const ROUNDS: usize = 3;

impl Table {
    /// Seat the pod and deal round 1. `packs` is `packs[seat][round]`, as
    /// `generate_draft_packs` returns them.
    ///
    /// # Errors
    /// Fewer than one seat, a seat with other than `ROUNDS` packs, an
    /// empty pack, or packs of unequal size — the per-seat pick count
    /// assumes every pack holds the same number of cards, which a
    /// generated box always does (a foil displaces a common).
    pub fn new(packs: &[Vec<BoosterPack>]) -> Result<Self, String> {
        let cards: Vec<Vec<Vec<String>>> = packs
            .iter()
            .map(|seat_packs| seat_packs.iter().map(BoosterPack::all_cards).collect())
            .collect();
        Self::from_cards(cards)
    }

    /// [`Table::new`] on bare card lists, `cards[seat][round]`.
    ///
    /// # Errors
    /// As [`Table::new`].
    pub fn from_cards(cards: Vec<Vec<Vec<String>>>) -> Result<Self, String> {
        let pod_size = cards.len();
        if pod_size == 0 {
            return Err("a draft table needs at least one seat".to_string());
        }
        let pack_size = cards[0].first().map_or(0, Vec::len);
        if pack_size == 0 {
            return Err("a draft table needs packs with cards in them".to_string());
        }
        for (seat, seat_packs) in cards.iter().enumerate() {
            if seat_packs.len() != ROUNDS {
                return Err(format!(
                    "seat {seat} has {} packs; a draft is {ROUNDS} a seat", seat_packs.len()));
            }
            for (round, pack) in seat_packs.iter().enumerate() {
                if pack.len() != pack_size {
                    return Err(format!(
                        "seat {seat}'s round {} pack has {} cards and seat 0's round 1 pack has \
                         {pack_size}; every pack has to be the same size", round + 1, pack.len()));
                }
            }
        }
        let mut table = Self {
            pod_size,
            pack_size,
            packs: Vec::with_capacity(pod_size * ROUNDS),
            queues: vec![VecDeque::new(); pod_size],
            picks: vec![Vec::new(); pod_size],
            pools: vec![Vec::new(); pod_size],
            round: 0,
            original: cards,
        };
        table.deal(0);
        Ok(table)
    }

    /// Put round `round`'s packs in front of every seat.
    fn deal(&mut self, round: usize) {
        for seat in 0..self.pod_size {
            let cards = self.original[seat][round].clone();
            let id = self.packs.len();
            self.packs.push(Pack { round, size: cards.len(), cards });
            self.queues[seat].push_back(id);
        }
    }

    #[must_use]
    pub fn pod_size(&self) -> usize {
        self.pod_size
    }

    /// Cards per pack.
    #[must_use]
    pub fn pack_size(&self) -> usize {
        self.pack_size
    }

    /// The round in progress, 1-based, or `None` once the draft is over.
    #[must_use]
    pub fn round(&self) -> Option<usize> {
        (self.round < ROUNDS).then_some(self.round + 1)
    }

    /// Which way the round in progress passes, or `None` once the draft is
    /// over.
    #[must_use]
    pub fn pass_direction(&self) -> Option<Direction> {
        (self.round < ROUNDS).then(|| Direction::for_round(self.round))
    }

    /// Every pack a seat was dealt, `[round]`, as dealt — for the log.
    #[must_use]
    pub fn dealt(&self, seat: usize) -> &[Vec<String>] {
        &self.original[seat]
    }

    /// The pack in front of `seat`, or `None` when it has nothing to pick
    /// from right now.
    #[must_use]
    pub fn in_front(&self, seat: usize) -> Option<PackInFront<'_>> {
        let queue = self.queues.get(seat)?;
        let id = *queue.front()?;
        let pack = &self.packs[id];
        Some(PackInFront {
            id,
            round: pack.round + 1,
            pick: pack.size - pack.cards.len() + 1,
            size: pack.size,
            cards: &pack.cards,
            waiting: queue.len() - 1,
        })
    }

    /// Packs queued in front of `seat`, the one it is looking at included.
    #[must_use]
    pub fn queued(&self, seat: usize) -> usize {
        self.queues.get(seat).map_or(0, VecDeque::len)
    }

    #[must_use]
    pub fn pool(&self, seat: usize) -> &[String] {
        &self.pools[seat]
    }

    #[must_use]
    pub fn picks(&self, seat: usize) -> &[TablePick] {
        &self.picks[seat]
    }

    /// Whether `seat` has made every pick it will make: its draft is over
    /// even if the table's is not, and it can go and build its deck.
    #[must_use]
    pub fn seat_done(&self, seat: usize) -> bool {
        self.picks[seat].len() >= self.pack_size * ROUNDS
    }

    /// Every seat is done.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.round >= ROUNDS
    }

    /// Take card `index` of pack `pack_id` for `seat`. The pack has to be
    /// the one in front of the seat — a pick names the pack so that an
    /// answer to a pack that has since been auto-picked and passed is
    /// refused rather than landing on the next one.
    ///
    /// # Errors
    /// No pack in front of the seat, `pack_id` is not the one in front, or
    /// `index` is past the pack's end.
    pub fn pick(&mut self, seat: usize, pack_id: usize, index: usize, auto: bool) -> Result<Picked, String> {
        if seat >= self.pod_size {
            return Err(format!("there is no seat {seat} at a {}-seat table", self.pod_size));
        }
        let Some(&head) = self.queues[seat].front() else {
            return Err(format!("seat {seat} has no pack in front of it"));
        };
        if head != pack_id {
            return Err(format!(
                "pack {pack_id} is not the pack in front of seat {seat} (pack {head} is)"));
        }
        let pack = &mut self.packs[head];
        if index >= pack.cards.len() {
            return Err(format!(
                "card {index} is past the end of the pack ({} cards left)", pack.cards.len()));
        }
        let available = pack.cards.clone();
        let card = pack.cards.remove(index);
        let round = pack.round;
        let pick_number = pack.size - pack.cards.len();
        let emptied = pack.cards.is_empty();

        self.queues[seat].pop_front();
        let passed_to = if emptied {
            None
        } else {
            let to = neighbour(seat, self.pod_size, Direction::for_round(round));
            self.queues[to].push_back(head);
            Some(to)
        };

        self.picks[seat].push(TablePick {
            round: round + 1,
            pick: pick_number,
            card: card.clone(),
            auto,
            available,
        });
        self.pools[seat].push(card.clone());

        let round_over = self.packs.iter().filter(|p| p.round == self.round).all(|p| p.cards.is_empty());
        let mut draft_over = false;
        if round_over {
            self.round += 1;
            if self.round < ROUNDS {
                self.deal(self.round);
            } else {
                draft_over = true;
            }
        }

        Ok(Picked { card, round: round + 1, pick: pick_number, passed_to, round_over, draft_over })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pod of `pod` seats whose packs hold `size` distinct, traceable
    /// names: `s<seat>r<round>c<card>`.
    fn table(pod: usize, size: usize) -> Table {
        let cards = (0..pod)
            .map(|seat| {
                (0..ROUNDS)
                    .map(|round| (0..size).map(|c| format!("s{seat}r{round}c{c}")).collect())
                    .collect()
            })
            .collect();
        Table::from_cards(cards).unwrap()
    }

    fn pick_first(t: &mut Table, seat: usize) -> Picked {
        let id = t.in_front(seat).expect("a pack in front").id;
        t.pick(seat, id, 0, false).unwrap()
    }

    #[test]
    fn round_one_passes_left_round_two_right_round_three_left() {
        let mut t = table(4, 2);
        assert_eq!(t.pass_direction(), Some(Direction::Left));
        let p = pick_first(&mut t, 0);
        assert_eq!(p.passed_to, Some(1));
        assert_eq!(t.in_front(1).unwrap().waiting, 1, "seat 1 has its own pack and seat 0's");
        // Finish round 1: everybody picks until nothing is left.
        for _ in 0..2 {
            for seat in 0..4 {
                while t.in_front(seat).is_some() && t.round() == Some(1) {
                    pick_first(&mut t, seat);
                }
            }
        }
        assert_eq!(t.round(), Some(2));
        assert_eq!(t.pass_direction(), Some(Direction::Right));
        let p = pick_first(&mut t, 0);
        assert_eq!(p.passed_to, Some(3), "round 2 passes right");
        assert_eq!(Direction::for_round(2), Direction::Left);
    }

    #[test]
    fn the_last_card_of_a_pack_passes_nowhere() {
        let mut t = table(2, 1);
        let p = pick_first(&mut t, 0);
        assert_eq!(p.passed_to, None);
        assert!(!p.round_over, "seat 1's pack is still full");
        assert!(t.in_front(0).is_none());
        let p = pick_first(&mut t, 1);
        assert!(p.round_over);
        assert_eq!(t.round(), Some(2), "and the next round is dealt");
        assert!(t.in_front(0).is_some());
    }

    #[test]
    fn a_slow_seat_queues_packs_and_sees_how_many() {
        let mut t = table(3, 3);
        // Seat 0 and seat 2 both pick; seat 1 does nothing. Seat 0's pack
        // reaches seat 1 (left), and seat 2's goes to seat 0.
        pick_first(&mut t, 0);
        pick_first(&mut t, 2);
        let front = t.in_front(1).unwrap();
        assert_eq!(front.id, 1, "its own pack first");
        assert_eq!(front.waiting, 1);
        assert_eq!(t.queued(1), 2);
        // Seat 0 now has seat 2's pack and can keep going.
        let front = t.in_front(0).unwrap();
        assert_eq!(front.id, 2);
        assert_eq!(front.pick, 2, "the second pick from that pack");
        assert_eq!(front.cards, &["s2r0c1", "s2r0c2"]);
    }

    #[test]
    fn a_pick_is_numbered_by_the_cards_left_in_its_pack() {
        let mut t = table(2, 3);
        let p = pick_first(&mut t, 0);
        assert_eq!((p.round, p.pick), (1, 1));
        // Seat 1 picks its own pack first (pick 1), then seat 0's (pick 2).
        let p = pick_first(&mut t, 1);
        assert_eq!(p.pick, 1);
        let front = t.in_front(1).unwrap();
        assert_eq!(front.pick, 2);
        assert_eq!(front.size, 3);
        let p = pick_first(&mut t, 1);
        assert_eq!(p.pick, 2);
        assert_eq!(t.picks(1)[1], TablePick {
            round: 1, pick: 2, card: "s0r0c1".into(), auto: false,
            available: vec!["s0r0c1".into(), "s0r0c2".into()],
        });
    }

    #[test]
    fn a_pick_names_its_pack_and_is_refused_for_any_other() {
        let mut t = table(2, 2);
        assert!(t.pick(0, 1, 0, false).unwrap_err().contains("not the pack in front of seat 0"));
        assert!(t.pick(0, 0, 2, false).unwrap_err().contains("past the end"));
        assert!(t.pick(5, 0, 0, false).unwrap_err().contains("no seat 5"));
        pick_first(&mut t, 0);
        assert!(t.pick(0, 0, 0, false).unwrap_err().contains("no pack in front"));
    }

    #[test]
    fn a_seat_is_done_when_it_has_picked_three_packs_worth_and_the_table_when_all_are() {
        let mut t = table(2, 2);
        let mut last = None;
        // Seat 0 races ahead each round; seat 1 trails. Whoever has a pack
        // picks, seat 0 first.
        while !t.is_complete() {
            let seat = (0..2).find(|&s| t.in_front(s).is_some()).expect("somebody can pick");
            last = Some(pick_first(&mut t, seat));
        }
        assert!(last.unwrap().draft_over);
        assert!(t.seat_done(0) && t.seat_done(1));
        assert_eq!(t.round(), None);
        assert_eq!(t.pass_direction(), None);
        assert_eq!(t.pool(0).len(), 6);
        assert_eq!(t.pool(1).len(), 6);
        assert_eq!(t.picks(0).len(), 6);
        assert!(t.in_front(0).is_none());
    }

    #[test]
    fn a_seat_can_finish_before_the_table_does() {
        let mut t = table(2, 2);
        // Round 1 and 2 fully.
        while t.round() != Some(3) {
            let seat = (0..2).find(|&s| t.in_front(s).is_some()).unwrap();
            pick_first(&mut t, seat);
        }
        // Round 3: seat 0 picks from its pack and seat 1's as they come;
        // seat 1 picks only once, so seat 0 gets both second picks.
        pick_first(&mut t, 0); // s0 pack -> seat 1
        pick_first(&mut t, 1); // s1 pack -> seat 0 (its own pack first)
        pick_first(&mut t, 0); // last of s1 pack
        assert!(t.seat_done(0), "seat 0 has made its six picks");
        assert!(!t.seat_done(1));
        assert!(!t.is_complete());
        let p = pick_first(&mut t, 1);
        assert!(p.draft_over);
        assert!(t.is_complete());
    }

    #[test]
    fn a_one_seat_pod_passes_to_itself() {
        let mut t = table(1, 2);
        let p = pick_first(&mut t, 0);
        assert_eq!(p.passed_to, Some(0));
        assert_eq!(t.in_front(0).unwrap().pick, 2);
    }

    #[test]
    fn unequal_packs_are_refused() {
        let cards = vec![vec![vec!["a".to_string()], vec!["b".into()], vec!["c".into(), "d".into()]]];
        assert!(Table::from_cards(cards).unwrap_err().contains("same size"));
        assert!(Table::from_cards(vec![]).is_err());
        assert!(Table::from_cards(vec![vec![vec![], vec![], vec![]]]).is_err());
        assert!(Table::from_cards(vec![vec![vec!["a".into()]]]).unwrap_err().contains("3 a seat"));
    }

    #[test]
    fn the_async_table_and_the_lockstep_draft_agree() {
        // Picking in lockstep order through the table reproduces the
        // lockstep draft's pools exactly: same rotation, same numbering.
        let cards: Vec<Vec<Vec<String>>> = (0..4)
            .map(|seat| (0..ROUNDS).map(|r| (0..3).map(|c| format!("s{seat}r{r}c{c}")).collect()).collect())
            .collect();
        let packs: Vec<Vec<BoosterPack>> = cards.iter().map(|seat| seat.iter().map(|p| BoosterPack {
            commons: p[..1].to_vec(), uncommons: vec![], rare: p[1].clone(), dfc: p[2].clone(), foil: None,
        }).collect()).collect();
        let mut lock = crate::draft::DraftState::new(&packs);
        let mut t = Table::new(&packs).unwrap();
        for round in 0..ROUNDS {
            if round > 0 {
                lock.start_next_pack_round();
            }
            for _ in 0..3 {
                for seat in 0..4 {
                    let card = lock.current_pack_for(seat)[0].clone();
                    lock.make_pick(seat, &card).unwrap();
                    let front = t.in_front(seat).unwrap();
                    assert_eq!(front.cards[0], card);
                    t.pick(seat, front.id, 0, false).unwrap();
                }
                lock.rotate_packs();
            }
        }
        for seat in 0..4 {
            assert_eq!(t.pool(seat), lock.players[seat].pool.as_slice());
            for (a, b) in t.picks(seat).iter().zip(&lock.players[seat].picks) {
                assert_eq!((a.round, a.pick), (b.pack_number, b.pick_number));
                assert_eq!(a.available, b.available);
            }
        }
    }
}
