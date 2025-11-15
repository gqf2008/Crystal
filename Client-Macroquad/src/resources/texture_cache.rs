//! 纹理缓存管理系统
//!
//! 提供高效的纹理缓存，支持：
//! - LRU (最近最少使用) 淘汰策略
//! - 自动内存管理和清理
//! - egui 和 macroquad 纹理的统一管理

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use egui_macroquad::egui;
use macroquad::prelude::*;

/// 纹理缓存项
struct CacheEntry {
    /// macroquad 纹理
    mq_texture: Option<Texture2D>,
    /// egui 纹理句柄
    egui_texture: Option<egui::TextureHandle>,
    /// 最后访问时间
    last_access: Instant,
    /// 访问次数（用于统计）
    access_count: u32,
}

/// 纹理缓存键
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    /// 库名称
    pub library: String,
    /// 图像索引
    pub index: usize,
}

impl CacheKey {
    pub fn new(library: impl Into<String>, index: usize) -> Self {
        Self {
            library: library.into(),
            index,
        }
    }
}

/// LRU 纹理缓存管理器
pub struct TextureCache {
    /// 纹理缓存
    cache: HashMap<CacheKey, CacheEntry>,
    /// LRU 访问顺序队列
    lru_queue: VecDeque<CacheKey>,
    /// 最大缓存条目数
    max_entries: usize,
    /// 缓存过期时间
    expire_duration: Duration,
    /// 统计信息
    stats: CacheStats,
}

/// 缓存统计信息
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_accesses: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_accesses == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_accesses as f64
        }
    }
}

impl TextureCache {
    /// 创建新的纹理缓存
    ///
    /// # 参数
    /// - `max_entries`: 最大缓存条目数
    /// - `expire_duration`: 缓存过期时间
    pub fn new(max_entries: usize, expire_duration: Duration) -> Self {
        Self {
            cache: HashMap::with_capacity(max_entries),
            lru_queue: VecDeque::with_capacity(max_entries),
            max_entries,
            expire_duration,
            stats: CacheStats::default(),
        }
    }

    /// 创建默认配置的缓存
    /// - 最大1000个纹理
    /// - 30秒过期时间
    pub fn with_defaults() -> Self {
        Self::new(1000, Duration::from_secs(30))
    }

    /// 获取 macroquad 纹理（从缓存或创建）
    pub fn get_mq_texture<F>(&mut self, key: CacheKey, creator: F) -> Option<Texture2D>
    where
        F: FnOnce() -> Option<Texture2D>,
    {
        self.stats.total_accesses += 1;

        // 检查缓存
        if let Some(entry) = self.cache.get_mut(&key) {
            entry.last_access = Instant::now();
            entry.access_count += 1;
            self.stats.hits += 1;
            
            // 更新 LRU 队列
            self.lru_queue.retain(|k| k != &key);
            self.lru_queue.push_back(key.clone());
            
            if let Some(ref texture) = entry.mq_texture {
                return Some(texture.clone());
            }
        }

        // 缓存未命中，创建新纹理
        self.stats.misses += 1;
        let texture = creator()?;

        // 检查是否需要淘汰
        self.evict_if_needed();

        // 插入新条目
        let entry = CacheEntry {
            mq_texture: Some(texture.clone()),
            egui_texture: None,
            last_access: Instant::now(),
            access_count: 1,
        };

        self.cache.insert(key.clone(), entry);
        self.lru_queue.push_back(key);

        Some(texture)
    }

