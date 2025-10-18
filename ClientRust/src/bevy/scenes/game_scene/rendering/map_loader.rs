// Map Loader - 地图加载系统 (Bevy 版本)
//
// 功能说明:
// - 从文件加载地图数据
// - 填充 MapRenderData 资源
// - 设置摄像机边界
//
// 使用方式:
// 1. 设置 MapLoadRequest 资源
// 2. load_map_system 检测并加载
// 3. 地图数据填充到 MapRenderData

use bevy::prelude::*;
use std::path::PathBuf;

use crate::objects::MapReader;
use super::map_renderer::MapRenderData;
use super::camera::GameCamera;

/// 地图加载请求资源
#[derive(Resource, Default)]
pub struct MapLoadRequest {
    pub map_name: Option<String>,
    pub is_loading: bool,
}

impl MapLoadRequest {
    pub fn request(&mut self, map_name: String) {
        self.map_name = Some(map_name);
        self.is_loading = false;
    }

    pub fn is_requested(&self) -> bool {
        self.map_name.is_some() && !self.is_loading
    }

    pub fn start_loading(&mut self) {
        self.is_loading = true;
    }

    pub fn finish_loading(&mut self) {
        self.map_name = None;
        self.is_loading = false;
    }
}

/// 地图加载系统
///
/// 检测 MapLoadRequest,加载地图数据到 MapRenderData
pub fn load_map_system(
    mut load_request: ResMut<MapLoadRequest>,
    mut map_data: ResMut<MapRenderData>,
    mut camera_query: Query<&mut GameCamera>,
) {
    if !load_request.is_requested() {
        return;
    }

    let map_name = load_request.map_name.as_ref().unwrap().clone();
    load_request.start_loading();

    info!("🗺️  开始加载地图: {}", map_name);

    match load_map_file(&map_name) {
        Ok((cells, width, height)) => {
            // 填充地图数据
            map_data.cells = cells;
            map_data.width = width;
            map_data.height = height;
            map_data.doors.clear();

            // 提取门信息
            extract_doors(&mut map_data);

            // 设置摄像机边界
            if let Ok(mut camera) = camera_query.single_mut() {
                let map_width_px = width as f32 * 48.0;  // CELL_WIDTH
                let map_height_px = height as f32 * 32.0; // CELL_HEIGHT
                camera.set_map_bounds(map_width_px, map_height_px);
                info!("✅ 摄像机边界已设置: {}x{} 像素", map_width_px, map_height_px);
            }

            info!("✅ 地图加载完成: {} ({}x{})", map_name, width, height);

            load_request.finish_loading();
        }
        Err(e) => {
            error!("❌ 加载地图失败: {} - {}", map_name, e);
            load_request.finish_loading();
        }
    }
}

/// 从文件加载地图 (复用 MapReader)
fn load_map_file(map_name: &str) -> std::io::Result<(Vec<Vec<crate::objects::CellInfo>>, i32, i32)> {
    // 尝试不同路径 - 优先 ClientRust/Map
    let paths = [
        PathBuf::from(format!("Map/{}.map", map_name)), // ClientRust/Map
        PathBuf::from(format!("./Map/{}.map", map_name)),
        PathBuf::from(format!("Data/Map/{}.map", map_name)),
        PathBuf::from(format!("./Data/Map/{}.map", map_name)),
        PathBuf::from(format!("../Data/Map/{}.map", map_name)),
        PathBuf::from(format!("../../Data/Map/{}.map", map_name)),
    ];

    for path in &paths {
        if path.exists() {
            info!("🗺️  找到地图文件: {:?}", path);
            match MapReader::new(path.to_str().unwrap()) {
                Ok(reader) => {
                    let width = reader.width;
                    let height = reader.height;
                    let cells = reader.map_cells;

                    info!(
                        "✅ 地图文件解析成功: {}x{} ({}个格子)",
                        width,
                        height,
                        width * height
                    );

                    return Ok((cells, width, height));
                }
                Err(e) => {
                    warn!("⚠️  解析地图文件失败 {:?}: {}", path, e);
                    continue;
                }
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("地图文件未找到: {}", map_name),
    ))
}

/// 从地图数据中提取门信息
fn extract_doors(map_data: &mut MapRenderData) {
    let mut door_id = 1u8;

    for y in 0..map_data.height {
        for x in 0..map_data.width {
            if let Some(cell) = map_data.get_cell(x, y) {
                if cell.door_offset > 0 {
                    map_data.doors.push(super::map_renderer::DoorInfo {
                        id: door_id,
                        x,
                        y,
                        offset: cell.door_offset,
                        is_open: false,
                        frame: 0,
                    });
                    door_id += 1;
                }
            }
        }
    }

    if !map_data.doors.is_empty() {
        info!("🚪 找到 {} 个门", map_data.doors.len());
    }
}

/// 测试用: 直接加载地图 (同步调用)
pub fn load_map_direct(
    map_name: &str,
    map_data: &mut MapRenderData,
    camera: Option<&mut GameCamera>,
) -> Result<(), String> {
    match load_map_file(map_name) {
        Ok((cells, width, height)) => {
            map_data.cells = cells;
            map_data.width = width;
            map_data.height = height;
            map_data.doors.clear();

            extract_doors(map_data);

            if let Some(camera) = camera {
                let map_width_px = width as f32 * 48.0;
                let map_height_px = height as f32 * 32.0;
                camera.set_map_bounds(map_width_px, map_height_px);
            }

            Ok(())
        }
        Err(e) => Err(format!("加载地图失败: {}", e)),
    }
}
