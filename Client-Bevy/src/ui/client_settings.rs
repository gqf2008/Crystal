// ============================================================================
// 客户端设置持久化（#216）
// C# Settings 对齐：OptionState + ChatFilter → config.ini [Settings] 段
// 保留 [Network]/[Login] 等既有段；启动应用，变更防抖 1s 保存。
// ============================================================================

use bevy::prelude::*;

use crate::game::chat::ChatFilter;
use crate::game::dialogs::option::OptionState;
use crate::scenes::AppState;

/// 持久化设置快照（仅含需要保存的字段）
#[derive(Debug, Clone, PartialEq)]
pub struct PersistedSettings {
    // OptionDialog
    pub skill_mode_ctrl: bool,
    pub skill_bar: bool,
    pub effect: bool,
    pub drop_view: bool,
    pub name_view: bool,
    pub hp_view: bool,
    pub sound_volume: f32,
    pub music_volume: f32,
    pub allow_observe: bool,
    pub new_move: bool,
    // ChatFilter（聊天过滤 + 透明）
    pub filter_normal: bool,
    pub filter_whisper: bool,
    pub filter_shout: bool,
    pub filter_system: bool,
    pub filter_lover: bool,
    pub filter_mentor: bool,
    pub filter_group: bool,
    pub filter_guild: bool,
    pub transparent_chat: bool,
}

impl Default for PersistedSettings {
    fn default() -> Self {
        Self {
            skill_mode_ctrl: true,
            skill_bar: true,
            effect: true,
            drop_view: true,
            name_view: true,
            hp_view: true,
            sound_volume: 0.8,
            music_volume: 0.6,
            allow_observe: false,
            new_move: true,
            filter_normal: false,
            filter_whisper: false,
            filter_shout: false,
            filter_system: false,
            filter_lover: false,
            filter_mentor: false,
            filter_group: false,
            filter_guild: false,
            transparent_chat: false,
        }
    }
}

impl From<(&OptionState, &ChatFilter)> for PersistedSettings {
    fn from((opt, chat): (&OptionState, &ChatFilter)) -> Self {
        Self {
            skill_mode_ctrl: opt.skill_mode_ctrl,
            skill_bar: opt.skill_bar,
            effect: opt.effect,
            drop_view: opt.drop_view,
            name_view: opt.name_view,
            hp_view: opt.hp_view,
            sound_volume: opt.sound_volume,
            music_volume: opt.music_volume,
            allow_observe: opt.allow_observe,
            new_move: opt.new_move,
            filter_normal: chat.normal,
            filter_whisper: chat.whisper,
            filter_shout: chat.shout,
            filter_system: chat.system,
            filter_lover: chat.lover,
            filter_mentor: chat.mentor,
            filter_group: chat.group,
            filter_guild: chat.guild,
            transparent_chat: chat.transparent,
        }
    }
}

impl PersistedSettings {
    /// 应用到运行时资源
    pub fn apply(&self, opt: &mut OptionState, chat: &mut ChatFilter) {
        opt.skill_mode_ctrl = self.skill_mode_ctrl;
        opt.skill_bar = self.skill_bar;
        opt.effect = self.effect;
        opt.drop_view = self.drop_view;
        opt.name_view = self.name_view;
        opt.hp_view = self.hp_view;
        opt.sound_volume = self.sound_volume.clamp(0.0, 1.0);
        opt.music_volume = self.music_volume.clamp(0.0, 1.0);
        opt.allow_observe = self.allow_observe;
        opt.new_move = self.new_move;
        chat.normal = self.filter_normal;
        chat.whisper = self.filter_whisper;
        chat.shout = self.filter_shout;
        chat.system = self.filter_system;
        chat.lover = self.filter_lover;
        chat.mentor = self.filter_mentor;
        chat.group = self.filter_group;
        chat.guild = self.filter_guild;
        chat.transparent = self.transparent_chat;
    }

