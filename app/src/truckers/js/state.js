function safeGet(key, fallback = "") {
    try {
        return localStorage.getItem(key) || fallback;
    } catch (_err) {
        return fallback;
    }
}

function safeSet(key, value) {
    try {
        localStorage.setItem(key, value);
    } catch (_err) {
        // Storage can be blocked by privacy settings; keep the app running.
    }
}

export const ui = {
    screen: "title",
    playerName: safeGet("truckers_player_name", ""),
    langButton: "🇯🇵 / 🇬🇧",
    langCode: safeGet("gc_lang", "ja"),
    titleScores: [],
    gameoverScores: [],
    goScoreText: "",
    goRankText: "",
    clearVisible: false,
    canContinue: false,
    clearTitle: "",
    clearSub: "",
    clearGirlLine: "",
    clearManLine: "",
    hudScore: "",
    hudLife: "",
    hudRound: "",
    gmComment: "",
    gmVisible: false,
    showTopButtons: true,
    showTouchControls: false,
};

export let lang = ui.langCode;
export let playerName = ui.playerName;

export function setLang(nextLang) {
    lang = nextLang;
    ui.langCode = nextLang;
    safeSet("gc_lang", nextLang);
}

export function setPlayerName(nextName) {
    playerName = nextName;
    ui.playerName = nextName;
    safeSet("truckers_player_name", nextName);
}

export const canvas = document.getElementById("canvas");
export const ctx = canvas.getContext("2d");
//export const W = 540;
//export const H = 540;
// @@@ 202604118 canvas size change 540 -> 360, 540 -> 600
export const W = 375;
export const H = 640;
export const BOOSTER_Y = 185;
export const FUEL_STAND_Y = 250;

export const keys = {
    up: false,
    down: false,
    left: false,
    right: false,
    manip: false,
};

export const isTouchDevice = "ontouchstart" in window;
export const vstickArea = document.getElementById("vstick-area");
export const vstickKnob = document.getElementById("vstick-knob");
export const btnManip = document.getElementById("btn-manip");
export const actionButtons = document.getElementById("action-buttons");
export const touchControls = document.getElementById("touch-controls");
export const fpsMeter = document.getElementById("fps-meter");
