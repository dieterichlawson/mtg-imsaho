# Audit: Gnaw to the Bone

## Oracle Reference (Scryfall)
- Cost: {2}{G}
- Type: Instant
- Oracle: "You gain 2 life for each creature card in your graveyard.
  Flashback {3}{G}"

## Implementation: gnaw_to_the_bone.rs

## Issues Found

No issues found. Name, cost ({2}{G}), type (Instant), oracle text, flashback cost ({3}{G}), and effect (gain 2 life per creature card in graveyard) all match. The implementation correctly counts creature cards in the controller's graveyard, excluding the spell itself (still on stack).

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
You gain 2 life for each creature card in your graveyard.
Flashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

### Findings
- Name, cost ({2}{G}), type (Instant) all match.
- Life gain logic correctly counts creature cards in controller's graveyard (excluding self on stack) and gains 2 life per creature -- correct.

### ISSUE: Flashback cost mismatch
- **Oracle**: Flashback {2}{G}
- **Code**: `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Green)]))` which equals {3}{G}

The flashback cost should be Generic(2) + Green, not Generic(3) + Green. The previous audit incorrectly listed the oracle flashback as {3}{G} but Scryfall confirms it is {2}{G}.

### Verdict: ISSUE
Flashback cost is {3}{G} in code but oracle specifies {2}{G}.
