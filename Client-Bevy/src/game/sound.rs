// ============================================================================
// 音效系统（M11）
// 数据：Client-Macroquad/Sound/SoundList.lst（sound_id → 文件名）+ *.wav
// 用法：play_sound(commands, assets, bank, id)
// 挂接：攻击 Swing(10050)、受击 Struck(10060)
// ============================================================================

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings};
use bevy::prelude::*;

/// 音效库（SoundList.lst 映射）
#[derive(Resource, Default)]
pub struct SoundBank {
    pub map: HashMap<u32, String>,
    pub root: PathBuf,
}

impl SoundBank {
    /// 加载 SoundList.lst（参考 Client-Macroquad/src/systems/presentation/sound_system.rs）
    pub fn load(&mut self) {
        // 共享主仓库的 Sound 目录（独立 worktree 场景：Crystal-bevy → ../Crystal）
        let candidate = [
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Client-Macroquad/Sound"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Crystal/Client-Macroquad/Sound"),
        ];
        self.root = candidate
            .into_iter()
            .find(|p| p.join("SoundList.lst").exists())
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Crystal/Client-Macroquad/Sound"));
        let lst = self.root.join("SoundList.lst");
        let Ok(text) = std::fs::read_to_string(&lst) else {
            tracing::warn!("🔊 SoundList.lst 未找到: {}", lst.display());
            return;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            if let Some((id, file)) = line.split_once(':') {
                if let Ok(id) = id.trim().parse::<u32>() {
                    self.map.insert(id, file.trim().to_string());
                }
            }
        }
        tracing::info!("🔊 SoundList 加载: {} 条音效", self.map.len());
    }

    pub fn file_for(&self, id: u32) -> Option<PathBuf> {
        self.map.get(&id).map(|f| self.root.join(f))
    }
}

/// 播放音效（读取 wav → AudioSource → 一次性播放）
pub fn play_sound(
    commands: &mut Commands,
    assets: &mut Assets<AudioSource>,
    bank: &SoundBank,
    id: u32,
) {
    let Some(path) = bank.file_for(id) else {
        return;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let source = AudioSource {
        bytes: Arc::from(bytes),
    };
    let handle = assets.add(source);
    commands.spawn((
        AudioPlayer(handle),
        PlaybackSettings::DESPAWN,
    ));
}

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundBank>();
        app.add_systems(Startup, load_sound_bank);
    }
}

fn load_sound_bank(mut bank: ResMut<SoundBank>) {
    bank.load();
}
