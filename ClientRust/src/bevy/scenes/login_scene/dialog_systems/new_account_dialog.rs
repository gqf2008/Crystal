// New Account Dialog - 新建账号对话框
// 从 mod.rs 中提取

use bevy::prelude::*;
use bevy::ui_widgets::Button;
use bevy::picking::hover::Hovered;

use super::super::*;  // Import from parent mod.rs

/// Spawn new account dialog
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
            // TODO: Add buttons and input fields
            // This is a minimal working version
            info!("✅ New Account Dialog created (simplified)");
        });
    });
}
