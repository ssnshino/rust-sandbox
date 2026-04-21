import { H, keys, W } from "./state.js";

const SHIP_MAX_SPEED = 5.5;
const SHIP_ACCEL = 0.35;
const SHIP_DRAG = 0.975;
const SHIP_R = 10;
const LAUNCH_TARGET_Y = H * 0.67;
const CRUISE_TICKS = 1100;
const MANIP_MAX_LEN = 44;
const MANIP_GROW = 3.5;
const MANIP_SHRINK = 5.5;
const INVINCIBLE_TICKS = 60;
const SHIP_FUEL_MAX = 100;
const MINERAL_R = 5;
const BOOSTER_SCROLL_BONUS_Y = 1.85;

let local = null;
let lastFrameMs = 0;
let eventSeq = 0;
const asteroidPlans = new Map();
const mineralPlans = new Map();
const routeConfigs = new Map();
const DEFAULT_ROUTE_CONFIG = {
  dock_y: 90,
  dock_x_ok: 22,
  booster_y: 185,
  booster_x_ok: 24,
  fuel_stand_y: 250,
  fuel_stand_x_ok: 26,
};

// Scene boundaries must drop every browser-owned simulation cache. Without
// this, sparse WebSocket snapshots can leave old gameover/route objects alive
// behind the next launch screen.
export function resetClientGame() {
  local = null;
  lastFrameMs = 0;
  eventSeq = 0;
  asteroidPlans.clear();
  mineralPlans.clear();
  routeConfigs.clear();
}

function activeLocalPhase(phase) {
  return phase === "launching" || phase === "playing" || phase === "booster_docking" || phase === "fuel_docking" || phase === "docking";
}

function mineralCollectPhase(phase) {
  return phase === "playing" || phase === "booster_docking" || phase === "fuel_docking" || phase === "docking";
}

function localKey(state) {
  return `${state.round}:${state.stage}`;
}

function cloneShip(ship) {
  return {
    x: Number(ship?.x || W / 2),
    y: Number(ship?.y || H * 0.67),
    vx: Number(ship?.vx || 0),
    vy: Number(ship?.vy || 0),
    invincible: !!ship?.invincible,
  };
}

function resetLocalFromServer(state) {
  const key = localKey(state);
  local = {
    key,
    phase: state.phase,
    ship: cloneShip(state.ship),
    hp: Number(state.hp || 0),
    fuel: Number(state.fuel ?? SHIP_FUEL_MAX),
    manipLen: Number(state.manip_len || 0),
    invincibleTicks: state.ship?.invincible ? INVINCIBLE_TICKS : 0,
    event: null,
    eventToken: null,
    sentEvent: null,
    renderTick: Number(state.tick || 0),
    stageTick: Number(state.stage_tick || 0),
    lastResourceSyncTick: Number(state.tick || 0),
    collectedMinerals: new Set(),
    asteroidPlan: asteroidPlans.get(key) || [],
    mineralPlan: mineralPlans.get(key) || [],
    routeConfig: routeConfigs.get(key) || DEFAULT_ROUTE_CONFIG,
    asteroids: [],
    minerals: [],
    nextAsteroidIndex: 0,
    nextMineralIndex: 0,
    routeTick: Math.max(0, Number(state.stage_tick || 0) - 90),
  };
  lastFrameMs = performance.now();
}

function routePaused(phase) {
  return phase === "booster_docking" || phase === "fuel_docking";
}

function spaceMotionPaused(phase) {
  return phase === "booster_docking" || phase === "fuel_docking" || phase === "docking";
}

function asteroidDamage(radius) {
  if (radius <= 5.5) return 5;
  if (radius <= 9.5) return 8;
  if (radius <= 14.5) return 12;
  if (radius <= 20.5) return 16;
  return 20;
}

function spendFuel(stepScale) {
  let usePoints = 0;
  if (keys.up) usePoints += 0.5;
  if (keys.down) usePoints += 0.1;
  if (keys.left) usePoints += 0.1;
  if (keys.right) usePoints += 0.1;
  local.fuel = Math.max(0, local.fuel - usePoints * stepScale);
}

