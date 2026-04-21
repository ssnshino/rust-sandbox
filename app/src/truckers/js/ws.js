import { beep, noiseBurst } from "./audio.js";
import { lang, ui } from "./state.js";
import { recordNetworkFrame } from "./render.js";
import { renderGame, renderTitleBg } from "./render.js";
import { applyClientGame, resetClientGame } from "./game.js";
import { getLangDict, t, tf } from "./i18n.js";
import { setNextRouteState, showClear, showGm, showScreen, renderScoreList, updateGameoverUi } from "./ui.js";

let ws = null;
let pendingMessages = [];
let lastEvent = null;
let prevPhase = null;
let lastFastScroll = false;
let latestPlayState = null;
let animationFrameId = null;

let statusCounter = 0;

// Pick one random line from an array of GM phrases.
function pick(arr) {
  return arr[Math.floor(Math.random() * arr.length)];
}

// Open the gameplay WebSocket and reconnect automatically if it drops.
export function connect() {
  const proto = location.protocol === "https:" ? "wss" : "ws";
  ws = new WebSocket(`${proto}://${location.host}/ws/truckers`);
  ws.onopen = () => {
    if (pendingMessages.length) {
      for (const msg of pendingMessages) {
        ws.send(msg);
      }
      pendingMessages = [];
    }
  };
  ws.onmessage = (event) => handleMsg(JSON.parse(event.data));
  ws.onclose = () => {
    ws = null;
    setTimeout(connect, 2000);
  };
}

// Send a JSON message to the server when the socket is ready.
export function send(obj) {
  const payload = JSON.stringify(obj);
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(payload);

    // @@@ キー入力、ボタン押下時
//    console.log(obj);
//    statusCodeView(payload.phase + ', cnt: ' + statusCounter);
//    statusCounter++;
    //

    return;
  }
  if (!ws || ws.readyState === WebSocket.CLOSING || ws.readyState === WebSocket.CLOSED) {
    connect();
  }
  pendingMessages.push(payload);
}

// @@@ 20260420 add.
function statusCodeView(code) {
//  const msg = document.getElementById('status-code');
  const statusCode = document.getElementById("status-code");
  statusCode.textContent = 'DEBUG: ' + code;
//  console.log(state.phase);
  // @@@
}
function statusPhaseView(code) {
  const statusPhase = document.getElementById("status-phase");
  statusPhase.textContent = 'Phase: ' + code;
}

// Gameplay rendering must not depend on WebSocket packet frequency.
// The server sends coarse state snapshots; the browser simulates and draws
// each animation frame from the latest snapshot.
function startPlayLoop() {
  if (animationFrameId !== null) return;
  const frame = () => {
    animationFrameId = null;
    if (!latestPlayState) return;
    try {
      const renderState = applyClientGame(latestPlayState, send);
      renderGame(renderState);
      handlePlayEvents(latestPlayState, renderState);
    } catch (err) {
      console.error("truckers renderGame failed", err, latestPlayState);
      showGm(`render error: ${err?.message || err}`);
      return;
    }
    animationFrameId = requestAnimationFrame(frame);
  };
  animationFrameId = requestAnimationFrame(frame);
}

function stopPlayLoop() {
  latestPlayState = null;
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId);
    animationFrameId = null;
  }
}

function resetClientScene() {
  stopPlayLoop();
  resetClientGame();
  latestPlayState = null;
  lastEvent = null;
  prevPhase = null;
  lastFastScroll = false;
}

