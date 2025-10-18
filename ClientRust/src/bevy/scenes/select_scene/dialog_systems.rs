// Dialog Systems for SelectScene
// 处理新建角色、删除角色、制作人员等对话框

use bevy::prelude::*;
use mir2_shared::enums::{MirClass, MirGender};
use super::components::*;

// ============================================================================
// Dialog Components
// ============================================================================

/// 新建角色对话框标记
#[derive(Component)]
pub struct NewCharacterDialog;

/// 删除角色对话框标记
#[derive(Component)]
pub struct DeleteCharacterDialog;

/// 制作人员对话框标记
#[derive(Component)]
pub struct CreditsDialog;

/// 对话框根节点标记（用于关闭所有对话框）
#[derive(Component)]
pub struct DialogRoot;

/// 对话框输入框组件
#[derive(Component)]
pub struct DialogInputField {
    pub field_type: DialogInputType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogInputType {
    CharacterName,  // 角色名称
}

/// 对话框按钮组件
#[derive(Component)]
pub struct DialogButton {
    pub button_type: DialogButtonType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogButtonType {
    // 新建角色对话框
    Warrior,
    Wizard,
    Taoist,
    Assassin,
    Archer,
    Male,
    Female,
    CreateOK,
    CreateCancel,
    
    // 删除角色对话框
    DeleteOK,
    DeleteCancel,
    
    // 制作人员对话框
    CreditsClose,
}

/// 角色预览组件（对话框内的小预览）
#[derive(Component)]
pub struct DialogCharacterPreview;

// ============================================================================
// Dialog State Resource
// ============================================================================

/// 对话框状态资源
#[derive(Resource, Default)]
pub struct DialogState {
    /// 当前打开的对话框类型
    pub active_dialog: Option<ActiveDialog>,
    
    /// 新建角色对话框数据
    pub new_character: NewCharacterData,
    
