import { BOOSTER_Y, canvas, ctx, fpsMeter, H, keys, lang, ui, W } from "./state.js";
import { localizedStationName, t, tf } from "./i18n.js";
import { updateHudUi } from "./ui.js";

// Generate a deterministic pseudo-random integer from a seed.
function lcg(s) {
  return ((s >>> 0) * 1664525 + 1013904223) >>> 0;
}

// Generate a deterministic pseudo-random float from a seed.
function lcgf(s) {
  return (lcg(s) >>> 0) / 4294967295;
}

const BG_SEED = (Date.now() ^ Math.floor(Math.random() * 0xffffffff)) >>> 0;

const STAR_LAYERS = [
  Array.from({ length: 80 }, (_, i) => {
    const s = lcg(BG_SEED ^ (i * 1009 + 1));
    return { x: lcgf(s) * W, y: lcgf(lcg(s)) * H, r: 0.55, parallax: 0.5 };
  }),
  Array.from({ length: 40 }, (_, i) => {
    const s = lcg(BG_SEED ^ (i * 7919 + 2));
    return { x: lcgf(s) * W, y: lcgf(lcg(s)) * H, r: 1.0, parallax: 1.0 };
  }),
];

const NEBULAS = Array.from({ length: 6 }, (_, i) => {
  const s = lcg(BG_SEED ^ (i * 31337 + 3));
  return {
    x: lcgf(s) * W,
    y: lcgf(lcg(s)) * H,
    r: 55 + lcgf(lcg(s)) * 90,
    hue: 200 + lcgf(lcg(lcg(s))) * 70,
    parallax: 0.18,
  };
});

const TITLE_SYMBOLS = Array.from({ length: 12 }, (_, i) => {
  const s = lcg(BG_SEED ^ (i * 12347 + 77));
  return {
    symbol: "♈♉♊♋♌♍♎♏♐♑♒♓"[i],
    x: 24 + lcgf(s) * (W - 48),
    y0: lcgf(lcg(s)) * H,
    speed: 0.2 + lcgf(lcg(lcg(s))) * 0.8,
    alphaPhase: lcgf(lcg(lcg(lcg(s)))) * Math.PI * 2,
    size: 15 + lcgf(lcg(lcg(lcg(lcg(s))))) * 10,
  };
});

const BASE_SCROLL = 0.7;
const SCROLL_MIN = 0.2;
const SCROLL_MAX = 3.5;
let scrollSpeed = BASE_SCROLL;
let scrollOffset = 0;
let titleTick = 0;
let renderFrames = 0;
let networkFrames = 0;
let shownFps = 0;
let shownNet = 0;
let meterLastMs = performance.now();

// Count one authoritative state packet received over WebSocket.
export function recordNetworkFrame() {
  networkFrames += 1;
}

// Update the small FPS/NET meter roughly twice per second.
function updateFpsMeter() {
  renderFrames += 1;
  const now = performance.now();
  const elapsed = now - meterLastMs;
  if (elapsed >= 500) {
    shownFps = Math.round((renderFrames * 1000) / elapsed);
    shownNet = Math.round((networkFrames * 1000) / elapsed);
    renderFrames = 0;
    networkFrames = 0;
    meterLastMs = now;
  }
  if (fpsMeter) {
    fpsMeter.textContent = `FPS ${shownFps || "--"} / NET ${shownNet || "--"}`;
  }
}


function roundedRectPath(x, y, w, h, r) {
  const radius = Math.max(0, Math.min(r, w / 2, h / 2));
  if (typeof ctx.roundRect === "function") {
    ctx.beginPath();
    ctx.roundRect(x, y, w, h, radius);
    return;
  }
  ctx.beginPath();
  ctx.moveTo(x + radius, y);
  ctx.lineTo(x + w - radius, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + radius);
  ctx.lineTo(x + w, y + h - radius);
  ctx.quadraticCurveTo(x + w, y + h, x + w - radius, y + h);
  ctx.lineTo(x + radius, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - radius);
  ctx.lineTo(x, y + radius);
  ctx.quadraticCurveTo(x, y, x + radius, y);
  ctx.closePath();
}

