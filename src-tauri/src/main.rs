#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use chrono::{Local, Timelike};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Semaphore;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ClockStyle {
    ClockFace,
    ClockFaceWide,
    Address,
    Angles,
}

impl ClockStyle {
    fn dir_name(&self) -> &str {
        match self {
            ClockStyle::ClockFace     => "Clock_Face",
            ClockStyle::ClockFaceWide => "Clock_Face-Wide",
            ClockStyle::Address       => "Address",
            ClockStyle::Angles        => "Angles",
        }
    }
    fn extension(&self) -> &str {
        match self {
            ClockStyle::ClockFace | ClockStyle::ClockFaceWide => "jpg",
            ClockStyle::Address   | ClockStyle::Angles        => "JPG",
        }
    }
}

/// Deterministically pick one style from a list for a given minute.
fn pick_style<'a>(styles: &'a [ClockStyle], hour: u32, minute: u32) -> &'a ClockStyle {
    let idx = (hour * 60 + minute) as usize % styles.len();
    &styles[idx]
}

fn image_filename(hour: u32, minute: u32, ext: &str) -> String {
    format!("{:02}{:02}.{}", hour, minute, ext)
}

fn image_url(style: &ClockStyle, hour: u32, minute: u32) -> String {
    format!(
        "https://www.theclockproject.com/{}/{}",
        style.dir_name(),
        image_filename(hour, minute, style.extension())
    )
}

fn cached_path(base: &Path, style: &ClockStyle, hour: u32, minute: u32) -> PathBuf {
    base.join(style.dir_name())
        .join(image_filename(hour, minute, style.extension()))
}

// ── Settings ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct Settings {
    styles: Vec<ClockStyle>,
}

fn load_settings(base: &Path) -> Option<Settings> {
    let data = std::fs::read_to_string(base.join("settings.json")).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_settings(base: &Path, s: &Settings) {
    if let Ok(json) = serde_json::to_string(s) {
        let _ = std::fs::write(base.join("settings.json"), json);
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

pub struct AppState {
    pub styles: Mutex<Vec<ClockStyle>>,
    pub cache_base: PathBuf,
    pub is_downloading: Mutex<bool>,
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct AppStatus {
    has_styles: bool,
    styles: Vec<ClockStyle>,
    priority_cached: usize,
}

#[tauri::command]
fn get_status(state: State<'_, Arc<AppState>>) -> AppStatus {
    let styles = state.styles.lock().unwrap().clone();
    let now = Local::now();
    let h = now.hour();
    let m = now.minute();

    let priority_cached = if styles.is_empty() {
        0
    } else {
        (0u32..60).filter(|i| {
            let total = h * 60 + m + i;
            let th = (total / 60) % 24;
            let tm = total % 60;
            let s = pick_style(&styles, th, tm);
            cached_path(&state.cache_base, s, th, tm).exists()
        }).count()
    };

    AppStatus {
        has_styles: !styles.is_empty(),
        styles,
        priority_cached,
    }
}

#[tauri::command]
async fn start_with_styles(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    styles: Vec<ClockStyle>,
) -> Result<(), String> {
    if styles.is_empty() {
        return Err("No styles selected".into());
    }

    save_settings(&state.cache_base, &Settings { styles: styles.clone() });
    *state.styles.lock().unwrap() = styles;

    {
        let mut dl = state.is_downloading.lock().unwrap();
        if *dl { return Ok(()); }
        *dl = true;
    }

    let state_arc = state.inner().clone();
    let app_clone = app.clone();
    tokio::spawn(async move {
        run_download(app_clone, state_arc).await;
    });

    Ok(())
}

async fn run_download(app: AppHandle, state: Arc<AppState>) {
    let styles = state.styles.lock().unwrap().clone();
    let now = Local::now();
    let start_h = now.hour();
    let start_m = now.minute();

    // Priority: next 60 minutes
    let mut order: Vec<(u32, u32)> = (0u32..60)
        .map(|i| {
            let t = start_h * 60 + start_m + i;
            ((t / 60) % 24, t % 60)
        })
        .collect();

    // Remaining 1380 minutes
    let priority_set: HashSet<(u32, u32)> = order.iter().cloned().collect();
    for h in 0u32..24 {
        for m in 0u32..60 {
            if !priority_set.contains(&(h, m)) {
                order.push((h, m));
            }
        }
    }

    let client = Arc::new(reqwest::Client::new());
    let sem = Arc::new(Semaphore::new(8));
    let mut priority_emitted = false;

    for (idx, (h, m)) in order.iter().enumerate() {
        let style = pick_style(&styles, *h, *m).clone();
        let dir = state.cache_base.join(style.dir_name());
        let _ = std::fs::create_dir_all(&dir);
        let path = cached_path(&state.cache_base, &style, *h, *m);

        if !path.exists() {
            let url = image_url(&style, *h, *m);
            let client = client.clone();
            let permit = sem.clone().acquire_owned().await.unwrap();
            let path = path.clone();
            tokio::spawn(async move {
                let _p = permit;
                if let Ok(resp) = client.get(&url).send().await {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            let _ = std::fs::write(&path, bytes);
                        }
                    }
                }
            }).await.ok();
        }

        if !priority_emitted && idx >= 59 {
            priority_emitted = true;
            app.emit("priority-ready", ()).ok();
        }
    }

    if !priority_emitted {
        app.emit("priority-ready", ()).ok();
    }

    app.emit("download-complete", ()).ok();
    *state.is_downloading.lock().unwrap() = false;
}

#[tauri::command]
async fn get_current_image(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let now = Local::now();
    get_image_for_time(state, now.hour(), now.minute()).await
}

#[tauri::command]
async fn get_image_for_time(
    state: State<'_, Arc<AppState>>,
    hour: u32,
    minute: u32,
) -> Result<String, String> {
    let styles = state.styles.lock().unwrap().clone();
    if styles.is_empty() {
        return Err("No style configured".into());
    }
    let style = pick_style(&styles, hour, minute).clone();
    let path = cached_path(&state.cache_base, &style, hour, minute);

    if path.exists() {
        return Ok(path.to_string_lossy().into_owned());
    }

    // On-demand fetch
    let dir = state.cache_base.join(style.dir_name());
    let _ = std::fs::create_dir_all(&dir);
    let url = image_url(&style, hour, minute);
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().into_owned())
    } else {
        Err(format!("No image for {:02}:{:02}", hour, minute))
    }
}

