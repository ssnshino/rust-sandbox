import { beep } from "./audio.js";
import { lang } from "./state.js";
import { renderGame, renderTitleBg } from "./render.js";
import { getLangDict, t, tf } from "./i18n.js";
import { showClear, showGm, showScreen, renderScoreList, updateGameoverUi } from "./ui.js";

let ws = null;
let pendingMessages = [];
let lastEvent = null;
let prevPhase = null;
let lastFastScroll = false;

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
    return;
  }
  if (!ws || ws.readyState === WebSocket.CLOSING || ws.readyState === WebSocket.CLOSED) {
    connect();
  }
  pendingMessages.push(payload);
}

// Consume state packets from the server and route them to the proper screen.
function handleMsg(msg) {
  if (msg.type !== "state") return;
  switch (msg.phase) {
    case "title":
      showScreen("title");
      renderTitleBg();
      renderScoreList("title-scores", msg.scores || []);
      break;
    case "launching":
    case "playing":
    case "booster_docking":
    case "docking":
      showScreen("playing");
      try {
        renderGame(msg);
      } catch (err) {
        console.error("truckers renderGame failed", err, msg);
        showGm(`render error: ${err?.message || err}`);
        return;
      }
      if (msg.event && msg.event !== lastEvent) {
        if (msg.event === "damage") showGm(pick(t("gm_hit")));
        if (msg.event === "delivery") showGm(pick(t("gm_ok")));
        if (msg.event === "booster_call") showGm(pick(t("gm_booster")));
        if (msg.event === "booster_attach") showGm(pick(t("gm_booster_ok")));
        if (msg.event === "mineral_gold") {
          showGm(lang === "ja" ? "金鉱石ゲット！" : "Gold ore get!");
          beep(1040, 0.07, "square", 0.05);
        }
        if (msg.event === "mineral_rare") {
          showGm(lang === "ja" ? "レアメタルきた！" : "Rare metal!");
          beep(1320, 0.09, "triangle", 0.06);
        }
      }
      if (msg.phase === "booster_docking" && prevPhase !== "booster_docking") {
        showGm(pick(t("gm_booster")));
      }
      if (msg.phase === "docking" && prevPhase !== "docking") {
        showGm(pick(t("gm_dock")));
      }
      if (msg.fast_scroll && !lastFastScroll) {
        showGm(pick(t("gm_fast")));
      }
      lastEvent = msg.event;
      prevPhase = msg.phase;
      lastFastScroll = !!msg.fast_scroll;
      break;
    case "stage_clear":
      showScreen("playing");
      renderGame(msg);
      showClear(
        tf("clear_title", {
          from_symbol: msg.from.symbol,
          to_symbol: msg.to.symbol,
        }),
        msg.next_stage_booster
          ? t("clear_booster_notice")
          : t("score_lbl") + (msg.score || 0).toLocaleString(),
        getLangDict(),
      );
      showGm(msg.next_stage_booster ? t("clear_booster_notice") : pick(t("gm_ok")));
      prevPhase = msg.phase;
      break;
    case "lap_clear":
      showScreen("playing");
      renderGame(msg);
      showClear(t("lap_title"), tf("lap_sub", { round: msg.round }), getLangDict());
      showGm(pick(t("gm_lap")));
      prevPhase = msg.phase;
      break;
    case "gameover":
      showScreen("gameover");
      updateGameoverUi(msg);
      renderScoreList("go-scores", msg.scores || []);
      break;
  }
}
