// DeleteCharacterDialog - Character deletion confirmation dialog
// Mirrors Client/MirScenes/SelectScene.cs DeleteCharacter() method

use ggez::{Context, GameResult};
use ggez::graphics::{self, Color, DrawParam, Rect, Canvas};
use ggez::input::keyboard::KeyCode;
use ggez::mint::Point2;

/// Character deletion confirmation dialog
/// 
/// Mirrors C# DeleteCharacter() flow with MirMessageBox and MirInputBox:
/// ```csharp
/// MirMessageBox message = new MirMessageBox("Are you sure...", MirMessageBoxButtons.YesNo);
/// message.YesButton.Click += (o1, e1) => {
///     MirInputBox inputBox = new MirInputBox("Please enter the characters name.");
///     inputBox.OKButton.Click += (o, e) => {
///         if (inputBox.InputTextBox.Text == name) {
///             Network.Enqueue(new C.DeleteCharacter { CharacterIndex = index });
///         }
///     };
/// };
/// ```
#[derive(Debug, Clone)]
pub struct DeleteCharacterDialog {
    /// The character name that needs to be confirmed
    pub character_name: String,
    
    /// The character index to delete
    pub character_index: i32,
    
    /// Current input text
    pub input_text: String,
    
    /// IME preedit text (拼音输入)
    pub ime_preedit: String,
    
    /// Error message to display
    pub error_message: Option<String>,
    
    /// Waiting for server response
    pub deleting: bool,
    
    /// Dialog state
    pub state: DialogState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogState {
    /// Initial confirmation: "Are you sure?"
    Confirmation,
    /// Name input: "Please enter the character's name"
    NameInput,
}

impl DeleteCharacterDialog {
    /// Create a new delete character dialog
    pub fn new(character_name: String, character_index: i32) -> Self {
        Self {
            character_name,
            character_index,
            input_text: String::new(),
            ime_preedit: String::new(),
            error_message: None,
            deleting: false,
            state: DialogState::Confirmation,
        }
    }
    
    /// Check if dialog is visible
    pub fn is_visible(&self) -> bool {
        // Dialog is always visible once created (until explicitly hidden)
        true
    }
    
    /// Reset dialog state
    pub fn reset(&mut self) {
        self.input_text.clear();
        self.ime_preedit.clear();
        self.error_message = None;
        self.deleting = false;
        self.state = DialogState::Confirmation;
    }
    
    /// Move to name input state (user clicked "Yes")
    pub fn confirm(&mut self) {
        self.state = DialogState::NameInput;
        self.input_text.clear();
        self.error_message = None;
    }
    
    /// Check if user can submit (name matches)
    pub fn can_submit(&self) -> bool {
        self.state == DialogState::NameInput 
            && self.input_text == self.character_name 
            && !self.deleting
    }
    
    /// Handle text input from IME
    pub fn handle_ime_commit(&mut self, text: String) {
        if self.state == DialogState::NameInput {
            self.input_text.push_str(&text);
            self.ime_preedit.clear();
            tracing::info!("✓ IME 确认输入: {} -> 当前输入: {}", text, self.input_text);
        }
    }
    
    /// Handle IME preedit (拼音编辑)
    pub fn handle_ime_preedit(&mut self, text: String) {
        if self.state == DialogState::NameInput {
            self.ime_preedit = text.clone();
            tracing::debug!("IME 拼音编辑: {}", text);
        }
    }
    
    /// Handle character input
    pub fn handle_char(&mut self, ch: char) {
        if self.state == DialogState::NameInput && !self.deleting {
            self.input_text.push(ch);
            tracing::info!("→ 输入字符: '{}' -> 当前输入: {}", ch, self.input_text);
        }
    }
    
    /// Handle backspace
    pub fn handle_backspace(&mut self) {
        if self.state == DialogState::NameInput && !self.deleting && !self.input_text.is_empty() {
            self.input_text.pop();
        }
    }
    
    /// Render the dialog
    /// 
    /// Note: This is a simplified version that uses basic graphics.
    /// The actual rendering with textures should be done in SelectScene::draw_delete_character_dialog()
    /// which has access to GgezManager for loading Prguse_360 (MessageBox) and Title buttons.
    pub fn render(
        &self,
        ctx: &Context,
        canvas: &mut Canvas,
        font: &graphics::Text,
        window_width: f32,
        window_height: f32,
    ) -> GameResult<()> {
        // This method is kept for compatibility but actual rendering should be done
        // in SelectScene with proper texture support
        Ok(())
    }
    
