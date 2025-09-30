use std::{
    collections::BTreeMap,
    fmt,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use config::{
    builder::DefaultState, Config, ConfigBuilder, Environment, File as ConfigFile, FileFormat,
};
use serde::{Deserialize, Serialize};

const DEFAULT_RESOURCES_PATH: &str = "resources";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientSettings {
    #[serde(rename = "UseTestConfig")]
    pub use_test_config: bool,
    #[serde(rename = "Graphics")]
    pub graphics: GraphicsSettings,
    #[serde(rename = "Network")]
    pub network: NetworkSettings,
    #[serde(rename = "Logs")]
    pub logs: LogSettings,
    #[serde(rename = "Sound")]
    pub sound: SoundSettings,
    #[serde(rename = "Launcher")]
    pub launcher: LauncherSettings,
    #[serde(rename = "Game")]
    pub game: GameSettings,
    #[serde(rename = "Chat")]
    pub chat: ChatSettings,
    #[serde(rename = "Filter")]
    pub filters: FilterSettings,
    pub resources_path: PathBuf,
    #[serde(skip)]
    pub root_path: PathBuf,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            use_test_config: false,
            graphics: GraphicsSettings::default(),
            network: NetworkSettings::default(),
            logs: LogSettings::default(),
            sound: SoundSettings::default(),
            launcher: LauncherSettings::default(),
            game: GameSettings::default(),
            chat: ChatSettings::default(),
            filters: FilterSettings::default(),
            resources_path: PathBuf::from(DEFAULT_RESOURCES_PATH),
            root_path: PathBuf::from("."),
        }
    }
}

