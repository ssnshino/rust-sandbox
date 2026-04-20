import { ensureAudio, resumeAudio } from "./audio.js";
import { applyLang, t, tf, toggleLang } from "./i18n.js";
import { isTouchDevice, lang, setPlayerName, touchControls, ui } from "./state.js";

let gmTimeout = null;
let routeTimeout = null;
let clearTimeoutId = null;
let sendMessage = () => {};
let fallbackBound = false;
let routePendingMode = null;
let routePendingName = null;
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

function bonusRows(state) {
    const rows = [];
    if (!state) return rows;
    rows.push({
        label: t("clear_delivery_bonus"),
        value: `+${Number(state.delivery_score || 0).toLocaleString()}`,
    });
    rows.push({
        label: t("clear_dock_bonus"),
        value: `+${Number(state.dock_bonus || 0).toLocaleString()}`,
    });
    rows.push({
        label: t("clear_cargo_bonus"),
        value: `+${Number(state.cargo_bonus || 0).toLocaleString()}`,
    });
    if (state.hp_bonus) {
        rows.push({
            label: t("clear_hp_bonus"),
            value: `+${Number(state.hp_bonus || 0).toLocaleString()}`,
        });
    }
    if (state.late_fine) {
        rows.push({
            label: t("clear_late_fine"),
            value: `-${Number(state.late_fine || 0).toLocaleString()}`,
            negative: true,
        });
    }
    if (state.repair_cost) {
        rows.push({
            label: t("clear_repair_cost"),
            value: `-${Number(state.repair_cost || 0).toLocaleString()}$`,
            negative: true,
        });
    }
    if (state.fuel_cost) {
        rows.push({
            label: t("clear_fuel_cost"),
            value: `-${Number(state.fuel_cost || 0).toLocaleString()}$`,
            negative: true,
        });
    }
    rows.push({
        label: t("clear_total_score"),
        value: Number(state.score || 0).toLocaleString(),
    });
    return rows;
}

function meterBar(value, max = 100, cells = 10) {
    const filled = Math.max(0, Math.min(cells, Math.round((Number(value || 0) / max) * cells)));
    return "■".repeat(filled) + "□".repeat(cells - filled);
}

function speedKmh(state) {
    const vy = Number(state?.ship?.vy || 0);
    if (vy < 0) {
        return Math.round(Math.min(120, Math.abs(vy / 5.5) * 120));
    }
    if (vy > 0) {
        return Math.round(Math.min(30, Math.abs(vy / 5.5) * 30));
    }
    return 0;
}

function bonusHtml(state) {
    return bonusRows(state)
        .map(
            (row) => `
                <div class="clear-breakdown-row${row.negative ? " negative" : ""}">
                    <span class="label">${row.label}</span>
                    <span class="value">${row.value}</span>
                </div>
            `,
        )
        .join("");
}

function startGameDirect() {
    ensureAudio();
    resumeAudio();
    const nextName = currentName();
    setPlayerName(nextName);
    const input = el("name-input");
    if (input) input.value = nextName;
    routePendingState = { stage: 0 };
    routePendingMode = "start";
    routePendingName = nextName;
    showRouteScreen(routePendingState);
}

function triggerContinueFlow() {
    if (!routePendingMode) {
        routePendingMode = "continue";
    }
    showRouteScreen(routePendingState || { stage: 0 });
}

function closeRouteScreen(sendAction = false) {
    clearTimeout(routeTimeout);
    routeTimeout = null;
    setVisible("screen-route", false, "flex");
    const nextMode = routePendingMode;
    const nextName = routePendingName;
    routePendingMode = null;
    routePendingName = null;
    routePendingState = null;
    if (!sendAction) {
        return;
    }
    if (nextMode === "start") {
        sendMessage({ type: "start", name: nextName || currentName() });
    } else if (nextMode === "continue") {
        sendMessage({ type: "continue" });
    }
}

function hideRouteScreen() {
    closeRouteScreen(true);
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
            routePendingMode = null;
            routePendingName = null;
            routePendingState = null;
            sendMessage({ type: "restart" });
            return;
        }
        // 配達完了「次のステーションへ！」ボタン
        if (target.closest("#btn-continue")) {
            event.preventDefault();
            clearTimeout(clearTimeoutId);
            clearTimeoutId = null;
            triggerContinueFlow();
            return;
        }
        // 航路マップ「出発」ボタン
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

