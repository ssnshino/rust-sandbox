import { keys } from "./state.js";

let audioCtx = null;
let mainEngineOsc = null;
let mainEngineGain = null;
let sideThrusterLast = { left: false, right: false, down: false };

// Lazily create the audio context on first interaction.
export function ensureAudio() {
  if (audioCtx) return;
  const AC = window.AudioContext || window.webkitAudioContext;
  if (!AC) return;
  audioCtx = new AC();
}

// Resume audio if the browser auto-suspended the context.
export function resumeAudio() {
  if (audioCtx && audioCtx.state === "suspended") audioCtx.resume();
}

// Play a simple pitched synth beep for pickup and UI cues.
export function beep(freq = 880, duration = 0.08, type = "square", gain = 0.06) {
  ensureAudio();
  if (!audioCtx) return;
  resumeAudio();
  const osc = audioCtx.createOscillator();
  const g = audioCtx.createGain();
  osc.type = type;
  osc.frequency.value = freq;
  g.gain.setValueAtTime(gain, audioCtx.currentTime);
  g.gain.exponentialRampToValueAtTime(0.0001, audioCtx.currentTime + duration);
  osc.connect(g).connect(audioCtx.destination);
  osc.start();
  osc.stop(audioCtx.currentTime + duration);
}

// Play a short noise burst used for side thruster pops.
export function noiseBurst(duration = 0.06, gain = 0.045) {
  ensureAudio();
  if (!audioCtx) return;
  resumeAudio();
  const len = Math.floor(audioCtx.sampleRate * duration);
  const buf = audioCtx.createBuffer(1, len, audioCtx.sampleRate);
  const data = buf.getChannelData(0);
  for (let i = 0; i < len; i++) {
    data[i] = (Math.random() * 2 - 1) * (1 - i / len);
  }
  const src = audioCtx.createBufferSource();
  const g = audioCtx.createGain();
  src.buffer = buf;
  g.gain.value = gain;
  src.connect(g).connect(audioCtx.destination);
  src.start();
}

// Start or stop the continuous main engine hum.
function setMainEngine(on) {
  ensureAudio();
  if (!audioCtx) return;
  resumeAudio();
  if (on) {
    if (!mainEngineOsc) {
      mainEngineOsc = audioCtx.createOscillator();
      mainEngineGain = audioCtx.createGain();
      mainEngineOsc.type = "sawtooth";
      mainEngineOsc.frequency.value = 92;
      mainEngineGain.gain.value = 0.0001;
      mainEngineOsc.connect(mainEngineGain).connect(audioCtx.destination);
      mainEngineOsc.start();
    }
    mainEngineGain.gain.cancelScheduledValues(audioCtx.currentTime);
    mainEngineGain.gain.linearRampToValueAtTime(0.04, audioCtx.currentTime + 0.05);
  } else if (mainEngineGain) {
    mainEngineGain.gain.cancelScheduledValues(audioCtx.currentTime);
    mainEngineGain.gain.linearRampToValueAtTime(0.0001, audioCtx.currentTime + 0.08);
  }
}

// Reflect the current input state into engine and thruster sound effects.
export function updateThrusterAudio() {
  setMainEngine(!!keys.up);
  for (const key of ["left", "right", "down"]) {
    if (keys[key] && !sideThrusterLast[key]) {
      noiseBurst();
    }
    sideThrusterLast[key] = !!keys[key];
  }
}

