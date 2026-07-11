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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_within_valid_range() {
        let s = Settings::default();
        assert!(s.interval_minutes >= 1 && s.interval_minutes <= 120);
        assert!(s.duration_seconds >= 3 && s.duration_seconds <= 120);
        assert!(s.cockroach_count >= 1 && s.cockroach_count <= 50);
        assert!(s.cockroach_size_percent >= 10.0 && s.cockroach_size_percent <= 80.0);
        assert!(s.normal_speed_fps >= 5.0 && s.normal_speed_fps <= 30.0);
        assert!(s.fast_speed_min_fps >= 5.0 && s.fast_speed_min_fps <= 30.0);
        assert!(s.fast_speed_max_fps >= 15.0 && s.fast_speed_max_fps <= 60.0);
        assert!(s.fast_speed_probability >= 0.0 && s.fast_speed_probability <= 1.0);
        assert!(s.movement_percent >= 5.0 && s.movement_percent <= 50.0);
    }

    #[test]
    fn default_values_pass_clamp_unchanged() {
        let mut s = Settings::default();
        let original = s.clone();
        s.clamp();
        assert_eq!(s.interval_minutes, original.interval_minutes);
        assert_eq!(s.duration_seconds, original.duration_seconds);
        assert_eq!(s.cockroach_count, original.cockroach_count);
        assert_eq!(s.cockroach_size_percent, original.cockroach_size_percent);
        assert_eq!(s.normal_speed_fps, original.normal_speed_fps);
        assert_eq!(s.fast_speed_min_fps, original.fast_speed_min_fps);
        assert_eq!(s.fast_speed_max_fps, original.fast_speed_max_fps);
        assert_eq!(s.fast_speed_probability, original.fast_speed_probability);
        assert_eq!(s.movement_percent, original.movement_percent);
    }

    #[test]
    fn clamp_corrects_values_below_minimum() {
        let mut s = Settings {
            interval_minutes: 0,
            duration_seconds: 1,
            cockroach_count: 0,
            cockroach_size_percent: -5.0,
            normal_speed_fps: 0.0,
            fast_speed_min_fps: -1.0,
            fast_speed_max_fps: 10.0,
            fast_speed_probability: -0.5,
            movement_percent: 0.0,
            ..Settings::default()
        };
        s.clamp();
        assert_eq!(s.interval_minutes, 1);
        assert_eq!(s.duration_seconds, 3);
        assert_eq!(s.cockroach_count, 1);
        assert_eq!(s.cockroach_size_percent, 10.0);
        assert_eq!(s.normal_speed_fps, 5.0);
        assert_eq!(s.fast_speed_min_fps, 5.0);
        assert_eq!(s.fast_speed_max_fps, 15.0);
        assert_eq!(s.fast_speed_probability, 0.0);
        assert_eq!(s.movement_percent, 5.0);
    }

    #[test]
    fn clamp_corrects_values_above_maximum() {
        let mut s = Settings {
            interval_minutes: 500,
            duration_seconds: 300,
            cockroach_count: 999,
            cockroach_size_percent: 200.0,
            normal_speed_fps: 100.0,
            fast_speed_min_fps: 60.0,
            fast_speed_max_fps: 200.0,
            fast_speed_probability: 5.0,
            movement_percent: 200.0,
            ..Settings::default()
        };
        s.clamp();
        assert_eq!(s.interval_minutes, 120);
        assert_eq!(s.duration_seconds, 120);
        assert_eq!(s.cockroach_count, 50);
        assert_eq!(s.cockroach_size_percent, 80.0);
        assert_eq!(s.normal_speed_fps, 30.0);
        assert_eq!(s.fast_speed_min_fps, 30.0);
        assert_eq!(s.fast_speed_max_fps, 60.0);
        assert_eq!(s.fast_speed_probability, 1.0);
        assert_eq!(s.movement_percent, 50.0);
    }

    #[test]
    fn clamp_preserves_boolean_fields() {
        let mut s = Settings::default();
        s.auto_start = true;
        s.launch_at_login = true;
        s.show_notifications = true;
        s.sound_enabled = true;
        s.clamp();
        assert!(s.auto_start);
        assert!(s.launch_at_login);
        assert!(s.show_notifications);
        assert!(s.sound_enabled);
    }

    #[test]
    fn serde_round_trip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s.interval_minutes, deserialized.interval_minutes);
        assert_eq!(s.duration_seconds, deserialized.duration_seconds);
        assert_eq!(s.cockroach_count, deserialized.cockroach_count);
        assert_eq!(s.launch_at_login, deserialized.launch_at_login);
    }

    #[test]
    fn serde_field_name_mapping() {
        let json = r#"{
            "intervalMinutes": 30,
            "durationSeconds": 20,
            "cockroachCount": 5,
            "cockroachSizePercent": 50.0,
            "normalSpeedFps": 15.0,
            "fastSpeedMinFps": 10.0,
            "fastSpeedMaxFps": 30.0,
            "fastSpeedProbability": 0.5,
            "movementPercent": 20.0,
            "autoStart": false,
            "launchAtLogin": true,
            "showNotifications": false,
            "soundEnabled": true
        }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.interval_minutes, 30);
        assert_eq!(s.duration_seconds, 20);
        assert_eq!(s.cockroach_count, 5);
        assert_eq!(s.launch_at_login, true);
        assert_eq!(s.sound_enabled, true);
    }

    #[test]
    fn serde_partial_json_parse_error() {
        // Missing required fields should fail to parse — the Settings struct
        // has no serde default annotations on individual fields.
        let json = r#"{"intervalMinutes": 10}"#;
        let result: Result<Settings, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