    /// 序列化为 [Settings] 段文本
    pub fn to_ini(&self) -> String {
        let mut s = String::from("[Settings]\n");
        let b = |v: bool| if v { "true" } else { "false" };
        s.push_str(&format!("SkillModeCtrl={}\n", b(self.skill_mode_ctrl)));
        s.push_str(&format!("SkillBar={}\n", b(self.skill_bar)));
        s.push_str(&format!("Effect={}\n", b(self.effect)));
        s.push_str(&format!("DropView={}\n", b(self.drop_view)));
        s.push_str(&format!("NameView={}\n", b(self.name_view)));
        s.push_str(&format!("HPView={}\n", b(self.hp_view)));
        s.push_str(&format!("SoundVolume={:.2}\n", self.sound_volume));
        s.push_str(&format!("MusicVolume={:.2}\n", self.music_volume));
        s.push_str(&format!("AllowObserve={}\n", b(self.allow_observe)));
        s.push_str(&format!("NewMove={}\n", b(self.new_move)));
        s.push_str(&format!("FilterNormal={}\n", b(self.filter_normal)));
        s.push_str(&format!("FilterWhisper={}\n", b(self.filter_whisper)));
        s.push_str(&format!("FilterShout={}\n", b(self.filter_shout)));
        s.push_str(&format!("FilterSystem={}\n", b(self.filter_system)));
        s.push_str(&format!("FilterLover={}\n", b(self.filter_lover)));
        s.push_str(&format!("FilterMentor={}\n", b(self.filter_mentor)));
        s.push_str(&format!("FilterGroup={}\n", b(self.filter_group)));
        s.push_str(&format!("FilterGuild={}\n", b(self.filter_guild)));
        s.push_str(&format!("TransparentChat={}\n", b(self.transparent_chat)));
        s
    }
}

/// 解析布尔/浮点配置值
fn parse_bool(v: &str) -> Option<bool> {
    match v.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}

/// 从 config.ini 读取 [Settings] 段（工作目录优先，其次 crate 根）
pub fn load_persisted_settings() -> PersistedSettings {
    let mut s = PersistedSettings::default();
    let mut content = std::fs::read_to_string("config.ini").ok();
    if content.is_none() {
        content = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config.ini"),
        )
        .ok();
    }
    let Some(content) = content else { return s };
    let mut section = String::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        if !section.eq_ignore_ascii_case("Settings") {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let value = v.trim();
        match key.to_ascii_lowercase().as_str() {
            "skillmodectrl" => s.skill_mode_ctrl = parse_bool(value).unwrap_or(s.skill_mode_ctrl),
            "skillbar" => s.skill_bar = parse_bool(value).unwrap_or(s.skill_bar),
            "effect" => s.effect = parse_bool(value).unwrap_or(s.effect),
            "dropview" => s.drop_view = parse_bool(value).unwrap_or(s.drop_view),
            "nameview" => s.name_view = parse_bool(value).unwrap_or(s.name_view),
            "hpview" => s.hp_view = parse_bool(value).unwrap_or(s.hp_view),
            "soundvolume" => s.sound_volume = value.parse().unwrap_or(s.sound_volume),
            "musicvolume" => s.music_volume = value.parse().unwrap_or(s.music_volume),
            "allowobserve" => s.allow_observe = parse_bool(value).unwrap_or(s.allow_observe),
            "newmove" => s.new_move = parse_bool(value).unwrap_or(s.new_move),
            "filternormal" => s.filter_normal = parse_bool(value).unwrap_or(s.filter_normal),
            "filterwhisper" => s.filter_whisper = parse_bool(value).unwrap_or(s.filter_whisper),
            "filtershout" => s.filter_shout = parse_bool(value).unwrap_or(s.filter_shout),
            "filtersystem" => s.filter_system = parse_bool(value).unwrap_or(s.filter_system),
            "filterlover" => s.filter_lover = parse_bool(value).unwrap_or(s.filter_lover),
            "filtermentor" => s.filter_mentor = parse_bool(value).unwrap_or(s.filter_mentor),
            "filtergroup" => s.filter_group = parse_bool(value).unwrap_or(s.filter_group),
            "filterguild" => s.filter_guild = parse_bool(value).unwrap_or(s.filter_guild),
            "transparentchat" => {
                s.transparent_chat = parse_bool(value).unwrap_or(s.transparent_chat);
            }
            _ => {}
        }
    }
    s
}

/// 写回 config.ini：保留 [Network]/[Login] 等段，替换/追加 [Settings] 段
pub fn save_persisted_settings(settings: &PersistedSettings) {
    let path = config_path();
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    // 移除旧 [Settings] 段（到下一个 [ 段或文件尾）
    let mut out = String::new();
    let mut skip = false;
    let mut removed = false;
    for raw in existing.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            if line.eq_ignore_ascii_case("[settings]") {
                skip = true;
                removed = true;
                continue;
            }
            skip = false;
        }
        if !skip {
            out.push_str(raw);
            out.push('\n');
        }
    }
    if !removed && !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&settings.to_ini());
    let _ = std::fs::write(&path, out);
}

