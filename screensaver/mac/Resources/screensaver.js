// Injected by Swift before this runs:
//   window.CACHE_DIR   — absolute path to cache directory
//   window.CLOCK_STYLES — array of selected style keys e.g. ["clock_face", "angles"]

const STYLE_MAP = {
  clock_face:      { dir: "Clock_Face",       ext: "jpg" },
  clock_face_wide: { dir: "Clock_Face-Wide",  ext: "jpg" },
  address:         { dir: "Address",           ext: "JPG" },
  angles:          { dir: "Angles",            ext: "JPG" },
};

const styles  = window.CLOCK_STYLES || ["clock_face"];
const cacheDir = window.CACHE_DIR || "";

const imgA = document.getElementById("img-a");
const imgB = document.getElementById("img-b");
const timeLabel = document.getElementById("time-label");

let activeSlot = "a";
let clockTimer = null;

function pickStyle(hour, minute) {
  const idx = (hour * 60 + minute) % styles.length;
  return STYLE_MAP[styles[idx]] || STYLE_MAP["clock_face"];
}

function imageFileURL(hour, minute) {
  const style = pickStyle(hour, minute);
  const hh = String(hour).padStart(2, "0");
  const mm = String(minute).padStart(2, "0");
  // file:// URL — WKWebView has read access to cacheDir
  return `file://${cacheDir}/${style.dir}/${hh}${mm}.${style.ext}`;
}

function showImage(hour, minute) {
  const url = imageFileURL(hour, minute);
  const next = activeSlot === "a" ? imgB : imgA;
  const cur  = activeSlot === "a" ? imgA : imgB;

  next.onload = () => {
    next.classList.add("active");
    cur.classList.remove("active");
    activeSlot = activeSlot === "a" ? "b" : "a";
  };
  next.onerror = () => {
    // Image not cached yet — keep showing current, silently skip
  };
  next.src = url;

  const hh = String(hour).padStart(2, "0");
  const mm = String(minute).padStart(2, "0");
  timeLabel.textContent = `${hh}:${mm}`;
}

function scheduleNext() {
  const now = new Date();
  const msUntilNextMinute =
    (60 - now.getSeconds()) * 1000 - now.getMilliseconds();
  clockTimer = setTimeout(() => {
    const t = new Date();
    showImage(t.getHours(), t.getMinutes());
    scheduleNext();
  }, msUntilNextMinute);
}

window.startClock = function () {
  if (clockTimer) clearTimeout(clockTimer);
  const now = new Date();
  showImage(now.getHours(), now.getMinutes());
  scheduleNext();
};

window.stopClock = function () {
  if (clockTimer) clearTimeout(clockTimer);
  clockTimer = null;
};

// Auto-start (called when Swift calls startAnimation, but also start on load)
window.startClock();
