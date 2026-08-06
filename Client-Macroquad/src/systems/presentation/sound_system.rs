// ============================================================================
// Layer 5: State Update - SoundSystem
// Priority: 520
// ============================================================================
//
// **职责**：
// - 音效触发管理
// - 3D音效位置计算
// - 音量控制
//
// **逻辑来源**：
// - C# SoundManager.PlaySound(): 播放音效
// - 根据距离调整音量
//
// ============================================================================

use crate::components::{OneShotSoundEmitter, PersistentSound, Position, SoundTrigger, SoundType};
use crate::game::GameContext;
use crate::game::GameResult;
use crate::systems::LogicSystem;
use macroquad::audio::{play_sound, stop_sound, PlaySoundParams, Sound};
use macroquad::experimental::coroutines::{start_coroutine, Coroutine};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

fn parse_sound_list(text: &str) -> HashMap<u32, String> {
    let mut map = HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        let Some((id_str, rhs)) = line.split_once(':') else {
            continue;
        };

        let id_str = id_str.trim();
        let Some(file) = rhs.split_whitespace().next() else {
            continue;
        };

        let Ok(id) = id_str.parse::<u32>() else {
            continue;
        };

        map.insert(id, file.to_string());
    }

    map
}

enum PlayAttempt {
    Played,
    Loading,
    Missing,
}

/// 音效系统
#[derive(ecs_macros::LogicSystem)]
pub struct SoundSystem {
    /// 听者位置(通常是摄像机/玩家位置)
    listener_pos: (f32, f32),
    /// 最大听音距离
    max_distance: f32,

    /// 已加载的音频缓存（key 为 SoundTrigger.sound_file）
    cache: HashMap<String, Sound>,
    /// 正在加载中的音频（懒加载，避免在 update 里阻塞）
    loading: HashMap<String, Coroutine<Result<Sound, macroquad::Error>>>,
    /// 缺失/加载失败的音频（去重，避免刷屏）
    missing: HashSet<String>,

    /// SoundList.lst 映射：sound_id -> 文件名
    sound_list_loaded: bool,
    sound_list: HashMap<u32, String>,

    /// PersistentSound 播放跟踪：entity → (sound_file, looping)
    /// 用于避免每帧重复触发 play_sound（looped 声音会一直播放直到 stop）
    persistent_playing: HashMap<hecs::Entity, (String, bool)>,
}

impl SoundSystem {
    pub fn new() -> Self {
        Self {
            listener_pos: (0.0, 0.0),
            max_distance: 1000.0,
            cache: HashMap::new(),
            loading: HashMap::new(),
            missing: HashSet::new(),
            sound_list_loaded: false,
            sound_list: HashMap::new(),
            persistent_playing: HashMap::new(),
        }
    }

    fn sound_root_dir() -> PathBuf {
        // 跟 test_game_scene 的 Data/ 处理一致：使用构建期固定根目录，避免工作目录差异。
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Sound")
    }

    fn ensure_sound_list_loaded(&mut self) {
        if self.sound_list_loaded {
            return;
        }
        self.sound_list_loaded = true;

        let path = Self::sound_root_dir().join("SoundList.lst");
        match fs::read_to_string(&path) {
            Ok(text) => {
                self.sound_list = parse_sound_list(&text);
                if cfg!(debug_assertions) {
                    println!(
                        "🔊 [SOUND] SoundList loaded: {} entries",
                        self.sound_list.len()
                    );
                }
            }
            Err(e) => {
                if cfg!(debug_assertions) {
                    println!(
                        "⚠️ [SOUND] SoundList missing/unreadable: {} err={}",
                        path.display(),
                        e
                    );
                }
            }
        }
    }

    fn resolve_to_canonical_key(&mut self, sound_ref: &str) -> Option<String> {
        let s = sound_ref.trim();
        if s.is_empty() {
            return None;
        }

        // 如果是纯数字，优先当作 sound_id，通过 SoundList.lst 映射到真实文件名。
        if s.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(id) = s.parse::<u32>() {
                self.ensure_sound_list_loaded();

                if let Some(file) = self.sound_list.get(&id) {
                    return Some(file.clone());
                }

                // 兜底：有些资源可能本身就是 "{id}.wav"。
                return Some(format!("{}.wav", id));
            }
        }

