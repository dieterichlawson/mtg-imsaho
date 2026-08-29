# Playtest ledger

Append-only record of every playtest mission, so nights don't repeat each
other. One row per mission per night; details in `reports/playtests/
YYYY-MM-DD.md`. Missions are defined in `docs/plans/playtest-missions.md`;
issues are filed per `docs/plans/bug-pipeline.md` (`phase:playtest`).

| Date | Persona | Mission | Decks | Games | Outcome | Issues filed |
|---|---|---|---|---|---|---|
| 2026-08-29 | Competitor | C1 aggro mirror | rb-vampires vs br-coverage | 2 | g1 bug found mid-game; g2 went to t22, br-coverage won | #36 |
| 2026-08-29 | Competitor | C4 tribal synergy | gw-humans vs rb-vampires | 2 | tribal races played out; `f` auto-pass bug found | #39 |
| 2026-08-29 | Competitor | C4 tribal synergy | ub-zombies vs gw-humans | 2 | anthem/SBA ordering edge case; concede-prompt bug | #41, #42, comment on #38 |
| 2026-08-29 | Competitor | C5 planeswalker-centric | ug-coverage vs br-coverage | 2 | ug-coverage won both (g2 via reanimation, t25) | #43, comments on #36, #40 |
| 2026-08-29 | Competitor | C7 curses | wb-coverage vs br-coverage | 2 | 1-1 split; only 1 curse landed per game | comments on #36, #39, #38 |
| 2026-08-29 | Rules Lawyer | L1 stack battles | wu-coverage vs ur-coverage | 2 | deep stacks, trigger ordering, optional triggers all correct | none |
| 2026-08-29 | Rules Lawyer | L4 combat rules | gw-humans vs rb-vampires | 2 | block restrictions and trigger order verified correct | comment on #35 |
| 2026-08-29 | Rules Lawyer | L5 cost edges | ub-zombies vs ug-spider-spawning | 2 | flashback/additional costs correct; no true X-spell in this pairing | none |
| 2026-08-29 | Rules Lawyer | L7 zone identity | br-coverage vs ub-zombies | 2 | confirmed new-object rule on recast (counters/damage cleared) | none |
| 2026-08-29 | Rules Lawyer | L8 SBA order | rg-coverage vs bg-coverage | 2 | simultaneous deaths and fight resolution correct | none |
| 2026-08-29 | Vandal | V1 input garbage | wr-coverage vs wg-coverage | 2 | most garbage input rejected cleanly; stale-buffer bug found | #35 |
| 2026-08-29 | Vandal | V2 the wrong number | ub-coverage vs bg-coverage | 2 | all out-of-range menu inputs rejected cleanly | none |
| 2026-08-29 | Vandal | V3 save/reload abuse | rb-vampires vs gw-humans | 2 | resume correct across mid-combat/mid-choice; `rr` hot-reload bug found | #37 |
| 2026-08-29 | Vandal | V4 degenerate decks | custom all-curses vs ub-zombies | 2 | no crashes under simultaneous curses / library depletion at scale | none |
| 2026-08-29 | Vandal | V7 UI overflow | ug-spider-spawning vs wb-coverage | 2 | rendering held up at scale; search-library picker bug found | #38 |
