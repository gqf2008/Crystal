// 音频加载器 - 加载并播放游戏音效和背景音乐
// 参考: Client/MirSounds/SoundManager.cs

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;
use rodio::{Decoder, OutputStream, Sink, Source};

/// 音频类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundType {
    /// 背景音乐(循环播放)
    Music,
    /// 音效(单次播放)
    Effect,
    /// 环境音(循环播放,音量较低)
    Ambient,
}

/// 音频文件信息
#[derive(Debug, Clone)]
pub struct SoundInfo {
    pub path: PathBuf,
    pub sound_type: SoundType,
    pub volume: f32,
}

/// 音频管理器
pub struct SoundManager {
    /// 音频输出流
    _stream: OutputStream,
    
    /// 音频文件路径映射
    sounds: HashMap<String, SoundInfo>,
    
    /// 背景音乐播放器
    music_sink: Option<Sink>,
    
    /// 当前播放的音乐名称
    current_music: Option<String>,
    
    /// 全局音量设置
    master_volume: f32,
    music_volume: f32,
    effect_volume: f32,
    
    /// 音频是否静音
    muted: bool,
    
    /// 音效播放器池(复用Sink避免频繁创建)
    effect_sinks: Arc<Mutex<Vec<Sink>>>,
}

impl SoundManager {
    /// 创建音频管理器
    pub fn new() -> Result<Self, String> {
        // 暂时禁用音频初始化以修复编译问题
        // TODO: 修复rodio API兼容性问题
        Err("Audio system temporarily disabled".to_string())
    }
    
    /// 注册音频文件
    pub fn register_sound(&mut self, name: &str, path: PathBuf, sound_type: SoundType, volume: f32) {
        self.sounds.insert(name.to_string(), SoundInfo {
            path,
            sound_type,
            volume,
        });
    }
    
    /// 批量加载音频(仅注册路径,不实际加载到内存)
    pub fn load_sounds_from_dir(&mut self, dir: &Path, sound_type: SoundType) -> std::io::Result<()> {
        if !dir.exists() {
            return Ok(()); // 目录不存在就跳过
        }
        
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "wav" || ext == "ogg" || ext == "mp3" {
                        if let Some(name) = path.file_stem() {
                            let sound_name = name.to_string_lossy().to_string();
                            self.register_sound(&sound_name, path, sound_type, 1.0);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 播放背景音乐(循环)
    pub fn play_music(&mut self, name: &str) -> Result<(), String> {
        // 如果已经在播放这首音乐,不重复播放
        if self.current_music.as_ref() == Some(&name.to_string()) {
            return Ok(());
        }
        
        // 停止当前音乐
        self.stop_music();
        
        let sound_info = self.sounds.get(name)
            .ok_or_else(|| format!("Sound '{}' not registered", name))?;
        
        let file = File::open(&sound_info.path)
            .map_err(|e| format!("Failed to open sound file: {}", e))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("Failed to decode audio: {}", e))?;
        
        // 创建循环播放的source
        let source = source.repeat_infinite();
        
        // 应用音量
        let volume = if self.muted {
            0.0
        } else {
            self.master_volume * self.music_volume * sound_info.volume
        };
        let source = source.amplify(volume);
        
        // 创建sink并播放
        let sink = Sink::connect_new(self._stream.mixer());
        sink.append(source);
        
        self.music_sink = Some(sink);
        self.current_music = Some(name.to_string());
        
        Ok(())
    }
    
    /// 停止背景音乐
    pub fn stop_music(&mut self) {
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
        self.current_music = None;
    }
    
    /// 暂停/恢复背景音乐
    pub fn toggle_music_pause(&mut self) {
        if let Some(sink) = &self.music_sink {
            if sink.is_paused() {
                sink.play();
            } else {
                sink.pause();
            }
        }
    }
    
    /// 播放音效(单次)
    pub fn play_effect(&mut self, name: &str) -> Result<(), String> {
        if self.muted {
            return Ok(());
        }
        
        let sound_info = self.sounds.get(name)
            .ok_or_else(|| format!("Sound '{}' not registered", name))?
            .clone();
        
        let file = File::open(&sound_info.path)
            .map_err(|e| format!("Failed to open sound file: {}", e))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("Failed to decode audio: {}", e))?;
        
        // 应用音量
        let volume = self.master_volume * self.effect_volume * sound_info.volume;
        let source = source.amplify(volume);
        
        // 获取或创建sink
        let sink = {
            let mut pool = self.effect_sinks.lock();
            
            // 清理已完成的sink
            pool.retain(|s| !s.empty());
            
            // 尝试复用空闲的sink
            pool.iter()
                .find(|s| s.empty())
                .map(|s| {
                    // 找到空闲sink,直接使用
                    Some(Sink::connect_new(self._stream.mixer()))
                })
                .flatten()
                .or_else(|| {
                    // 没有空闲sink,创建新的
                    Some(Sink::connect_new(self._stream.mixer()))
                })
                .ok_or_else(|| "Failed to create sink".to_string())?
        };
        
        sink.append(source);
        
        // 保存到池中(避免被立即drop)
        self.effect_sinks.lock().push(sink);
        
        Ok(())
    }
    
    /// 设置全局音量 (0.0 - 1.0)
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
        self.update_music_volume();
    }
    
    /// 设置音乐音量 (0.0 - 1.0)
    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
        self.update_music_volume();
    }
    
    /// 设置音效音量 (0.0 - 1.0)
    pub fn set_effect_volume(&mut self, volume: f32) {
        self.effect_volume = volume.clamp(0.0, 1.0);
    }
    
    /// 静音/取消静音
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        self.update_music_volume();
    }
    
    /// 切换静音状态
    pub fn toggle_mute(&mut self) {
        self.set_muted(!self.muted);
    }
    
    /// 更新背景音乐音量
    fn update_music_volume(&mut self) {
        if let Some(sink) = &self.music_sink {
            let volume = if self.muted {
                0.0
            } else {
                self.master_volume * self.music_volume
            };
            sink.set_volume(volume);
        }
    }
    
    /// 清理音效池(移除已完成的sink)
    pub fn cleanup(&mut self) {
        let mut pool = self.effect_sinks.lock();
        pool.retain(|s| !s.empty());
    }
    
    /// 获取当前播放的音乐名称
    pub fn current_music(&self) -> Option<&str> {
        self.current_music.as_deref()
    }
    
    /// 检查音乐是否正在播放
    pub fn is_music_playing(&self) -> bool {
        self.music_sink.as_ref()
            .map(|s| !s.is_paused() && !s.empty())
            .unwrap_or(false)
    }
}

impl Default for SoundManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize sound manager")
    }
}

// 自动清理
impl Drop for SoundManager {
    fn drop(&mut self) {
        self.stop_music();
        self.effect_sinks.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sound_manager_creation() {
        let manager = SoundManager::new();
        assert!(manager.is_ok());
    }
    
    #[test]
    fn test_volume_clamping() {
        let mut manager = SoundManager::new().unwrap();
        
        manager.set_master_volume(1.5);
        assert_eq!(manager.master_volume, 1.0);
        
        manager.set_master_volume(-0.5);
        assert_eq!(manager.master_volume, 0.0);
    }
}