impl ClientSettings {
    pub fn load(use_test_config: bool, path: Option<&Path>) -> Result<Self> {
        let defaults = ClientSettings::default();
        let config_root = path
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let mut builder = Config::builder()
            .set_default("UseTestConfig", use_test_config)?
            .set_default(
                "ResourcesPath",
                defaults.resources_path.to_string_lossy().to_string(),
            )?
            .set_default("Graphics.FullScreen", defaults.graphics.full_screen)?
            .set_default("Graphics.Borderless", defaults.graphics.borderless)?
            .set_default("Graphics.MouseClip", defaults.graphics.mouse_clip)?
            .set_default("Graphics.AlwaysOnTop", defaults.graphics.always_on_top)?
            .set_default("Graphics.FPSCap", defaults.graphics.fps_cap)?
            .set_default("Graphics.MaxFPS", i64::from(defaults.graphics.max_fps))?
            .set_default(
                "Graphics.Resolution",
                defaults.graphics.resolution.to_string(),
            )?
            .set_default("Graphics.DebugMode", defaults.graphics.debug_mode)?
            .set_default(
                "Graphics.UseMouseCursors",
                defaults.graphics.use_mouse_cursors,
            )?
            .set_default("Network.UseConfig", defaults.network.use_config)?
            .set_default("Network.IPAddress", defaults.network.ip_address.clone())?
            .set_default("Network.Port", i64::from(defaults.network.port))?
            .set_default("Network.TimeOut", defaults.network.timeout_ms as i64)?
            .set_default("Logs.LogErrors", defaults.logs.log_errors)?
            .set_default("Logs.LogChat", defaults.logs.log_chat)?
            .set_default(
                "Logs.RemainingErrorLogs",
                i64::from(defaults.logs.remaining_error_logs),
            )?
            .set_default("Sound.Volume", i64::from(defaults.sound.volume))?
            .set_default("Sound.Music", i64::from(defaults.sound.music))?
            .set_default(
                "Sound.SoundOverLap",
                i64::from(defaults.sound.sound_overlap),
            )?
            .set_default(
                "Sound.CleanMinutes",
                i64::from(defaults.sound.clean_minutes),
            )?
            .set_default("Sound.Muted", defaults.sound.muted)?
            .set_default("Game.AccountID", defaults.game.account_id.clone())?
            .set_default("Game.Password", defaults.game.password.clone())?
            .set_default("Game.SkillMode", defaults.game.skill_mode)?
            .set_default("Game.SkillBar", defaults.game.skill_bar)?
            .set_default("Game.Effect", defaults.game.effect)?
            .set_default("Game.LevelEffect", defaults.game.level_effect)?
            .set_default("Game.DropView", defaults.game.drop_view)?
            .set_default("Game.NameView", defaults.game.name_view)?
            .set_default("Game.HPMPView", defaults.game.hp_view)?
            .set_default("Game.TransparentChat", defaults.game.transparent_chat)?
            .set_default("Game.ModeView", defaults.game.mode_view)?
            .set_default("Game.DuraWindow", defaults.game.dura_view)?
            .set_default("Game.DisplayDamage", defaults.game.display_damage)?
            .set_default("Game.TargetDead", defaults.game.target_dead)?
            .set_default("Game.HighlightTarget", defaults.game.highlight_target)?
            .set_default(
                "Game.ExpandedBuffWindow",
                defaults.game.expanded_buff_window,
            )?
            .set_default(
                "Game.ExpandedHeroBuffWindow",
                defaults.game.expanded_hero_buff_window,
            )?
            .set_default("Game.DisplayBodyName", defaults.game.display_body_name)?
            .set_default("Game.NewMove", defaults.game.new_move)?
            .set_default("Game.Skillbar0X", i64::from(defaults.game.skillbar0_x))?
            .set_default("Game.Skillbar0Y", i64::from(defaults.game.skillbar0_y))?
            .set_default("Game.Skillbar1X", i64::from(defaults.game.skillbar1_x))?
            .set_default("Game.Skillbar1Y", i64::from(defaults.game.skillbar1_y))?
            .set_default("Game.FontName", defaults.game.font_name.clone())?
            .set_default("Game.FontSize", defaults.game.font_size as f64)?
            .set_default("Chat.ShowNormalChat", defaults.chat.show_normal_chat)?
            .set_default("Chat.ShowYellChat", defaults.chat.show_yell_chat)?
            .set_default("Chat.ShowWhisperChat", defaults.chat.show_whisper_chat)?
            .set_default("Chat.ShowLoverChat", defaults.chat.show_lover_chat)?
            .set_default("Chat.ShowMentorChat", defaults.chat.show_mentor_chat)?
            .set_default("Chat.ShowGroupChat", defaults.chat.show_group_chat)?
            .set_default("Chat.ShowGuildChat", defaults.chat.show_guild_chat)?
            .set_default(
                "Filter.FilterNormalChat",
                defaults.filters.filter_normal_chat,
            )?
            .set_default(
                "Filter.FilterWhisperChat",
                defaults.filters.filter_whisper_chat,
            )?
            .set_default("Filter.FilterShoutChat", defaults.filters.filter_shout_chat)?
            .set_default(
                "Filter.FilterSystemChat",
                defaults.filters.filter_system_chat,
            )?
            .set_default("Filter.FilterLoverChat", defaults.filters.filter_lover_chat)?
            .set_default(
                "Filter.FilterMentorChat",
                defaults.filters.filter_mentor_chat,
            )?
            .set_default("Filter.FilterGroupChat", defaults.filters.filter_group_chat)?
            .set_default("Filter.FilterGuildChat", defaults.filters.filter_guild_chat)?
            .set_default("Launcher.Enabled", defaults.launcher.enabled)?
            .set_default("Launcher.Host", defaults.launcher.host.clone())?
            .set_default("Launcher.PatchFile", defaults.launcher.patch_file.clone())?
            .set_default("Launcher.NeedLogin", defaults.launcher.need_login)?
            .set_default("Launcher.Login", defaults.launcher.login.clone())?
            .set_default("Launcher.Password", defaults.launcher.password.clone())?
            .set_default("Launcher.ServerName", defaults.launcher.server_name.clone())?
            .set_default("Launcher.Browser", defaults.launcher.browser.clone())?
            .set_default("Launcher.AutoStart", defaults.launcher.auto_start)?
            .set_default(
                "Launcher.ConcurrentDownloads",
                i64::from(defaults.launcher.concurrent_downloads),
            )?;

        builder = add_optional_file(
            builder,
            config_root.join(if use_test_config {
                "Mir2Test.ini"
            } else {
                "Mir2Config.ini"
            }),
            FileFormat::Ini,
        );

        builder = add_optional_file(
            builder,
            config_root.join("config").join("client.json"),
            FileFormat::Json,
        );

        builder = add_optional_file(
            builder,
            config_root.join("config").join("client.yaml"),
            FileFormat::Yaml,
        );

        builder = add_optional_file(
            builder,
            config_root.join("config").join("client.yml"),
            FileFormat::Yaml,
        );

        builder = builder.add_source(Environment::with_prefix("MIR2_CLIENT").separator("__"));

        let mut settings: ClientSettings = builder
            .build()
            .context("failed to load client settings")?
            .try_deserialize()
            .context("invalid client settings schema")?;

        settings.use_test_config = use_test_config;
        settings.normalize(&config_root);
        settings.root_path = config_root;

        Ok(settings)
    }