/// config.ini 路径：工作目录优先，其次 crate 根；不存在则用工作目录
fn config_path() -> std::path::PathBuf {
    if std::path::Path::new("config.ini").exists() {
        std::path::PathBuf::from("config.ini")
    } else {
        let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config.ini");
        if crate_root.exists() {
            crate_root
        } else {
            std::path::PathBuf::from("config.ini")
        }
    }
}

#[derive(Resource)]
struct SettingsSaveState {
    snapshot: PersistedSettings,
    dirty_since: Option<f64>,
}

impl Default for SettingsSaveState {
    fn default() -> Self {
        Self {
            snapshot: PersistedSettings::default(),
            dirty_since: None,
        }
    }
}

/// 启动进游戏时应用持久化设置
fn apply_settings_on_start(
    mut opt: ResMut<OptionState>,
    mut chat: ResMut<ChatFilter>,
    mut state: ResMut<SettingsSaveState>,
) {
    let loaded = load_persisted_settings();
    loaded.apply(&mut opt, &mut chat);
    state.snapshot = loaded;
    tracing::info!("⚙️ 已加载 config.ini [Settings]");
}

/// 变更检测 + 防抖 1s 保存
fn persist_settings_system(
    opt: Res<OptionState>,
    chat: Res<ChatFilter>,
    time: Res<Time>,
    mut state: ResMut<SettingsSaveState>,
) {
    let cur = PersistedSettings::from((&*opt, &*chat));
    if cur != state.snapshot {
        state.snapshot = cur.clone();
        state.dirty_since = Some(time.elapsed_secs_f64());
    }
    if let Some(t) = state.dirty_since {
        if time.elapsed_secs_f64() - t >= 1.0 {
            state.dirty_since = None;
            save_persisted_settings(&cur);
            tracing::debug!("💾 设置已保存到 config.ini");
        }
    }
}

pub struct ClientSettingsPlugin;

impl Plugin for ClientSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SettingsSaveState>();
        app.add_systems(OnEnter(AppState::Game), apply_settings_on_start);
        app.add_systems(
            Update,
            persist_settings_system.run_if(in_state(AppState::Game)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_ini_roundtrip() {
        let mut s = PersistedSettings::default();
        s.skill_mode_ctrl = false;
        s.effect = false;
        s.sound_volume = 0.35;
        s.music_volume = 0.0;
        s.filter_guild = true;
        s.transparent_chat = true;
        s.new_move = false;

        let ini = s.to_ini();
        // 通过临时文件读写模拟 load
        let dir = std::env::temp_dir().join(format!("bevy_settings_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.ini");
        std::fs::write(&path, &ini).unwrap();

        // 直接复用 load 逻辑：手工解析段
        let content = std::fs::read_to_string(&path).unwrap();
        let mut loaded = PersistedSettings::default();
        let mut section = String::new();
        for raw in content.lines() {
            let line = raw.trim();
            if line.starts_with('[') {
                section = line[1..line.len() - 1].trim().to_string();
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let key = k.trim().to_ascii_lowercase();
            let value = v.trim();
            match key.as_str() {
                "skillmodectrl" => loaded.skill_mode_ctrl = parse_bool(value).unwrap(),
                "effect" => loaded.effect = parse_bool(value).unwrap(),
                "soundvolume" => loaded.sound_volume = value.parse().unwrap(),
                "musicvolume" => loaded.music_volume = value.parse().unwrap(),
                "filterguild" => loaded.filter_guild = parse_bool(value).unwrap(),
                "transparentchat" => loaded.transparent_chat = parse_bool(value).unwrap(),
                "newmove" => loaded.new_move = parse_bool(value).unwrap(),
                _ => {}
            }
        }
        assert_eq!(loaded, s);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_apply_writes_resources() {
        let mut opt = OptionState::default();
        let mut chat = ChatFilter::default();
        let mut s = PersistedSettings::default();
        s.skill_mode_ctrl = false;
        s.filter_whisper = true;
        s.apply(&mut opt, &mut chat);
        assert!(!opt.skill_mode_ctrl);
        assert!(chat.whisper);
    }
}