    /// Render confirmation state
    fn render_confirmation(
        &self,
        ctx: &Context,
        canvas: &mut Canvas,
        font: &graphics::Text,
        dialog_x: f32,
        dialog_y: f32,
        dialog_width: f32,
    ) -> GameResult<()> {
        // Title
        let title = graphics::Text::new(format!("删除角色确认"));
        canvas.draw(
            &title,
            DrawParam::default()
                .dest(Point2 { x: dialog_x + 20.0, y: dialog_y + 20.0 })
                .color(Color::from_rgb(255, 200, 100)),
        );
        
        // Message
        let message = graphics::Text::new(format!(
            "您确定要删除角色 {} 吗？\n\n此操作无法撤销！",
            self.character_name
        ));
        canvas.draw(
            &message,
            DrawParam::default()
                .dest(Point2 { x: dialog_x + 20.0, y: dialog_y + 60.0 })
                .color(Color::WHITE),
        );
        
        // Buttons (Yes / No)
        let button_y = dialog_y + 180.0;
        
        // Yes button
        let yes_rect = Rect::new(dialog_x + 50.0, button_y, 120.0, 40.0);
        let yes_button = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            yes_rect,
            Color::from_rgb(60, 120, 60),
        )?;
        canvas.draw(&yes_button, DrawParam::default());
        
        let yes_text = graphics::Text::new("是");
        canvas.draw(
            &yes_text,
            DrawParam::default()
                .dest(Point2 { x: dialog_x + 100.0, y: button_y + 10.0 })
                .color(Color::WHITE),
        );
        
        // No button
        let no_rect = Rect::new(dialog_x + 230.0, button_y, 120.0, 40.0);
        let no_button = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            no_rect,
            Color::from_rgb(120, 60, 60),
        )?;
        canvas.draw(&no_button, DrawParam::default());
        
        let no_text = graphics::Text::new("否");
        canvas.draw(
            &no_text,
            DrawParam::default()
                .dest(Point2 { x: dialog_x + 280.0, y: button_y + 10.0 })
                .color(Color::WHITE),
        );
        