function moveShip(stepScale) {
  const canThrust = local.fuel > 0;
  const ax = canThrust && keys.right ? SHIP_ACCEL : canThrust && keys.left ? -SHIP_ACCEL : 0;
  const ay = canThrust && keys.down ? SHIP_ACCEL : canThrust && keys.up ? -SHIP_ACCEL : 0;

  if (canThrust) spendFuel(stepScale);

  local.ship.vx = (local.ship.vx + ax * stepScale) * SHIP_DRAG;
  local.ship.vy = (local.ship.vy + ay * stepScale) * SHIP_DRAG;

  const speed = Math.hypot(local.ship.vx, local.ship.vy);
  if (speed > SHIP_MAX_SPEED) {
    local.ship.vx = (local.ship.vx / speed) * SHIP_MAX_SPEED;
    local.ship.vy = (local.ship.vy / speed) * SHIP_MAX_SPEED;
  }

  local.ship.x = Math.max(SHIP_R, Math.min(W - SHIP_R, local.ship.x + local.ship.vx * stepScale));
  local.ship.y = Math.max(SHIP_R, Math.min(H - SHIP_R, local.ship.y + local.ship.vy * stepScale));
}

function moveLaunching(stepScale) {
  const dist = local.ship.y - LAUNCH_TARGET_Y;
  if (dist > 5) {
    local.ship.vy -= SHIP_ACCEL * 0.6 * stepScale;
  } else {
    local.ship.vy *= 0.80;
  }
  local.ship.vx *= 0.85;
  local.ship.vy *= SHIP_DRAG;

  const speed = Math.hypot(local.ship.vx, local.ship.vy);
  if (speed > SHIP_MAX_SPEED) {
    local.ship.vx = 0;
    local.ship.vy = -SHIP_MAX_SPEED;
  }

  local.ship.x = Math.max(SHIP_R, Math.min(W - SHIP_R, local.ship.x + local.ship.vx * stepScale));
  const nextY = local.ship.y + local.ship.vy * stepScale;
  if (nextY < LAUNCH_TARGET_Y) {
    local.ship.y = LAUNCH_TARGET_Y;
    local.ship.vy = 0;
  } else {
    local.ship.y = Math.max(SHIP_R, Math.min(H - SHIP_R, nextY));
  }
}

function updateManipulator(stepScale) {
  if (keys.manip) {
    local.manipLen = Math.min(MANIP_MAX_LEN, local.manipLen + MANIP_GROW * stepScale);
  } else {
    local.manipLen = Math.max(0, local.manipLen - MANIP_SHRINK * stepScale);
  }
}

function collideAsteroids(asteroids) {
  if (local.invincibleTicks > 0) return;
  for (const asteroid of asteroids || []) {
    const dx = local.ship.x - Number(asteroid.x || 0);
    const dy = local.ship.y - Number(asteroid.y || 0);
    const radius = Number(asteroid.r || 0);
    if (Math.hypot(dx, dy) < SHIP_R + radius) {
      local.hp = Math.max(0, local.hp - asteroidDamage(radius));
      local.invincibleTicks = INVINCIBLE_TICKS;
      local.event = local.hp <= 0 ? "gameover" : "damage";
      local.eventToken = `${local.event}:${++eventSeq}`;
      return;
    }
  }
}

function spawnPlannedAsteroids() {
  while (
    local.nextAsteroidIndex < local.asteroidPlan.length &&
    Number(local.asteroidPlan[local.nextAsteroidIndex].spawn_at || 0) <= local.routeTick
  ) {
    const planned = local.asteroidPlan[local.nextAsteroidIndex];
    local.asteroids.push({
      x: Number(planned.x || 0),
      y: -Number(planned.r || 0) - 2,
      vx: Number(planned.vx || 0),
      vy: Number(planned.vy || 0),
      r: Number(planned.r || 0),
      tier: Number(planned.tier || 0),
      seed: Number(planned.seed || 0),
    });
    local.nextAsteroidIndex += 1;
  }
}

function moveAsteroids(stepScale, fastScroll) {
  const scrollBonus = fastScroll ? BOOSTER_SCROLL_BONUS_Y : 0;
  let index = 0;
  while (index < local.asteroids.length) {
    const asteroid = local.asteroids[index];
    asteroid.x += asteroid.vx * stepScale;
    asteroid.y += (asteroid.vy + scrollBonus) * stepScale;
    if (asteroid.x < -asteroid.r - 10) asteroid.x += W + asteroid.r * 2;
    if (asteroid.x > W + asteroid.r + 10) asteroid.x -= W + asteroid.r * 2;
    if (asteroid.y > H + asteroid.r + 20) {
      local.asteroids.splice(index, 1);
    } else {
      index += 1;
    }
  }
}

