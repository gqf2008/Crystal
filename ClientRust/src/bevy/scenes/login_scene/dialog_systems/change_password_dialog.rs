// Change Password Dialog - 修改密码对话框
// 从 mod.rs 中提取的完整实现

use bevy::prelude::*;
use bevy::ui_widgets::Button;
use bevy::picking::hover::Hovered;

use super::super::*;  // Import from parent mod.rs

/// Spawn change password dialog with all 4 input fields
pub fn spawn_change_password_dialog(
    commands: &mut Commands,
    mlibrary_assets: &mut ResMut<crate::bevy::MLibraryAssets>,
    images: &mut ResMut<Assets<Image>>,
    asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    info!("🔑 Creating Change Password Dialog");
    
    // Load dialog background texture (index 50 from Prguse - C# original)
    let dialog_bg = mlibrary_assets.get_texture("Prguse", 50, images);
    
    if dialog_bg.is_none() {
        warn!("❌ Failed to load change password dialog background texture");
        return;
    }
    
    let dialog_bg = dialog_bg.unwrap();
    
    // Get texture size from image
    let bg_image = images.get(&dialog_bg).unwrap();
    let dialog_width = bg_image.width() as f32;
    let dialog_height = bg_image.height() as f32;
    
    info!("📐 Dialog size: {}x{}", dialog_width, dialog_height);
    
    // Load button textures from Title library (C# original)
    // OK: 107/108/109, Cancel: 110/111/112
    let ok_button_tex = mlibrary_assets.get_texture("Title", 107, images)
        .expect("Failed to load OK button texture");
    let cancel_button_tex = mlibrary_assets.get_texture("Title", 110, images)
        .expect("Failed to load Cancel button texture");
    
    // Calculate dialog position (centered)
    let dialog_x = (1024.0 - dialog_width) / 2.0;
    let dialog_y = (768.0 - dialog_height) / 2.0;
    
    // Load font for input fields
    let font = asset_server.load("fonts/NotoSansSC-Regular.ttf");
    
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(dialog_x),
                top: Val::Px(dialog_y),
                width: Val::Px(dialog_width),
                height: Val::Px(dialog_height),
                ..default()
            },
            ZIndex(100), // On top of everything
            ImageNode::from(dialog_bg.clone()),
            ChangePasswordDialog,
            Dialog,
            Name::new("ChangePasswordDialog"),
        )).with_children(|dialog| {
            // OK Button (80, 236) - with button texture and hover effect
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(80.0),
                    top: Val::Px(236.0),
                    width: Val::Px(80.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                ImageNode::from(ok_button_tex.clone()),
                Button,
                Hovered::default(),
                Interaction::default(),
                ButtonType(LoginButtonType::DialogOK),
                ButtonTextures {
                    normal_index: 107,
                    hover_index: 108,
                    pressed_index: 109,
                },
                Name::new("DialogOKButton"),
            ));
            
            // Cancel Button (222, 236) - with button texture and hover effect
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(222.0),
                    top: Val::Px(236.0),
                    width: Val::Px(80.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                ImageNode::from(cancel_button_tex.clone()),
                Button,
                Hovered::default(),
                Interaction::default(),
                ButtonType(LoginButtonType::DialogCancel),
                ButtonTextures {
                    normal_index: 110,
                    hover_index: 111,
                    pressed_index: 112,
                },
                Name::new("DialogCancelButton"),
            ));
            
            // AccountID Input (178, 75, 136x18) from C# original
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(178.0),
                    top: Val::Px(75.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_FOCUSED),
                DialogInputField {
                    field_type: DialogFieldType::ChangeAccountId,
                },
                InputFocused, // Set initial focus
                Button,
                Interaction::default(),
                Name::new("ChangeAccountIdInput"),
            )).with_children(|input_parent| {
                // Add text child
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Name::new("InputText"),
                ));
                
                // Add cursor child
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: true,
                    },
                    Visibility::Inherited,
                    Name::new("InputCursor"),
                ));
            });
            
            // CurrentPassword Input (178, 113, 136x18) from C# original
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(178.0),
                    top: Val::Px(113.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::ChangeCurrentPassword,
                },
                Button,
                Interaction::default(),
                Name::new("ChangeCurrentPasswordInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Name::new("InputText"),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: true,
                    },
                    Visibility::Hidden,
                    Name::new("InputCursor"),
                ));
            });
            
            // NewPassword1 Input (178, 151, 136x18) from C# original
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(178.0),
                    top: Val::Px(151.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::ChangeNewPassword1,
                },
                Button,
                Interaction::default(),
                Name::new("ChangeNewPassword1Input"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Name::new("InputText"),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: true,
                    },
                    Visibility::Hidden,
                    Name::new("InputCursor"),
                ));
            });
            
            // NewPassword2 Input (178, 188, 136x18) from C# original
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(178.0),
                    top: Val::Px(188.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::ChangeNewPassword2,
                },
                Button,
                Interaction::default(),
                Name::new("ChangeNewPassword2Input"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Name::new("InputText"),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: true,
                    },
                    Visibility::Hidden,
                    Name::new("InputCursor"),
                ));
            });
        });
    });
    
    info!("✅ Change Password Dialog created with 4 input fields");
}
