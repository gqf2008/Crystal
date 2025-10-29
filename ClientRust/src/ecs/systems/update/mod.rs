//! 更新系统模块 (Layer 1-6)
//! 
//! 所有系统都实现 System trait
//! 
//! 层级结构：
//! - Layer 1: input - 输入处理 (50-199)
//! - Layer 2: decision - 决策层 (200-299)
//! - Layer 3: combat_skill - 战斗技能 (300-399)
//! - Layer 4: pyhsics - 物理运动 (400-499)
//! - Layer 5: state_udpate - 状态更新 (500-599)
//! - Layer 6: network_sync - 网络同步 (600-699)

pub mod input;
pub mod decision;
pub mod combat_skill;
pub mod pyhsics;
pub mod state_udpate;
pub mod network_sync;