import { lang, setLang, ui } from "./state.js";

let I18N = {};

// Return the active language dictionary, with Japanese as the fallback.
export function getLangDict() {
  return I18N[lang] || I18N.ja || {};
}

// Resolve one i18n key from the loaded dictionary.
export function t(key) {
  return getLangDict()[key] ?? key;
}

// Interpolate placeholders like {name} in i18n templates.
export function tf(key, params = {}) {
  const template = t(key);
  if (typeof template !== "string") {
    return key;
  }
  return template.replace(/\{(\w+)\}/g, (_, token) => params[token] ?? "");
}

// Pick the correct station display name for the current language.
export function localizedStationName(station) {
  return lang === "ja" ? station.name_ja : station.name_en;
}

// Load the external truckers.i18n.json file before the game bootstraps.
export async function loadI18n() {
  const res = await fetch("/truckers.i18n.json");
  if (!res.ok) {
    throw new Error(`failed to load i18n json: ${res.status}`);
  }
  I18N = await res.json();
}

// Keep DOM UI labels and document metadata in sync with the active language.
export function applyLang() {
  ui.langButton = lang === "ja" ? "🇯🇵 / 🇬🇧" : "🇬🇧 / 🇯🇵";
  document.documentElement.lang = lang;
  document.title = t("title");
}

// Toggle between Japanese and English.
export function toggleLang() {
  setLang(lang === "ja" ? "en" : "ja");
  applyLang();
}

