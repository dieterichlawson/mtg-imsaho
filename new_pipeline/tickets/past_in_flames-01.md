---
id: past_in_flames-01
status: fixed
card: Past in Flames
audit_run_id: 2026-04-19-past_in_flames-audit
audit_model: sonnet
audit_tokens: 47972
audit_duration: 925
fixed_sha: 8f754da4380b632f90aa42b773f2c5f872a1fa27
fixed_at: 2026-08-23T23:10:08Z
test_file: mtg-engine/tests/flashback_multiple_instances.rs
fix_note: cluster fix: every available flashback cost is offered as its own castable option (CR 702.33); no-mana-cost cards no longer get a free one (702.33a)
---

## Audit Finding

**Oracle text:**
> If a card has multiple instances of flashback, you may choose any of its flashback costs to pay.

**Code:**
> let fb_cost = match dynamic_fb {
    Some(ref c) => c,
    None => match &data.flashback_cost {
        Some(c) => c,
        None => if cast_from_gy { ... } else { continue; },
    },
};

**Description:**
When Past in Flames grants dynamic flashback to a card that already has a printed flashback cost, the legal action generator picks `dynamic_fb` first and never falls through to `data.flashback_cost`. Only ONE CastSpell action is generated per graveyard card, using whichever flashback source wins the match. The printed flashback cost is silently discarded. Per the ruling, both costs must be offered as separate choices. The failure is not just a missing choice — in cross-color situations it causes a complete loss of access to an affordable option. For example: Bump in the Night (mana cost {B}, printed flashback {5}{R}) in the graveyard after Past in Flames resolves. Past in Flames adds GrantFlashback { cost: {B} }. If the player's mana pool has {5}{R} available but no black source, the engine uses dynamic_fb = {B} as the sole fb_cost, finds it unaffordable, and never offers the {5}{R} printed flashback that the player COULD pay. The player loses access to a legally castable option. Affected cards with printed flashback that would compete with dynamic costs include: Think Twice ({1}{U} mana cost vs {2}{U} flashback), Dream Twist ({U} vs {1}{U}), Silent Departure ({U} vs {4}{U}), Bump in the Night ({B} vs {5}{R}), Desperate Ravings ({1}{R} vs {2}{U}), Devil's Play ({X}{R} vs {X}{R}{R}{R}), Geistflame ({R} vs {3}{R}), and many others.

**Engine path:** mtg-engine/src/engine.rs:1231

**Required check:** 8j

**Affected cards:**
- Think Twice
- Dream Twist
- Silent Departure
- Grasp of Phantoms
- Desperate Ravings
- Forbidden Alchemy
- Feeling of Dread
- Bump in the Night
- Geistflame
- Rolling Temblor
- Nightbird's Clutches
- Rally the Peasants
- Travel Preparations
- Ancient Grudge
- Gnaw to the Bone
- Spider Spawning
- Purify the Grave
- Memory's Journey
- Unburial Rites
- Divine Reckoning
- Cackling Counterpart
- Army of the Damned
- Sever the Bloodline
- Creeping Renaissance
- Moan of the Unhallowed
- Devil's Play
- Past in Flames

## Tests

### past_in_flames_cross_color_flashback_still_available
Scenario: Past in Flames resolves with Bump in the Night in graveyard; player then has only red/colorless mana — the printed flashback {5}{R} should be a legal cast action even though the dynamic {B} is not affordable

### past_in_flames_both_flashback_costs_offered
Scenario: Past in Flames resolves with Think Twice in graveyard; the player should see two distinct flashback cast options: {1}{U} (mana cost / dynamic) and {2}{U} (printed flashback)

