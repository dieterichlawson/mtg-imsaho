## Audit — 2026-04-01

**Scryfall Oracle text**: Put a +1/+1 counter on each of up to two target creatures.\nFlashback {1}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Scryfall type line**: Sorcery
**Status**: PASS

- Name: correct ("Travel Preparations")
- Cost: {1}{G} -- correct
- Type: Sorcery -- correct
- Oracle text: matches
- Flashback cost: {1}{W} -- correct
- Target: UpToTargets(2, Creature) -- correct (up to two target creatures)
- On resolve: puts a +1/+1 counter on each target that is still on the battlefield -- correct
- Tests exist in `flashback.rs`
- No issues found

## Audit — 2026-04-01

**Scryfall Oracle text**: Put a +1/+1 counter on each of up to two target creatures. Flashback {1}{W}
**Scryfall type line**: Sorcery
**Mana cost**: {1}{G}
**Status**: ISSUE

1. **LLM card knowledge inaccurate** (`mtg-player/src/llm.rs`, line 109): Says "Put a +1/+1 counter on target creature" but Oracle says "each of up to two target creatures."
