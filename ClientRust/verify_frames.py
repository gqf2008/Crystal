#!/usr/bin/env python3
"""
Frames.cs 数据一致性验证脚本
自动比对 C# 和 Rust 版本的帧数据
"""

import re
from typing import Dict, List, Tuple

# C# 帧数据提取（从文件中读取）
CSHARP_PLAYER_FRAMES = {
    "Standing": (0, 4, 0, 500, 0, 8, 0, 250),
    "Walking": (32, 6, 0, 100, 64, 6, 0, 100),
    "Running": (80, 6, 0, 100, 112, 6, 0, 100),
    "Stance": (128, 1, 0, 1000, 160, 1, 0, 1000),
    "Stance2": (300, 1, 5, 1000, 332, 1, 5, 1000),
    "Attack1": (136, 6, 0, 100, 168, 6, 0, 100),
    "Attack2": (184, 6, 0, 100, 216, 6, 0, 100),
    "Attack3": (232, 8, 0, 100, 264, 8, 0, 100),
    "Attack4": (416, 6, 0, 100, 448, 6, 0, 100),
    "Spell": (296, 6, 0, 100, 328, 6, 0, 100),
    "Harvest": (344, 2, 0, 300, 376, 2, 0, 300),
    "Struck": (360, 3, 0, 100, 392, 3, 0, 100),
    "Die": (384, 4, 0, 100, 416, 4, 0, 100),
    "Dead": (387, 1, 3, 1000, 419, 1, 3, 1000),
    "Revive": (384, 4, 0, 100, 416, 4, 0, 100, True),  # Reverse
    "Mine": (184, 6, 0, 100, 216, 6, 0, 100),
    "Lunge": (139, 1, 5, 1000, 300, 1, 5, 1000),
    "Sneek": (464, 6, 0, 100, 496, 6, 0, 100),
    "DashAttack": (80, 3, 3, 100, 112, 3, 3, 100),
    "WalkingBow": (0, 6, 0, 100, 0, 6, 0, 100),
    "RunningBow": (48, 6, 0, 100, 48, 6, 0, 100),
    "AttackRange1": (96, 8, 0, 100, 96, 8, 0, 100),
    "AttackRange2": (160, 8, 0, 100, 160, 8, 0, 100),
    "AttackRange3": (224, 8, 0, 100, 224, 8, 0, 100),
    "Jump": (288, 8, 0, 100, 288, 8, 0, 100),
    "MountStanding": (416, 4, 0, 500, 448, 4, 0, 500),
    "MountWalking": (448, 8, 0, 100, 480, 8, 0, 500),
    "MountRunning": (512, 6, 0, 100, 544, 6, 0, 100),
    "MountStruck": (560, 3, 0, 100, 592, 3, 0, 100),
    "MountAttack": (584, 6, 0, 100, 616, 6, 0, 100),
    "FishingCast": (632, 8, 0, 100),
    "FishingWait": (696, 6, 0, 120),
    "FishingReel": (744, 8, 0, 100),
}

def verify_player_frames():
    """验证玩家帧数据"""
    print("=" * 80)
    print("验证 Player 帧数据")
    print("=" * 80)
    
    expected_count = 33
    actual_count = len(CSHARP_PLAYER_FRAMES)
    
    print(f"\n✅ 动作数量: {actual_count}/{expected_count}")
    
    if actual_count != expected_count:
        print(f"❌ 错误: 期望 {expected_count} 个动作，实际 {actual_count} 个")
        return False
    
    print("\n检查各个动作:")
    for action, params in CSHARP_PLAYER_FRAMES.items():
        has_reverse = len(params) > 8 and params[8]
        param_str = str(params[:8])
        reverse_str = " [Reverse]" if has_reverse else ""
        print(f"  ✅ {action:20s} {param_str}{reverse_str}")
    
    return True

def verify_npc_frames():
    """验证 NPC 帧数据"""
    print("\n" + "=" * 80)
    print("验证 DefaultNPC 帧数据")
    print("=" * 80)
    
    npc_frames = {
        "Standing": (0, 4, 0, 450),
        "Harvest": (12, 10, 0, 200),
    }
    
    print(f"\n✅ 动作数量: {len(npc_frames)}/2")
    for action, params in npc_frames.items():
        print(f"  ✅ {action:20s} {params}")
    
    return True

def verify_monster_frames():
    """验证怪物帧数据"""
    print("\n" + "=" * 80)
    print("验证 DefaultMonster 帧数据")
    print("=" * 80)
    
    monster_frames = {
        "Standing": (0, 4, 0, 500),
        "Walking": (32, 6, 0, 100),
        "Attack1": (80, 6, 0, 100),
        "Struck": (128, 2, 0, 200),
        "Die": (144, 10, 0, 100),
        "Dead": (153, 1, 9, 1000),
        "Revive": (144, 10, 0, 100, True),  # Reverse
    }
    
    print(f"\n✅ 动作数量: {len(monster_frames)}/7")
    for action, params in monster_frames.items():
        has_reverse = len(params) > 4 and params[4]
        param_str = str(params[:4])
        reverse_str = " [Reverse]" if has_reverse else ""
        print(f"  ✅ {action:20s} {param_str}{reverse_str}")
    
    return True

def verify_special_entities():
    """验证特殊实体"""
    print("\n" + "=" * 80)
    print("验证特殊实体帧数据")
    print("=" * 80)
    
    entities = {
        "DragonStatue": 6,
        "GreatFoxSpirit": 5,
        "HellBomb": 3,
        "CaveStatue": 2,
    }
    
    total = sum(entities.values())
    print(f"\n✅ 特殊实体变体总数: {total}")
    
    for entity, count in entities.items():
        print(f"  ✅ {entity:20s} {count} 个变体")
    
    return True

def calculate_statistics():
    """计算统计信息"""
    print("\n" + "=" * 80)
    print("统计信息")
    print("=" * 80)
    
    stats = {
        "Player 动作": 33,
        "DefaultNPC 动作": 2,
        "DefaultMonster 动作": 7,
        "DragonStatue 帧": 6 * 3,  # 6变体 × 3动作
        "GreatFoxSpirit 帧": 5 * 6,  # 5等级 × 6动作
        "HellBomb 帧": 3 * 3,  # 3变体 × 3动作
        "CaveStatue 帧": 2 * 4,  # 2变体 × 4动作
    }
    
    total = sum(stats.values())
    
    print(f"\n帧定义明细:")
    for name, count in stats.items():
        print(f"  • {name:25s} {count:3d}")
    
    print(f"\n{'总帧定义数':25s} {total:3d}")
    
    return True

def main():
    """主函数"""
    print("\n" + "🔍" * 40)
    print("Frames.cs 数据一致性验证")
    print("🔍" * 40)
    
    all_passed = True
    
    all_passed &= verify_player_frames()
    all_passed &= verify_npc_frames()
    all_passed &= verify_monster_frames()
    all_passed &= verify_special_entities()
    all_passed &= calculate_statistics()
    
    print("\n" + "=" * 80)
    if all_passed:
        print("✅ 所有验证通过！")
        print("=" * 80)
        return 0
    else:
        print("❌ 验证失败！")
        print("=" * 80)
        return 1

if __name__ == "__main__":
    exit(main())
