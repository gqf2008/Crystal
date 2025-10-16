// GameScene 子系统架构
//
// 设计原则:
// - 单一职责: 每个子系统只负责一件事
// - 明确边界: 通过 trait 定义接口,避免循环依赖
// - 实例状态: 避免全局静态变量
// - 组合优于继承: 使用 Rust 的 trait 和泛型

pub mod input_system;
pub mod object_manager;
pub mod rendering_pipeline;

pub use input_system::InputSystem;
pub use object_manager::ObjectManager;
pub use rendering_pipeline::RenderingPipeline;

// 游戏上下文 - 用于子系统间通信
pub struct GameContext<'a> {
    pub objects: &'a mut ObjectManager,
    // pub effects: &'a mut EffectSystem,        // TODO: 后续添加
    // pub ui: &'a mut UIManager,                 // TODO: 后续添加
    // pub data: &'a DataManager,                 // TODO: 后续添加
}
