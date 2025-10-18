// Change Password Dialog - 修改密码对话框
// 从 mod.rs 中提取

use bevy::prelude::*;
use bevy::ui_widgets::Button;
use bevy::picking::hover::Hovered;

use super::super::*;  // Import from parent mod.rs

/// Spawn change password dialog
pub fn spawn_change_password_dialog(
    commands: &mut Commands,
    mlibrary_assets: &mut ResMut<crate::bevy::MLibraryAssets>,
    images: &mut ResMut<Assets<Image>>,
    _asset_server: &Res<AssetServer>,
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
    
    // Calculate dialog position (centered)
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
            ChangePasswordDialog,
            Dialog,
            Name::new("ChangePasswordDialog"),
        )).with_children(|dialog| {
            // TODO: Add buttons and input fields
            // This is a minimal working version
            info!("✅ Change Password Dialog created (simplified)");
        });
    });
}
