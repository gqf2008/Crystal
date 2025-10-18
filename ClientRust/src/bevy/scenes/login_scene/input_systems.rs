// Input handling systems for LoginScene
// Handles text input, Tab switching, cursor blinking, and validation borders

use bevy::prelude::*;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::ui_widgets::Button;

use super::components::*;

// ============================================================================
// Input Handling Systems
// ============================================================================

/// Handle text input for focused text boxes
pub fn handle_text_input(
    mut keyboard_events: EventReader<KeyboardInput>,
    mut login_state: ResMut<LoginState>,
    account_query: Query<Entity, (With<AccountIdInput>, With<InputFocused>)>,
    password_query: Query<Entity, (With<PasswordInput>, With<InputFocused>)>,
) {
    let has_account_focus = !account_query.is_empty();
    let has_password_focus = !password_query.is_empty();
    
    if !has_account_focus && !has_password_focus {
        return; // No input has focus
    }
    
    for event in keyboard_events.read() {
        info!("⌨️ Keyboard event: {:?}", event);
        
        // Only process when key is first pressed
        if !event.state.is_pressed() {
            continue;
        }
        
        match &event.logical_key {
            // Handle Tab key to switch focus
            Key::Tab => {
                if has_account_focus {
                    info!("⇥ Tab pressed: switching to password");
                } else if has_password_focus {
                    info!("⇥ Tab pressed: switching to account");
                }
                continue;
            }
            // Handle backspace
            Key::Backspace => {
                if has_account_focus && !login_state.account_id.is_empty() {
                    login_state.account_id.pop();
                    validate_account_id(&mut login_state);
                } else if has_password_focus && !login_state.password.is_empty() {
                    login_state.password.pop();
                    validate_password(&mut login_state);
                }
            }
            // Handle character input
            Key::Character(text) => {
                for ch in text.chars() {
                    if !is_printable_char(ch) {
                        continue;
                    }
                    
                    if has_account_focus {
                        if login_state.account_id.len() < MAX_ACCOUNT_ID_LENGTH {
                            login_state.account_id.push(ch);
                            validate_account_id(&mut login_state);
                        }
                    } else if has_password_focus {
                        if login_state.password.len() < MAX_PASSWORD_LENGTH {
                            login_state.password.push(ch);
                            validate_password(&mut login_state);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

// Helper function to check if character is printable
fn is_printable_char(chr: char) -> bool {
    let is_in_private_use_area = ('\u{e000}'..='\u{f8ff}').contains(&chr)
        || ('\u{f0000}'..='\u{ffffd}').contains(&chr)
        || ('\u{100000}'..='\u{10fffd}').contains(&chr);

    !is_in_private_use_area && !chr.is_ascii_control()
}

/// Update text display for inputs
pub fn update_input_display(
    login_state: Res<LoginState>,
    mut account_query: Query<&Children, With<AccountIdInput>>,
    mut password_query: Query<&Children, (With<PasswordInput>, Without<AccountIdInput>)>,
    mut text_query: Query<(&mut Text, &mut TextColor), Without<InputCursor>>,
) {
    // Update account ID display
    for children in account_query.iter_mut() {
        if let Some(child) = children.first() {
            if let Ok((mut text, mut color)) = text_query.get_mut(*child) {
                if login_state.account_id.is_empty() {
                    text.0 = "账号".to_string();
                    color.0 = Color::srgba(0.5, 0.5, 0.5, 1.0);
                } else {
                    text.0 = login_state.account_id.clone();
                    color.0 = Color::srgba(1.0, 1.0, 1.0, 1.0);
                }
                info!("📝 Account text updated: '{}'", text.0);
            }
        }
    }
    
    // Update password display (masked)
    for children in password_query.iter_mut() {
        if let Some(child) = children.first() {
            if let Ok((mut text, mut color)) = text_query.get_mut(*child) {
                if login_state.password.is_empty() {
                    text.0 = "密码".to_string();
                    color.0 = Color::srgba(0.5, 0.5, 0.5, 1.0);
                } else {
                    text.0 = "*".repeat(login_state.password.len());
                    color.0 = Color::srgba(1.0, 1.0, 1.0, 1.0);
                }
                info!("📝 Password text updated: '{}'", text.0);
            }
        }
    }
}

/// Handle input focus on click
pub fn handle_input_focus(
    mut commands: Commands,
    mut interaction_query: Query<
        (Entity, &Interaction, &Children, Option<&AccountIdInput>, Option<&PasswordInput>),
        (Changed<Interaction>, With<Button>),
    >,
    focused_query: Query<(Entity, &Children), With<InputFocused>>,
    mut cursor_query: Query<&mut Visibility, With<InputCursor>>,
) {
    for (entity, interaction, children, is_account, is_password) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed && (is_account.is_some() || is_password.is_some()) {
            // Remove focus from all inputs and hide their cursors
            for (focused_entity, focused_children) in focused_query.iter() {
                commands.entity(focused_entity).remove::<InputFocused>();
                
                for child in focused_children.iter() {
                    if let Ok(mut visibility) = cursor_query.get_mut(child) {
                        *visibility = Visibility::Hidden;
                    }
                }
            }
            
            // Add focus to clicked input
            commands.entity(entity).insert(InputFocused);
            
            // Show cursor for focused input
            for child in children.iter() {
                if let Ok(mut visibility) = cursor_query.get_mut(child) {
                    *visibility = Visibility::Inherited;
                }
            }
            
            let input_type = if is_account.is_some() { "账号" } else { "密码" };
            info!("✅ Input focus changed to {}", input_type);
        }
    }
}

/// Handle Tab key to switch focus between inputs
pub fn handle_tab_focus(
    mut commands: Commands,
    mut keyboard_events: EventReader<KeyboardInput>,
    account_query: Query<(Entity, &Children), (With<AccountIdInput>, With<InputFocused>)>,
    password_query: Query<(Entity, &Children), (With<PasswordInput>, With<InputFocused>)>,
    account_entity_query: Query<(Entity, &Children), (With<AccountIdInput>, Without<InputFocused>)>,
    password_entity_query: Query<(Entity, &Children), (With<PasswordInput>, Without<InputFocused>)>,
    mut cursor_query: Query<&mut Visibility, With<InputCursor>>,
) {
    for event in keyboard_events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        
        if let Key::Tab = event.logical_key {
            if let Ok((account_entity, account_children)) = account_query.single() {
                commands.entity(account_entity).remove::<InputFocused>();
                
                for child in account_children.iter() {
                    if let Ok(mut visibility) = cursor_query.get_mut(child) {
                        *visibility = Visibility::Hidden;
                    }
                }
                
                if let Ok((password_entity, password_children)) = password_entity_query.single() {
                    commands.entity(password_entity).insert(InputFocused);
                    
                    for child in password_children.iter() {
                        if let Ok(mut visibility) = cursor_query.get_mut(child) {
                            *visibility = Visibility::Inherited;
                        }
                    }
                    
                    info!("⇥ Tab: Account → Password");
                }
            } else if let Ok((password_entity, password_children)) = password_query.single() {
                commands.entity(password_entity).remove::<InputFocused>();
                
                for child in password_children.iter() {
                    if let Ok(mut visibility) = cursor_query.get_mut(child) {
                        *visibility = Visibility::Hidden;
                    }
                }
                
                if let Ok((account_entity, account_children)) = account_entity_query.single() {
                    commands.entity(account_entity).insert(InputFocused);
                    
                    for child in account_children.iter() {
                        if let Ok(mut visibility) = cursor_query.get_mut(child) {
                            *visibility = Visibility::Inherited;
                        }
                    }
                    
                    info!("⇥ Tab: Password → Account");
                }
            }
        }
    }
}

/// Update cursor blink animation
pub fn update_cursor_blink(
    time: Res<Time>,
    input_query: Query<&Children, With<InputFocused>>,
    mut cursor_query: Query<(&mut InputCursor, &mut Visibility)>,
) {
    for children in input_query.iter() {
        for child in children.iter() {
            if let Ok((mut cursor, mut visibility)) = cursor_query.get_mut(child) {
                cursor.blink_timer += time.delta_secs();
                
                if cursor.blink_timer >= 0.5 {
                    cursor.blink_timer = 0.0;
                    *visibility = match *visibility {
                        Visibility::Inherited => Visibility::Hidden,
                        _ => Visibility::Inherited,
                    };
                }
            }
        }
    }
}

/// Update input border colors based on validation
pub fn update_input_borders(
    login_state: Res<LoginState>,
    mut account_query: Query<&mut BorderColor, With<AccountIdInput>>,
    mut password_query: Query<&mut BorderColor, (With<PasswordInput>, Without<AccountIdInput>)>,
) {
    // Update account input border
    for mut border in account_query.iter_mut() {
        let new_color = if login_state.account_id.is_empty() {
            INPUT_BORDER_NORMAL
        } else if login_state.account_id_valid {
            INPUT_BORDER_VALID
        } else {
            INPUT_BORDER_INVALID
        };
        *border = BorderColor::all(new_color);
    }
    
    // Update password input border
    for mut border in password_query.iter_mut() {
        let new_color = if login_state.password.is_empty() {
            INPUT_BORDER_NORMAL
        } else if login_state.password_valid {
            INPUT_BORDER_VALID
        } else {
            INPUT_BORDER_INVALID
        };
        *border = BorderColor::all(new_color);
    }
}

// ============================================================================
// Validation Helper Functions
// ============================================================================

use regex::Regex;

fn validate_account_id(login_state: &mut LoginState) {
    if login_state.account_id.is_empty() {
        login_state.account_id_valid = false;
    } else {
        let pattern = format!(r"^[a-zA-Z0-9]{{{},{}}}$", MIN_ACCOUNT_ID_LENGTH, MAX_ACCOUNT_ID_LENGTH);
        login_state.account_id_valid = Regex::new(&pattern)
            .map(|re| re.is_match(&login_state.account_id))
            .unwrap_or(false);
    }
    update_login_enabled(login_state);
}

fn validate_password(login_state: &mut LoginState) {
    if login_state.password.is_empty() {
        login_state.password_valid = false;
    } else {
        let pattern = format!(r"^[a-zA-Z0-9]{{{},{}}}$", MIN_PASSWORD_LENGTH, MAX_PASSWORD_LENGTH);
        login_state.password_valid = Regex::new(&pattern)
            .map(|re| re.is_match(&login_state.password))
            .unwrap_or(false);
    }
    update_login_enabled(login_state);
}

fn update_login_enabled(login_state: &mut LoginState) {
    login_state.login_enabled = login_state.account_id_valid && login_state.password_valid;
}
