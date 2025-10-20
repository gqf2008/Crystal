// Bevy Resources - 全局资源定义
use bevy::prelude::*;
use std::collections::HashMap;

use crate::graphics::libraries::{LibraryName, get_library};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// MLibrary 资源包装器
#[derive(Resource)]
pub struct MLibraryAssets {
    // 纹理缓存: "LibraryName:Index" -> Handle<Image>
    pub textures: HashMap<String, Handle<Image>>,
}

impl MLibraryAssets {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }
    
    /// 获取纹理 Handle (如果不存在则创建)
    pub fn get_texture(
        &mut self,
        library_name: &str,
        image_index: i32,
        images: &mut Assets<Image>,
    ) -> Option<Handle<Image>> {
        let key = format!("{}:{}", library_name, image_index);
        
        // 如果已经缓存,直接返回
        if let Some(handle) = self.textures.get(&key) {
            return Some(handle.clone());
        }
        
        // 解析库名
        let lib_name = match library_name {
            "ChrSel" => LibraryName::ChrSel,
            "Title" => LibraryName::Title,
            "Prguse" => LibraryName::Prguse,
            "Prguse2" => LibraryName::Prguse2,
            "Magic" => LibraryName::Magic,
            "Magic2" => LibraryName::Magic2,
            "Weather" => LibraryName::Weather,
            "Effect" => LibraryName::Effect,
            "Items" => LibraryName::Items,
            "MagIcon" => LibraryName::MagIcon,
            "BuffIcon" => LibraryName::BuffIcon,
            _ => return None,
        };
        
        // 从全局库系统加载图像
        let lib_arc = get_library(lib_name)?;
        let mut lib = lib_arc.lock().unwrap();
        
        // 获取图像信息和 BGRA 数据
        let (info, bgra_data) = lib.get_image_with_data(image_index as usize).ok()?;
        
        // 创建 Bevy Image
        let image = Image {
            data: Some(bgra_data),
            texture_descriptor: bevy::render::render_resource::TextureDescriptor {
                label: None, // 移除 label 以避免生命周期问题
                size: Extent3d {
                    width: info.width as u32,
                    height: info.height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Bgra8UnormSrgb,
                usage: bevy::render::render_resource::TextureUsages::TEXTURE_BINDING
                    | bevy::render::render_resource::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            ..default()
        };
        
        // 添加到 Bevy 资产系统
        let handle = images.add(image);
        
        // 缓存 Handle
        self.textures.insert(key, handle.clone());
        
        Some(handle)
    }
}

/// 地图资源
#[derive(Resource)]
pub struct MapAssets {
    pub current_map: Option<String>,
}

impl MapAssets {
    pub fn new() -> Self {
        Self {
            current_map: None,
        }
    }
}

/// 游戏配置
#[derive(Resource, Debug)]
pub struct GameConfig {
    pub cell_width: f32,
    pub cell_height: f32,
    pub screen_width: f32,
    pub screen_height: f32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            cell_width: 48.0,
            cell_height: 32.0,
            screen_width: 1024.0,
            screen_height: 768.0,
        }
    }
}
