// Crystal Server - Legend of Mir 2 game server
// Rust port of OpenMir2, built on tokio + kameo actors

pub mod actors;
pub mod combat;
pub mod db;
pub mod gate;
pub mod maps;
// 子系统（聊天/交易/组队/邮件/商城/任务）- Phase 3+ 实现后取消注释
// pub mod systems;
/// 内存探针（计数分配器）：只在 `--features mem-probe` 时编译，默认构建零开销。
#[cfg(feature = "mem-probe")]
pub mod mem_probe;
pub mod util;
