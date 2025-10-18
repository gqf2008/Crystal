// New Account Dialog - 新建账号对话框
// 从 mod.rs 中提取的完整实现

use bevy::prelude::*;
use bevy::ui_widgets::Button;
use bevy::picking::hover::Hovered;

use super::super::*;  // Import from parent mod.rs

/// Spawn new account dialog with all 8 input fields
pub fn spawn_new_account_dialog(
    commands: &mut Commands,
    mlibrary_assets: &mut ResMut<crate::bevy::MLibraryAssets>,
    images: &mut ResMut<Assets<Image>>,
    _asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    info!("📝 Creating New Account Dialog");
    
    // Load dialog background texture (index 63 from Prguse)
    let dialog_bg = mlibrary_assets.get_texture("Prguse", 63, images);
    
    if dialog_bg.is_none() {
        warn!("❌ Failed to load new account dialog background texture");
        return;
    }
    
    let dialog_bg = dialog_bg.unwrap();
    
    // Load button textures from Title library
    // New Account Dialog: OK: 200/201/202, Cancel: 203/204/205 (from C# original)
    let ok_button_tex = mlibrary_assets.get_texture("Title", 200, images)
        .expect("Failed to load OK button texture");
    let cancel_button_tex = mlibrary_assets.get_texture("Title", 203, images)
        .expect("Failed to load Cancel button texture");
    
    // Get texture size from image
    let bg_image = images.get(&dialog_bg).unwrap();
    let dialog_width = bg_image.width() as f32;
    let dialog_height = bg_image.height() as f32;
    
    info!("📐 Dialog size: {}x{}", dialog_width, dialog_height);
    
    // Calculate dialog position (centered on screen - 1024x768 like C# original)
    let dialog_x = (1024.0 - dialog_width) / 2.0;
    let dialog_y = (768.0 - dialog_height) / 2.0;
    
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
            NewAccountDialog,
            Dialog,
            Name::new("NewAccountDialog"),
        )).with_children(|dialog| {
            // OK Button - with button texture and hover effect
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(135.0),
                    top: Val::Px(425.0),
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
                    normal_index: 200,
                    hover_index: 201,
                    pressed_index: 202,
                },
                Name::new("DialogOKButton"),
            ));
            
            // Cancel Button - with button texture and hover effect
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(409.0),
                    top: Val::Px(425.0),
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
                    normal_index: 203,
                    hover_index: 204,
                    pressed_index: 205,
                },
                Name::new("DialogCancelButton"),
            ));
            
            // Add input fields for account creation
            // AccountID Input (226, 103, 136, 18) from C# original
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(103.0),
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
                    field_type: DialogFieldType::NewAccountId,
                },
                InputFocused, // Set initial focus here
                Button,
                Interaction::default(),
                Name::new("NewAccountIdInput"),
            )).with_children(|input_parent| {
                // Add text child
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                // Add cursor child
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: true,
                    },
                ));
            });
            
            // Password1 Input (226, 129, 136, 18)
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(129.0),
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
                    field_type: DialogFieldType::NewPassword1,
                },
                Button,
                Interaction::default(),
                Name::new("NewPassword1Input"),
            )).with_children(|input_parent| {
                // Add text child
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                // Add cursor child
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // Password2 Input (226, 155, 136, 18)
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(155.0),
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
                    field_type: DialogFieldType::NewPassword2,
                },
                Button,
                Interaction::default(),
                Name::new("NewPassword2Input"),
            )).with_children(|input_parent| {
                // Add text child
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                // Add cursor child
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // UserName Input (226, 189, 136, 18) - optional field
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(189.0),
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
                    field_type: DialogFieldType::NewUserName,
                },
                Button,
                Interaction::default(),
                Name::new("NewUserNameInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // BirthDate Input (226, 215, 136, 18) - optional field
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(215.0),
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
                    field_type: DialogFieldType::NewBirthDate,
                },
                Button,
                Interaction::default(),
                Name::new("NewBirthDateInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // Question Input (226, 250, 190, 18) - optional field, wider
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(250.0),
                    width: Val::Px(190.0),
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
                    field_type: DialogFieldType::NewQuestion,
                },
                Button,
                Interaction::default(),
                Name::new("NewQuestionInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // Answer Input (226, 276, 190, 18) - optional field, wider
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(276.0),
                    width: Val::Px(190.0),
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
                    field_type: DialogFieldType::NewAnswer,
                },
                Button,
                Interaction::default(),
                Name::new("NewAnswerInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // Email Input (226, 311, 136, 18) - optional field
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(311.0),
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
                    field_type: DialogFieldType::NewEmail,
                },
                Button,
                Interaction::default(),
                Name::new("NewEmailInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
        });
    });
    
    info!("✅ New Account Dialog created with 8 input fields");
}
