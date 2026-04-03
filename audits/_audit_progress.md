# ISD Audit Progress

## Status
- **Card list**: alphabetical list of all .rs files in `mtg-engine/src/cards/isd/` (249 cards)
- **Batches completed**: 1-12 (cards 1-120)
- **Next batch**: 13 (cards 121-130)
- **To get card list**: `ls mtg-engine/src/cards/isd/*.rs | grep -v mod.rs | xargs -I{} basename {} .rs | sort`

## Running totals through batch 12
- **100 PASS, 20 ISSUE out of 120 cards (48% complete)**

## Issues found

| Card | Issue |
|------|-------|
| Back from the Brink | X-cost creatures should treat X as 0 per ruling |
| Brain Weevil | Only discards 1 instead of 2 when hand has 3+ cards (missing on_discard_choice chain) |
| Burning Vengeance | Trigger too narrow — checks cast_with_flashback only, not all graveyard casts; engine SpellCast filter excludes non-instant/sorcery |
| Creepy Doll | Cosmetic oracle text mismatch ("Creepy Doll" vs "this creature") |
| Curse of the Pierced Heart | PendingEffect::DealDamage doesn't remove planeswalker loyalty (engine bug) |
| Dearly Departed | Trigger system only scans battlefield watchers, but ability works from graveyard (engine bug) |
| Delver of Secrets | Manual transform doesn't use apply_transform(), subtypes not updated (Human Wizard instead of Human Insect) |
| Elder Cathar | LLM card knowledge omits Human bonus (minor) |
| Forbidden Alchemy | Revealed cards display as raw ObjectIds in CLI/LLM views; misleading LLM description |
| Geistflame | Oracle text field + LLM knowledge both missing flashback info |
| Ghost Quarter | Missing library shuffle after search |
| Grave Bramble | LLM knowledge omits protection from Zombies; engine doesn't check protection for targeting |
| Bonds of Faith | Snapshot evaluation instead of continuous "as long as" — Human check doesn't update on type changes |
| Darkthicket Wolf | abilities_activated_this_turn never cleared between turns (engine bug, reclassified from PASS) |
| Falkenrath Noble | Simultaneous death triggers only fire once instead of N (engine bug, reclassified from PASS) |
| Hinterland Harbor | Cosmetic oracle text mismatch (old templating) |
| Hollowhenge Scavenger | ETB trigger skipped when source leaves battlefield before resolution (engine bug) |
| Intangible Virtue | LLM knowledge said "your creatures" instead of "creature tokens", omitted vigilance |

## False PASSes (engine issues marked PASS before prompt was updated)
- **Darkthicket Wolf** — `abilities_activated_this_turn` never cleared between turns (once-per-turn permanently locked)
- **Falkenrath Noble** — simultaneous death triggers only fire once instead of N

These ran before the "engine limits are ISSUE" rule was added to the prompt. Need re-auditing.

## Engine bugs discovered
- `abilities_activated_this_turn` never cleared between turns (found via Darkthicket Wolf)
- `PendingEffect::DealDamage` doesn't handle planeswalker loyalty removal
- Trigger system only scans battlefield for watchers (misses graveyard abilities like Dearly Departed)
- Simultaneous death triggers — only one fires instead of N (found via Falkenrath Noble)

## How to continue
1. Read this file for context
2. Check which batches are done: `ls audits/ | wc -l` and cross-reference with `/tmp/isd_cards.txt`
3. Get the card list: `sed -n '{START},{END}p' /tmp/isd_cards.txt`
4. Get current time: `date "+%Y-%m-%d %H:%M"`
5. Launch 10 agents with the prompt from `.claude/commands/check-card.md`
6. After batch completes: `git add audits/ && git commit -m "Audit batch N: ..." && git push`
7. Update this file with results

## Prompt template (condensed version)
The full prompt is in `.claude/commands/check-card.md` — use the agent prompt section.
Key: include card name, current time, file path, critical rules (no training data, quote both sides, engine limits are ISSUE not PASS), procedure steps 1-10, exact output format.
