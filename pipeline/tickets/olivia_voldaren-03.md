---
id: olivia_voldaren-03
status: new
card: Olivia Voldaren
card_file: mtg-engine/src/cards/isd/olivia_voldaren.rs
created: 2026-04-14T20:44:31Z
audit_run_id: 2026-04-14-olivia_voldaren-audit
audit_model: opus
audit_tokens: 17927
audit_duration: 323
---

## Audit Finding

**Oracle text:**
> {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.

**Code:**
> `olivia_voldaren.rs:163-197` — `on_leave_battlefield` returns stolen creatures when Olivia leaves the battlefield.
> `olivia_voldaren.rs:32-37` — registers `TriggerKind::LeavesBattlefield` as the mechanism for returning creatures.

**Description:**
The "for as long as you control Olivia Voldaren" duration is implemented solely via a LeavesBattlefield triggered ability. This has two problems:

(1) Per CR 611.2b, a "for as long as" duration is continuously re-evaluated and ends immediately when the condition becomes false. If an opponent gains control of Olivia without her leaving the battlefield (e.g., Act of Treason, Zealous Conscripts, Olivia's own second ability from an opponent), the condition "you control Olivia" becomes false but the stolen creatures are NOT returned — the `on_leave_battlefield` hook never fires because Olivia is still on the battlefield.

(2) The LTB trigger goes on the stack (triggers.rs:664-674 adds to `ap_triggers`/`nap_triggers`), meaning opponents get priority between Olivia leaving and the stolen creatures being returned. Per CR 611.2b, the control effect should end simultaneously with the condition becoming false, not after a stack-based delay. During the window, the stolen creatures remain under the wrong player's control and could be sacrificed, activated, or otherwise used.

This pattern is documented in auditor-insights.md ("For as long as you control [source] requires continuous re-evaluation").

**Engine path:**
- `olivia_voldaren.rs:163-197` (LTB cleanup)
- `triggers.rs:653-675` (LTB dispatch — adds to stack)
- `triggers.rs:1327-1330` (LTB resolution calls `on_leave_battlefield`)

**Required check:** 8h

**Affected cards:**
- Olivia Voldaren
- Any card using `on_leave_battlefield` for "for as long as" durations

