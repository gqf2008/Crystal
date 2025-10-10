// frames_test.rs - FrameSet 功能测试

#[cfg(test)]
mod tests {
    use crate::objects::frames::*;
    use mir2_shared::enums::MirAction;

    #[test]
    fn test_frame_creation() {
        let frame = Frame::new(0, 4, 0, 500, 0, 8, 0, 250);
        assert_eq!(frame.start, 0);
        assert_eq!(frame.count, 4);
        assert_eq!(frame.interval, 500);
        assert_eq!(frame.effect_count, 8);
    }

    #[test]
    fn test_frame_basic() {
        let frame = Frame::basic(32, 6, 0, 100);
        assert_eq!(frame.start, 32);
        assert_eq!(frame.count, 6);
        assert_eq!(frame.skip, 0);
        assert_eq!(frame.interval, 100);
        assert_eq!(frame.effect_start, 0);
        assert_eq!(frame.effect_count, 0);
    }

    #[test]
    fn test_frame_offset() {
        let frame = Frame::basic(0, 4, 2, 500);
        assert_eq!(frame.offset(), 6); // count + skip = 4 + 2
    }

    #[test]
    fn test_frame_effect_offset() {
        let frame = Frame::new(0, 4, 0, 500, 0, 8, 2, 250);
        assert_eq!(frame.effect_offset(), 10); // effect_count + effect_skip = 8 + 2
    }

    #[test]
    fn test_frame_builder_pattern() {
        let frame = Frame::basic(52, 9, -9, 100)
            .with_blend(true)
            .with_reverse(false);
        
        assert!(frame.blend);
        assert!(!frame.reverse);
    }

