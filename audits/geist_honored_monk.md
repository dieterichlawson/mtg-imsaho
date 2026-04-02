# Audit: Geist-Honored Monk

## Oracle Reference
- **Name:** Geist-Honored Monk
- **Mana Cost:** {3}{W}{W}
- **Type:** Creature — Human Monk
- **P/T:** */*
- **Oracle Text:** Vigilance / Geist-Honored Monk's power and toughness are each equal to the number of creatures you control. / When this creature enters, create two 1/1 white Spirit creature tokens with flying.

## Card Data Audit
- **Name:** Correct ("Geist-Honored Monk")
- **Mana Cost:** Correct (Generic(3), White, White)
- **Type:** Correct (Creature)
- **Subtypes:** Correct ("Human", "Monk")
- **Base P/T:** Set to (0, 0) with dynamic_pt override. Acceptable for */* implementation.
- **Keywords:** Correct (Keyword::Vigilance)

## Behavior Audit
- **Dynamic P/T:** `dynamic_pt` counts all creatures on the battlefield controlled by the same player (including itself). Matches oracle "equal to the number of creatures you control." Correct.
- **ETB trigger:** `on_enter_battlefield` creates two 1/1 white Spirit tokens with flying via `create_token_with_subtypes`. Correct.
- **Token details:** Color White, type Creature, keyword Flying, subtype "Spirit", P/T 1/1. All correct.
- **Oracle text wording:** Code uses "enters the battlefield" while current oracle uses "When this creature enters". This is a cosmetic oracle text string difference only; behavior is correct.

## Result: PASS
