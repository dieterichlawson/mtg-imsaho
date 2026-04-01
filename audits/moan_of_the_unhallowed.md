## Audit — 2026-04-01

**Scryfall Oracle text**: Create two 2/2 black Zombie creature tokens.\nFlashback {5}{B}{B}
**Scryfall type line**: Sorcery
**Status**: PASS

- Name: Moan of the Unhallowed -- correct
- Cost: {2}{B}{B} -- correct
- Type: Sorcery -- correct
- Effect: creates two 2/2 black Zombie creature tokens -- correctly implemented
- Flashback: {5}{B}{B} -- correctly implemented
- Tokens have correct stats (2/2), color (black), type (Creature), subtype (Zombie) -- correct
- Tests: no dedicated test found, but implementation is straightforward

No issues found. Implementation matches Oracle text.

## Audit — 2026-04-01

**Scryfall Oracle text**: Create two 2/2 black Zombie creature tokens. Flashback {5}{B}{B}
**Scryfall type line**: Sorcery
**Status**: PASS

No issues found. Tokens created with correct subtypes. Flashback cost {5}{B}{B} is correct. Uses move_spell_after_resolve.
