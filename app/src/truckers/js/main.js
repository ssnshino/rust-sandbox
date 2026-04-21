import { ensureAudio, resumeAudio, updateThrusterAudio } from "./audio.js";
import { applyLang, loadI18n } from "./i18n.js";
import { initInput, bindInputHandlers } from "./input.js";
import { createTruckersUi, bindUiActions, showScreen } from "./ui.js";
import { connect, send } from "./ws.js";
import { renderTitleBg } from "./render.js";

// Input now stays local; game.js reads keys directly every animation frame.
function sendKeys() {
}

// Boot the truckers client: i18n, DOM UI, input handlers and WebSocket.
async function bootTruckers() {
  await loadI18n();
  createTruckersUi();
  applyLang();

  bindUiActions(send);
  bindInputHandlers({
    sendKeysFn: sendKeys,
    updateThrusterAudioFn: updateThrusterAudio,
    ensureAudioFn: ensureAudio,
    resumeAudioFn: resumeAudio,
  });
  initInput();

  showScreen("title");
  renderTitleBg();
  connect();
}

bootTruckers().catch((err) => {
  console.error(err);
});