    fn normalize(&mut self, config_root: &Path) {
        if self.resources_path.as_os_str().is_empty() {
            self.resources_path = PathBuf::from(DEFAULT_RESOURCES_PATH);
        }

        if self.resources_path.is_relative() {
            self.resources_path = config_root.join(&self.resources_path);
        }

        self.sound.normalize();
        self.network.normalize();
        self.launcher.normalize();
        self.game.normalize();
    }

    pub fn resolution(&self) -> ResolutionSize {
        self.graphics.resolution.dimensions()
    }

    pub fn server_address(&self) -> (&str, u16) {
        (&self.network.ip_address, self.network.port)
    }

    #[allow(dead_code)]
    pub fn resources_path(&self) -> &Path {
        &self.resources_path
    }

    #[allow(dead_code)]
    pub fn config_root(&self) -> &Path {
        &self.root_path
    }

    #[allow(dead_code)]
    pub fn quest_tracking_path(&self) -> PathBuf {
        quest_tracking_file(&self.root_path)
    }

    #[allow(dead_code)]
    pub fn load_quest_tracking(&self, character: &str) -> Result<QuestTracking> {
        QuestTracking::load_from(&self.root_path, character)
    }

    #[allow(dead_code)]
    pub fn save_quest_tracking(&self, character: &str, quests: &QuestTracking) -> Result<()> {
        quests.save_to(&self.root_path, character)
    }
}

fn add_optional_file(
    builder: ConfigBuilder<DefaultState>,
    path: PathBuf,
    format: FileFormat,
) -> ConfigBuilder<DefaultState> {
    builder.add_source(
        ConfigFile::from(path.clone())
            .format(format)
            .required(false),
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct GraphicsSettings {
    pub full_screen: bool,
    pub borderless: bool,
    pub mouse_clip: bool,
    #[serde(rename = "AlwaysOnTop")]
    pub always_on_top: bool,
    #[serde(rename = "FPSCap")]
    pub fps_cap: bool,
    pub max_fps: u16,
    pub resolution: SupportedResolution,
    pub debug_mode: bool,
    pub use_mouse_cursors: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            full_screen: true,
            borderless: true,
            mouse_clip: false,
            always_on_top: true,
            fps_cap: true,
            max_fps: 100,
            resolution: SupportedResolution::W1024H768,
            debug_mode: false,
            use_mouse_cursors: true,
        }
    }
}

impl GraphicsSettings {
    #[allow(dead_code)]
    pub fn dimensions(&self) -> ResolutionSize {
        self.resolution.dimensions()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct NetworkSettings {
    pub use_config: bool,
    #[serde(rename = "IPAddress")]
    pub ip_address: String,
    pub port: u16,
    #[serde(rename = "TimeOut")]
    pub timeout_ms: u64,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            use_config: false,
            ip_address: "127.0.0.1".into(),
            port: 7000,
            timeout_ms: 5000,
        }
    }
}

