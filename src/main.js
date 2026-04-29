import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { convertFileSrc } from "@tauri-apps/api/core";

// ── Screens ───────────────────────────────────────────────────────────────────

function show(id) {
  document.querySelectorAll(".screen").forEach((s) => s.classList.add("hidden"));
  document.getElementById(id).classList.remove("hidden");
}

// ── Clock display ─────────────────────────────────────────────────────────────

const imgA = document.getElementById("img-a");
const imgB = document.getElementById("img-b");
let activeSlot = "a";
let clockTimer = null;

async function loadImage(hour, minute) {
  try {
    const path = await invoke("get_image_for_time", { hour, minute });
    const url = convertFileSrc(path);
    const next = activeSlot === "a" ? imgB : imgA;
    const cur  = activeSlot === "a" ? imgA : imgB;
    next.src = url;
    await new Promise((res) => { next.onload = res; next.onerror = res; });
    next.classList.add("active");
    cur.classList.remove("active");
    activeSlot = activeSlot === "a" ? "b" : "a";
    document.getElementById("time-label").textContent =
      new Date().toTimeString().slice(0, 5);
  } catch (e) {
    console.warn("Image load:", e);
  }
}

function startClock() {
  show("clock-screen");
  const now = new Date();
  loadImage(now.getHours(), now.getMinutes());
  function scheduleNext() {
    const now = new Date();
    clockTimer = setTimeout(async () => {
      const t = new Date();
      await loadImage(t.getHours(), t.getMinutes());
      scheduleNext();
    }, (60 - now.getSeconds()) * 1000 - now.getMilliseconds());
  }
  scheduleNext();
}

function stopClock() {
  if (clockTimer) clearTimeout(clockTimer);
  clockTimer = null;
}

// ── Multi-select ──────────────────────────────────────────────────────────────

const selectedStyles = new Set();
const hint = document.getElementById("selection-hint");
const startBtn = document.getElementById("start-btn");

const styleLabels = {
  clock_face:      "Clock Face",
  clock_face_wide: "Clock Wide",
  address:         "Street Numbers",
  angles:          "Angles",
};

function updateFooter() {
  const n = selectedStyles.size;
  if (n === 0) {
    hint.textContent = "Select one or more styles";
    hint.classList.remove("has-selection");
    startBtn.disabled = true;
    startBtn.textContent = "Select a style to continue";
  } else {
    const names = [...selectedStyles].map((s) => styleLabels[s]).join(" + ");
    hint.textContent = n === 1 ? names : `${names} — alternating every minute`;
    hint.classList.add("has-selection");
    startBtn.disabled = false;
    startBtn.textContent = n === 1 ? `Start →` : `Start mix of ${n} →`;
  }
}

document.querySelectorAll(".style-card").forEach((card) => {
  card.addEventListener("click", () => {
    const s = card.dataset.style;
    if (selectedStyles.has(s)) {
      selectedStyles.delete(s);
      card.classList.remove("selected");
    } else {
      selectedStyles.add(s);
      card.classList.add("selected");
    }
    updateFooter();
  });
});

// ── Start button ──────────────────────────────────────────────────────────────

startBtn.addEventListener("click", async () => {
  if (selectedStyles.size === 0) return;

  startBtn.disabled = true;
  startBtn.textContent = "Getting ready…";
  document.getElementById("getting-ready").classList.remove("hidden");
  hint.classList.add("hidden");

  const unlisten = await listen("priority-ready", () => {
    unlisten();
    startClock();
  });

  try {
    await invoke("start_with_styles", { styles: [...selectedStyles] });
  } catch (e) {
    console.error(e);
    startBtn.disabled = false;
    hint.classList.remove("hidden");
    document.getElementById("getting-ready").classList.add("hidden");
    updateFooter();
  }
});

// ── HUD ───────────────────────────────────────────────────────────────────────

document.getElementById("fullscreen-btn").addEventListener("click", () => {
  if (!document.fullscreenElement) document.documentElement.requestFullscreen();
  else document.exitFullscreen();
});

document.getElementById("back-btn").addEventListener("click", () => {
  stopClock();
  show("onboarding");
  document.getElementById("getting-ready").classList.add("hidden");
  hint.classList.remove("hidden");
  updateFooter();
});

// ── Init ──────────────────────────────────────────────────────────────────────

async function init() {
  let previews = {};
  try {
    previews = await invoke("get_preview_images");
  } catch (e) {
    console.warn("Previews unavailable:", e);
  }

  // Populate preview images
  for (const [key, filePath] of Object.entries(previews)) {
    document.querySelectorAll(`[data-preview="${key}"]`).forEach((img) => {
      img.src = convertFileSrc(filePath);
      img.onload = () => img.classList.add("loaded");
    });
  }

  show("onboarding");

  // Restore saved selection if enough cache exists
  try {
    const status = await invoke("get_status");
    if (status.has_styles && status.priority_cached >= 60) {
      startClock();
      return;
    }
    if (status.has_styles && status.styles.length > 0) {
      // Pre-select saved styles
      status.styles.forEach((s) => {
        selectedStyles.add(s);
        const card = document.querySelector(`[data-style="${s}"]`);
        if (card) card.classList.add("selected");
      });
      updateFooter();
    }
  } catch (e) {
    console.warn("Status:", e);
  }
}

init();
