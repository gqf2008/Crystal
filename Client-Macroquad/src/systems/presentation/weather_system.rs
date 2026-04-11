/// 天气效果系统
///
/// 当前状态：
/// - 地图数据中的 `weather` 字段在 mock 下硬编码为 0（晴天/无天气）
/// - 粒子系统已支持 21 种 ParticleType 的生成逻辑（Rain/Snow/Fog/Sand/Ember 等）
/// - WeatherType 枚举和 ChangeWeather 事件已在组件层定义
///
/// 完整实现路径：
/// 1. MapLoadSystem 从服务器响应中读取 weather 字段
/// 2. 本系统根据 weather 值创建对应的 ParticleEmitter 实体
/// 3. 天气切换时清理旧发射器并创建新的
/// 4. 昼夜效果可叠加全屏色彩滤镜
///
/// 由于当前无天气数据源，系统作为空桩运行。
/// 粒子系统本身已就绪：一旦有 ParticleEmitter 实体被创建，
/// ParticleSystem 会自动根据 ParticleType 生成对应粒子。
#[derive(ecs_macros::LogicSystem)]
pub struct WeatherSystem;

impl Default for WeatherSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherSystem {
    pub fn new() -> Self {
        Self
    }
}

impl crate::systems::LogicSystem for WeatherSystem {
    fn update(
        &mut self,
        _ctx: &mut crate::game::GameContext,
        _dt: f32,
    ) -> crate::game::GameResult {
        // 等待 MapLoadSystem 提供天气数据后再实现
        Ok(())
    }
}