impl NetworkSettings {
    fn normalize(&mut self) {
        if self.port == 0 {
            self.port = 7000;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct LogSettings {
    pub log_errors: bool,
    pub log_chat: bool,
    pub remaining_error_logs: u32,
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            log_errors: true,
            log_chat: true,
            remaining_error_logs: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SoundSettings {
    #[serde(rename = "Volume")]
    pub volume: u8,
    #[serde(rename = "Music")]
    pub music: u8,
    #[serde(rename = "SoundOverLap")]
    pub sound_overlap: u32,
    #[serde(rename = "CleanMinutes")]
    pub clean_minutes: u32,
    #[serde(rename = "Muted")]
    pub muted: bool,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            volume: 100,
            music: 100,
            sound_overlap: 3,
            clean_minutes: 5,
            muted: false,
        }
    }
}

impl SoundSettings {
    fn normalize(&mut self) {
        self.volume = self.volume.min(100);
        self.music = self.music.min(100);

        if self.clean_minutes == 0 || self.clean_minutes > 180 {
            self.clean_minutes = 5;
        }
    }

    pub fn master_volume_scalar(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            (self.volume.min(100) as f32) / 100.0
        }
    }

    #[allow(dead_code)]
    pub fn music_volume_scalar(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            (self.music.min(100) as f32) / 100.0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameSettings {
    #[serde(rename = "AccountID")]
    pub account_id: String,
    #[serde(rename = "Password")]
    pub password: String,
    #[serde(rename = "SkillMode")]
    pub skill_mode: bool,
    #[serde(rename = "SkillBar")]
    pub skill_bar: bool,
    #[serde(rename = "Effect")]
    pub effect: bool,
    #[serde(rename = "LevelEffect")]
    pub level_effect: bool,
    #[serde(rename = "DropView")]
    pub drop_view: bool,
    #[serde(rename = "NameView")]
    pub name_view: bool,
    #[serde(rename = "HPMPView")]
    pub hp_view: bool,
    #[serde(rename = "TransparentChat")]
    pub transparent_chat: bool,
    #[serde(rename = "ModeView")]
    pub mode_view: bool,
    #[serde(rename = "DuraWindow")]
    pub dura_view: bool,
    #[serde(rename = "DisplayDamage")]
    pub display_damage: bool,
    #[serde(rename = "TargetDead")]
    pub target_dead: bool,
    #[serde(rename = "HighlightTarget")]
    pub highlight_target: bool,
    #[serde(rename = "ExpandedBuffWindow")]
    pub expanded_buff_window: bool,
    #[serde(rename = "ExpandedHeroBuffWindow")]
    pub expanded_hero_buff_window: bool,
    #[serde(rename = "DisplayBodyName")]
    pub display_body_name: bool,
    #[serde(rename = "NewMove")]
    pub new_move: bool,
    #[serde(rename = "Skillbar0X")]
    pub skillbar0_x: i32,
    #[serde(rename = "Skillbar0Y")]
    pub skillbar0_y: i32,
    #[serde(rename = "Skillbar1X")]
    pub skillbar1_x: i32,
    #[serde(rename = "Skillbar1Y")]
    pub skillbar1_y: i32,
    #[serde(rename = "FontName")]
    pub font_name: String,
    #[serde(rename = "FontSize")]
    pub font_size: f32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            account_id: String::new(),
            password: String::new(),
            skill_mode: false,
            skill_bar: true,
            effect: true,
            level_effect: true,
            drop_view: true,
            name_view: true,
            hp_view: true,
            transparent_chat: false,
            mode_view: false,
            dura_view: false,
            display_damage: true,
            target_dead: false,
            highlight_target: true,
            expanded_buff_window: true,
            expanded_hero_buff_window: true,
            display_body_name: false,
            new_move: false,
            skillbar0_x: 0,
            skillbar0_y: 0,
            skillbar1_x: 216,
            skillbar1_y: 0,
            font_name: "Arial".into(),
            font_size: 8.0,
        }
    }
}

impl GameSettings {
    fn normalize(&mut self) {
        if !self.font_size.is_finite() || self.font_size <= 0.0 {
            self.font_size = 8.0;
        }

        if self.font_size > 72.0 {
            self.font_size = 72.0;
        }
    }