#[tauri::command]
async fn get_preview_images(
    state: State<'_, Arc<AppState>>,
) -> Result<HashMap<String, String>, String> {
    let previews: &[(&str, ClockStyle, u32, u32)] = &[
        ("clock_face",      ClockStyle::ClockFace,     10, 35),
        ("clock_face_wide", ClockStyle::ClockFaceWide, 10, 35),
        ("address",         ClockStyle::Address,       12,  8),
        ("angles",          ClockStyle::Angles,          9, 22),
    ];

    let client = reqwest::Client::new();
    let mut result = HashMap::new();

    for (key, style, h, m) in previews {
        let dir = state.cache_base.join(style.dir_name());
        let _ = std::fs::create_dir_all(&dir);
        let path = cached_path(&state.cache_base, style, *h, *m);
        if !path.exists() {
            if let Ok(resp) = client.get(&image_url(style, *h, *m)).send().await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        let _ = std::fs::write(&path, &bytes);
                    }
                }
            }
        }
        if path.exists() {
            result.insert(key.to_string(), path.to_string_lossy().into_owned());
        }
    }

    Ok(result)
}

#[tauri::command]
fn get_styles(state: State<'_, Arc<AppState>>) -> Vec<ClockStyle> {
    state.styles.lock().unwrap().clone()
}

// ── Entry ─────────────────────────────────────────────────────────────────────

fn main() {
    let cache_base = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("theclockproject-saver");
    let _ = std::fs::create_dir_all(&cache_base);

    let saved_styles = load_settings(&cache_base)
        .map(|s| s.styles)
        .unwrap_or_default();

    let state = Arc::new(AppState {
        styles: Mutex::new(saved_styles),
        cache_base,
        is_downloading: Mutex::new(false),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_with_styles,
            get_current_image,
            get_image_for_time,
            get_preview_images,
            get_styles,
        ])
        .run(tauri::generate_context!())
        .expect("error");
}
