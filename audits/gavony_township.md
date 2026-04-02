# Audit: Gavony Township

## Oracle Reference
- **Name:** Gavony Township
- **Type:** Land
- **Oracle Text:** {T}: Add {C}. / {2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.

## Card Data Audit
- **Name:** Correct ("Gavony Township")
- **Mana Cost:** Correct (None -- it is a land)
- **Type:** Correct (Land)

## Behavior Audit
- **Mana ability:** Produces 1 colorless mana, requires tap, only available when untapped on battlefield. Correct.
- **Activated ability:** Costs {2}{G}{W} and tap. Correct.
- **Counter placement:** `on_activate_ability` finds all creatures controlled by the activator (filtered by `o.power.is_some()`) and adds 1 PlusOnePlusOne counter to each. Correct.
- **No sorcery speed restriction:** `sorcery_speed_only: false`. Correct -- the oracle text has no timing restriction beyond the tap cost.

## Result: PASS
