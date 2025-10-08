// Tile texture manager - manages loading and caching of map tile textures
// Mirrors Client/MirScenes/GameScene.cs tile rendering logic

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::graphics::mlibrary::MLibrary;
use crate::graphics::GgezManager;

/// Tile texture cache entry
#[derive(Debug, Clone)]
pub struct TileTexture {
    pub texture_name: String,  // 纹理在 ggez_manager 中的名称
    pub width: u32,
    pub height: u32,
    pub offset_x: i16,
    pub offset_y: i16,
}

/// Tile texture manager
#[derive(Debug)]
pub struct TileTextureManager {
    /// Tiles libraries (Tiles.lib, Tiles2.lib, etc.)
    tiles_libraries: Vec<Arc<Mutex<MLibrary>>>,
    
    /// Map from file_index (MapLibs index) to Vec index
    /// e.g., {0 → 0, 1 → 1, 2 → 2, ..., 90 → 30, 100 → 31, ...}
    index_map: HashMap<i32, usize>,
    
    /// Texture cache: (file_index, tile_index) -> TileTexture
    texture_cache: HashMap<(i32, u16), TileTexture>,
    
    /// Statistics
    cache_hits: usize,
    cache_misses: usize,
}

impl TileTextureManager {
    /// Create new tile texture manager
    pub fn new() -> Self {
        Self {
            tiles_libraries: Vec::new(),
            index_map: HashMap::new(),
            texture_cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }
    
    /// Load Map Tiles libraries (MapLibs array)
    /// 
    /// Mirrors C# Client/MirGraphics/MLibrary.cs Libraries.MapLibs initialization
    /// 
    /// Map library indices (sparse array in C#, sequential Vec in Rust):
    /// - WemadeMir2: 0-29, 90 (Tiles, SmTiles, Objects, Objects2-27, Objects_32bit)
    /// - ShandaMir2: 100-150, 190 (Tiles1-9, SmTiles1-9, Objects1-30, AniTiles1)
    pub fn load_tiles_libraries(&mut self) -> Result<usize, std::io::Error> {
        let mut loaded_count = 0;
        let mut vec_index = 0;
        
        // Helper macro to load a library and update index_map
        macro_rules! load_lib {
            ($file_index:expr, $path:expr, $name:expr) => {
                if let Ok(lib) = MLibrary::open($path) {
                    tracing::info!("✅ Loaded MapLibs[{}]: {} ({} images)", 
                        $file_index, $name, lib.count());
                    self.tiles_libraries.push(Arc::new(Mutex::new(lib)));
                    self.index_map.insert($file_index, vec_index);
                    vec_index += 1;
                    loaded_count += 1;
                    true
                } else {
                    false
                }
            };
        }
        
        // WemadeMir2: MapLibs[0-2]
        load_lib!(0, "Data/Map/WemadeMir2/Tiles", "Tiles.lib");
        load_lib!(1, "Data/Map/WemadeMir2/SmTiles", "SmTiles.lib");
        load_lib!(2, "Data/Map/WemadeMir2/Objects", "Objects.lib");
        
        // WemadeMir2: MapLibs[3-29] = Objects2-27
        for i in 2..=27 {
            let file_index = i + 1;
            let path = format!("Data/Map/WemadeMir2/Objects{}", i);
            let name = format!("Objects{}.lib", i);
            load_lib!(file_index, &path, &name);
        }
        
        // WemadeMir2: MapLibs[90] = Objects_32bit
        load_lib!(90, "Data/Map/WemadeMir2/Objects_32bit", "Objects_32bit.lib");
        
        // ShandaMir2: MapLibs[100] = Tiles, MapLibs[101-109] = Tiles2-9
        load_lib!(100, "Data/Map/ShandaMir2/Tiles", "Tiles.lib");
        for i in 1..=9 {
            let file_index = 100 + i;
            let path = format!("Data/Map/ShandaMir2/Tiles{}", i + 1);
            let name = format!("Tiles{}.lib", i + 1);
            load_lib!(file_index, &path, &name);
        }
        
        // ShandaMir2: MapLibs[110] = SmTiles, MapLibs[111-119] = SmTiles2-9
        load_lib!(110, "Data/Map/ShandaMir2/SmTiles", "SmTiles.lib");
        for i in 1..=9 {
            let file_index = 110 + i;
            let path = format!("Data/Map/ShandaMir2/SmTiles{}", i + 1);
            let name = format!("SmTiles{}.lib", i + 1);
            load_lib!(file_index, &path, &name);
        }
        
        // ShandaMir2: MapLibs[120] = Objects, MapLibs[121-150] = Objects2-30
        load_lib!(120, "Data/Map/ShandaMir2/Objects", "Objects.lib");
        for i in 1..=30 {
            let file_index = 120 + i;
            let path = format!("Data/Map/ShandaMir2/Objects{}", i + 1);
            let name = format!("Objects{}.lib", i + 1);
            load_lib!(file_index, &path, &name);
        }
        
        // ShandaMir2: MapLibs[190] = AniTiles1
        load_lib!(190, "Data/Map/ShandaMir2/AniTiles1", "AniTiles1.lib");
        
        if loaded_count == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No map tile libraries found in Data/Map/"
            ));
        }
        
        tracing::info!("📚 Loaded {} map libraries total", loaded_count);
        tracing::debug!("Index map: {} entries", self.index_map.len());
        Ok(loaded_count)
    }
    
    /// Map file_index from .map file to Vec index using the index_map
    /// 
    /// The index_map is built during load_tiles_libraries() and maps:
    /// MapLibs[file_index] → Vec[vec_index]
    /// 
    /// This handles the sparse MapLibs array (0,1,2,...,90,100,110,...,190)
    /// being stored in a dense Vec
    fn map_file_index_to_lib_index(&self, file_index: i32) -> Option<usize> {
        match self.index_map.get(&file_index) {
            Some(&vec_index) => Some(vec_index),
            None => {
                tracing::debug!("⚠️  MapLibs[{}] not loaded or doesn't exist", file_index);
                None
            }
        }
    }
    
    /// Get or load tile texture
    /// 
    /// Arguments:
    /// - ctx: ggez Context for creating textures
    /// - file_index: MapLibs index from .map file (0, 1, 2, 100, 110, 120, 190)
    /// - tile_index: Image index within the library
    /// - ggez_manager: Texture manager for uploading to GPU
    pub fn get_tile_texture(
        &mut self,
        ctx: &mut ggez::Context,
        file_index: i32,
        tile_index: u16,
        ggez_manager: &mut GgezManager,
    ) -> Option<TileTexture> {
        // Check cache first
        let cache_key = (file_index, tile_index);
        if let Some(texture) = self.texture_cache.get(&cache_key) {
            self.cache_hits += 1;
            return Some(texture.clone());
        }
        
        self.cache_misses += 1;
        
        // Map file_index to library array index
        let lib_index = self.map_file_index_to_lib_index(file_index)?;
        
        // Check bounds
        if lib_index >= self.tiles_libraries.len() {
            tracing::warn!("⚠️  MapLibs[{}] not loaded (mapped from file_index {})", 
                lib_index, file_index);
            return None;
        }
        
        let library = self.tiles_libraries[lib_index].clone();
        let mut library_lock = library.lock().unwrap();
        
        // Get image data
        match library_lock.load_rgba_data(tile_index as usize) {
            Ok((info, pixels)) => {
                // Create unique texture name
                let texture_name = format!("Tile_{}_{}", file_index, tile_index);
                
                // Upload to GPU
                if let Err(e) = ggez_manager.create_texture_from_rgba(
                    ctx,
                    info.width as u16,
                    info.height as u16,
                    &pixels,
                    texture_name.clone()
                ) {
                    tracing::error!("❌ Failed to upload tile texture {}: {}", texture_name, e);
                    return None;
                }
                
                let texture = TileTexture {
                    texture_name: texture_name.clone(),
                    width: info.width as u32,
                    height: info.height as u32,
                    offset_x: info.x,
                    offset_y: info.y,
                };
                
                // Cache it
                self.texture_cache.insert(cache_key, texture.clone());
                
                tracing::trace!("✅ Loaded tile texture: {} ({}x{})", 
                    texture_name, info.width, info.height);
                
                Some(texture)
            }
            Err(e) => {
                tracing::warn!("⚠️  Failed to load image from library: file={}, index={}: {}", 
                    file_index, tile_index, e);
                None
            }
        }
    }
    
    /// Clear texture cache
    pub fn clear_cache(&mut self) {
        self.texture_cache.clear();
        tracing::info!("🗑️  Cleared tile texture cache");
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize, f32) {
        let total = self.cache_hits + self.cache_misses;
        let hit_rate = if total > 0 {
            self.cache_hits as f32 / total as f32 * 100.0
        } else {
            0.0
        };
        (self.cache_hits, self.cache_misses, hit_rate)
    }
    
    /// Get number of loaded libraries
    pub fn library_count(&self) -> usize {
        self.tiles_libraries.len()
    }
    
    /// Get texture from cache only (does not load if missing)
    /// Used during draw() to access preloaded textures
    pub fn get_texture_from_cache(&self, file_index: i32, tile_index: u16) -> Option<&TileTexture> {
        let cache_key = (file_index, tile_index);
        self.texture_cache.get(&cache_key)
    }
}

impl Default for TileTextureManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tile_texture_manager_creation() {
        let manager = TileTextureManager::new();
        assert_eq!(manager.library_count(), 0);
        assert_eq!(manager.texture_cache.len(), 0);
    }
    
    #[test]
    fn test_cache_stats() {
        let manager = TileTextureManager::new();
        let (hits, misses, rate) = manager.get_cache_stats();
        assert_eq!(hits, 0);
        assert_eq!(misses, 0);
        assert_eq!(rate, 0.0);
    }
}