    #[allow(dead_code)]
    pub fn skillbar_positions(&self) -> [Point2D; 2] {
        [
            Point2D::new(self.skillbar0_x, self.skillbar0_y),
            Point2D::new(self.skillbar1_x, self.skillbar1_y),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct ChatSettings {
    pub show_normal_chat: bool,
    pub show_yell_chat: bool,
    pub show_whisper_chat: bool,
    pub show_lover_chat: bool,
    pub show_mentor_chat: bool,
    pub show_group_chat: bool,
    pub show_guild_chat: bool,
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self {
            show_normal_chat: true,
            show_yell_chat: true,
            show_whisper_chat: true,
            show_lover_chat: true,
            show_mentor_chat: true,
            show_group_chat: true,
            show_guild_chat: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct FilterSettings {
    pub filter_normal_chat: bool,
    pub filter_whisper_chat: bool,
    pub filter_shout_chat: bool,
    pub filter_system_chat: bool,
    pub filter_lover_chat: bool,
    pub filter_mentor_chat: bool,
    pub filter_group_chat: bool,
    pub filter_guild_chat: bool,
}

impl Default for FilterSettings {
    fn default() -> Self {
        Self {
            filter_normal_chat: false,
            filter_whisper_chat: false,
            filter_shout_chat: false,
            filter_system_chat: false,
            filter_lover_chat: false,
            filter_mentor_chat: false,
            filter_group_chat: false,
            filter_guild_chat: false,
        }
    }
}

#[allow(dead_code)]
pub const QUEST_TRACKING_SLOTS: usize = 5;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestTracking {
    entries: [i32; QUEST_TRACKING_SLOTS],
}

#[allow(dead_code)]
impl QuestTracking {
    pub fn new() -> Self {
        Self {
            entries: [-1; QUEST_TRACKING_SLOTS],
        }
    }

    pub fn load_from(root: &Path, character: &str) -> Result<Self> {
        let path = quest_tracking_file(root);
        let mut tracking = Self::new();

        let data = read_quest_file(&path)?;
        if let Some(entries) = data.get(character) {
            tracking.entries = *entries;
        }

        Ok(tracking)
    }

    pub fn save_to(&self, root: &Path, character: &str) -> Result<()> {
        let path = quest_tracking_file(root);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create quest tracking directory `{}`",
                    parent.display()
                )
            })?;
        }

        let mut data = read_quest_file(&path)?;
        data.insert(character.to_string(), self.entries);

        let file = File::create(&path)
            .with_context(|| format!("failed to write quest tracking to `{}`", path.display()))?;
        let mut writer = BufWriter::new(file);

        for (index, (section, entries)) in data.iter().enumerate() {
            writeln!(writer, "[{}]", section)?;
            for (quest_idx, value) in entries.iter().enumerate() {
                writeln!(writer, "Quest-{}={}", quest_idx, value)?;
            }

            if index + 1 < data.len() {
                writeln!(writer)?;
            }
        }

        writer.flush()?;

        Ok(())
    }

    pub fn get(&self, index: usize) -> Result<i32> {
        if let Some(value) = self.entries.get(index) {
            Ok(*value)
        } else {
            bail!("quest tracking index {} out of range", index)
        }
    }

    pub fn set(&mut self, index: usize, value: i32) -> Result<()> {
        if let Some(slot) = self.entries.get_mut(index) {
            *slot = value;
            Ok(())
        } else {
            bail!("quest tracking index {} out of range", index)
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = i32> + '_ {
        self.entries.iter().copied()
    }

    pub fn as_array(&self) -> &[i32; QUEST_TRACKING_SLOTS] {
        &self.entries
    }
}

impl Default for QuestTracking {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct LauncherSettings {
    #[serde(rename = "Enabled")]
    pub enabled: bool,
    #[serde(rename = "Host")]
    pub host: String,
    #[serde(rename = "PatchFile")]
    pub patch_file: String,
    #[serde(rename = "NeedLogin")]
    pub need_login: bool,
    #[serde(rename = "Login")]
    pub login: String,
    #[serde(rename = "Password")]
    pub password: String,
    #[serde(rename = "ServerName")]
    pub server_name: String,
    #[serde(rename = "Browser")]
    pub browser: String,
    #[serde(rename = "AutoStart")]
    pub auto_start: bool,
    #[serde(rename = "ConcurrentDownloads")]
    pub concurrent_downloads: u32,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "http://mirfiles.com/mir2/cmir/patch/".into(),
            patch_file: "PList.gz".into(),
            need_login: false,
            login: String::new(),
            password: String::new(),
            server_name: String::new(),
            browser: "https://www.lomcn.org/mir2-patchsite/".into(),
            auto_start: false,
            concurrent_downloads: 1,
        }
    }
}

impl LauncherSettings {
    fn normalize(&mut self) {
        if !self.host.is_empty() {
            if !self.host.starts_with("http://") && !self.host.starts_with("https://") {
                if self.host.starts_with("www.") {
                    self.host = format!("http://{}", self.host);
                } else {
                    self.host = format!("http://{}", self.host);
                }
            }
            if !self.host.ends_with('/') {
                self.host.push('/');
            }
            if self
                .host
                .eq_ignore_ascii_case("http://mirfiles.co.uk/mir2/cmir/patch/")
            {
                self.host = "http://mirfiles.com/mir2/cmir/patch/".into();
            }
        }

        if self.browser.starts_with("www.") {
            self.browser = format!("http://{}", self.browser);
        }

        if self.concurrent_downloads == 0 {
            self.concurrent_downloads = 1;
        } else if self.concurrent_downloads > 100 {
            self.concurrent_downloads = 100;
        }
    }
}

#[allow(dead_code)]
fn quest_tracking_file(root: &Path) -> PathBuf {
    root.join("Data").join("UserData").join("QuestTracking.ini")
}

#[allow(dead_code)]
fn read_quest_file(path: &Path) -> Result<BTreeMap<String, [i32; QUEST_TRACKING_SLOTS]>> {
    let mut data = BTreeMap::new();

    if !path.exists() {
        return Ok(data);
    }

    let file = File::open(path)
        .with_context(|| format!("failed to read quest tracking from `{}`", path.display()))?;
    let reader = BufReader::new(file);
    let mut current_section: Option<String> = None;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1];
            current_section = Some(section.to_string());
            continue;
        }

