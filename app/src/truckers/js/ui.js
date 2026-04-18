import { ensureAudio, resumeAudio } from "./audio.js";
import { applyLang, t, tf, toggleLang } from "./i18n.js";
import { isTouchDevice, lang, setPlayerName, touchControls, ui } from "./state.js";

let gmTimeout = null;
let routeTimeout = null;
let clearTimeoutId = null;
let sendMessage = () => {};
let fallbackBound = false;
let routePendingAction = null;
let routePendingState = null;

const STATION_SYMBOLS = ["♈", "♉", "♊", "♋", "♌", "♍", "♎", "♏", "♐", "♑", "♒", "♓"];

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

function stationNames() {
    return t("station_names") || [];
}

function routeStationHtml(index, currentIndex, nextIndex) {
    const names = stationNames();
    const classes = ["route-station"];
    let tag = "&nbsp;";

    if (index === currentIndex) {
        classes.push("current");
        tag = t("route_current");
    } else if (index === nextIndex) {
        classes.push("next");
        tag = "";
    }

    const angle = ((index / 12) * Math.PI * 2) - Math.PI / 2;
    const radius = 148;
    const x = Math.cos(angle) * radius;
    const y = Math.sin(angle) * radius;

    return `
        <div class="${classes.join(" ")}" style="left: calc(50% + ${x.toFixed(1)}px); top: calc(50% + ${y.toFixed(1)}px);">
            <div class="num">${index + 1}</div>
            <div class="symbol">${STATION_SYMBOLS[index]}</div>
            <div class="name">${names[index] || ""}</div>
            <div class="tag">${tag}</div>
        </div>
    `;
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
    routePendingState = { stage: 0 };
    routePendingAction = () => {
        sendMessage({ type: "start", name: nextName });
    };
    showRouteScreen(routePendingState);
}

function triggerContinueFlow() {
    if (!routePendingAction) {
        routePendingAction = () => {
            sendMessage({ type: "continue" });
        };
    }
    showRouteScreen(routePendingState || { stage: 0 });
}

function hideRouteScreen() {
    clearTimeout(routeTimeout);
    routeTimeout = null;
    setVisible("screen-route", false, "flex");
    const nextAction = routePendingAction;
    routePendingAction = null;
    routePendingState = null;
    if (typeof nextAction === "function") {
        nextAction();
    }
}

function hideGm() {
    clearTimeout(gmTimeout);
    ui.gmVisible = false;
    text("gm-comment", "");
    el("gm-comment")?.classList.remove("show");
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
            clearTimeout(clearTimeoutId);
            clearTimeoutId = null;
            clearTimeout(routeTimeout);
            routeTimeout = null;
            routePendingAction = null;
            routePendingState = null;
            sendMessage({ type: "restart" });
            return;
        }
        if (target.closest("#btn-continue")) {
            event.preventDefault();
            clearTimeout(clearTimeoutId);
            clearTimeoutId = null;
            triggerContinueFlow();
            return;
        }
        if (target.closest("#btn-route-go")) {
            event.preventDefault();
            hideRouteScreen();
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
    hideGm();
    clearTimeout(clearTimeoutId);
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
    clearTimeoutId = setTimeout(() => {
        triggerContinueFlow();
    }, 10000);
}

export function setNextRouteState(state) {
    routePendingState = state;
    routePendingAction = () => {
        sendMessage({ type: "continue" });
    };
}

export function showRouteScreen(state) {
    hideGm();
    ui.clearVisible = false;
    ui.canContinue = false;
    const currentIndex = Number((state && state.stage) || 0) % 12;
    const nextIndex = (currentIndex + 1) % 12;
    const routeMap = `
        <div class="route-core">
            <div class="route-core-title">${lang === "ja" ? "アステロイドベルト" : "ASTEROID BELT"}</div>
            <div class="route-core-sub">${lang === "ja" ? "12宇宙ステーション" : "12 STATIONS"}</div>
        </div>
        ${STATION_SYMBOLS.map((_, index) => routeStationHtml(index, currentIndex, nextIndex)).join("")}
    `;

    text("route-title", t("route_title"));
    text(
        "route-sub",
        tf("route_sub", {
            current_num: currentIndex + 1,
            current: stationNames()[currentIndex] || "",
            next_num: nextIndex + 1,
            next: stationNames()[nextIndex] || "",
        }),
    );
    html("route-map", routeMap);
    text("btn-route-go", t("btn_route_go"));
    setVisible("screen-clear", false, "flex");
    setVisible("screen-title", false, "flex");
    setVisible("screen-gameover", false, "flex");
    setVisible("screen-route", true, "flex");

    clearTimeout(routeTimeout);
    routeTimeout = setTimeout(() => {
        hideRouteScreen();
    }, 10000);
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
    if (name !== "playing") {
        hideGm();
        hideRouteScreen();
    }
    if (name !== "playing" && clearTimeoutId) {
        clearTimeout(clearTimeoutId);
        clearTimeoutId = null;
    }
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
    text("btn-route-go", t("btn_route_go"));
    text("btn-back", `← ${t("back")}`);
    text("btn-lang", ui.langButton);
    text("touch-hint", "MANIP");
    text("go-title", t("go_title"));
    text("title-heading", t("title"));
    text("title-sub", t("title_sub"));
    text("title-help", t("controls"));
}
