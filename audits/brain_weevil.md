# Audit: Brain Weevil

## Oracle Reference
- **Name:** Brain Weevil
- **Mana Cost:** {3}{B}
- **Type:** Creature — Insect
- **P/T:** 1/1
- **Oracle Text:** Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.) / Sacrifice this creature: Target player discards two cards. Activate only as a sorcery.
- **Keywords:** Intimidate

## Card Data Audit
- **Name:** Correct ("Brain Weevil")
- **Mana Cost:** Correct (Generic(3), Black)
- **Type:** Correct (Creature)
- **Subtypes:** Correct ("Insect")
- **P/T:** Correct (1/1)
- **Keywords:** Correct (Keyword::Intimidate)

## Behavior Audit
- **Intimidate:** Present in keywords vec; checked by engine `has_keyword` for blocking restrictions. Correct.
- **Activated ability:** Sacrifice cost via `SacrificeCost::SacrificeThis`, no mana cost, `sorcery_speed_only: true`, targets a player (`TargetRequirement::PlayerOnly`). All match oracle.
- **Discard two cards:** `on_activate_ability` discards up to 2 cards from target player's hand. If 2 or fewer cards in hand, discards all. If more, prompts for choice. Correct.

## Result: PASS
