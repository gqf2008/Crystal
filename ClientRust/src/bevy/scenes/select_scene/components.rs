// SelectScene Components - 角色选择场景的组件和资源定义
// 参考 ggez 版本 src/scenes/select_scene.rs 的架构

use bevy::prelude::*;
use mir2_shared::SelectInfo;

/// 角色选择场景的全局状态资源
#[derive(Resource, Debug)]
pub struct SelectSceneState {
    /// 角色列表（从服务器接收）
    pub characters: Vec<SelectInfo>,
    
    /// 当前选中的角色索引（-1 表示未选中）
    pub selected_index: i32,
    
    /// 角色预览动画帧（0-15，循环）
    pub character_animation_frame: usize,
    pub character_animation_timer: f32,
    
    /// 底部按钮悬停状态
    pub hovered_button: Option<BottomButtonType>,
    pub pressed_button: Option<BottomButtonType>,
    
    /// 网络命令发送器
    pub command_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,
}

impl Default for SelectSceneState {
    fn default() -> Self {
        Self {
            characters: Vec::new(),
            selected_index: -1,
            character_animation_frame: 0,
            character_animation_timer: 0.0,
            hovered_button: None,
            pressed_button: None,
            command_tx: None,
        }
    }
}

impl SelectSceneState {
    /// 创建带测试数据的状态（用于测试和演示）
    #[allow(dead_code)]
    pub fn with_test_data() -> Self {
        use mir2_shared::enums::{MirClass, MirGender};
        use chrono::Utc;
        
        let now = Utc::now();
        
        let characters = vec![
            SelectInfo {
                index: 0,
                name: "剑客无双".to_string(),
                level: 35,
                class: MirClass::Warrior,
                gender: MirGender::Male,
                last_access: now - chrono::Duration::hours(2),
            },
            SelectInfo {
                index: 1,
                name: "冰雪女神".to_string(),
                level: 28,
                class: MirClass::Wizard,
                gender: MirGender::Female,
                last_access: now - chrono::Duration::days(1),
            },
            SelectInfo {
                index: 2,
                name: "道行天下".to_string(),
                level: 42,
                class: MirClass::Taoist,
                gender: MirGender::Male,
                last_access: now - chrono::Duration::days(3),
            },
        ];
        
        Self {
            characters,
            selected_index: 0,  // 默认选中第一个
            character_animation_frame: 0,
            character_animation_timer: 0.0,
            hovered_button: None,
            pressed_button: None,
            command_tx: None,
        }
    }
}

/// 底部按钮类型（参考 ggez 版本）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BottomButtonType {
    StartGame,     // Title_340/341/342
    NewCharacter,  // Title_343/344/345
    DeleteCharacter, // Title_346/347/348
    Credits,       // Title_349/350/351
    ExitGame,      // Title_352/353/354
}

// ============================================================================
// UI Components（参考 ggez 版本的纹理系统）
// ============================================================================

/// SelectScene 根节点标记
#[derive(Component)]
pub struct SelectSceneRoot;

/// 角色槽位组件（位置：637, 194/298/402/506）
#[derive(Component)]
pub struct CharacterSlot {
    pub slot_index: usize,  // 0-3
}

/// 角色预览动画组件（位置：260, 420）
#[derive(Component)]
pub struct CharacterPreview;

/// 底部按钮组件
#[derive(Component)]
pub struct BottomButton {
    pub button_type: BottomButtonType,
    pub position_index: usize,  // 0-4
}

/// 服务器标签组件
#[derive(Component)]
pub struct ServerLabel;

/// 最后登录时间标签
#[derive(Component)]
pub struct LastAccessLabel;

/// 角色槽位文本（名称、等级、职业）
#[derive(Component)]
pub struct CharacterSlotText {
    pub slot_index: usize,
    pub text_type: SlotTextType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotTextType {
    Name,
    Level,
    Class,
}

// ============================================================================
// Constants（参考 C# 原版位置）
// ============================================================================

/// 角色预览动画帧数
pub const ANIMATION_FRAME_COUNT: usize = 16;

/// 动画延迟（秒）
pub const ANIMATION_DELAY: f32 = 0.25;  // 250ms per frame

/// 角色槽位位置（C# 原始位置）
pub const CHARACTER_SLOT_POSITIONS: [(f32, f32); 4] = [
    (637.0, 194.0),
    (637.0, 298.0),
    (637.0, 402.0),
    (637.0, 506.0),
];

/// 角色预览位置
pub const CHARACTER_PREVIEW_POS: (f32, f32) = (260.0, 420.0);

/// 底部按钮布局
pub const BUTTON_Y: f32 = 736.0;  // 768 - 32
pub const BUTTON_START_X: f32 = 100.0;
pub const BUTTON_SPACING: f32 = 150.0;

/// 底部按钮配置（button_type, base_index）
pub const BOTTOM_BUTTONS: [(BottomButtonType, i32); 5] = [
    (BottomButtonType::StartGame, 340),
    (BottomButtonType::NewCharacter, 343),
    (BottomButtonType::DeleteCharacter, 346),
    (BottomButtonType::Credits, 349),
    (BottomButtonType::ExitGame, 352),
];

// 角色动画基础索引（参考 ggez 版本）
pub fn get_character_animation_base(class: mir2_shared::enums::MirClass, gender: mir2_shared::enums::MirGender) -> i32 {
    use mir2_shared::enums::{MirClass, MirGender};
    match (class, gender) {
        (MirClass::Warrior, MirGender::Male) => 20,
        (MirClass::Warrior, MirGender::Female) => 300,
        (MirClass::Wizard, MirGender::Male) => 40,
        (MirClass::Wizard, MirGender::Female) => 320,
        (MirClass::Taoist, MirGender::Male) => 60,
        (MirClass::Taoist, MirGender::Female) => 340,
        (MirClass::Assassin, MirGender::Male) => 80,
        (MirClass::Assassin, MirGender::Female) => 360,
        (MirClass::Archer, MirGender::Male) => 100,
        (MirClass::Archer, MirGender::Female) => 140,
    }
}
