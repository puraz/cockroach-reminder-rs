//! Persistent settings. Ported from `src/shared/constants.js` (DEFAULTS + schema)
//! and `src/main/store.js` (load/save with clamping).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// User-configurable settings. Field names mirror the original camelCase keys via serde rename.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(rename = "intervalMinutes")]
    pub interval_minutes: u32,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: u32,
    #[serde(rename = "cockroachCount")]
    pub cockroach_count: u32,
    #[serde(rename = "cockroachSizePercent")]
    pub cockroach_size_percent: f32,
    #[serde(rename = "normalSpeedFps")]
    pub normal_speed_fps: f32,
    #[serde(rename = "fastSpeedMinFps")]
    pub fast_speed_min_fps: f32,
    #[serde(rename = "fastSpeedMaxFps")]
    pub fast_speed_max_fps: f32,
    #[serde(rename = "fastSpeedProbability")]
    pub fast_speed_probability: f32,
    #[serde(rename = "movementPercent")]
    pub movement_percent: f32,
    #[serde(rename = "autoStart")]
    pub auto_start: bool,
    #[serde(rename = "launchAtLogin")]
    pub launch_at_login: bool,
    #[serde(rename = "showNotifications")]
    pub show_notifications: bool,
    #[serde(rename = "soundEnabled")]
    pub sound_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        // Mirrors DEFAULTS in constants.js
        Self {
            interval_minutes: 25,
            duration_seconds: 15,
            cockroach_count: 10,
            cockroach_size_percent: 35.0,
            normal_speed_fps: 10.0,
            fast_speed_min_fps: 10.0,
            fast_speed_max_fps: 60.0,
            fast_speed_probability: 0.65,
            movement_percent: 13.5,
            auto_start: true,
            launch_at_login: false,
            show_notifications: true,
            sound_enabled: false,
        }
    }
}

impl Settings {
    /// Clamp every field to the same bounds enforced by the electron-store schema.
    pub fn clamp(&mut self) {
        self.interval_minutes = self.interval_minutes.clamp(1, 120);
        self.duration_seconds = self.duration_seconds.clamp(3, 120);
        self.cockroach_count = self.cockroach_count.clamp(1, 50);
        self.cockroach_size_percent = self.cockroach_size_percent.clamp(10.0, 80.0);
        self.normal_speed_fps = self.normal_speed_fps.clamp(5.0, 30.0);
        self.fast_speed_min_fps = self.fast_speed_min_fps.clamp(5.0, 30.0);
        self.fast_speed_max_fps = self.fast_speed_max_fps.clamp(15.0, 60.0);
        self.fast_speed_probability = self.fast_speed_probability.clamp(0.0, 1.0);
        self.movement_percent = self.movement_percent.clamp(5.0, 50.0);
    }

    /// Path to the on-disk config file (`<config_dir>/com.cockroach.reminder/config.json`).
    fn config_path() -> PathBuf {
        let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.push("com.cockroach.reminder");
        let _ = std::fs::create_dir_all(&dir);
        dir.push("config.json");
        dir
    }

    /// Load settings from disk, falling back to defaults for any missing/invalid data.
    pub fn load() -> Self {
        let path = Self::config_path();
        let mut settings = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
            .unwrap_or_default();
        settings.clamp();
        settings
    }

    /// Persist settings to disk (best-effort).
    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}