    #[test]
    fn test_player_frames_exists() {
        // 验证 PLAYER_FRAMES 已初始化
        assert!(!PLAYER_FRAMES.is_empty());
        
        // 验证包含基础动作
        assert!(PLAYER_FRAMES.contains_key(&MirAction::Standing));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::Walking));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::Running));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::Attack1));
    }

    #[test]
    fn test_player_standing_frame() {
        if let Some(frame) = get_player_frame(MirAction::Standing) {
            assert_eq!(frame.start, 0);
            assert_eq!(frame.count, 4);
            assert_eq!(frame.skip, 0);
            assert_eq!(frame.interval, 500);
            assert_eq!(frame.effect_start, 0);
            assert_eq!(frame.effect_count, 8);
            assert_eq!(frame.effect_interval, 250);
        } else {
            panic!("Player standing frame not found!");
        }
    }

    #[test]
    fn test_player_attack_frames() {
        // 验证所有攻击动作存在
        assert!(PLAYER_FRAMES.contains_key(&MirAction::Attack1));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::Attack2));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::Attack3));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::Attack4));
    }

    #[test]
    fn test_player_mount_frames() {
        // 验证坐骑动作
        assert!(PLAYER_FRAMES.contains_key(&MirAction::MountStanding));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::MountWalking));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::MountRunning));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::MountStruck));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::MountAttack));
    }

    #[test]
    fn test_player_fishing_frames() {
        // 验证钓鱼动作
        assert!(PLAYER_FRAMES.contains_key(&MirAction::FishingCast));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::FishingWait));
        assert!(PLAYER_FRAMES.contains_key(&MirAction::FishingReel));
    }

    #[test]
    fn test_default_npc_frames() {
        assert!(!DEFAULT_NPC_FRAMES.is_empty());
        assert!(DEFAULT_NPC_FRAMES.contains_key(&MirAction::Standing));
        assert!(DEFAULT_NPC_FRAMES.contains_key(&MirAction::Harvest));
        
        if let Some(frame) = get_default_npc_frame(MirAction::Standing) {
            assert_eq!(frame.start, 0);
            assert_eq!(frame.count, 4);
            assert_eq!(frame.interval, 450);
        }
    }

    #[test]
    fn test_default_monster_frames() {
        assert!(!DEFAULT_MONSTER_FRAMES.is_empty());
        
        // 验证怪物基础动作
        assert!(DEFAULT_MONSTER_FRAMES.contains_key(&MirAction::Standing));
        assert!(DEFAULT_MONSTER_FRAMES.contains_key(&MirAction::Walking));
        assert!(DEFAULT_MONSTER_FRAMES.contains_key(&MirAction::Attack1));
        assert!(DEFAULT_MONSTER_FRAMES.contains_key(&MirAction::Struck));
        assert!(DEFAULT_MONSTER_FRAMES.contains_key(&MirAction::Die));
        assert!(DEFAULT_MONSTER_FRAMES.contains_key(&MirAction::Dead));
        assert!(DEFAULT_MONSTER_FRAMES.contains_key(&MirAction::Revive));
    }

    #[test]
    fn test_default_monster_revive_reverse() {
        if let Some(frame) = get_default_monster_frame(MirAction::Revive) {
            assert!(frame.reverse, "Revive animation should be reversed");
        } else {
            panic!("Monster revive frame not found!");
        }
    }

    #[test]
    fn test_dragon_statue_frames() {
        assert_eq!(DRAGON_STATUE_FRAMES.len(), 6, "Should have 6 DragonStatue variations");
        
        // 验证第一个变体
        let frames = &DRAGON_STATUE_FRAMES[0];
        assert!(frames.contains_key(&MirAction::Standing));
        assert!(frames.contains_key(&MirAction::AttackRange1));
        assert!(frames.contains_key(&MirAction::Struck));
        
        // 验证帧数据
        if let Some(frame) = frames.get(&MirAction::Standing) {
            assert_eq!(frame.start, 300);
        }
    }

    #[test]
    fn test_great_fox_spirit_frames() {
        assert_eq!(GREAT_FOX_SPIRIT_FRAMES.len(), 5, "Should have 5 GreatFoxSpirit levels");
        
        // 验证等级 0
        let frames = &GREAT_FOX_SPIRIT_FRAMES[0];
        assert!(frames.contains_key(&MirAction::Standing));
        assert!(frames.contains_key(&MirAction::Attack1));
        assert!(frames.contains_key(&MirAction::Die));
        assert!(frames.contains_key(&MirAction::Dead));
        assert!(frames.contains_key(&MirAction::Revive));
        
        // 验证等级递增的帧起始位置
        let level0_start = GREAT_FOX_SPIRIT_FRAMES[0]
            .get(&MirAction::Standing)
            .unwrap()
            .start;
        let level1_start = GREAT_FOX_SPIRIT_FRAMES[1]
            .get(&MirAction::Standing)
            .unwrap()
            .start;
        
        assert_eq!(level0_start, 0);
        assert_eq!(level1_start, 60);
    }

    #[test]
    fn test_hell_bomb_frames() {
        assert_eq!(HELL_BOMB_FRAMES.len(), 3, "Should have 3 HellBomb variations");
        
        for (i, frames) in HELL_BOMB_FRAMES.iter().enumerate() {
            assert!(frames.contains_key(&MirAction::Standing), 
                "HellBomb {} should have Standing action", i);
            
            // 验证混合模式
            if let Some(frame) = frames.get(&MirAction::Standing) {
                assert!(frame.blend, "HellBomb frames should use blend mode");
            }
        }
    }

    #[test]
    fn test_cave_statue_frames() {
        assert_eq!(CAVE_STATUE_FRAMES.len(), 2, "Should have 2 CaveStatue variations");
        
        for (i, frames) in CAVE_STATUE_FRAMES.iter().enumerate() {
            assert!(frames.contains_key(&MirAction::Standing), 
                "CaveStatue {} should have Standing action", i);
            assert!(frames.contains_key(&MirAction::Die), 
                "CaveStatue {} should have Die action", i);
            
            // 验证不使用混合模式
            if let Some(frame) = frames.get(&MirAction::Standing) {
                assert!(!frame.blend, "CaveStatue frames should not use blend mode");
            }
        }
    }

    #[test]
    fn test_frame_with_negative_skip() {
        // 测试负数 skip 值（如 DragonStatue 使用的）
        let frame = Frame::basic(300, 1, -1, 1000);
        assert_eq!(frame.skip, -1);
        assert_eq!(frame.offset(), 0); // 1 + (-1) = 0
    }

    #[test]
    fn test_get_frame_helper() {
        // 测试通用的 get_frame 函数
        if let Some(frame) = get_frame(&PLAYER_FRAMES, MirAction::Running) {
            assert_eq!(frame.start, 80);
            assert_eq!(frame.count, 6);
        } else {
            panic!("Player running frame not found!");
        }
    }

    #[test]
    fn test_frame_from_reader() {
        use std::io::Cursor;
        
        // 创建测试数据（34字节）
        let data: Vec<u8> = vec![
            // Start: 100 (i32, little-endian)
            100, 0, 0, 0,
            // Count: 8 (i32)
            8, 0, 0, 0,
            // Skip: 0 (i32)
            0, 0, 0, 0,
            // Interval: 120 (i32)
            120, 0, 0, 0,
            // EffectStart: 200 (i32)
            200, 0, 0, 0,
            // EffectCount: 10 (i32)
            10, 0, 0, 0,
            // EffectSkip: 2 (i32)
            2, 0, 0, 0,
            // EffectInterval: 150 (i32)
            150, 0, 0, 0,
            // Reverse: true (1 byte)
            1,
            // Blend: false (1 byte)
            0,
        ];
        
        let mut cursor = Cursor::new(data);
        let frame = Frame::from_reader(&mut cursor).expect("Failed to read frame");
        
        assert_eq!(frame.start, 100);
        assert_eq!(frame.count, 8);
        assert_eq!(frame.skip, 0);
        assert_eq!(frame.interval, 120);
        assert_eq!(frame.effect_start, 200);
        assert_eq!(frame.effect_count, 10);
        assert_eq!(frame.effect_skip, 2);
        assert_eq!(frame.effect_interval, 150);
        assert!(frame.reverse);
        assert!(!frame.blend);
    }
    
    #[test]
    fn test_frame_from_reader_with_negative_skip() {
        use std::io::Cursor;
        
        // 测试负数 skip 值（如 DragonStatue 使用的）
        let data: Vec<u8> = vec![
            // Start: 300
            44, 1, 0, 0,
            // Count: 1
            1, 0, 0, 0,
            // Skip: -1 (0xFFFFFFFF in two's complement)
            255, 255, 255, 255,
            // Interval: 1000
            232, 3, 0, 0,
            // EffectStart: 0
            0, 0, 0, 0,
            // EffectCount: 0
            0, 0, 0, 0,
            // EffectSkip: 0
            0, 0, 0, 0,
            // EffectInterval: 0
            0, 0, 0, 0,
            // Reverse: false
            0,
            // Blend: true
            1,
        ];
        
        let mut cursor = Cursor::new(data);
        let frame = Frame::from_reader(&mut cursor).expect("Failed to read frame");
        
        assert_eq!(frame.start, 300);
        assert_eq!(frame.count, 1);
        assert_eq!(frame.skip, -1);
        assert_eq!(frame.interval, 1000);
        assert!(!frame.reverse);
        assert!(frame.blend);
    }
    
    #[test]
    fn test_frame_from_reader_error_handling() {
        use std::io::Cursor;
        
        // 不完整的数据（只有10字节）
        let data: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        
        let mut cursor = Cursor::new(data);
        let result = Frame::from_reader(&mut cursor);
        
        // 应该返回错误
        assert!(result.is_err());
    }

    #[test]
    fn test_player_frame_count() {
        // 验证玩家帧数据的完整性
        let expected_actions = vec![
            MirAction::Standing,
            MirAction::Walking,
            MirAction::Running,
            MirAction::Stance,
            MirAction::Stance2,
            MirAction::Attack1,
            MirAction::Attack2,
            MirAction::Attack3,
            MirAction::Attack4,
            MirAction::Spell,
            MirAction::Harvest,
            MirAction::Struck,
            MirAction::Die,
            MirAction::Dead,
            MirAction::Revive,
            MirAction::Mine,
            MirAction::Lunge,
            MirAction::Sneek,
            MirAction::DashAttack,
            MirAction::WalkingBow,
            MirAction::RunningBow,
            MirAction::AttackRange1,
            MirAction::AttackRange2,
            MirAction::AttackRange3,
            MirAction::Jump,
            MirAction::MountStanding,
            MirAction::MountWalking,
            MirAction::MountRunning,
            MirAction::MountStruck,
            MirAction::MountAttack,
            MirAction::FishingCast,
            MirAction::FishingWait,
            MirAction::FishingReel,
        ];

        assert_eq!(
            PLAYER_FRAMES.len(),
            expected_actions.len(),
            "Player should have {} actions",
            expected_actions.len()
        );

        for action in expected_actions {
            assert!(
                PLAYER_FRAMES.contains_key(&action),
                "Player frames missing action: {:?}",
                action
            );
        }
    }
}