const ASTEROID_COLORS = [
  ["rgba(90,85,80,0.88)", "rgba(140,135,130,0.65)"],
  ["rgba(105,85,55,0.88)", "rgba(160,140,110,0.65)"],
  ["rgba(140,80,40,0.88)", "rgba(200,140,80,0.65)"],
  ["rgba(160,60,30,0.88)", "rgba(220,110,60,0.65)"],
  ["rgba(180,40,20,0.88)", "rgba(240,80,40,0.85)"],
];

// Update downward background speed from ship Y and fast-scroll mode.
function updateScroll(shipY, fastScroll) {
  const y = shipY ?? H * 0.67;
  const pct = 1 - Math.max(0, Math.min(1, y / H));
  const min = fastScroll ? 0.7 : SCROLL_MIN;
  const max = fastScroll ? 7.2 : SCROLL_MAX;
  const target = min + pct * (max - min);
  scrollSpeed += (target - scrollSpeed) * 0.08;
  scrollOffset += scrollSpeed;
}

// Draw a soft nebula blob used in the parallax background.
function drawNebula(x, y, r, hue) {
  const gradient = ctx.createRadialGradient(x, y, 0, x, y, r);
  gradient.addColorStop(0, `hsla(${hue},55%,22%,0.07)`);
  gradient.addColorStop(1, "transparent");
  ctx.beginPath();
  ctx.arc(x, y, r, 0, Math.PI * 2);
  ctx.fillStyle = gradient;
  ctx.fill();
}