        let section = match current_section.as_ref() {
            Some(section) => section,
            None => continue,
        };

        if let Some((key, value)) = trimmed.split_once('=') {
            if let Some(index) = quest_index_from_key(key.trim()) {
                let entry = data
                    .entry(section.clone())
                    .or_insert([-1; QUEST_TRACKING_SLOTS]);

                if let Ok(parsed) = value.trim().parse::<i32>() {
                    entry[index] = parsed;
                }
            }
        }
    }

    Ok(data)
}

#[allow(dead_code)]
fn quest_index_from_key(key: &str) -> Option<usize> {
    let suffix = key.strip_prefix("Quest-")?;
    let index = suffix.parse::<usize>().ok()?;

    if index < QUEST_TRACKING_SLOTS {
        Some(index)
    } else {
        None
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point2D {
    pub x: i32,
    pub y: i32,
}

impl Point2D {
    #[allow(dead_code)]
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ResolutionSize {
    pub width: u16,
    pub height: u16,
}

impl ResolutionSize {
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedResolution {
    W1024H768,
    W1280H720,
    W1366H768,
    W1920H1080,
}

impl SupportedResolution {
    pub fn dimensions(self) -> ResolutionSize {
        match self {
            SupportedResolution::W1024H768 => ResolutionSize::new(1024, 768),
            SupportedResolution::W1280H720 => ResolutionSize::new(1280, 720),
            SupportedResolution::W1366H768 => ResolutionSize::new(1366, 768),
            SupportedResolution::W1920H1080 => ResolutionSize::new(1920, 1080),
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "w1024h768" | "1024" => Some(SupportedResolution::W1024H768),
            "w1280h720" | "1280" => Some(SupportedResolution::W1280H720),
            "w1366h768" | "1366" => Some(SupportedResolution::W1366H768),
            "w1920h1080" | "1920" => Some(SupportedResolution::W1920H1080),
            _ => None,
        }
    }
}

impl Default for SupportedResolution {
    fn default() -> Self {
        SupportedResolution::W1024H768
    }
}

impl fmt::Display for SupportedResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            SupportedResolution::W1024H768 => "w1024h768",
            SupportedResolution::W1280H720 => "w1280h720",
            SupportedResolution::W1366H768 => "w1366h768",
            SupportedResolution::W1920H1080 => "w1920h1080",
        };
        f.write_str(value)
    }
}

impl Serialize for SupportedResolution {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SupportedResolution {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ResolutionVisitor;

        impl<'de> serde::de::Visitor<'de> for ResolutionVisitor {
            type Value = SupportedResolution;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a supported resolution identifier or width value")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                SupportedResolution::from_str(v)
                    .ok_or_else(|| E::custom(format!("unsupported resolution `{}`", v)))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&v.to_string())
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&v.to_string())
            }
        }

        deserializer.deserialize_any(ResolutionVisitor)
    }
}
