import { ensureAudio, resumeAudio } from "./audio.js";
import { applyLang, t, tf, toggleLang } from "./i18n.js";
import { isTouchDevice, lang, setPlayerName, touchControls, ui } from "./state.js";

let gmTimeout = null;
let sendMessage = () => {};
let fallbackBound = false;

function el(id) {
    return document.getElementById(id);
}

function setVisible(id, visible, display = "") {
    const node = el(id);
    if (!node) return;
    node.style.display = visible ? display : "none";
}

function text(id, value) {
    const node = el(id);
    if (node) node.textContent = value;
}

function html(id, value) {
    const node = el(id);
    if (node) node.innerHTML = value;
}

function currentName() {
    const input = el("name-input");
    return ((input && input.value) || ui.playerName || "").trim() || (lang === "ja" ? "野郎" : "Trucker");
}

function startGameDirect() {
    ensureAudio();
    resumeAudio();
    const nextName = currentName();
    setPlayerName(nextName);
    const input = el("name-input");
    if (input) input.value = nextName;
    sendMessage({ type: "start", name: nextName });
}

function bindFallbackDom() {
    if (fallbackBound) return;
    fallbackBound = true;

    document.addEventListener("click", (event) => {
        const target = event.target;
        if (!(target instanceof Element)) return;
        if (target.closest("#btn-start")) {
            event.preventDefault();
            startGameDirect();
            return;
        }
        if (target.closest("#btn-restart")) {
            event.preventDefault();
            sendMessage({ type: "restart" });
            return;
        }
        if (target.closest("#btn-continue")) {
            event.preventDefault();
            sendMessage({ type: "continue" });
            return;
        }
        if (target.closest("#btn-back")) {
            event.preventDefault();
            location.href = "/";
            return;
        }
        if (target.closest("#btn-lang")) {
            event.preventDefault();
            toggleLang();
            syncUiLanguage();
        }
    });

    document.addEventListener("keydown", (event) => {
        const target = event.target;
        if (!(target instanceof HTMLInputElement)) return;
        if (target.id !== "name-input") return;
        if (event.key === "Enter") {
            event.preventDefault();
            startGameDirect();
        }
    });
}

export function bindUiActions(send) {
    sendMessage = send;
    bindFallbackDom();
}

export function createTruckersUi() {
    bindFallbackDom();
    const input = el("name-input");
    if (input && !input.value) input.value = ui.playerName || "";
    syncUiLanguage();
}

export function showClear(title, sub, i18n) {
    ui.clearTitle = title;
    ui.clearSub = sub;
    const isBoosterNotice = sub === t("clear_booster_notice");
    const isLap = title === t("lap_title");
    ui.clearGirlLine =
        `<strong>${lang === "ja" ? "ミサキ" : "Misaki"}</strong>` +
        (isBoosterNotice ? t("clear_booster_girl") : isLap ? t("clear_lap_girl") : t("clear_girl"));
    ui.clearManLine =
        `<strong>${lang === "ja" ? "シノヤマ" : "Shinoyama"}</strong>` +
        (isBoosterNotice
            ? t("clear_booster_man")
            : isLap
                ? i18n[lang].clear_man_lap || t("clear_man")
                : t("clear_man"));
    ui.clearVisible = true;
    ui.canContinue = true;

    text("cl-title", ui.clearTitle);
    text("cl-sub", ui.clearSub);
    html("clear-girl-line", ui.clearGirlLine);
    html("clear-man-line", ui.clearManLine);
    setVisible("btn-continue", true, "inline-flex");
    setVisible("screen-clear", true, "flex");
}

export function showScreen(name) {
    ui.screen = name;
    ui.clearVisible = false;
    ui.canContinue = false;
    ui.showTopButtons = name !== "playing";
    ui.showTouchControls = name === "playing" && isTouchDevice;

    setVisible("screen-title", name === "title", "flex");
    setVisible("screen-gameover", name === "gameover", "flex");
    setVisible("screen-clear", false, "flex");
    setVisible("game-hud", name === "playing", "flex");
    setVisible("btn-back", name !== "playing", "block");
    setVisible("btn-lang", name !== "playing", "block");
    if (touchControls) {
        touchControls.style.display = ui.showTouchControls ? "flex" : "none";
    }
}

export function showGm(textValue) {
    ui.gmComment = textValue;
    ui.gmVisible = true;
    text("gm-comment", textValue);
    el("gm-comment")?.classList.add("show");
    clearTimeout(gmTimeout);
    gmTimeout = setTimeout(() => {
        ui.gmVisible = false;
        el("gm-comment")?.classList.remove("show");
    }, 2200);
}

function tableHtml(rows) {
    if (!rows.length) return "";
    return `<table>${rows
        .map(
            (score, index) =>
                `<tr><td class="rank">${index + 1}.</td><td>${score.name}</td><td>${Number(
                    score.score || 0,
                ).toLocaleString()}</td></tr>`,
        )
        .join("")}</table>`;
}

export function renderScoreList(id, scores) {
    const rows = (scores || []).slice(0, 5).map((score) => ({
        name: score.name,
        score: score.score,
    }));
    if (id === "title-scores") {
        ui.titleScores = rows;
    }
    if (id === "go-scores") {
        ui.gameoverScores = rows;
    }
    html(id, tableHtml(rows));
}

export function updateHudUi(state) {
    ui.hudScore = t("score_lbl") + (state.score || 0).toLocaleString();
    ui.hudLife = "❤️".repeat(state.hp) + "🖤".repeat(Math.max(0, 3 - state.hp));
    ui.hudRound = t("round_lbl") + state.round + "  " + t("lap_lbl") + state.lap + "  " + (state.stage + 1) + "/12";

    text("hud-score", ui.hudScore);
    text("hud-life", ui.hudLife);
    text("hud-round", ui.hudRound);
}

export function updateGameoverUi(state) {
    ui.goScoreText = t("score_lbl") + (state.score || 0).toLocaleString();
    ui.goRankText = state.rank ? tf("rank_msg", { rank: state.rank }) : t("no_rank");
    text("go-score", ui.goScoreText);
    text("go-rank", ui.goRankText);
}

export function syncUiLanguage() {
    applyLang();
    const input = el("name-input");
    if (input) input.placeholder = t("name_ph");
    text("btn-start", t("btn_start"));
    text("btn-restart", t("btn_restart"));
    text("btn-continue", t("btn_continue"));
    text("btn-back", `← ${t("back")}`);
    text("btn-lang", ui.langButton);
    text("touch-hint", "MANIP");
    text("go-title", t("go_title"));
    text("title-heading", t("title"));
    text("title-sub", t("title_sub"));
    text("title-help", t("controls"));
}
