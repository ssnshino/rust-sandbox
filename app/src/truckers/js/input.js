import { btnManip, actionButtons, isTouchDevice, keys, touchControls, vstickArea, vstickKnob } from "./state.js";

let sendKeys = () => {};
let updateThrusterAudio = () => {};
let ensureAudio = () => {};
let resumeAudio = () => {};
let vstickCenter = { x: 0, y: 0 };

const STICK_MAX = 44;

export function bindInputHandlers({ sendKeysFn, updateThrusterAudioFn, ensureAudioFn, resumeAudioFn }) {
  sendKeys = sendKeysFn;
  updateThrusterAudio = updateThrusterAudioFn;
  ensureAudio = ensureAudioFn;
  resumeAudio = resumeAudioFn;
}

// Translate keyboard codes into the internal input state object.
function applyKey(code, val) {
  const map = {
    ArrowUp: "up",
    KeyW: "up",
    ArrowDown: "down",
    KeyS: "down",
    ArrowLeft: "left",
    KeyA: "left",
    ArrowRight: "right",
    KeyD: "right",
    Space: "manip",
    KeyJ: "manip",
  };
  const key = map[code];
  if (key && keys[key] !== val) {
    keys[key] = val;
    return true;
  }
  return false;
}

// Convert touch-stick vector into four-way movement inputs.
function updateStick(touch) {
  const dx = touch.clientX - vstickCenter.x;
  const dy = touch.clientY - vstickCenter.y;
  const d = Math.sqrt(dx * dx + dy * dy);
  const c = Math.min(d, STICK_MAX);
  const nx = d > 0 ? dx / d : 0;
  const ny = d > 0 ? dy / d : 0;
  vstickKnob.style.transform = `translate(calc(-50% + ${nx * c}px),calc(-50% + ${ny * c}px))`;
  const threshold = 0.4;
  keys.up = ny < -threshold;
  keys.down = ny > threshold;
  keys.left = nx < -threshold;
  keys.right = nx > threshold;
  updateThrusterAudio();
  sendKeys();
}

// Register keyboard and touch controls for truckers.
export function initInput() {
  document.addEventListener("keydown", (e) => {
    if (applyKey(e.code, true)) {
      updateThrusterAudio();
      sendKeys();
    }
    if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", " "].includes(e.key)) {
      e.preventDefault();
    }
  });

  document.addEventListener("keyup", (e) => {
    if (applyKey(e.code, false)) {
      updateThrusterAudio();
      sendKeys();
    }
  });

  if (isTouchDevice) {
    vstickArea.style.display = "block";
    actionButtons.style.display = "flex";
    touchControls.style.display = "flex";
  }

  vstickArea.addEventListener(
    "touchstart",
    (e) => {
      e.preventDefault();
      ensureAudio();
      resumeAudio();
      const rect = vstickArea.getBoundingClientRect();
      vstickCenter = { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
      updateStick(e.touches[0]);
    },
    { passive: false },
  );

  vstickArea.addEventListener(
    "touchmove",
    (e) => {
      e.preventDefault();
      updateStick(e.touches[0]);
    },
    { passive: false },
  );

  document.addEventListener("touchend", () => {
    vstickKnob.style.transform = "translate(-50%,-50%)";
    keys.up = false;
    keys.down = false;
    keys.left = false;
    keys.right = false;
    updateThrusterAudio();
    sendKeys();
  });

  ["touchstart", "mousedown"].forEach((eventName) => {
    btnManip.addEventListener(
      eventName,
      (e) => {
        e.preventDefault();
        ensureAudio();
        resumeAudio();
        keys.manip = true;
        sendKeys();
      },
      { passive: false },
    );
  });

  ["touchend", "touchcancel", "mouseup", "mouseleave"].forEach((eventName) => {
    btnManip.addEventListener(eventName, (e) => {
      e.preventDefault();
      keys.manip = false;
      sendKeys();
    });
  });
}