        Some(s.to_string())
    }

    fn resolve_sound_path(sound_file: &str) -> PathBuf {
        let p = Path::new(sound_file);
        if p.is_absolute() {
            return p.to_path_buf();
        }

        // 允许调用方传入 "Sound/xxx.wav" 或 "sound/xxx.wav"。
        if sound_file.starts_with("Sound/")
            || sound_file.starts_with("Sound\\")
            || sound_file.starts_with("sound/")
            || sound_file.starts_with("sound\\")
        {
            return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(sound_file);
        }

        Self::sound_root_dir().join(sound_file)
    }

    fn poll_loading(&mut self) {
        // 将已完成的 coroutine 收割进 cache/missing。
        let mut finished: Vec<String> = Vec::new();
        for (k, co) in self.loading.iter() {
            if co.is_done() {
                finished.push(k.clone());
            }
        }

        for k in finished {
            let Some(co) = self.loading.remove(&k) else {
                continue;
            };

            let Some(result) = co.retrieve() else {
                // 理论上 is_done 后应当能 retrieve；拿不到就先跳过。
                continue;
            };

            match result {
                Ok(sound) => {
                    self.cache.insert(k, sound);
                }
                Err(e) => {
                    self.missing.insert(k.clone());
                    if cfg!(debug_assertions) {
                        println!("⚠️ [SOUND] load failed: {} err={}", k, e);
                    }
                }
            }
        }
    }

    fn ensure_loading(&mut self, key: &str) {
        if self.cache.contains_key(key)
            || self.missing.contains(key)
            || self.loading.contains_key(key)
        {
            return;
        }

        let path = Self::resolve_sound_path(key);
        let path_str = path.to_string_lossy().replace('\\', "/");
        let key = key.to_string();

        let co = start_coroutine(async move {
            let bytes = macroquad::file::load_file(&path_str).await?;
            macroquad::audio::load_sound_from_bytes(&bytes).await
        });

        self.loading.insert(key, co);
    }

    fn try_play(&mut self, sound_ref: &str, looping: bool, volume: f32) -> PlayAttempt {
        let Some(key) = self.resolve_to_canonical_key(sound_ref) else {
            return PlayAttempt::Missing;
        };

        if let Some(sound) = self.cache.get(&key) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: looping,
                    volume: volume.clamp(0.0, 1.0),
                },
            );
            return PlayAttempt::Played;
        }

        if self.missing.contains(&key) {
            return PlayAttempt::Missing;
        }

        self.ensure_loading(&key);
        PlayAttempt::Loading
    }

    /// 计算3D音效音量(基于距离衰减)
    #[allow(dead_code)]
    fn calculate_volume(&self, sound_pos: (f32, f32)) -> f32 {
        let dx = sound_pos.0 - self.listener_pos.0;
        let dy = sound_pos.1 - self.listener_pos.1;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance >= self.max_distance {
            0.0
        } else {
            (1.0 - distance / self.max_distance).max(0.0)
        }
    }

    fn try_update_listener_pos(&mut self, ctx: &GameContext) {
        // 监听者位置优先取 Camera，其次取 LocalPlayer。
        // 这里只做“有就用”的轻量策略，避免引入额外耦合。
        if let Some(entity) = ctx
            .world
            .iter()
            .find_map(|e| e.get::<&crate::components::Camera>().map(|_| e.entity()))
        {
            // Camera 的位置通常放在同实体的 Position 上。
            if let Ok(p) = ctx.world.get::<&Position>(entity) {
                self.listener_pos = (p.x, p.y);
                return;
            }
        }

        if let Some(entity) = ctx.world.iter().find_map(|e| {
            e.get::<&crate::components::LocalPlayer>()
                .map(|_| e.entity())
        }) {
            if let Ok(p) = ctx.world.get::<&Position>(entity) {
                self.listener_pos = (p.x, p.y);
            }
        }
    }

    fn attenuation_for_entity(&self, ctx: &GameContext, entity: hecs::Entity) -> f32 {
        let Ok(pos) = ctx.world.get::<&Position>(entity) else {
            return 1.0;
        };
        self.calculate_volume((pos.x, pos.y))
    }

    fn global_volume_for_type(&self, ctx: &GameContext, sound_type: SoundType) -> f32 {
        // 目前 Settings 只有一个总音量；后续可扩展为 sound/music 独立音量。
        let base = ctx.settings.volume.clamp(0.0, 1.0);

        match sound_type {
            SoundType::BackgroundMusic => {
                if ctx.settings.music_enabled {
                    base
                } else {
                    0.0
                }
            }
            _ => {
                if ctx.settings.sound_enabled {
                    base
                } else {
                    0.0
                }
            }
        }
    }

    fn should_play(&self, ctx: &GameContext, trigger: &SoundTrigger) -> bool {
        self.global_volume_for_type(ctx, trigger.sound_type) > 0.0
            && trigger.volume > 0.0
            && !trigger.sound_file.is_empty()
    }
}

impl Default for SoundSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicSystem for SoundSystem {
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        // 这是一个“可渐进完善”的声音系统骨架：
        // - 读取/消费 SoundTrigger（一次性触发）
        // - 读取 PersistentSound（持续音效）
        // - 应用 Settings + 3D 衰减（基于 Position 距离）
        // - 目前不做真正音频播放，仅打点日志；后续接入 macroquad::audio 或自定义后端即可。

        self.poll_loading();
        self.try_update_listener_pos(ctx);