function spawnPlannedMinerals() {
  while (
    local.nextMineralIndex < local.mineralPlan.length &&
    Number(local.mineralPlan[local.nextMineralIndex].spawn_at || 0) <= local.routeTick
  ) {
    const planned = local.mineralPlan[local.nextMineralIndex];
    const id = Number(planned.id);
    if (!local.collectedMinerals.has(id)) {
      local.minerals.push({
        id,
        x: Number(planned.x || 0),
        y: -MINERAL_R - 2,
        vx: Number(planned.vx || 0),
        vy: Number(planned.vy || 0),
        kind: planned.kind,
        seed: Number(planned.seed || 0),
      });
    }
    local.nextMineralIndex += 1;
  }
}

function moveMinerals(stepScale, fastScroll) {
  const scrollBonus = fastScroll ? BOOSTER_SCROLL_BONUS_Y * 0.9 : 0;
  let index = 0;
  while (index < local.minerals.length) {
    const mineral = local.minerals[index];
    mineral.x += mineral.vx * stepScale;
    mineral.y += (mineral.vy + scrollBonus) * stepScale;
    if (mineral.x < -MINERAL_R - 10) mineral.x += W + MINERAL_R * 2;
    if (mineral.x > W + MINERAL_R + 10) mineral.x -= W + MINERAL_R * 2;
    if (mineral.y > H + MINERAL_R + 20) {
      local.minerals.splice(index, 1);
    } else {
      index += 1;
    }
  }
}

function shipSnapshot(kind, extra = {}) {
  return {
    type: "client_event",
    event: kind,
    ship: {
      x: local.ship.x,
      y: local.ship.y,
      vx: local.ship.vx,
      vy: local.ship.vy,
    },
    hp: Math.round(local.hp),
    fuel: local.fuel,
    manip_len: local.manipLen,
    ...extra,
  };
}

function sendEventOnce(send, kind) {
  if (local.sentEvent === kind) return;
  local.sentEvent = kind;
  send(shipSnapshot(kind));
}

function maybeSendDockEvent(serverState, send) {
  const config = local.routeConfig || DEFAULT_ROUTE_CONFIG;
  let targetX = null;
  let targetY = null;
  let xLimit = 0;
  let kind = null;

  if (serverState.phase === "booster_docking") {
    targetX = Number(serverState.booster_x || 0);
    targetY = Number(config.booster_y ?? 185);
    xLimit = Number(config.booster_x_ok ?? 0);
    kind = "booster_dock";
  } else if (serverState.phase === "fuel_docking") {
    targetX = Number(serverState.fuel_stand_x || 0);
    targetY = Number(config.fuel_stand_y ?? 250);
    xLimit = Number(config.fuel_stand_x_ok ?? 0);
    kind = "fuel_stand_dock";
  } else if (serverState.phase === "docking") {
    targetX = Number(serverState.airlock_x || 0);
    targetY = Number(config.dock_y ?? 90);
    xLimit = Number(config.dock_x_ok ?? 0);
    kind = "dock";
  }

  if (!kind) return;
  const dx = Math.abs(local.ship.x - targetX);
  const dy = Math.abs(local.ship.y - targetY);
  if (dy < 35 && dx < xLimit) {
    sendEventOnce(send, kind);
  } else if (local.sentEvent === kind) {
    local.sentEvent = null;
  }
}

function collectMinerals(send) {
  if (local.manipLen <= 2) return;
  const tipX = local.ship.x;
  const tipY = local.ship.y - SHIP_R - local.manipLen;
  for (const mineral of local.minerals || []) {
    const id = Number(mineral.id);
    if (local.collectedMinerals.has(id)) continue;
    const dx = tipX - Number(mineral.x || 0);
    const dy = tipY - Number(mineral.y || 0);
    if (Math.hypot(dx, dy) < MINERAL_R + 6) {
      local.collectedMinerals.add(id);
      local.event = mineral.kind === "rare" ? "mineral_rare" : "mineral_gold";
      local.eventToken = `${local.event}:${++eventSeq}`;
      send(shipSnapshot("mineral_collect", { mineral_id: id, mineral_kind: mineral.kind }));
      local.minerals = local.minerals.filter((item) => item.id !== id);
      return;
    }
  }
}

function sendDamageEvent(send) {
  if (!local.event) return;
  send({
    ...shipSnapshot(local.event),
  });
}

