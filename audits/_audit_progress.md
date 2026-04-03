# ISD Audit Progress

## Status
- **Card list**: `/tmp/isd_cards.txt` (249 cards, alphabetical)
- **Batches completed**: 1-8 (cards 1-80), batch 9 in progress (cards 81-90)
- **Next batch after 9**: batch 10 (cards 91-100)

## Running totals through batch 8
- **71 PASS, 9 ISSUE out of 80 cards**

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