    /// 删除角色对话框数据
    pub delete_character: DeleteCharacterData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveDialog {
    NewCharacter,
    DeleteCharacter,
    Credits,
}

/// 新建角色对话框数据
pub struct NewCharacterData {
    pub name: String,
    pub selected_class: MirClass,
    pub selected_gender: MirGender,
    pub animation_frame: usize,
    pub animation_timer: f32,
    pub error_message: Option<String>,
}

impl Default for NewCharacterData {
    fn default() -> Self {
        Self {
            name: String::new(),
            selected_class: MirClass::Warrior,
            selected_gender: MirGender::Male,
            animation_frame: 0,
            animation_timer: 0.0,
            error_message: None,
        }
    }
}

/// 删除角色对话框数据
#[derive(Default)]
pub struct DeleteCharacterData {
    pub character_index: usize,
    pub confirmation_name: String,
}

// ============================================================================
// Dialog Spawn Systems
// ============================================================================

/// 生成新建角色对话框
pub fn spawn_new_character_dialog(
    mut commands: Commands,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    root_query: Query<Entity, With<SelectSceneRoot>>,
) {
    let root = match root_query.iter().next() {
        Some(r) => r,
        None => {
            warn!("⚠️ 无法找到 SelectSceneRoot");
            return;
        }
    };
    
    info!("🎨 创建新建角色对话框...");
    
    // 对话框尺寸（Prguse_73）
    const DIALOG_WIDTH: f32 = 656.0;
    const DIALOG_HEIGHT: f32 = 537.0;
    const DIALOG_X: f32 = (1024.0 - DIALOG_WIDTH) / 2.0;
    const DIALOG_Y: f32 = (768.0 - DIALOG_HEIGHT) / 2.0;
    
    // 加载对话框背景纹理
    let dialog_bg = mlibrary_assets.get_texture("Prguse", 73, &mut images)
        .expect("无法加载对话框背景 Prguse_73");
    
    // 创建对话框根节点
    let dialog_entity = commands.spawn((
        Node {
            width: Val::Px(DIALOG_WIDTH),
            height: Val::Px(DIALOG_HEIGHT),
            position_type: PositionType::Absolute,
            left: Val::Px(DIALOG_X),
            top: Val::Px(DIALOG_Y),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        ImageNode {
            image: dialog_bg,
            ..default()
        },
        NewCharacterDialog,
        DialogRoot,
        Name::new("NewCharacterDialog"),
    )).id();
    
    // 添加到场景根节点
    commands.entity(root).add_child(dialog_entity);
    
    // 添加对话框内容
    spawn_new_character_dialog_contents(&mut commands, dialog_entity, &mut mlibrary_assets, &mut images, &asset_server);
    
    info!("✅ 新建角色对话框已创建");
}

/// 生成新建角色对话框内容
fn spawn_new_character_dialog_contents(
    commands: &mut Commands,
    dialog: Entity,
    mlibrary_assets: &mut crate::bevy::MLibraryAssets,
    images: &mut Assets<Image>,
    asset_server: &Res<AssetServer>,
) {
    // 对话框内容位置（相对于对话框）
    commands.entity(dialog).with_children(|parent| {
        // 标题文本
        parent.spawn((
            Text::new("创建新角色"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(250.0),
                top: Val::Px(20.0),
                ..default()
            },
            TextFont {
                font: asset_server.load("fonts/NotoSansSC-Bold.ttf"),
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.9, 0.6)),
            Name::new("DialogTitle"),
        ));
        
        // 输入框标签
        parent.spawn((
            Text::new("角色名称:"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(80.0),
                top: Val::Px(80.0),
                ..default()
            },
            TextFont {
                font: asset_server.load("fonts/NotoSansSC-Regular.ttf"),
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            Name::new("NameLabel"),
        ));
        
        // 输入框背景
        parent.spawn((
            Node {
                width: Val::Px(300.0),
                height: Val::Px(35.0),
                position_type: PositionType::Absolute,
                left: Val::Px(180.0),
                top: Val::Px(75.0),
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
            BorderColor::from(Color::srgb(0.5, 0.5, 0.5)),
            DialogInputField {
                field_type: DialogInputType::CharacterName,
            },
            Interaction::default(),
            Name::new("NameInput"),
        )).with_children(|input| {
            // 输入框文本
            input.spawn((
                Text::new(""),
                TextFont {
                    font: asset_server.load("fonts/NotoSansSC-Regular.ttf"),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                Name::new("NameText"),
            ));
        });
        
        // 职业选择标签
        parent.spawn((
            Text::new("选择职业:"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(80.0),
                top: Val::Px(140.0),
                ..default()
            },
            TextFont {
                font: asset_server.load("fonts/NotoSansSC-Regular.ttf"),
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            Name::new("ClassLabel"),
        ));
        
        // 职业按钮（5个）- 使用简单的文本按钮
        let classes = [
            (DialogButtonType::Warrior, "战士", 180.0),
            (DialogButtonType::Wizard, "法师", 260.0),
            (DialogButtonType::Taoist, "道士", 340.0),
            (DialogButtonType::Assassin, "刺客", 420.0),
            (DialogButtonType::Archer, "弓手", 500.0),
        ];
        
        for (button_type, text, x) in classes.iter() {
            let bt = *button_type;
            let t = *text;
            let px = *x;
            parent.spawn((
                Node {
                    width: Val::Px(70.0),
                    height: Val::Px(30.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(px),
                    top: Val::Px(135.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
                BorderColor::from(Color::srgb(0.6, 0.6, 0.6)),
                DialogButton { button_type: bt },
                Interaction::default(),
                Name::new(format!("Button_{:?}", bt)),
            )).with_children(|button| {
                button.spawn((
                    Text::new(t),
                    TextFont {
                        font: asset_server.load("fonts/NotoSansSC-Regular.ttf"),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                ));
            });
        }
        
        // 性别选择标签
        parent.spawn((
            Text::new("选择性别:"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(80.0),
                top: Val::Px(200.0),
                ..default()
            },
            TextFont {
                font: asset_server.load("fonts/NotoSansSC-Regular.ttf"),
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            Name::new("GenderLabel"),
        ));
        
        // 性别按钮（2个）
        let genders = [
            (DialogButtonType::Male, "男性", 180.0),
            (DialogButtonType::Female, "女性", 280.0),
        ];
        for (button_type, text, x) in genders.iter() {
            let bt = *button_type;
            let t = *text;
            let px = *x;
            parent.spawn((
                Node {
                    width: Val::Px(70.0),
                    height: Val::Px(30.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(px),
                    top: Val::Px(195.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
                BorderColor::from(Color::srgb(0.6, 0.6, 0.6)),
                DialogButton { button_type: bt },
                Interaction::default(),
                Name::new(format!("Button_{:?}", bt)),
            )).with_children(|button| {
                button.spawn((
                    Text::new(t),
                    TextFont {
                        font: asset_server.load("fonts/NotoSansSC-Regular.ttf"),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                ));
            });
        }
        
        // 角色预览区域
        parent.spawn((
            Node {
                width: Val::Px(200.0),
                height: Val::Px(200.0),
                position_type: PositionType::Absolute,
                left: Val::Px(230.0),
                top: Val::Px(250.0),
                ..default()
            },
            DialogCharacterPreview,
            Name::new("CharacterPreview"),
        ));
        
        // 确定按钮
        let buttons = [
            (DialogButtonType::CreateOK, "确 定", 180.0),
            (DialogButtonType::CreateCancel, "取 消", 380.0),
        ];
        for (button_type, text, x) in buttons.iter() {
            let bt = *button_type;
            let t = *text;
            let px = *x;
            parent.spawn((
                Node {
                    width: Val::Px(120.0),
                    height: Val::Px(40.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(px),
                    top: Val::Px(470.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.9)),
                BorderColor::from(Color::srgb(0.8, 0.8, 0.8)),
                DialogButton { button_type: bt },
                Interaction::default(),
                Name::new(format!("Button_{:?}", bt)),
            )).with_children(|button| {
                button.spawn((
                    Text::new(t),
                    TextFont {
                        font: asset_server.load("fonts/NotoSansSC-Bold.ttf"),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                ));
            });
        }
    });
}

// ============================================================================
// Dialog Button Systems
// ============================================================================

/// 处理对话框按钮点击
pub fn handle_dialog_button_clicks(
    button_query: Query<(&Interaction, &DialogButton), Changed<Interaction>>,
    mut dialog_state: ResMut<DialogState>,
    state: Res<super::components::SelectSceneState>,
    mut commands: Commands,
    dialog_query: Query<Entity, With<DialogRoot>>,
) {
    for (interaction, button) in button_query.iter() {
        if *interaction == Interaction::Pressed {
            match button.button_type {
                // 职业选择
                DialogButtonType::Warrior => {
                    dialog_state.new_character.selected_class = MirClass::Warrior;
                    info!("⚔️ 选择职业: 战士");
                }
                DialogButtonType::Wizard => {
                    dialog_state.new_character.selected_class = MirClass::Wizard;
                    info!("🔮 选择职业: 法师");
                }
                DialogButtonType::Taoist => {
                    dialog_state.new_character.selected_class = MirClass::Taoist;
                    info!("☯️ 选择职业: 道士");
                }
                DialogButtonType::Assassin => {
                    dialog_state.new_character.selected_class = MirClass::Assassin;
                    info!("🗡️ 选择职业: 刺客");
                }
                DialogButtonType::Archer => {
                    dialog_state.new_character.selected_class = MirClass::Archer;
                    info!("🏹 选择职业: 弓手");
                }
                
                // 性别选择
                DialogButtonType::Male => {
                    dialog_state.new_character.selected_gender = MirGender::Male;
                    info!("♂️ 选择性别: 男性");
                }
                DialogButtonType::Female => {
                    dialog_state.new_character.selected_gender = MirGender::Female;
                    info!("♀️ 选择性别: 女性");
                }
                
                // 确认创建
                DialogButtonType::CreateOK => {
                    info!("✅ 创建角色: 名字={}, 职业={:?}, 性别={:?}", 
                        dialog_state.new_character.name,
                        dialog_state.new_character.selected_class,
                        dialog_state.new_character.selected_gender
                    );
                    
                    // 发送创建角色网络命令
                    if let Some(tx) = &state.command_tx {
                        let command = crate::network::NetworkCommand::NewCharacter {
                            name: dialog_state.new_character.name.clone(),
                            class: dialog_state.new_character.selected_class as u8,
                            gender: dialog_state.new_character.selected_gender as u8,
                        };
                        
                        match tx.send(command) {
                            Ok(_) => {
                                info!("📤 已发送 NewCharacter 命令: name={}, class={}, gender={}", 
                                    dialog_state.new_character.name,
                                    dialog_state.new_character.selected_class as u8,
                                    dialog_state.new_character.selected_gender as u8
                                );
                            }
                            Err(e) => {
                                error!("❌ 发送 NewCharacter 命令失败: {}", e);
                            }
                        }
                    } else {
                        warn!("⚠️ 网络命令通道未初始化");
                        info!("📤 [TESTING] 测试模式: 假装创建角色 - {}", dialog_state.new_character.name);
                    }
                    
                    // TODO: 等待服务器响应后再关闭对话框
                    // close_dialog(&mut commands, &dialog_query);
                }
                
                // 取消创建
                DialogButtonType::CreateCancel => {
                    info!("❌ 取消创建角色");
                    close_dialog(&mut commands, &dialog_query);
                }
                
                _ => {}
            }
        }
    }
}

/// 处理对话框按钮悬停效果
pub fn handle_dialog_button_hover(
    mut button_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<DialogButton>)
    >,
) {
    for (interaction, mut bg_color, mut border_color) in button_query.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.95));
                *border_color = BorderColor::from(Color::srgb(1.0, 1.0, 0.5));
            }
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgba(0.5, 0.5, 0.6, 1.0));
                *border_color = BorderColor::from(Color::srgb(1.0, 1.0, 0.3));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8));
                *border_color = BorderColor::from(Color::srgb(0.6, 0.6, 0.6));
            }
        }
    }
}

/// 更新对话框角色预览动画
pub fn update_dialog_character_preview(
    time: Res<Time>,
    mut dialog_state: ResMut<DialogState>,
    mut preview_query: Query<&mut ImageNode, With<DialogCharacterPreview>>,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    if dialog_state.active_dialog != Some(ActiveDialog::NewCharacter) {
        return;
    }
    
    // 更新动画计时器
    dialog_state.new_character.animation_timer += time.delta_secs();
    if dialog_state.new_character.animation_timer >= 0.25 {
        dialog_state.new_character.animation_timer = 0.0;
        dialog_state.new_character.animation_frame = 
            (dialog_state.new_character.animation_frame + 1) % 16;
        
        // 计算纹理索引
        let base_index = get_character_animation_base(
            dialog_state.new_character.selected_class,
            dialog_state.new_character.selected_gender
        );
        let texture_index = base_index + dialog_state.new_character.animation_frame as i32;
        
        // 更新预览纹理
        if let Some(texture) = mlibrary_assets.get_texture("ChrSel", texture_index, &mut images) {
            for mut image_node in preview_query.iter_mut() {
                image_node.image = texture.clone();
            }
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 关闭所有对话框
fn close_dialog(
    commands: &mut Commands,
    dialog_query: &Query<Entity, With<DialogRoot>>,
) {
    for entity in dialog_query.iter() {
        commands.entity(entity).despawn();
    }
    info!("🚪 对话框已关闭");
}

/// 打开新建角色对话框
pub fn open_new_character_dialog(
    mut dialog_state: ResMut<DialogState>,
) {
    dialog_state.active_dialog = Some(ActiveDialog::NewCharacter);
    dialog_state.new_character = NewCharacterData::default();
    info!("📝 打开新建角色对话框");
}
