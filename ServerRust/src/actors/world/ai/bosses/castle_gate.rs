//! CastleGate（城门）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/CastleGate.cs（抽象基类，子类 SabukGate 等）
//! 机制：
//!   - 不可移动、不可攻击（CanMove=false, CanAttack=false）
//!   - 仅关门（Closed）时阻挡 + 可被攻击；开门时透人
//!   - 自动门：公会成员靠近 4 格开门，10s 后自动关门（简化：POC 无公会系统，保持关门）
//!   - 死亡时关闭门墙阻挡（ActiveDoorWall(false)）
//!
//! ProcessSearch（C# :83-105）：Closed && AutoOpen → 公会成员靠近开门。
//! 注意：本 POC 无公会/攻城系统，简化为固定关门阻挡的木桩。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

pub struct CastleGateBehavior;

impl CastleGateBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for CastleGateBehavior {
    fn can_move(&self) -> bool { false }

    fn process_tick(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // 城门无主动 AI：不动、不攻击。
        // 开关门/公会判定由攻城系统驱动（POC 暂不实现）。
    }
}
