//! 地图加载系统 (MapLoadSystem)
//! 
//! **优先级**: 510 (STATE_UPDATE)
//! **职责**: 处理地图切换和加载
//! 
//! ## ECS 架构
//! 
//! ### 输入
//! - 从 `GameContext.net_events().map_changed()` 读取地图切换事件
//! 
//! ### 输出
//! - 加载新地图的 MapData 和瓦片实体
//! - 更新 MapManager 组件状态
//! 
//! ### 组件依赖
//! - **读取**: GameContext (net_events)
//! - **写入**: MapData, MapManager
//! 
//! ## 地图加载流程
//! 
//! 1. 监听 `MapChanged` 事件 → 获取地图文件名
//! 2. 使用 `MapReader::new()` 读取地图文件
//! 3. 使用 `MapLoader::load_map()` 加载瓦片到 World
//! 4. 更新 MapManager 状态
//! 
//! ## 示例
//! 
//! ```rust
//! // 通过网络接收地图切换事件后，系统会自动处理：
//! // ctx.net_events().map_changed() -> [(map_index, file_name, title)]
//! 
//! // MapLoadSystem 自动加载地图
//! ```

use crate::core::GameError;
use crate::game::GameResult;
use crate::resources::MapReader;
use tracing::{info, error};

use crate::components::MapData;
use crate::game::{GameContext};
use crate::network::handlers::NetworkEvent;

/// 地图管理组件
/// 
/// **单例组件**: 记录当前地图状态
pub struct MapManager {
    /// 当前加载的地图索引
    pub current_map_index: i32,
    /// 当前地图文件名（不含路径和扩展名）
    pub current_map_file: String,
    /// 当前地图标题
    pub current_map_title: String,
    /// 是否正在加载中
    pub is_loading: bool,
}

impl MapManager {
    pub fn new() -> Self {
        Self {
            current_map_index: -1,
            current_map_file: String::new(),
            current_map_title: String::new(),
            is_loading: false,
        }
    }
}

/// 地图加载系统
/// 
/// **优先级**: 510 (STATE_UPDATE 层)
#[derive(ecs_macros::LogicSystem)]
pub struct MapLoadSystem;

impl MapLoadSystem {
    /// 内部加载逻辑（使用 GameContext）
    fn do_update(ctx: &mut GameContext) -> GameResult {
        if !ctx.events().has_network_events() {
            return Ok(());
        }

        // 取本帧最后一个 MapChanged/MapInformation（后发更权威）
        let mut selected: Option<(i32, String, String)> = None;
        for event in ctx.events().network_events() {
            match event {
                NetworkEvent::MapChanged { packet } => {
                    selected = Some((packet.map_index, packet.file_name.clone(), packet.title.clone()));
                }
                NetworkEvent::MapInformation { packet } => {
                    // MapInformation 也包含 file/title，但一般 MapChanged 才携带落点
                    if selected.is_none() {
                        selected = Some((packet.map_index, packet.file_name.clone(), packet.title.clone()));
                    }
                }
                _ => {}
            }
        }

        let Some((map_index, map_file, map_title)) = selected else {
            return Ok(());
        };

        // 如果 MapManager 里已经是同一张图，则不重复加载
        let current_map = {
            let mut q = ctx.world.query::<&MapManager>();
            q.iter()
                .next()
                .map(|mgr| (mgr.current_map_index, mgr.current_map_file.clone()))
        };
        if let Some((idx, file)) = current_map {
            if idx == map_index && file == map_file {
                return Ok(());
            }
        }

        info!("📂 MapLoadSystem: map_index={} file={} title={}", map_index, map_file, map_title);

        // ====================================================================
        // 执行地图加载
        // ====================================================================
        
        info!("🗺️  开始加载地图: {} (文件: {})", map_title, map_file);

        // map_file 通常是纯文件名（不含扩展名）；兼容 "0"/"0.map"/"Map/0.map"。
        let map_path = normalize_map_path(&map_file);
        
        match MapReader::new(&map_path) {
            Ok(reader) => {
                info!("✅ 地图文件读取成功: {}x{}", reader.width, reader.height);

                // 只更新 MapData（寻路/碰撞依赖），不在 ECS 内生成大量瓦片实体。
                let new_map = MapData {
                    cells: reader.map_cells.clone(),
                    width: reader.width,
                    height: reader.height,
                };

                let existing_map_entity = ctx.world.iter().find_map(|e| e.get::<&MapData>().map(|_| e.entity()));
                match existing_map_entity {
                    Some(entity) => {
                        if let Ok(mut map) = ctx.world.get::<&mut MapData>(entity) {
                            map.cells = new_map.cells;
                            map.width = new_map.width;
                            map.height = new_map.height;
                        }
                    }
                    None => {
                        ctx.world.spawn((new_map,));
                    }
                }

                // MapManager 作为单例：清理旧的再创建
                let old: Vec<_> = ctx.world.iter().filter_map(|eref| {
                    if eref.get::<&MapManager>().is_some() {
                        Some(eref.entity())
                    } else {
                        None
                    }
                }).collect();
                for e in old {
                    let _ = ctx.world.despawn(e);
                }
                ctx.world.spawn((MapManager {
                    current_map_index: map_index,
                    current_map_file: map_file.clone(),
                    current_map_title: map_title.clone(),
                    is_loading: false,
                },));

                info!("✅ MapLoadSystem: MapData/MapManager 已更新");
            }
            Err(e) => {
                error!("❌ 地图文件读取失败: {}", e);
                return Err(GameError::ResourceLoadError(
                    format!("Failed to load map: {}", e)
                ));
            }
        }
        
        Ok(())
    }

}

fn normalize_map_path(file_name: &str) -> String {
    crate::resources::map_reader::resolve_map_path(file_name)
}

// ============================================================================
// System Trait 实现
// ============================================================================

use crate::systems::LogicSystem;

impl LogicSystem for MapLoadSystem {
   
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        Self::do_update(ctx)
    }
}
