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
| 2026-08-30 | Competitor | C2 control mirror | wu-coverage vs ub-coverage | 2 | split 1-1, t22/t24 | none |
| 2026-08-30 | Competitor | C3 attrition | br-coverage vs bg-coverage | 2 | bg-coverage won both, t16/t16 | none |
| 2026-08-30 | Competitor | C6 equipment voltron vs token swarm | wr-coverage vs wg-coverage (sub for gw-humans) | 2 | wg-coverage won both, t22/t24 | none |
| 2026-08-30 | Competitor | C8 transform tempo | rg-coverage vs wr-coverage | 2 | split 1-1, t13/t16; werewolf flip logic verified correct | none |
| 2026-08-30 | Competitor | C9 aristocrats/sac vs tokens | ub-coverage vs wg-coverage (sub for ub-zombies/gw-humans) | 2 | ub-coverage won both, t15/t17, close races | #45, #48 |
| 2026-08-30 | Rules Lawyer | L2 targeting edges | ur-coverage vs wb-coverage | 2 | illegal graveyard target offered, found | #46 |
| 2026-08-30 | Rules Lawyer | L3 optional everything | wu-coverage vs bg-coverage | 2 | every "may" verified genuinely optional | none |
| 2026-08-30 | Rules Lawyer | L6 copy/DFC | rg-coverage vs ug-coverage | 2 | DFC transform object-identity verified; no copy effects in this pairing | none |
| 2026-08-30 | Rules Lawyer | L9 replacement effects | br-coverage vs wg-coverage | 2 | no CR 616 dual-replacement case exists in this pairing; source review confirmed correct ordering elsewhere | none |
| 2026-08-30 | Rules Lawyer | L10 mana ability edges | ub-coverage vs rg-coverage | 2 | mana abilities confirmed to bypass the stack correctly | none |
| 2026-08-30 | Vandal | V5 stall | ug-spider-spawning vs bg-coverage | 2 | both games deck-out at t64, ended cleanly | none |
| 2026-08-30 | Vandal | V6 concede at weirdest moment | rb-vampires vs wb-coverage | 2 (+1 short) | all concede paths safe; found game-over screen render bug | #47 |
| 2026-08-30 | Vandal | V8 search/menu abuse | wu-coverage vs ur-coverage | 2 | all panes handled cleanly at every prompt, no corruption | none |
| 2026-08-30 | Vandal | V9 rapid concede/new-game churn | gw-humans vs rb-vampires | ~12 rapid cycles | clean state on every relaunch; #37 fix holds under churn | none |
| 2026-08-30 | Vandal | V10 priority-mash marathon | ub-zombies vs ug-spider-spawning | 2 (of 3 launched) | mashed to t108, no hangs/skips/double-resolution | none |
| 2026-08-31 | Competitor | C10 mulligan-to-five resource grind | wu-coverage vs br-coverage | 2 | br-coverage won both, t14/t14; London mulligan + bottoming counts verified correct | #58, #56, #60, #61 |
| 2026-08-31 | Competitor | C11 lifegain vs burn race | wb-coverage vs rg-coverage | 2 | rg-coverage won both, t20/t14; deliberate exact-lethal to 0 verified (CR 704.5a) | #59 |
| 2026-08-31 | Competitor | C12 mill race / winning by decking | ub-coverage vs gw-humans | 2 | gw-humans won both, t10/t15; **decking rule NOT exercised** — ub-coverage has no real mill, needs a new pairing | none |
| 2026-08-31 | Competitor | C13 flyers vs ground stall | wu-coverage vs bg-coverage | 2 | bg-coverage won both, t16/t10; blocker eligibility correct in every combat | none |
| 2026-08-31 | Competitor | C14 topdeck war | rb-vampires vs ur-coverage | 2 | rb-vampires won both, t11/t15; draw/hand-size/discard accounting all correct | none |
| 2026-08-31 | Rules Lawyer | L11 layers (CR 613) | gw-humans vs wg-coverage | 2 | gw-humans won both, t24/t30; layer order and recompute-on-removal correct | #57, #56 |
| 2026-08-31 | Rules Lawyer | L12 attack/block requirements vs restrictions | rb-vampires vs wb-coverage | 2 | 1-1 split; per-pair block legality correct, blocker-count rule missing | #62, #65, #66 |
| 2026-08-31 | Rules Lawyer | L13 LTB / exile-and-return ordering | wu-coverage vs ub-zombies | 2 (g1 ended in an engine panic, t34) | g2 ub-zombies won ~t90; CR 400.7 new-object rule verified correct | #64, comment on #60 |
| 2026-08-31 | Rules Lawyer | L14 timing and priority enforcement | ur-coverage vs bg-coverage | 2 | bg-coverage won both; ~350 prompts probed, came back completely clean | none |
| 2026-08-31 | Rules Lawyer | L15 attachment legality and SBAs | wr-coverage vs ug-coverage | 2 | 1-1 split; CR 704.5m (aura to owner's gy) vs 704.5n (equipment unattaches) correct | none |
| 2026-08-31 | Vandal | V11 terminal resize storm | rg-coverage vs wb-coverage | 2 | wb-coverage won both, t26/t18; no panic/hang/misrouted input at any size 20x5–300x100 | #53, #49 |
| 2026-08-31 | Vandal | V12 control-char / ANSI injection | br-coverage vs wg-coverage | 2 | br-coverage won both, t21/t15; Ctrl chords leak as plain text and menu digits | #51 |
| 2026-08-31 | Vandal | V13 paste-flood | br-coverage vs wg-coverage | 2 | 1-1 split, t25/t16; one paste executed 11 turns of real actions across both seats | #50 |
| 2026-08-31 | Vandal | V14 save/resume corruption abuse | gw-humans vs ur-coverage | 2 | gw-humans won both, t9/t11; every bad save panics, honest resume correct | #52 |
| 2026-08-31 | Vandal | V15 mulligan-phase abuse | ug-spider-spawning vs bg-coverage | 2 | bg-coverage won both, t14/t14; cap real and counter honest | #54, #63 |
| 2026-09-01 | Competitor | C12 mill race (re-probe) | ub-zombies vs wg-coverage | 2 | ub-zombies decked p1 both games, t22/t20; **decking rule now genuinely exercised** (CR 704.5b verified) | #86 |
| 2026-09-01 | Competitor | C15 mana pool and land-drop accounting | rg-coverage vs ur-coverage | 2 | 1-1 split, t18/t21; pool emptying, no mana burn, land-drop gating all correct | #72, #90 |
| 2026-09-01 | Competitor | C16 combat priority windows | wr-coverage vs bg-coverage | 2 | wr-coverage won both, t15/t17; all five combat windows exercised; CR 509.1h correct | #88, #89 |
| 2026-09-01 | Competitor | C17 activated-ability value engines | wu-coverage vs ub-coverage | 2 | wu-coverage won both, t13/t15; costs, stack use and counters all correct | #82, #83, #84 |
| 2026-09-01 | Competitor | C18 sweeper vs go-wide | wu-coverage vs ug-spider-spawning | 2 | ug-spider-spawning won both, t18/t22; 5 Divine Reckoning resolutions, APNAP + simultaneity + tokens all correct | #94, #95 |
| 2026-09-01 | Rules Lawyer | L16 copy effects (CR 706) | ub-coverage vs rg-coverage | 2 | 1-1 split, t35/t36; **two real copy-rules bugs**; CR 706.2 copiable values otherwise correct | #74, #85, #87, #93, comment on #80 |
| 2026-09-01 | Rules Lawyer | L17 morbid | br-coverage vs wg-coverage | 2 | br-coverage won both, t19/t47; morbid correct on all 10 checks (resolution timing, tokens, exile/discard exclusions, per-turn reset) | #96, comments on #80, #95 |
| 2026-09-01 | Rules Lawyer | L18 token existence + doublers | ug-spider-spawning vs wg-coverage | 2 | 1-1 split, t19/t30; CR 111.7 and Parallel Lives 2N-once-per-event correct | #91, #92 |
| 2026-09-01 | Rules Lawyer | L19 curse / enchant-player legality | wb-coverage vs ur-coverage | 2 | 1-1 split, t30/t31; every curse rule verified correct (targeting, self-curse, 7c layers, 508.1d, upkeep timing) | #81 |
| 2026-09-01 | Rules Lawyer | L20 evasion and blocking legality | wu-coverage vs rb-vampires | 2 (+1 abandoned) | rb-vampires won both, t18/t18; evasion/flying/hexproof/must-attack all correct. **Invisible Stalker + Blazing Torch never drawn — those two items still unverified** | comment on #80 |
| 2026-09-01 | Vandal | V16 deck-file abuse | 15 malformed decks vs rg-coverage | 15 runs (3 full games) | completely clean: every malformed deck a clear error with exit 1, no panics | none |
| 2026-09-01 | Vandal | V17 CLI flag abuse | rg-coverage vs wb-coverage | ~20 runs (1 hotseat) | seeds/flags/resume all clean; --log and --save path handling is not | #69, #70 |
| 2026-09-01 | Vandal | V18 EOF, signals, terminal detach | wr-coverage vs ub-coverage | 2 (+12 kill-tests) | Ctrl-D/detach/resize all safe; signal death and --log are not | #77, #78, #79, comment on #73 |
| 2026-09-01 | Vandal | V19 type-ahead race | gw-humans vs br-coverage | 2 (+6 controls) | br-coverage won both, t18/t16; **type-ahead crosses seats and takes irreversible actions** | #71, #73, #80, comments on #71 |
| 2026-09-01 | Vandal | V20 concurrent save contention | rg-coverage vs wb-coverage | 2 (+load runs) | wb-coverage won g1 t22; saves are non-atomic, but no corrupt save ever loaded | #75, #76, comment on #69 |
