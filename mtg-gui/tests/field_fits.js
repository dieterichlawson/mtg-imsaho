// The typing box fits the slot the modal painted for it, at every scale (#741).
//
//   cargo build -p mtg-runner
//   NODE_PATH=$(npm root -g) node mtg-gui/tests/field_fits.js
//
// The <input> was sized to the slot but kept content-box sizing with a
// desktop-sized border and padding, so it was always 12x8 CSS px larger
// than the slot; at 640x360 and phone width that covered "1 - 13" and the
// first summary line of the damage division.

const { chromium } = require("playwright");
const { startRunner, finish } = require("./children.js");

let failures = 0;
function fail(msg) { console.error("FAIL: " + msg); failures++; }

async function main() {
  const { runner, urls } = await startRunner(["--p1", "gui:0", "--p2", "random", "--seed", "11", "-q"]);
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    await page.goto(urls["--p1"], { timeout: 15000 });
    await page.waitForFunction(() => window.mtg && window.mtg.decision, null, { timeout: 30000 });
    await page.evaluate(() => { const m = window.mtg; m.ws.onclose = () => {}; m.ws.close(); });
    const ids = await page.evaluate(() => [window.mtg.view.your_hand[0].object_id, window.mtg.view.your_hand[1].object_id]);
    let seq = 100;
    for (const [w, h] of [[1280, 720], [640, 360], [390, 844]]) {
      await page.setViewportSize({ width: w, height: h });
      await page.waitForTimeout(100);
      seq++;
      const got = await page.evaluate(({ seq, ids }) => {
        const options = Array.from({ length: 13 }, (_, i) => `${i + 1} to Bear`);
        window.mtgDebug.stage({ seq, combat: null, legal: { actions: [], combat_prompt: null, castable_spells: [],
          activatable_abilities: [], context: "TEST", set_prompt: null,
          resolution_prompt: { AssignCombatDamage: { description: "Combat damage: how much goes to Bear?",
            attacker: ids[0], blocker: ids[1], min: 1, max: 13, options, first_strike_only: false } } } });
        window.mtgDebug.render();
        const m = window.mtg;
        const c = document.querySelector("canvas").getBoundingClientRect();
        const f = document.getElementById("field").getBoundingClientRect();
        const r = m.fieldRect;
        return { mode: m.ui && m.ui.mode, scale: m.scale,
          slot: r && { x: c.left + r.x * m.scale, y: c.top + r.y * m.scale, w: r.w * m.scale, h: r.h * m.scale },
          field: { x: f.left, y: f.top, w: f.width, h: f.height } };
      }, { seq, ids });
      if (got.mode !== "number" || !got.slot) { fail(`${w}x${h}: no number widget (${JSON.stringify(got)})`); continue; }
      const { slot, field } = got;
      const eps = 0.51;
      if (field.x < slot.x - eps || field.y < slot.y - eps
          || field.x + field.w > slot.x + slot.w + eps || field.y + field.h > slot.y + slot.h + eps) {
        fail(`${w}x${h} (scale ${got.scale}): field ${JSON.stringify(field)} overhangs its slot ${JSON.stringify(slot)}`);
      } else {
        console.log(`ok: ${w}x${h} field fits its slot`);
      }
    }
  } finally {
    await browser.close();
    runner.kill();
  }
  if (failures) { console.error(`${failures} failure(s)`); process.exitCode = 1; }
}

finish(main);