// Draw the animated space background behind gameplay objects.
function renderBg(tick, shipY, fastScroll) {
  updateScroll(shipY, fastScroll);
  ctx.fillStyle = "#050510";
  ctx.fillRect(0, 0, W, H);
  for (const nebula of NEBULAS) {
    const y = (((nebula.y + scrollOffset * nebula.parallax) % H) + H) % H;
    drawNebula(nebula.x, y, nebula.r, nebula.hue);
    if (y < nebula.r) drawNebula(nebula.x, y + H, nebula.r, nebula.hue);
    if (y > H - nebula.r) drawNebula(nebula.x, y - H, nebula.r, nebula.hue);
  }
  for (const layer of STAR_LAYERS) {
    for (const star of layer) {
      const y = (((star.y + scrollOffset * star.parallax) % H) + H) % H;
      const alpha = 0.3 + 0.25 * Math.sin(tick * 0.04 + star.x * 0.1);
      ctx.beginPath();
      ctx.arc(star.x, y, star.r, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(210,225,255,${alpha.toFixed(2)})`;
      ctx.fill();
      if (y > H - 3) {
        ctx.beginPath();
        ctx.arc(star.x, y - H, star.r, 0, Math.PI * 2);
        ctx.fill();
      }
      if (y < 3) {
        ctx.beginPath();
        ctx.arc(star.x, y + H, star.r, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }
}

// Draw a station body, labels and optional destination or booster airlock.
function drawStation(cx, cy, symbol, name, isDestination, airlockX, pulse, mode = "destination") {
  const isBooster = mode === "booster";
  const ringStroke = isBooster
    ? `rgba(34,197,94,${0.45 + 0.4 * pulse})`
    : isDestination
      ? `rgba(251,191,36,${0.5 + 0.4 * pulse})`
      : "rgba(99,102,241,0.5)";
  const bodyFill = isBooster
    ? "rgba(8,28,18,0.88)"
    : isDestination
      ? "rgba(45,30,10,0.8)"
      : "rgba(20,20,40,0.8)";
  const panelFill = isBooster
    ? "rgba(34,197,94,0.30)"
    : isDestination
      ? "rgba(251,191,36,0.35)"
      : "rgba(99,102,241,0.3)";
  const panelStroke = isBooster
    ? "rgba(134,239,172,0.55)"
    : isDestination
      ? "rgba(251,191,36,0.5)"
      : "rgba(99,102,241,0.4)";
  const labelFill = isBooster
    ? `rgba(187,247,208,${0.75 + 0.2 * pulse})`
    : isDestination
      ? `rgba(253,230,138,${0.7 + 0.3 * pulse})`
      : "rgba(165,180,252,0.8)";
  const subFill = isBooster
    ? "rgba(187,247,208,0.7)"
    : isDestination
      ? "rgba(253,230,138,0.6)"
      : "rgba(148,163,184,0.5)";

  ctx.save();
  ctx.translate(cx, cy);
  ctx.beginPath();
  ctx.arc(0, 0, 24, 0, Math.PI * 2);
  ctx.strokeStyle = ringStroke;
  ctx.lineWidth = 3;
  ctx.stroke();
  ctx.beginPath();
  ctx.arc(0, 0, 24, 0, Math.PI * 2);
  ctx.fillStyle = bodyFill;
  ctx.fill();

  ctx.fillStyle = panelFill;
  ctx.fillRect(-52, -6, 24, 12);
  ctx.fillRect(28, -6, 24, 12);
  ctx.strokeStyle = panelStroke;
  ctx.lineWidth = 1;
  ctx.strokeRect(-52, -6, 24, 12);
  ctx.strokeRect(28, -6, 24, 12);

  ctx.beginPath();
  ctx.moveTo(-24, 0);
  ctx.lineTo(-28, 0);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(24, 0);
  ctx.lineTo(28, 0);
  ctx.stroke();

  ctx.textAlign = "center";
  ctx.font = "14px DotGothic16, monospace";
  ctx.fillStyle = labelFill;
  ctx.fillText(isBooster ? "BOOST" : symbol, 0, 5);
  ctx.font = "9px DotGothic16, monospace";
  ctx.fillStyle = subFill;
  ctx.fillText(isBooster ? (lang === "ja" ? "中継" : "Relay") : name, 0, 20);
  ctx.restore();

  if (isDestination || isBooster) {
    const ax = airlockX;
    const ay = cy;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(ax, ay);
    ctx.strokeStyle = isBooster
      ? `rgba(34,197,94,${0.22 + 0.18 * pulse})`
      : `rgba(251,191,36,${0.2 + 0.15 * pulse})`;
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 6]);
    ctx.stroke();
    ctx.setLineDash([]);

    const glowR = 14 + 4 * pulse;
    const gradient = ctx.createRadialGradient(ax, ay, 2, ax, ay, glowR);
    gradient.addColorStop(
      0,
      isBooster
        ? `rgba(96,165,250,${0.6 + 0.3 * pulse})`
        : `rgba(34,197,94,${0.6 + 0.3 * pulse})`,
    );
    gradient.addColorStop(1, isBooster ? "rgba(96,165,250,0)" : "rgba(34,197,94,0)");
    ctx.beginPath();
    ctx.arc(ax, ay, glowR, 0, Math.PI * 2);
    ctx.fillStyle = gradient;
    ctx.fill();

    ctx.beginPath();
    ctx.arc(ax, ay, 8, 0, Math.PI * 2);
    ctx.fillStyle = isBooster
      ? `rgba(96,165,250,${0.45 + 0.3 * pulse})`
      : `rgba(34,197,94,${0.4 + 0.3 * pulse})`;
    ctx.fill();
    ctx.strokeStyle = isBooster
      ? `rgba(191,219,254,${0.7 + 0.25 * pulse})`
      : `rgba(134,239,172,${0.7 + 0.3 * pulse})`;
    ctx.lineWidth = 2;
    ctx.stroke();

    ctx.fillStyle = isBooster
      ? `rgba(191,219,254,${0.65 + 0.25 * pulse})`
      : `rgba(134,239,172,${0.6 + 0.3 * pulse})`;
    ctx.textAlign = "center";
    ctx.font = "12px sans-serif";
    ctx.fillText("▼", ax, ay + 28);
  }
}

// Draw the lower departure station used during the launch phase.
function drawDepartStation(cx, sy, symbol, departX) {
  ctx.save();
  ctx.translate(cx, sy);
  ctx.beginPath();
  ctx.arc(0, 0, 20, 0, Math.PI * 2);
  ctx.strokeStyle = "rgba(99,102,241,0.4)";
  ctx.lineWidth = 2;
  ctx.stroke();
  ctx.beginPath();
  ctx.arc(0, 0, 20, 0, Math.PI * 2);
  ctx.fillStyle = "rgba(20,20,40,0.8)";
  ctx.fill();
  ctx.fillStyle = "rgba(99,102,241,0.25)";
  ctx.fillRect(-44, -5, 20, 10);
  ctx.fillRect(24, -5, 20, 10);
  ctx.textAlign = "center";
  ctx.font = "12px DotGothic16, monospace";
  ctx.fillStyle = "rgba(165,180,252,0.6)";
  ctx.fillText(symbol, 0, 4);
  ctx.restore();

  ctx.beginPath();
  ctx.arc(departX, sy, 7, 0, Math.PI * 2);
  ctx.fillStyle = "rgba(34,197,94,0.3)";
  ctx.fill();
  ctx.strokeStyle = "rgba(134,239,172,0.5)";
  ctx.lineWidth = 1.5;
  ctx.stroke();
}

// Draw thrust flames only while control input is held.
function drawThrusters(tw, th, boosterAttached) {
  const MAIN_COLOR = "rgba(251,191,36,";
  const SIDE_COLOR = "rgba(147,197,253,";
  if (keys.up) {
    if (boosterAttached) {
      const fl = 22 + Math.random() * 24;
      const fw = 5 + Math.random() * 4;
      [-4.5, 4.5].forEach((ox) => {
        ctx.beginPath();
        ctx.moveTo(ox - fw / 2, th / 2);
        ctx.lineTo(ox + (Math.random() - 0.5) * 4, th / 2 + fl);
        ctx.lineTo(ox + fw / 2, th / 2);
        ctx.closePath();
        ctx.fillStyle = "rgba(248,113,113," + (0.75 + Math.random() * 0.2) + ")";
        ctx.fill();
      });
    } else {
      const fl = 10 + Math.random() * 12;
      const fw = 4 + Math.random() * 3;
      ctx.beginPath();
      ctx.moveTo(-fw / 2, th / 2);
      ctx.lineTo((Math.random() - 0.5) * 4, th / 2 + fl);
      ctx.lineTo(fw / 2, th / 2);
      ctx.closePath();
      ctx.fillStyle = MAIN_COLOR + (0.7 + Math.random() * 0.3) + ")";
      ctx.fill();
    }
  }
  if (keys.down) {
    const fl = 5 + Math.random() * 7;
    ctx.beginPath();
    ctx.moveTo(-3, -th / 2);
    ctx.lineTo((Math.random() - 0.5) * 3, -th / 2 - fl);
    ctx.lineTo(3, -th / 2);
    ctx.closePath();
    ctx.fillStyle = SIDE_COLOR + (0.55 + Math.random() * 0.3) + ")";
    ctx.fill();
  }
  if (keys.right) {
    const fl = 4 + Math.random() * 6;
    ctx.beginPath();
    ctx.moveTo(-tw / 2, -4);
    ctx.lineTo(-tw / 2 - fl, (Math.random() - 0.5) * 4);
    ctx.lineTo(-tw / 2, 4);
    ctx.closePath();
    ctx.fillStyle = SIDE_COLOR + (0.5 + Math.random() * 0.3) + ")";
    ctx.fill();
  }
  if (keys.left) {
    const fl = 4 + Math.random() * 6;
    ctx.beginPath();
    ctx.moveTo(tw / 2, -4);
    ctx.lineTo(tw / 2 + fl, (Math.random() - 0.5) * 4);
    ctx.lineTo(tw / 2, 4);
    ctx.closePath();
    ctx.fillStyle = SIDE_COLOR + (0.5 + Math.random() * 0.3) + ")";
    ctx.fill();
  }
}

// Draw the player truck body and optional booster pod.
function drawTruck(x, y, invincible, tick, boosterAttached) {
  if (invincible && Math.floor(tick / 4) % 2 === 1) return;
  ctx.save();
  ctx.translate(x, y);
  const tw = 14;
  const th = 28;
  drawThrusters(tw, th, boosterAttached);
  ctx.fillStyle = "#2563eb";
  ctx.fillRect(-tw / 2, -th / 2, tw, th);
  ctx.fillStyle = "#bfdbfe";
  ctx.fillRect(-tw / 2 + 2, -th / 2 + 2, tw - 4, 9);
  ctx.strokeStyle = "rgba(147,197,253,0.4)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(-tw / 2 + 4, -th / 2 + 4);
  ctx.lineTo(-tw / 2 + 4, -th / 2 + 10);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(-tw / 2 + 8, -th / 2 + 4);
  ctx.lineTo(-tw / 2 + 8, -th / 2 + 10);
  ctx.stroke();
  ctx.fillStyle = "#1e3a8a";
  ctx.fillRect(-tw / 2, -th / 2 + 13, tw, th / 2 + 2);
  if (boosterAttached) {
    ctx.fillStyle = "rgba(56,189,248,0.9)";
    ctx.fillRect(-tw / 2 - 4, -2, 3, 16);
    ctx.fillRect(tw / 2 + 1, -2, 3, 16);
    ctx.strokeStyle = "rgba(191,219,254,0.8)";
    ctx.lineWidth = 1;
    ctx.strokeRect(-tw / 2 - 4, -2, 3, 16);
    ctx.strokeRect(tw / 2 + 1, -2, 3, 16);
  }
  ctx.fillStyle = "#1d4ed8";
  ctx.fillRect(-tw / 2, -th / 2 + 12, tw, 2);
  ctx.strokeStyle = "#93c5fd";
  ctx.lineWidth = 1.5;
  ctx.strokeRect(-tw / 2, -th / 2, tw, th);
  ctx.fillStyle = "#fde68a";
  ctx.fillRect(-tw / 2 + 1, -th / 2 + 1, 3, 3);
  ctx.fillRect(tw / 2 - 4, -th / 2 + 1, 3, 3);
  ctx.fillStyle = "#ef4444";
  ctx.fillRect(-tw / 2 + 1, th / 2 - 4, 3, 3);
  ctx.fillRect(tw / 2 - 4, th / 2 - 4, 3, 3);
  ctx.restore();
}

// Draw collectible minerals drifting through space.
function drawMinerals(minerals) {
  if (!minerals) return;
  for (const mineral of minerals) {
    const isRare = mineral.kind === "rare";
    const fill = isRare ? "rgba(192,132,252,0.9)" : "rgba(251,191,36,0.92)";
    const stroke = isRare ? "rgba(233,213,255,0.7)" : "rgba(254,240,138,0.7)";
    const r = 5;
    ctx.save();
    ctx.translate(mineral.x, mineral.y);
    ctx.rotate((mineral.seed % 628) / 100);
    ctx.beginPath();
    ctx.moveTo(0, -r);
    ctx.lineTo(r, 0);
    ctx.lineTo(0, r);
    ctx.lineTo(-r, 0);
    ctx.closePath();
    ctx.fillStyle = fill;
    ctx.fill();
    ctx.strokeStyle = stroke;
    ctx.lineWidth = 1;
    ctx.stroke();
    ctx.restore();
  }
}

// Draw the manipulator arm used to collect minerals.
function drawManipulator(ship, manipLen) {
  if (!ship || !manipLen || manipLen <= 0) return;
  const x = ship.x;
  const y1 = ship.y - 10;
  const y2 = ship.y - 10 - manipLen;
  ctx.save();
  ctx.strokeStyle = "rgba(226,232,240,0.8)";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(x, y1);
  ctx.lineTo(x, y2);
  ctx.stroke();
  ctx.strokeStyle = "rgba(148,163,184,0.65)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(x - 4, y2 - 3);
  ctx.lineTo(x, y2);
  ctx.lineTo(x + 4, y2 - 3);
  ctx.stroke();
  ctx.restore();
}

// Draw falling asteroids, color-coded by speed tier.
function drawAsteroids(asteroids) {
  if (!asteroids) return;
  for (const asteroid of asteroids) {
    const tier = Math.min(4, asteroid.tier ?? 0);
    const [fill, stroke] = ASTEROID_COLORS[tier];
    ctx.save();
    ctx.translate(asteroid.x, asteroid.y);
    ctx.rotate(lcgf(asteroid.seed) * Math.PI * 2);
    const r = asteroid.r;
    const pts = 6 + (lcg(asteroid.seed) % 4);
    ctx.beginPath();
    for (let i = 0; i < pts; i++) {
      const ang = (i / pts) * Math.PI * 2;
      const j = 0.6 + lcgf(lcg(asteroid.seed + i * 31)) * 0.75;
      const px = Math.cos(ang) * r * j;
      const py = Math.sin(ang) * r * j;
      if (i === 0) ctx.moveTo(px, py);
      else ctx.lineTo(px, py);
    }
    ctx.closePath();
    ctx.fillStyle = fill;
    ctx.fill();
    ctx.strokeStyle = stroke;
    ctx.lineWidth = tier >= 3 ? 1.5 : 1;
    ctx.stroke();
    if (tier >= 3) {
      const glowLen = r * 0.8 * (tier === 4 ? 1.6 : 1.0);
      const gradient = ctx.createLinearGradient(0, 0, 0, glowLen);
      gradient.addColorStop(
        0,
        tier === 4 ? "rgba(255,120,60,0.35)" : "rgba(220,100,40,0.25)",
      );
      gradient.addColorStop(1, "transparent");
      ctx.beginPath();
      ctx.ellipse(0, 0, r * 0.55, glowLen, 0, 0, Math.PI * 2);
      ctx.fillStyle = gradient;
      ctx.fill();
    }
    ctx.restore();
  }
}

// Draw the right-side vertical progress bar for route completion.
function drawProgressBar(progress) {
  const bx = W - 14;
  const by = 40;
  const bh = H - 80;
  const bw = 8;
  ctx.fillStyle = "rgba(255,255,255,0.07)";
  roundedRectPath(bx, by, bw, bh, 4);
  ctx.fill();
  const pct = (progress || 0) / 100;
  if (pct > 0) {
    const fh = bh * pct;
    const gradient = ctx.createLinearGradient(0, by + bh, 0, by + bh - fh);
    gradient.addColorStop(0, "rgba(168,85,247,0.9)");
    gradient.addColorStop(0.8, "rgba(251,191,36,0.9)");
    gradient.addColorStop(1, "rgba(34,197,94,0.9)");
    ctx.fillStyle = gradient;
    roundedRectPath(bx, by + bh - fh, bw, fh, 4);
    ctx.fill();
  }
  const dockZoneY = by + bh * 0.2 - 3;
  ctx.strokeStyle = "rgba(34,197,94,0.5)";
  ctx.lineWidth = 1;
  ctx.setLineDash([3, 3]);
  ctx.beginPath();
  ctx.moveTo(bx - 4, dockZoneY);
  ctx.lineTo(bx + bw + 4, dockZoneY);
  ctx.stroke();
  ctx.setLineDash([]);
}

// Draw the HUD canvas layer and keep DOM HUD strings in sync.
function drawHUD(state) {
  updateHudUi(state);
  ctx.fillStyle = "rgba(5,5,20,0.55)";
  ctx.fillRect(0, H - 30, W, 30);
  ctx.textAlign = "center";
  const isDocking = state.phase === "docking";
  const isBoosterDocking = state.phase === "booster_docking";
  ctx.font = isDocking ? "14px DotGothic16, monospace" : "12px DotGothic16, monospace";
  ctx.fillStyle = isBoosterDocking ? "#bfdbfe" : isDocking ? "#86efac" : "#7dd3fc";
  const label = isBoosterDocking
    ? t("booster_lbl")
    : isDocking
      ? tf("dock_lbl", {
          symbol: state.to.symbol,
          name: localizedStationName(state.to),
        })
      : tf("dest_lbl", {
          symbol: state.to.symbol,
          name: localizedStationName(state.to),
        });
  ctx.fillText(label, W / 2, H - 9);
  ctx.textAlign = "left";
}

// Render a full gameplay frame from the latest authoritative server state.
export function renderGame(state) {
  updateFpsMeter();
  const tick = state.tick || 0;
  renderBg(tick, state.ship ? state.ship.y : undefined, !!state.fast_scroll);
  drawProgressBar(state.progress, state.phase);

  const pulse = 0.5 + 0.5 * Math.sin(tick * 0.1);
  const destinationName = lang === "ja" ? state.to.name_ja : state.to.name_en;

  if (state.phase === "launching" || state.stage_tick < 120) {
    const alpha = state.stage_tick < 90 ? 1.0 : Math.max(0, 1 - (state.stage_tick - 90) / 30);
    if (alpha > 0) {
      ctx.globalAlpha = alpha;
      const airlockY = H - 40;
      const gradient = ctx.createRadialGradient(state.depart_x, airlockY, 2, state.depart_x, airlockY, 18);
      gradient.addColorStop(0, "rgba(99,102,241,0.6)");
      gradient.addColorStop(1, "rgba(99,102,241,0)");
      ctx.beginPath();
      ctx.arc(state.depart_x, airlockY, 18, 0, Math.PI * 2);
      ctx.fillStyle = gradient;
      ctx.fill();
      ctx.beginPath();
      ctx.arc(state.depart_x, airlockY, 7, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(99,102,241,0.5)";
      ctx.fill();
      ctx.strokeStyle = "rgba(165,180,252,0.7)";
      ctx.lineWidth = 2;
      ctx.stroke();
      ctx.textAlign = "center";
      ctx.font = "10px DotGothic16, monospace";
      ctx.fillStyle = "rgba(165,180,252,0.7)";
      ctx.fillText(
        tf("launch_txt", {
          symbol: state.from.symbol,
          name: localizedStationName(state.from),
        }),
        state.depart_x,
        airlockY - 24,
      );
      ctx.globalAlpha = 1.0;
    }
  }

  if (
    state.phase === "booster_docking" ||
    (state.booster_enabled && state.booster_attached && (state.progress || 0) < 40)
  ) {
//    drawStation(270, BOOSTER_Y, "", "", true, state.booster_x, pulse, "booster");
    // @@@ 20260418 size change 270->180 (canvas w540->w360)
    drawStation(180, BOOSTER_Y, "", "", true, state.booster_x, pulse, "booster");
  }

  if (state.phase === "docking" || (state.progress || 0) >= 78) {
//    drawStation(270, 30, state.to.symbol, destinationName, true, state.airlock_x, pulse);
    // @@@ 20260418 size change 270->180 (canvas w540->w360)
    drawStation(180, 30, state.to.symbol, destinationName, true, state.airlock_x, pulse);
  }

  drawAsteroids(state.asteroids);
  drawMinerals(state.minerals);
  drawManipulator(state.ship, state.manip_len);
  drawTruck(state.ship.x, state.ship.y, state.ship.invincible, tick, state.booster_attached);
  drawHUD(state);
}

// Animate the title screen background while the title overlay is visible.
export function renderTitleBg() {
  titleTick += 1;
  renderBg(titleTick, 0);
  ctx.textAlign = "center";
  for (const item of TITLE_SYMBOLS) {
    const y = (((item.y0 + titleTick * item.speed) % H) + H) % H;
    ctx.font = `${item.size.toFixed(1)}px DotGothic16, monospace`;
    ctx.fillStyle = `rgba(165,180,252,${0.13 + 0.14 * Math.sin(titleTick * 0.05 + item.alphaPhase)})`;
    ctx.fillText(item.symbol, item.x, y);
  }
  requestAnimationFrame(() => {
    if (ui.screen === "title") renderTitleBg();
  });
}

export { canvas };
