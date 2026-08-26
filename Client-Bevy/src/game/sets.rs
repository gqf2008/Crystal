// ============================================================================
// 调度原语（#2632）：SystemSet 定义
// 背景：ECS 审查发现全仓零 SystemSet，系统顺序靠调度器默认（元组无 ordering），
// 健壮性差。此处集中定义游戏场景 Update 阶段的系统集，按真实数据依赖声明相对
// 顺序，供 game 各插件（player_control / skills / hud）复用。
// ============================================================================

use bevy::prelude::*;

/// 游戏场景 Update 阶段的系统集。
///
/// 只声明「确有数据依赖」的先后关系，其余保持无序以允许并行：
/// - 写 `ControlState`（attack_target/last_attack/pickup_target）的输入采集系统
///   排在消费它们的结算系统之前（`PlayerInput.before(Combat)`，见 player_control）。
/// - `Hud` / `Skills` 目前仅作分组与统一 run condition，无跨集排序（保持既有
///   「跨插件无显式时序」的行为）。
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// 玩家输入采集：点击/按键/按住移动（写 `ControlState`、经 Commands 写 `LocalMove`）。
    PlayerInput,
    /// 输入结算：`auto_attack` 读 `attack_target`/`last_attack`，`pickup_arrival`
    /// 读 `pickup_target`/`LocalMove`——须排在 `PlayerInput` 之后（同帧消费本帧输入）。
    Combat,
    /// 玩家状态写（#2633 批次4）：消费 `ServerEvent` 写玩家实体组件（Vitals/Inventory/…），
    /// HudState 已于步9 删除——唯一数据源是玩家实体组件。须排在 `Hud` 之前——
    /// 「先写状态、后 sync_hud_data 读」，维持原 hud.rs「写方须排在读方前，
    /// 晚一帧读会引入一帧滞后」的约束（设计 §12 R5）。
    PlayerState,
    /// HUD 显示刷新（hud.rs 内部另有按钮/数据两条有序链）。
    Hud,
    /// 技能快捷栏/施法（skills.rs，统一 in_state(Game) 门控）。
    Skills,
}