// Cache stage-start data delivered by the server. Moment-to-moment movement is
// owned by the browser, so these plans are the only continuous object source.
function applyServerPlans(serverState) {
  if (Array.isArray(serverState.asteroid_plan) && serverState.asteroid_plan.length > 0) {
    asteroidPlans.set(localKey(serverState), serverState.asteroid_plan);
  }
  if (Array.isArray(serverState.mineral_plan) && serverState.mineral_plan.length > 0) {
    mineralPlans.set(localKey(serverState), serverState.mineral_plan);
  }
  if (serverState.route_config) {
    routeConfigs.set(localKey(serverState), serverState.route_config);
  }
}

// Prepare or reuse the local simulation state for the current stage.
function ensureLocalState(serverState) {
  if (!local || local.key !== localKey(serverState)) {
    resetLocalFromServer(serverState);
  }
}

function nextStepScale() {
  const now = performance.now();
  const deltaMs = Math.min(100, Math.max(16, now - lastFrameMs));
  lastFrameMs = now;
  return deltaMs / 33;
}

// Pull server-authoritative resource increases only on fresh packets. This
// prevents old sparse snapshots from overwriting local HP/FUEL simulation.
function syncServerResources(serverState) {
  const serverTick = Number(serverState.tick || 0);
  if (serverTick === local.lastResourceSyncTick) return;
  if (serverState.event === "fuel_stand_refuel" || serverState.phase === "launching") {
    local.fuel = Number(serverState.fuel ?? local.fuel);
  }
  if (serverState.phase === "launching") {
    local.hp = Number(serverState.hp ?? local.hp);
  }
  local.lastResourceSyncTick = serverTick;
}

function stepShip(serverState, stepScale) {
  local.phase = serverState.phase;
  local.event = null;
  local.eventToken = null;
  local.renderTick += stepScale;
  local.stageTick += stepScale;
  if (serverState.phase === "launching") {
    moveLaunching(stepScale);
  } else {
    moveShip(stepScale);
    updateManipulator(stepScale);
  }
}

function stepRouteObjects(serverState, stepScale) {
  const paused = routePaused(serverState.phase);
  const spacePaused = spaceMotionPaused(serverState.phase);
  const routeActive = !paused && serverState.phase !== "launching";
  if (routeActive) {
    local.routeTick = Math.max(local.routeTick, Math.max(0, Number(serverState.stage_tick || 0) - 90));
    local.routeTick += stepScale;
    spawnPlannedAsteroids();
    spawnPlannedMinerals();
    if (!spacePaused) {
      moveAsteroids(stepScale, !!serverState.fast_scroll);
    }
  }
  moveMinerals(stepScale, !!serverState.fast_scroll);
  return { routeActive, spacePaused };
}

function resolveLocalEvents(serverState, send, stepScale) {
  if (serverState.phase === "playing") collideAsteroids(local.asteroids);
  if (mineralCollectPhase(serverState.phase)) collectMinerals(send);
  if (local.invincibleTicks > 0) local.invincibleTicks = Math.max(0, local.invincibleTicks - stepScale);

  if (local.fuel <= 0) {
    local.fuel = 0;
    local.event = "fuel_empty";
    local.eventToken = `${local.event}:${++eventSeq}`;
  } else if (local.hp <= 0) {
    local.event = "gameover";
    local.eventToken = `${local.event}:${++eventSeq}`;
  }
  sendDamageEvent(send);
  maybeSendDockEvent(serverState, send);
}

function buildRenderState(serverState, routeActive, spacePaused) {
  return {
    ...serverState,
    ship: {
      ...serverState.ship,
      x: local.ship.x,
      y: local.ship.y,
      vx: local.ship.vx,
      vy: local.ship.vy,
      invincible: local.invincibleTicks > 0,
    },
    hp: Math.round(local.hp),
    fuel: local.fuel,
    progress: routeActive ? Math.min(100, (local.routeTick / CRUISE_TICKS) * 100) : serverState.progress,
    tick: local.renderTick,
    stage_tick: local.stageTick,
    manip_len: local.manipLen,
    route_paused: spacePaused,
    asteroids: spacePaused ? [] : local.asteroids,
    minerals: local.minerals,
    event_token: local.eventToken,
    event: local.event || serverState.event,
  };
}

export function applyClientGame(serverState, send) {
  applyServerPlans(serverState);
  if (!activeLocalPhase(serverState.phase)) {
    local = null;
    return serverState;
  }

  ensureLocalState(serverState);
  const stepScale = nextStepScale();
  syncServerResources(serverState);
  stepShip(serverState, stepScale);
  const { routeActive, spacePaused } = stepRouteObjects(serverState, stepScale);
  resolveLocalEvents(serverState, send, stepScale);
  return buildRenderState(serverState, routeActive, spacePaused);
}