    /// 获取 egui 纹理（从缓存或创建）
    pub fn get_egui_texture<F>(
        &mut self,
        ctx: &egui::Context,
        key: CacheKey,
        creator: F,
    ) -> Option<egui::TextureHandle>
    where
        F: FnOnce() -> Option<egui::TextureHandle>,
    {
        self.stats.total_accesses += 1;

        // 检查缓存
        if let Some(entry) = self.cache.get_mut(&key) {
            entry.last_access = Instant::now();
            entry.access_count += 1;
            self.stats.hits += 1;
            
            // 更新 LRU 队列
            self.lru_queue.retain(|k| k != &key);
            self.lru_queue.push_back(key.clone());
            
            if let Some(ref texture) = entry.egui_texture {
                return Some(texture.clone());
            }
        }

        // 缓存未命中，创建新纹理
        self.stats.misses += 1;
        let texture = creator()?;

        // 检查是否需要淘汰
        self.evict_if_needed();

        // 更新或插入条目
        if let Some(entry) = self.cache.get_mut(&key) {
            entry.egui_texture = Some(texture.clone());
        } else {
            let entry = CacheEntry {
                mq_texture: None,
                egui_texture: Some(texture.clone()),
                last_access: Instant::now(),
                access_count: 1,
            };
            self.cache.insert(key.clone(), entry);
            self.lru_queue.push_back(key);
        }

        Some(texture)
    }

    /// 检查并执行淘汰策略
    fn evict_if_needed(&mut self) {
        while self.cache.len() >= self.max_entries {
            if let Some(oldest_key) = self.lru_queue.pop_front() {
                self.cache.remove(&oldest_key);
                self.stats.evictions += 1;
            } else {
                break;
            }
        }
    }

    /// 清理过期的纹理
    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        let mut expired_keys = Vec::new();

        for (key, entry) in &self.cache {
            if now.duration_since(entry.last_access) > self.expire_duration {
                expired_keys.push(key.clone());
            }
        }

        for key in expired_keys {
            self.cache.remove(&key);
            self.lru_queue.retain(|k| k != &key);
            self.stats.evictions += 1;
        }
    }

    /// 清空所有缓存
    pub fn clear(&mut self) {
        self.cache.clear();
        self.lru_queue.clear();
        self.stats = CacheStats::default();
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// 获取当前缓存大小
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// 移除指定键的缓存
    pub fn remove(&mut self, key: &CacheKey) {
        if self.cache.remove(key).is_some() {
            self.lru_queue.retain(|k| k != key);
        }
    }

    /// 预热缓存（批量加载）
    pub fn warmup<F>(&mut self, keys: Vec<CacheKey>, creator: F)
    where
        F: Fn(&CacheKey) -> Option<Texture2D>,
    {
        for key in keys {
            if !self.cache.contains_key(&key) {
                if let Some(texture) = creator(&key) {
                    let entry = CacheEntry {
                        mq_texture: Some(texture),
                        egui_texture: None,
                        last_access: Instant::now(),
                        access_count: 0,
                    };
                    self.cache.insert(key.clone(), entry);
                    self.lru_queue.push_back(key);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = TextureCache::new(3, Duration::from_secs(10));

        // 测试初始状态
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());

        // 测试插入和获取
        let key1 = CacheKey::new("test", 1);
        let texture = cache.get_mq_texture(key1.clone(), || {
            Some(Texture2D::from_rgba8(10, 10, &vec![0; 400]))
        });
        assert!(texture.is_some());
        assert_eq!(cache.len(), 1);

        // 测试缓存命中
        let texture2 = cache.get_mq_texture(key1.clone(), || {
            panic!("不应该调用创建函数");
        });
        assert!(texture2.is_some());
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = TextureCache::new(2, Duration::from_secs(10));

        let key1 = CacheKey::new("test", 1);
        let key2 = CacheKey::new("test", 2);
        let key3 = CacheKey::new("test", 3);

        // 填满缓存
        cache.get_mq_texture(key1.clone(), || {
            Some(Texture2D::from_rgba8(10, 10, &vec![0; 400]))
        });
        cache.get_mq_texture(key2.clone(), || {
            Some(Texture2D::from_rgba8(10, 10, &vec![0; 400]))
        });

        assert_eq!(cache.len(), 2);

        // 添加第三个，应该淘汰最旧的
        cache.get_mq_texture(key3.clone(), || {
            Some(Texture2D::from_rgba8(10, 10, &vec![0; 400]))
        });

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.stats().evictions, 1);
    }
}
