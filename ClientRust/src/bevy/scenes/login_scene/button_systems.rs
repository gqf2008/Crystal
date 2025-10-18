// Button interaction systems for LoginScene
// Handles button hover effects, press states, and click events

use bevy::prelude::*;
use bevy::ecs::message::MessageWriter;
use bevy::picking::hover::Hovered;
use bevy::ui_widgets::Button;

use super::components::*;

// ============================================================================
// Button Interaction Systems
// ============================================================================

/// Handle button hover effects - changes texture based on hover state
pub fn handle_button_hover(
    mut query: Query<
        (&Hovered, &ButtonTextures, &mut ImageNode),
        (Changed<Hovered>, With<ButtonType>),
    >,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    for (hovered, textures, mut image) in query.iter_mut() {
        if hovered.0 {
            // Load and apply hover texture
            if let Some(hover_tex) = mlibrary_assets.get_texture("Title", textures.hover_index as i32, &mut images) {
                image.image = hover_tex.clone();
                info!("🖱️ Button hover: ON (index {})", textures.hover_index);
            }
        } else {
            // Load and apply normal texture
            if let Some(normal_tex) = mlibrary_assets.get_texture("Title", textures.normal_index as i32, &mut images) {
                image.image = normal_tex.clone();
                info!("🖱️ Button hover: OFF (index {})", textures.normal_index);
            }
        }
    }
}

/// Handle button pressed state by changing textures
pub fn handle_button_press(
    mut query: Query<
        (&Interaction, &ButtonTextures, &Hovered, &mut ImageNode),
        (Changed<Interaction>, With<ButtonType>),
    >,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    for (interaction, textures, hovered, mut image) in query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                // Load and apply pressed texture
                if let Some(pressed_tex) = mlibrary_assets.get_texture("Title", textures.pressed_index as i32, &mut images) {
                    image.image = pressed_tex.clone();
                    info!("🖱️ Button pressed (index {})", textures.pressed_index);
                }
            }
            Interaction::Hovered => {
                // Load and apply hover texture
                if let Some(hover_tex) = mlibrary_assets.get_texture("Title", textures.hover_index as i32, &mut images) {
                    image.image = hover_tex.clone();
                }
            }
            Interaction::None => {
                // Restore to hover or normal based on hover state
                let index = if hovered.0 { textures.hover_index } else { textures.normal_index };
                if let Some(tex) = mlibrary_assets.get_texture("Title", index as i32, &mut images) {
                    image.image = tex.clone();
                }
            }
        }
    }
}

/// Handle button clicks and send messages
pub fn handle_button_clicks(
    mut query: Query<(&Interaction, &ButtonType), Changed<Interaction>>,
    login_state: Res<LoginState>,
    mut login_events: MessageWriter<LoginButtonPressed>,
    mut account_events: MessageWriter<NewAccountButtonPressed>,
    mut password_events: MessageWriter<PasswordChangeButtonPressed>,
    mut view_key_events: MessageWriter<ViewKeyButtonPressed>,
    mut close_events: MessageWriter<CloseButtonPressed>,
) {
    for (interaction, button_type) in query.iter_mut() {
        if *interaction == Interaction::Pressed {
            match button_type.0 {
                LoginButtonType::Login => {
                    if login_state.login_enabled {
                        info!("🔵 Login button clicked");
                        login_events.write(LoginButtonPressed {
                            account_id: login_state.account_id.clone(),
                            password: login_state.password.clone(),
                        });
                    }
                }
                LoginButtonType::NewAccount => {
                    info!("🟢 New Account button clicked");
                    account_events.write(NewAccountButtonPressed);
                }
                LoginButtonType::PasswordChange => {
                    info!("🟡 Password Change button clicked");
                    password_events.write(PasswordChangeButtonPressed);
                }
                LoginButtonType::ViewKey => {
                    info!("🔑 View Key button clicked");
                    view_key_events.write(ViewKeyButtonPressed);
                }
                LoginButtonType::Close => {
                    info!("🔴 Close button clicked");
                    close_events.write(CloseButtonPressed);
                }
                LoginButtonType::DialogOK => {
                    info!("✅ Dialog OK button clicked");
                    // Handled by dialog-specific systems
                }
                LoginButtonType::DialogCancel => {
                    info!("❌ Dialog Cancel button clicked");
                    // Handled by dialog-specific systems
                }
            }
        }
    }
}
