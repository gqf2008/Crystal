use anyhow::Result;
use ggez::graphics::{Image, ImageFormat};
use ggez::Context;
use std::collections::HashMap;
use std::path::Path;

/// 图片库项，对应 C# 的 MLibrary.MImage
#[derive(Debug)]
pub struct ImageItem {
    pub width: u32,
    pub height: u32,
    pub x: i16,
    pub y: i16,
    pub image: Option<Image>,
}

impl ImageItem {
    pub fn new(width: u32, height: u32) -> Self {
        ImageItem {
            width,
            height,
            x: 0,
            y: 0,
            image: None,
        }
    }
}

/// 图片库，对应 C# 的 MLibrary
pub struct Library {
    pub name: String,
    pub images: Vec<ImageItem>,
    loaded: bool,
}

impl Library {
    pub fn new(name: String) -> Self {
        Library {
            name,
            images: Vec::new(),
            loaded: false,
        }
    }

    /// 从文件加载库 (占位实现)
    pub fn load_from_file<P: AsRef<Path>>(&mut self, _ctx: &mut Context, _path: P) -> Result<()> {
        // TODO: 实现 .lib 文件格式解析
        // 这需要解析 Mir2/Mir3 的特定格式
        self.loaded = true;
        Ok(())
    }

    /// 检查并加载图片
    pub fn check_image(&mut self, _ctx: &mut Context, index: usize) -> Option<&ImageItem> {
        if index < self.images.len() {
            Some(&self.images[index])
        } else {
            None
        }
    }

    /// 获取图片尺寸
    pub fn get_size(&self, index: usize) -> Option<(u32, u32)> {
        if index < self.images.len() {
            let img = &self.images[index];
            Some((img.width, img.height))
        } else {
            None
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}

/// 库管理器
pub struct LibraryManager {
    libraries: HashMap<usize, Library>,
}

impl LibraryManager {
    pub fn new() -> Self {
        LibraryManager {
            libraries: HashMap::new(),
        }
    }

    /// 初始化所有库
    pub fn initialize(&mut self) {
        // Wemade Mir2 (0-99)
        self.libraries.insert(0, Library::new("Tiles".to_string()));
        self.libraries.insert(1, Library::new("Smtiles".to_string()));
        self.libraries.insert(2, Library::new("Objects".to_string()));

        // Shanda Mir2 (100-199)
        for i in 0..10 {
            self.libraries.insert(
                100 + i,
                Library::new(format!("Tiles{}", i + 1)),
            );
        }

        // Wemade Mir3 (200-299)
        let map_states = vec!["", "wood/", "sand/", "snow/", "forest/"];
        for (i, state) in map_states.iter().enumerate() {
            self.libraries.insert(
                200 + i * 15,
                Library::new(format!("{}Tilesc", state)),
            );
        }

        // Shanda Mir3 (300-399)
        // TODO: 添加更多库
    }

    pub fn get_library(&self, index: usize) -> Option<&Library> {
        self.libraries.get(&index)
    }

    pub fn get_library_mut(&mut self, index: usize) -> Option<&mut Library> {
        self.libraries.get_mut(&index)
    }

    /// 加载游戏库
    pub fn load_game_libraries(&mut self, ctx: &mut Context) -> Result<()> {
        // TODO: 实际加载库文件
        // 当前只是初始化结构
        self.initialize();
        Ok(())
    }
}

impl Default for LibraryManager {
    fn default() -> Self {
        Self::new()
    }
}
