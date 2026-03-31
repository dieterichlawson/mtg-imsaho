# Audit: Sever the Bloodline

## Official Oracle
- **Name:** Sever the Bloodline
- **Cost:** {3}{B}
- **Type:** Sorcery
- **Oracle Text:** Exile target creature and all other creatures with the same name as that creature.\nFlashback {5}{B}{B}
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {3}{B} — OK
- **Type:** Sorcery — OK
- **Oracle Text:** "Exile target creature and all other creatures with the same name.\nFlashback {5}{B}{B}" — close match (official says "same name as that creature") — OK
- **Flashback Cost:** {5}{B}{B} — OK
- **P/T:** N/A — OK
- **Target:** TargetRequirement::Creature — OK
- **on_resolve:** Gets name of target, finds all creatures with same name, exiles all — OK

## Issues
None found.

## Verdict: PASS