function handlePlayEvents(msg, renderState) {
  const eventKey = renderState.event_token || renderState.event;
  if (renderState.event && eventKey !== lastEvent) {
    if (renderState.event === "damage") {
      showGm(pick(t("gm_hit")));
      noiseBurst(0.13, 0.09);
      beep(150, 0.08, "sawtooth", 0.035);
    }
    if (renderState.event === "delivery") showGm(pick(t("gm_ok")));
    if (renderState.event === "booster_call") showGm(pick(t("gm_booster")));
    if (renderState.event === "booster_attach") showGm(pick(t("gm_booster_ok")));
    if (renderState.event === "booster_fee_short") showGm(pick(t("gm_booster_fee_short")));
    if (renderState.event === "fuel_stand_call") showGm(pick(t("gm_fuel_stand")));
    if (renderState.event === "fuel_stand_refuel") showGm(pick(t("gm_fuel_stand_ok")));
    if (renderState.event === "fuel_stand_fee_short") showGm(pick(t("gm_fuel_stand_short")));
    if (renderState.event === "late_fine") showGm(pick(t("gm_late_fine")));
    if (renderState.event === "fuel_empty") showGm(pick(t("gm_fuel_empty")));
    if (renderState.event === "mineral_gold") {
      showGm(lang === "ja" ? "金鉱石ゲット！" : "Gold ore get!");
      if (renderState.event_token) beep(1040, 0.07, "square", 0.05);
    }
    if (renderState.event === "mineral_rare") {
      showGm(lang === "ja" ? "レアメタルきた！" : "Rare metal!");
      if (renderState.event_token) beep(1320, 0.09, "triangle", 0.06);
    }
  }
  if (msg.phase === "booster_docking" && prevPhase !== "booster_docking") {
    showGm(pick(t("gm_booster")));
  }
  if (msg.phase === "fuel_docking" && prevPhase !== "fuel_docking") {
    showGm(pick(t("gm_fuel_stand")));
  }
  if (msg.phase === "docking" && prevPhase !== "docking") {
    showGm(pick(t("gm_dock")));
  }
  if (msg.fast_scroll && !lastFastScroll) {
    showGm(pick(t("gm_fast")));
  }
  lastEvent = eventKey;
  prevPhase = msg.phase;
  lastFastScroll = !!msg.fast_scroll;
}

// Consume state packets from the server and route them to the proper screen.
function handleMsg(msg) {
  if (msg.type !== "state") return;
  recordNetworkFrame();

  // @@@ 202600420 add.
  //statusCodeView(msg.phase + ', cnt: ' + statusCounter);
  //statusCounter++;
  statusPhaseView('[' + msg.phase + ']');

  switch (msg.phase) {
    case "title":
      // ゲームタイトル画面
      if (ui.clearVisible || ui.screen === "route") {
        break;
      }
      resetClientScene();
      showScreen("title");
      renderTitleBg();
      renderScoreList("title-scores", msg.scores || []);
      prevPhase = msg.phase;
      break;
    case "launching":
      // ステージスタート
    case "playing":
      // ゲーム画面
    case "booster_docking":
      // 高速ブースタードッキングステーション
    case "fuel_docking":
      // 燃料補給ステーション
    case "docking":
      // ゴール宇宙ステーション
      showScreen("playing");
      latestPlayState = msg;
      startPlayLoop();
      break;
    case "stage_clear":
      // 宇宙ステーションとのドッキング完了「ステージクリア」
      stopPlayLoop();
      resetClientGame();
      if (prevPhase !== "stage_clear") {
        showScreen("playing");
      }
      renderGame(msg);
      if (prevPhase !== "stage_clear") {
        showClear(
          tf("clear_title", {
            from_symbol: msg.from.symbol,
            to_symbol: msg.to.symbol,
          }),
          msg.next_stage_booster
            ? t("clear_booster_notice")
            : tf("clear_score_only", { score: Number(msg.score || 0).toLocaleString() }),
          getLangDict(),
          msg,
        );
        setNextRouteState({ stage: ((msg.stage || 0) + 1) % 12 });
      }
      prevPhase = msg.phase;
      break;
    case "lap_clear":
      // 全12ステージクリア
      stopPlayLoop();
      resetClientGame();
      if (prevPhase !== "lap_clear") {
        showScreen("playing");
      }
      renderGame(msg);
      if (prevPhase !== "lap_clear") {
        showClear(t("lap_title"), tf("lap_sub", { round: msg.round }), getLangDict(), msg);
        setNextRouteState({ stage: 0 });
      }
      prevPhase = msg.phase;
      break;
    case "gameover":
      // ゲームオーバーがm値
      resetClientScene();
      showScreen("gameover");
      updateGameoverUi(msg);
      renderScoreList("go-scores", msg.scores || []);
      prevPhase = msg.phase;
      break;
  }
}
