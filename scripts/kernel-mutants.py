#!/usr/bin/env python3
"""Mutants of mtg-kernel, run against the proofs and against the tests.

Each mutant is a small deliberate bug in the code `mtg-kernel` proves: an
operator flipped, a floor ignored, a bound one short. For each, this reports
whether Verus rejects the mutated kernel and whether the engine's tests fail
(the tests that exercise payment, X funding and combat, and the whole
`mtg-engine` suite for any mutant those miss). It is how the comparison in
reports/verification-mutants.md was made, after the post on LLM-driven
verification that injected seven bugs into a verified interpreter: five
survived the tests, all seven broke the proof.

cargo-mutants cannot do this: it does not mutate inside macro bodies, and
everything in mtg-kernel is inside `verus!`. So the mutants are listed here
by hand, one or more per executable statement.

Usage: scripts/kernel-mutants.py [PREFIX...]  only the mutants whose id
                                              starts with a PREFIX (M, C, F:
                                              payment, combat, funding; or
                                              F09), or all
        (needs Verus on PATH; scripts/prove.sh says how to install it)

Runs in a scratch git worktree of HEAD, so the working tree is never
touched; results go to logs/kernel-mutants/<timestamp>/results.jsonl. A
mutant whose site no longer matches the source is reported as stale rather
than run: update its text, or drop it if the code it targeted is gone.
"""
import datetime, json, os, re, subprocess, sys, tempfile, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# (id, file, old, new, what): `old` must occur exactly once in `file`.
M = [
 ("M01","mtg-kernel/src/mana.rs","    let mut remaining = cost.generic;\n    spend_generic(&mut left, &mut remaining, &FIXED_ORDER, &NO_FLOOR);","    let mut remaining = 0;\n    spend_generic(&mut left, &mut remaining, &FIXED_ORDER, &NO_FLOOR);","pay_fixed ignores the generic part"),
 ("M02","mtg-kernel/src/mana.rs","    spend_generic(&mut left, &mut remaining, &FIXED_ORDER, &NO_FLOOR);\n    if remaining == 0 {","    spend_generic(&mut left, &mut remaining, &FIXED_ORDER, &NO_FLOOR);\n    if remaining <= 1 {","pay_fixed accepts being one short"),
 ("M03","mtg-kernel/src/mana.rs","    let order = surplus_order(&left, keep);","    let order = FIXED_ORDER;","pay_around spends spare in slot order, not surplus order"),
 ("M04","mtg-kernel/src/mana.rs","    spend_generic(&mut left, &mut remaining, &order, keep);","    spend_generic(&mut left, &mut remaining, &order, &NO_FLOOR);","pay_around takes all of a type first (defect 2)"),
 ("M05","mtg-kernel/src/mana.rs","    spend_generic(&mut left, &mut remaining, &order, &NO_FLOOR);","    spend_generic(&mut left, &mut remaining, &order, keep);","pay_around never digs into the reserve"),
 ("M06","mtg-kernel/src/mana.rs","    spend_generic(&mut left, &mut remaining, &order, &NO_FLOOR);\n    proof {","    spend_generic(&mut left, &mut remaining, &order, &NO_FLOOR);\n    remaining = remaining.saturating_sub(1);\n    proof {","pay_around forgives one point"),
 ("M07","mtg-kernel/src/mana.rs","surplus(have, keep, order[j - 1]) < surplus(have, keep, order[j])","surplus(have, keep, order[j - 1]) <= surplus(have, keep, order[j])","surplus_order is not stable"),
 ("M08","mtg-kernel/src/mana.rs","surplus(have, keep, order[j - 1]) < surplus(have, keep, order[j])","surplus(have, keep, order[j - 1]) > surplus(have, keep, order[j])","surplus_order sorts smallest surplus first"),
 ("M09","mtg-kernel/src/mana.rs","            order[j] = earlier;","            order[j] = later + 0 * earlier;","surplus_order's swap duplicates a slot"),
 ("M10","mtg-kernel/src/mana.rs","    i64::from(have[slot]) - i64::from(keep[slot])","    i64::from(have[slot]) + i64::from(keep[slot])","surplus adds the reserve instead of subtracting it"),
 ("M11","mtg-kernel/src/mana.rs","    let mut i: usize = 1;\n    while i < 6","    let mut i: usize = 1;\n    while i < 5","surplus_order never places the last slot"),
 ("M12","mtg-kernel/src/mana.rs","        if left[i] < cost.specific[i] {","        if left[i] <= cost.specific[i] {","a type with exactly its pips is called short"),
 ("M13","mtg-kernel/src/mana.rs","        left[i] -= cost.specific[i];","        left[i] -= cost.specific[i].min(1);","a type pays at most one of its pips"),
 ("M14","mtg-kernel/src/mana.rs","    let mut i: usize = 0;\n    while i < 6","    let mut i: usize = 0;\n    while i < 5","green pips are never paid"),
 ("M15","mtg-kernel/src/mana.rs","        let spare = have[slot].saturating_sub(floor[slot]);","        let spare = have[slot].saturating_sub(floor[slot] * 0);","spend_generic ignores the floor"),
 ("M16","mtg-kernel/src/mana.rs","        let take = spare.min(*remaining);","        let take = spare.min(*remaining).min(1);","spend_generic takes at most one of each type"),
 ("M17","mtg-kernel/src/mana.rs","    let mut k: usize = 0;\n    while k < 6","    let mut k: usize = 0;\n    while k < 5","spend_generic never reaches the last slot in order"),
 ("C01","mtg-kernel/src/combat.rs","        let least = remaining.min(lethal_here);","        let least = lethal_here;","least ignores how much is left"),
 ("C02","mtg-kernel/src/combat.rs","        let amount = if i + 1 == n && !trample {","        let amount = if i + 1 == n {","a trampler's last blocker takes everything"),
 ("C03","mtg-kernel/src/combat.rs","        let amount = if i + 1 == n && !trample {","        let amount = if i + 1 == n && trample {","only a trampler's last blocker takes the rest"),
 ("C04","mtg-kernel/src/combat.rs","        } else if remaining == least {","        } else if remaining == least || i == 0 {","the first blocker is never asked about"),
 ("C05","mtg-kernel/src/combat.rs","            c.clamp(least, remaining)","            c.min(remaining)","an answer below lethal is taken"),
 ("C06","mtg-kernel/src/combat.rs","            c.clamp(least, remaining)","            c.clamp(least, remaining).max(least).min(least)","the player's answer is ignored"),
 ("C07","mtg-kernel/src/combat.rs","            if open.is_none() {","            if open.is_none() || i > 0 {","a later blocker's question replaces the first"),
 ("C08","mtg-kernel/src/combat.rs","                open = Some((i, least, remaining));","                open = Some((i, least, remaining - 1));","the range stops one short of everything left"),
 ("C09","mtg-kernel/src/combat.rs","    let overflow = if trample { remaining } else { 0 };","    let overflow = remaining;","a non-trampler's leftover goes past its blockers"),
 ("C10","mtg-kernel/src/combat.rs","    let overflow = if trample { remaining } else { 0 };","    let overflow = if trample { remaining.saturating_sub(1) } else { 0 };","trample loses a point"),
 ("C11","mtg-kernel/src/combat.rs","        } else if remaining == least {","        } else if remaining <= least + 1 {","a choice of exactly two amounts is not asked"),
 ("C12","mtg-kernel/src/combat.rs","        let least = remaining.min(lethal_here);","        let least = remaining.min(lethal_here.saturating_sub(1));","lethal is counted one short"),
]
F = "mtg-kernel/src/funding.rs"
M += [
 ("F01",F,"Some(d) => d <= max_taps as usize, None => false","Some(d) => d < max_taps as usize, None => false","a group's last source is never tapped"),
 ("F02",F,"                    Some(d + 1)","                    Some(d)","a group has as many sources as it likes"),
 ("F03",F,"        let next = step(&rows[0], q, count);","        let next = step(&rows[0], q, count.saturating_add(1));","a group has one source more than it does"),
 ("F04",F,"        } else if s >= q as usize {","        } else if s > q as usize {","one tap more than a reachable sum is missed when it starts from nothing"),
 ("F05",F,"    last.set(0, true);","    last.set(0, false);","tapping nothing is not a way to pay nothing"),
 ("F06",F,"    let floor = mana.saturating_sub(pool);","    let floor = mana;","the pool only tops up what taps cannot pay exactly"),
 ("F07",F,"    let mut sum: u64 = u64::from(floor);","    let mut sum: u64 = u64::from(floor) + 1;","a tap sum the whole pool tops up exactly is passed over"),
 ("F08",F,"        while below > 0 && !rows[0][below as usize]","        while below > 1 && !rows[0][below as usize]","a short X stops looking below one"),
 ("F09",F,"    let from_pool = if rest < pool { rest } else { pool };","    let from_pool = if rest < pool { rest } else { pool.saturating_sub(1) };","a short X leaves a point in the pool"),
 ("F10",F,"        let taken: u32 = if quantum == 0 || left == 0 {","        let taken: u32 = if quantum == 0 || left == 0 || i == 0 {","the first group is never tapped"),
 ("F11",F,"            let mut found = rows[i + 1][(left - taken * quantum) as usize];","            let mut found = true;","a group takes the most it can whether or not the rest can finish"),
 ("F12",F,"            let mut taken = if left / quantum < count { left / quantum } else { count };","            let mut taken = if left / quantum < count { left / quantum } else { count.saturating_sub(1) };","a group holds back its last source"),
 ("F13",F,"        if rows[0][m as usize] {","        if rows[0][m as usize] && best.is_none() {","a fundable X is measured from the smallest tap sum, not the nearest"),
 ("F14",F,"            Some(b) => (m as u32) - b <= pool,","            Some(b) => (m as u32) - b < pool,","the whole pool never counts towards a fundable X"),
]

