// Art, fonts and placeholders.
//
// Every image is optional: a card with no art file renders as a coloured
// frame with its initials, so the page works from a bare checkout and in
// CI with no generated assets at all.
const images = new Map(); // path -> image, or null once it failed
let manifest = { images: {} };
let byName = new Map(); // card or token name -> path
export async function loadManifest() {
    try {
        const r = await fetch("assets/art/manifest.json", { cache: "no-cache" });
        if (r.ok) {
            manifest = (await r.json());
            byName = new Map();
            for (const img of Object.values(manifest.images || {})) {
                if (img.kind === "cards" || img.kind === "tokens")
                    byName.set(img.name, `assets/art/${img.file}`);
            }
        }
    }
    catch {
        // No manifest: everything is a placeholder.
    }
}
export async function fontsReady() {
    try {
        await Promise.all([document.fonts.load("8px Silkscreen"), document.fonts.load("8px PressStart")]);
    }
    catch { /* system font fallback */ }
}
function load(path) {
    if (images.has(path))
        return images.get(path) ?? null;
    const img = new Image();
    images.set(path, img);
    img.onload = () => { img.ready = true; };
    img.onerror = () => { images.set(path, null); };
    img.src = path;
    return img;
}
/** The loaded art image for a card or token name, or null. */
export function artFor(name, isToken) {
    let path = byName.get(name);
    if (!path && isToken)
        path = `assets/art/tokens/${slug(name)}.png`;
    if (!path)
        return null;
    const img = load(path);
    return img && img.ready ? img : null;
}
/** A UI piece from assets/art/ui, or null. */
export function uiImage(name) {
    const img = load(`assets/art/ui/${name}.png`);
    return img && img.ready ? img : null;
}
export function slug(name) {
    return name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}
// Colour identity → palette. Multicolour is gold; none is stone.
export const COLOR_RGB = {
    White: [232, 220, 180], Blue: [80, 120, 200], Black: [70, 50, 80],
    Red: [190, 70, 50], Green: [70, 130, 70],
};
export function frameColor(colors) {
    if (!colors || colors.length === 0)
        return [120, 118, 125];
    if (colors.length > 1)
        return [190, 160, 70];
    return COLOR_RGB[colors[0]] || [120, 118, 125];
}
export function rgb([r, g, b], a = 1) {
    return a === 1 ? `rgb(${r},${g},${b})` : `rgba(${r},${g},${b},${a})`;
}
export function darker([r, g, b], f = 0.55) {
    return `rgb(${Math.round(r * f)},${Math.round(g * f)},${Math.round(b * f)})`;
}
/** Draw the art (or its placeholder) into a w×h window at x,y. */
export function drawArt(ctx, x, y, w, h, name, colors, isToken) {
    const img = artFor(name, isToken);
    if (img) {
        ctx.drawImage(img, 0, 0, img.width, img.height, x, y, w, h);
        return;
    }
    const c = frameColor(colors);
    ctx.fillStyle = darker(c, 0.35);
    ctx.fillRect(x, y, w, h);
    ctx.fillStyle = rgb(c, 0.5);
    // A diagonal band so a placeholder is never mistaken for a blank.
    ctx.beginPath();
    ctx.moveTo(x, y + h);
    ctx.lineTo(x + w, y);
    ctx.lineTo(x + w, y + Math.max(4, h / 3));
    ctx.lineTo(x + Math.max(4, w / 3), y + h);
    ctx.closePath();
    ctx.fill();
    const initials = name.split(/[\s,]+/).filter(Boolean).slice(0, 3).map(s => s[0].toUpperCase()).join("");
    ctx.fillStyle = "#f0e8d8";
    ctx.font = (h >= 40 ? "16px" : "8px") + " PressStart";
    ctx.textBaseline = "middle";
    ctx.textAlign = "center";
    ctx.fillText(initials, x + w / 2, y + h / 2 + 1);
    ctx.textAlign = "left";
    ctx.textBaseline = "top";
}