        Ok(())
    }
    
    /// Render name input state
    fn render_name_input(
        &self,
        ctx: &Context,
        canvas: &mut Canvas,
        font: &graphics::Text,
        dialog_x: f32,
        dialog_y: f32,
        dialog_width: f32,
    ) -> GameResult<()> {
        // Title
        let title = graphics::Text::new("请输入角色名称");
        canvas.draw(
            &title,
            DrawParam::default()
                .dest(Point2 { x: dialog_x + 20.0, y: dialog_y + 20.0 })
                .color(Color::from_rgb(255, 200, 100)),
        );
        
        // Instruction
        let instruction = graphics::Text::new(format!(
            "请输入角色名称以确认删除:\n{}", 
            self.character_name
        ));
        canvas.draw(
            &instruction,
            DrawParam::default()
                .dest(Point2 { x: dialog_x + 20.0, y: dialog_y + 60.0 })
                .color(Color::WHITE),
        );
        
        // Input box
        let input_y = dialog_y + 120.0;
        let input_rect = Rect::new(dialog_x + 20.0, input_y, dialog_width - 40.0, 30.0);
        let input_bg = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            input_rect,
            Color::from_rgb(40, 40, 50),
        )?;
        canvas.draw(&input_bg, DrawParam::default());
        
        let input_border = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(1.0),
            input_rect,
            Color::from_rgb(150, 150, 170),
        )?;
        canvas.draw(&input_border, DrawParam::default());
        
        // Display input text + IME preedit
        let display_text = if !self.ime_preedit.is_empty() {
            format!("{}|{}", self.input_text, self.ime_preedit)
        } else {
            self.input_text.clone()
        };
        
        let input_text = graphics::Text::new(&display_text);
        canvas.draw(
            &input_text,
            DrawParam::default()
                .dest(Point2 { x: dialog_x + 30.0, y: input_y + 5.0 })
                .color(Color::WHITE),
        );
        
        // Error message or status
        let status_y = dialog_y + 160.0;
        if let Some(error) = &self.error_message {
            let error_text = graphics::Text::new(error);
            canvas.draw(
                &error_text,
                DrawParam::default()
                    .dest(Point2 { x: dialog_x + 20.0, y: status_y })
                    .color(Color::from_rgb(255, 100, 100)),
            );
        } else if self.deleting {
            let status_text = graphics::Text::new("正在删除角色...");
            canvas.draw(
                &status_text,
                DrawParam::default()
                    .dest(Point2 { x: dialog_x + 20.0, y: status_y })
                    .color(Color::from_rgb(255, 200, 100)),
            );
        }
        
        // Buttons (OK / Cancel)
        let button_y = dialog_y + 200.0;
        
        // OK button (enabled only if name matches)
        let ok_color = if self.can_submit() {
            Color::from_rgb(60, 120, 60)
        } else {
            Color::from_rgb(80, 80, 80)
        };
        
        let ok_rect = Rect::new(dialog_x + 50.0, button_y, 120.0, 40.0);
        let ok_button = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            ok_rect,
            ok_color,
        )?;
        canvas.draw(&ok_button, DrawParam::default());
        
        let ok_text = graphics::Text::new("确定");
        canvas.draw(
            &ok_text,
            DrawParam::default()
                .dest(Point2 { x: dialog_x + 95.0, y: button_y + 10.0 })
                .color(Color::WHITE),
        );
        
        // Cancel button
        let cancel_rect = Rect::new(dialog_x + 230.0, button_y, 120.0, 40.0);
        let cancel_button = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            cancel_rect,
            Color::from_rgb(120, 60, 60),
        )?;
        canvas.draw(&cancel_button, DrawParam::default());
        
        let cancel_text = graphics::Text::new("取消");
        canvas.draw(
            &cancel_text,
            DrawParam::default()
                .dest(Point2 { x: dialog_x + 270.0, y: button_y + 10.0 })
                .color(Color::WHITE),
        );
        
        Ok(())
    }
    
    /// Handle mouse click
    /// Returns: (confirm_clicked, cancel_clicked, submit_clicked)
    /// 
    /// Mirrors C# button positions from MirMessageBox and MirInputBox
    pub fn handle_click(&self, x: f32, y: f32, window_width: f32, window_height: f32) -> (bool, bool, bool) {
        match self.state {
            DialogState::Confirmation => {
                // MessageBox dimensions: 464 x 260
                let dialog_width = 464.0;
                let dialog_height = 260.0;
                let dialog_x = (window_width - dialog_width) / 2.0;
                let dialog_y = (window_height - dialog_height) / 2.0;
                
                // Yes button: C# Location = 260, 157, Size from Title library ~71x29
                if x >= dialog_x + 260.0 && x <= dialog_x + 260.0 + 71.0
                    && y >= dialog_y + 157.0 && y <= dialog_y + 157.0 + 29.0 {
                    return (true, false, false);
                }
                
                // No button: C# Location = 360, 157
                if x >= dialog_x + 360.0 && x <= dialog_x + 360.0 + 71.0
                    && y >= dialog_y + 157.0 && y <= dialog_y + 157.0 + 29.0 {
                    return (false, true, false);
                }
            }
            DialogState::NameInput => {
                // InputBox dimensions: 290 x 188
                let dialog_width = 290.0;
                let dialog_height = 188.0;
                let dialog_x = (window_width - dialog_width) / 2.0;
                let dialog_y = (window_height - dialog_height) / 2.0;
                
                // OK button: C# Location = 60, 123
                if self.can_submit() 
                    && x >= dialog_x + 60.0 && x <= dialog_x + 60.0 + 71.0
                    && y >= dialog_y + 123.0 && y <= dialog_y + 123.0 + 29.0 {
                    return (false, false, true);
                }
                
                // Cancel button: C# Location = 160, 123
                if x >= dialog_x + 160.0 && x <= dialog_x + 160.0 + 71.0
                    && y >= dialog_y + 123.0 && y <= dialog_y + 123.0 + 29.0 {
                    return (false, true, false);
                }
            }
        }
        
        (false, false, false)
    }
}