        // 1) 一次性触发：播放后移除 SoundTrigger
        let mut to_remove: Vec<hecs::Entity> = Vec::new();
        let mut to_despawn: Vec<hecs::Entity> = Vec::new();
        for (entity, trigger) in ctx.world.iter().filter_map(|e| {
            let trigger = e.get::<&SoundTrigger>()?;
            Some((e.entity(), trigger))
        }) {
            if !self.should_play(ctx, &trigger) {
                to_remove.push(entity);
                continue;
            }

            let attenuation = self.attenuation_for_entity(ctx, entity);
            let global = self.global_volume_for_type(ctx, trigger.sound_type);
            let final_volume = (trigger.volume * global * attenuation).clamp(0.0, 1.0);

            // 这里保留“对齐 C# 逻辑”的落点：
            // - 技能/攻击/受击等由上游系统写入 SoundTrigger
            // - SoundSystem 统一做音量策略与播放/缓存
            if cfg!(debug_assertions) && sound_debug_log_enabled() {
                println!(
                    "🔊 [SOUND] SoundTrigger file={} type={:?} vol={:.3} (global={:.3} attn={:.3}) looping={} (listener=({:.1},{:.1}))",
                    trigger.sound_file,
                    trigger.sound_type,
                    final_volume,
                    global,
                    attenuation,
                    trigger.looping,
                    self.listener_pos.0,
                    self.listener_pos.1
                );
            }
            tracing::debug!(
                target: "sound",
                "SoundTrigger: file={} type={:?} vol={:.3} (global={:.3} attn={:.3}) looping={}",
                trigger.sound_file,
                trigger.sound_type,
                final_volume,
                global,
                attenuation,
                trigger.looping
            );

            // SoundTrigger 已接入实际播放：try_play 处理缓存/加载/播放
            match self.try_play(&trigger.sound_file, trigger.looping, final_volume) {
                PlayAttempt::Played | PlayAttempt::Missing => {
                    // 若是临时 OneShotSoundEmitter，则播放完成后直接销毁实体，避免空实体堆积。
                    if ctx.world.get::<&OneShotSoundEmitter>(entity).is_ok() {
                        to_despawn.push(entity);
                    } else {
                        to_remove.push(entity);
                    }
                }
                PlayAttempt::Loading => {
                    // 音频尚在加载：保留 SoundTrigger，待下帧加载完后播放。
                }
            }
        }

        for entity in to_remove {
            let _ = ctx.world.remove_one::<SoundTrigger>(entity);
        }

        for entity in to_despawn {
            let _ = ctx.world.despawn(entity);
        }

        // 2) 持续音效：维护 PersistentSound 的播放/停止状态
        // 收集当前存活的 PersistentSound entity（用于后续清理已移除的）
        let mut current_ps_entities: Vec<hecs::Entity> = Vec::new();

        for ps in ctx
            .world
            .query::<(&hecs::Entity, &PersistentSound)>()
            .iter()
        {
            let (entity, ps) = (ps.0, &ps.1);
            current_ps_entities.push(*entity);
            let global = self.global_volume_for_type(ctx, ps.sound_type);
            let final_volume = (ps.volume * global).clamp(0.0, 1.0);

            let was_playing = self.persistent_playing.contains_key(entity);

            if ps.is_playing && final_volume > 0.0 {
                // 需要播放
                if !was_playing {
                    // 首次播放或重新开始
                    if let Some(sound) = self.cache.get(&ps.sound_file) {
                        play_sound(
                            sound,
                            PlaySoundParams {
                                looped: ps.looping,
                                volume: final_volume,
                            },
                        );
                        tracing::debug!(
                            target: "sound",
                            "PersistentSound PLAY: file={} vol={:.3} looping={}",
                            ps.sound_file, final_volume, ps.looping
                        );
                    } else if !self.missing.contains(&ps.sound_file) {
                        self.ensure_loading(&ps.sound_file);
                    }
                    self.persistent_playing
                        .insert(*entity, (ps.sound_file.clone(), ps.looping));
                }
                // 已在播放：不重复调用 play_sound（looped 声音会持续播放）
            } else if was_playing {
                // 已播放但 now is_playing=false 或 volume=0 → 停止
                if let Some((ref file, _)) = self.persistent_playing.remove(entity) {
                    if let Some(sound) = self.cache.get(file) {
                        stop_sound(sound);
                        tracing::debug!(target: "sound", "PersistentSound STOP: file={}", file);
                    }
                }
            }
        }

        // 清理已不存在的 PersistentSound 实体（组件被移除或实体被 despawn）
        self.persistent_playing.retain(|entity, (file, _)| {
            if current_ps_entities.contains(entity) {
                true
            } else {
                if let Some(sound) = self.cache.get(file) {
                    stop_sound(sound);
                }
                false
            }
        });

        Ok(())
    }
}

fn sound_debug_log_enabled() -> bool {
    // 默认关闭，避免 dev 模式下每帧刷屏；需要时手动开启：
    // PowerShell: $env:CRYSTAL_SOUND_LOG='1'
    // CMD: set CRYSTAL_SOUND_LOG=1
    std::env::var("CRYSTAL_SOUND_LOG").is_ok_and(|v| v == "1")
}
