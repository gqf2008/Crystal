// Crystal Server - Legend of Mir 2 game server
// Rust port of OpenMir2, built on tokio + kameo actors

pub mod actors;
pub mod combat;
pub mod db;
pub mod gate;
pub mod maps;
// 子系统（聊天/交易/组队/邮件/商城/任务）- Phase 3+ 实现后取消注释
// pub mod systems;
pub mod util;
