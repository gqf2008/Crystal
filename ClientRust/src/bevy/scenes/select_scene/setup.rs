// SelectScene Setup Functions
// 负责场景初始化和 UI 元素生成

use bevy::prelude::*;
use crate::bevy::MLibraryAssets;
use super::components::*;

/// 生成背景（Prguse_65）
pub fn spawn_background(
    commands: &mut Commands,
    parent: Entity,
    mlibrary_assets: &mut MLibraryAssets,
    images: &mut Assets<Image>,
) {
    info!("🔄 加载背景纹理 (Prguse_65)...");
    
    if let Some(background_tex) = mlibrary_assets.get_texture("Prguse", 65, images) {
        commands.entity(parent).with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Px(1024.0),
                    height: Val::Px(768.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                ImageNode {
                    image: background_tex,
                    ..default()
                },
                Name::new("Background"),
            ));
        });
        info!("✅ 背景纹理已加载");
    } else {
        warn!("⚠️ 无法加载背景纹理 Prguse_65");
    }
}

/// 生成标题（Title_40）
pub fn spawn_title(
    commands: &mut Commands,
    parent: Entity,
    mlibrary_assets: &mut MLibraryAssets,
    images: &mut Assets<Image>,
) {
    info!("🔄 加载标题纹理 (Title_40)...");
    
    if let Some(title_tex) = mlibrary_assets.get_texture("Title", 40, images) {
        commands.entity(parent).with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Auto,
                    height: Val::Auto,
                    position_type: PositionType::Absolute,
                    left: Val::Px(468.0),
                    top: Val::Px(20.0),
                    ..default()
                },
                ImageNode {
                    image: title_tex,
                    ..default()
                },
                Name::new("Title"),
            ));
        });
        info!("✅ 标题纹理已加载");
    } else {
        warn!("⚠️ 无法加载标题纹理 Title_40");
    }
}

/// 生成服务器标签
pub fn spawn_server_label(
    commands: &mut Commands,
    parent: Entity,
    asset_server: &Res<AssetServer>,
) {
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Text::new("Legend of Mir 2"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(432.0 + 77.5),
                top: Val::Px(60.0),
                ..default()
            },
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.78, 0.78, 0.78)),  // 浅灰色
            ServerLabel,
            Name::new("ServerLabel"),
        ));
    });
}

/// 生成角色槽位（4个）
pub fn spawn_character_slots(
    commands: &mut Commands,
    parent: Entity,
    _mlibrary_assets: &mut MLibraryAssets,
    _images: &mut Assets<Image>,
    asset_server: &Res<AssetServer>,
) {
    info!("🔄 创建角色槽位...");
    
    for slot_index in 0..4 {
        let (x, y) = CHARACTER_SLOT_POSITIONS[slot_index];
        
        // 创建槽位容器（可点击）
        let slot_entity = commands.spawn((
            Node {
                width: Val::Px(200.0),  // 设置点击区域大小
                height: Val::Px(90.0),
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(y),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            CharacterSlot { slot_index },
            Interaction::default(),  // 添加交互能力
            Name::new(format!("CharacterSlot_{}", slot_index)),
        )).id();
        
        // 添加为父节点的子节点
        commands.entity(parent).add_children(&[slot_entity]);
        
        // 创建文本子元素
        commands.entity(slot_entity).with_children(|slot| {
            // 角色名称
            slot.spawn((
                Text::new(""),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.6)),  // 金黄色
                CharacterSlotText {
                    slot_index,
                    text_type: SlotTextType::Name,
                },
                Name::new(format!("SlotName_{}", slot_index)),
            ));
            
            // 等级
            slot.spawn((
                Text::new(""),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),  // 浅灰色
                CharacterSlotText {
                    slot_index,
                    text_type: SlotTextType::Level,
                },
                Name::new(format!("SlotLevel_{}", slot_index)),
            ));
            
            // 职业
            slot.spawn((
                Text::new(""),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.8, 1.0)),  // 浅蓝色
                CharacterSlotText {
                    slot_index,
                    text_type: SlotTextType::Class,
                },
                Name::new(format!("SlotClass_{}", slot_index)),
            ));
        });
    }
    
    info!("✅ 角色槽位已创建");
}

/// 生成角色预览动画区域
pub fn spawn_character_preview(
    commands: &mut Commands,
    parent: Entity,
) {
    let (x, y) = CHARACTER_PREVIEW_POS;
    
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Node {
                width: Val::Auto,
                height: Val::Auto,
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(y),
                ..default()
            },
            CharacterPreview,
            Name::new("CharacterPreview"),
        ));
    });
    
    info!("✅ 角色预览区域已创建");
}

/// 生成底部按钮（5个）
pub fn spawn_bottom_buttons(
    commands: &mut Commands,
    parent: Entity,
    mlibrary_assets: &mut MLibraryAssets,
    images: &mut Assets<Image>,
) {
    info!("🔄 创建底部按钮...");
    
    for (i, (button_type, base_index)) in BOTTOM_BUTTONS.iter().enumerate() {
        let x = BUTTON_START_X + (i as f32) * BUTTON_SPACING;
        let y = BUTTON_Y;
        
        // 加载按钮纹理（默认状态）
        if let Some(button_tex) = mlibrary_assets.get_texture("Title", *base_index, images) {
            commands.entity(parent).with_children(|parent| {
                parent.spawn((
                    Node {
                        width: Val::Auto,
                        height: Val::Auto,
                        position_type: PositionType::Absolute,
                        left: Val::Px(x),
                        top: Val::Px(y),
                        ..default()
                    },
                    ImageNode {
                        image: button_tex,
                        ..default()
                    },
                    BottomButton {
                        button_type: *button_type,
                        position_index: i,
                    },
                    Interaction::default(),
                    Name::new(format!("Button_{:?}", button_type)),
                ));
            });
        } else {
            warn!("⚠️ 无法加载按钮纹理 Title_{}", base_index);
        }
    }
    
    info!("✅ 底部按钮已创建");
}