FOCUSED = ["lands_and_mana", "mana_filters", "autotap_brute_force", "autotap_beyond_the_hand",
           "x_cost_funding_flow", "xcost_mana", "spell_costs", "combat_damage_division",
           "combat", "combat_rules", "planeswalker_combat", "damage_assignment_order",
           "funding_build_options", "x_cost_spells"]
# The page's X-funding fixture: 168 boards whose answers the engine must
# still give (`x-funding-cases.jsonl`).
FOCUSED_PLAYER = ["gui_protocol"]


def main():
    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    out_dir = os.path.join(ROOT, "logs", "kernel-mutants", stamp)
    os.makedirs(out_dir, exist_ok=True)
    out = os.path.join(out_dir, "results.jsonl")
    wt = tempfile.mkdtemp(prefix="kernel-mutants-")
    subprocess.run(["git", "worktree", "add", "-q", "--detach", wt, "HEAD"], cwd=ROOT, check=True)
    env = dict(os.environ, CARGO_INCREMENTAL="0", ANTHROPIC_API_KEY="dummy")

    def run(cmd):
        p = subprocess.run(cmd, cwd=wt, env=env, capture_output=True, text=True)
        return p.returncode, p.stdout + p.stderr

    try:
        prefixes = sys.argv[1:] or [""]
        for (mid, f, old, new, what) in (m for m in M if any(m[0].startswith(p) for p in prefixes)):
            path = os.path.join(wt, f)
            src = open(path).read()
            if src.count(old) != 1:
                rec = {"id": mid, "what": what, "stale": True}
            else:
                open(path, "w").write(src.replace(old, new))
                try:
                    t0 = time.time()
                    rc_v, _ = run(["cargo", "verus", "focus", "-p", "mtg-kernel", "--", "--triggers-mode", "silent"])
                    rc_k, o_k = run(["cargo", "test", "-p", "mtg-kernel"])
                    cmd = ["cargo", "test", "-p", "mtg-engine", "--lib"]
                    for t in FOCUSED:
                        cmd += ["--test", t]
                    rc_e, o_e = run(cmd)
                    cmd = ["cargo", "test", "-p", "mtg-player"]
                    for t in FOCUSED_PLAYER:
                        cmd += ["--test", t]
                    rc_p, o_p = run(cmd)
                    o_e += o_p
                    compiled = "test result" in o_k and "test result" in o_e and "test result" in o_p
                    tests = "caught" if rc_k or rc_e or rc_p else "missed"
                    if not compiled:
                        tests = "does not compile"
                    full = None
                    if tests == "missed":
                        rc_f, _ = run(["cargo", "test", "-p", "mtg-engine", "--no-fail-fast"])
                        full = "caught" if rc_f else "missed"
                    failing = sorted(set(re.findall(r"^    ([a-z_0-9:]+)$", o_k + o_e, re.M)))[:6]
                    rec = {"id": mid, "what": what, "proof": "caught" if rc_v else "missed",
                           "tests": tests, "whole_engine_suite": full, "failing": failing,
                           "secs": round(time.time() - t0)}
                finally:
                    open(path, "w").write(src)
            with open(out, "a") as fh:
                fh.write(json.dumps(rec) + "\n")
            print(json.dumps(rec), flush=True)
    finally:
        subprocess.run(["git", "worktree", "remove", "--force", wt], cwd=ROOT)
    print(f"results: {out}")


if __name__ == "__main__":
    main()