/*
 * ステージクリア画面
 */
export function showClear(title, sub, i18n, state) {
    hideGm();
    clearTimeout(clearTimeoutId);
    ui.clearTitle = title;
    ui.clearSub = sub;
    const isBoosterNotice = sub === t("clear_booster_notice");
    const isLap = title === t("lap_title");

    // クリア時キャラセリフ表示
    ui.clearGirlLine =
        `<strong>${lang === "ja" ? "ミサキ" : "Misaki"}</strong>` +
        (isBoosterNotice ? t("clear_booster_girl") : isLap ? t("clear_lap_girl") : t("clear_girl"));
    ui.clearManLine =
        `<strong>${lang === "ja" ? "シノヤマ" : "Shinoyama"}</strong>` +
        (isBoosterNotice
            ? t("clear_booster_man")
            : isLap
                ? t("clear_man_lap") || t("clear_man")
                : t("clear_man"));
    ui.clearVisible = true;
    ui.canContinue = true;

    text("cl-title", ui.clearTitle);
    text("cl-sub", ui.clearSub);
    html("cl-breakdown", bonusHtml(state));
    html("clear-girl-line", ui.clearGirlLine);
    html("clear-man-line", ui.clearManLine);
    setVisible("btn-continue", true, "inline-flex");
    setVisible("screen-clear", true, "flex");
    clearTimeoutId = setTimeout(() => {
        // 次ステージへのボタンクリック
        //triggerContinueFlow();
    }, 10000); // - 10 sec.
}

export function setNextRouteState(state) {
    routePendingState = state;
    routePendingMode = "continue";
    routePendingName = null;
}

/*
 * 現在ステージ表示画面
 * 第一宇宙ステーションから第十二宇宙ステーションまでを円表示
 */
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
        //hideRouteScreen();
    }, 10000);
}

/*
 * 画面表示
 *
 * 1 = title スタート画面
 * 2 = playing ゲーム画面
 * 3 = clear ステージクリア画面
 * 4 = gameover ゲームオーバー画面
 *
 */
export function showScreen(name) {
    ui.screen = name;
    ui.clearVisible = false;
    ui.canContinue = false;
    ui.showTopButtons = name !== "playing";
    ui.showTouchControls = name === "playing" && isTouchDevice;

    setVisible("screen-title", name === "title", "flex");
    setVisible("screen-gameover", name === "gameover", "flex");
    setVisible("screen-clear", false, "flex");
    // ゲーム中画面
    if (name !== "playing") {
        hideGm();
        // ルート一覧非表示
        closeRouteScreen(false);
    }
    // ゲーム中以外画面 and クリアタイムアウト
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

/*
 * GM(game manager) コメント欄表示
 */
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
    ui.hudLife =
        `<div class="hud-meter-line"><span class="hud-meter-label">${t("damage_lbl")}</span><span>${Number(state.hp || 0)}%</span></div>` +
        `<div class="hud-meter-bar hull">${meterBar(state.hp)}</div>` +
        `<span class="hud-minerals">` +
        `<span>🟡x${Number(state.gold_count || 0)}</span>` +
        `<span>🟣x${Number(state.rare_count || 0)}</span>` +
        `<span class="hud-money">${t("money_lbl")}${Number(state.money || 0).toLocaleString()}</span>` +
        `</span>`;
    ui.hudRound =
        `<div class="hud-meter-line"><span class="hud-meter-label">${t("fuel_lbl")}</span><span>F ${Number(state.fuel || 0).toFixed(2)}% E</span></div>` +
        `<div class="hud-meter-bar fuel">${meterBar(state.fuel)}</div>` +
        `<div class="hud-speed-line"><span>${t("speed_lbl")}${speedKmh(state)}km/h</span><span>${t("round_lbl")}${state.round} ${(state.stage + 1)}/12</span></div>`;

    text("hud-score", ui.hudScore);
    html("hud-life", ui.hudLife);
    html("hud-round", ui.hudRound);
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
