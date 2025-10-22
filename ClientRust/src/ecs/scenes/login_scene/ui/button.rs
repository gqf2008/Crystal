//! 按钮UI组件
//! 
//! 将按钮抽象为ECS实体，包含位置、边界、渲染、交互状态

use hecs::{Entity, World};
use crate::graphics::LibraryName;
use super::super::components::*;

/// 按钮构建器
pub struct ButtonBuilder {
    library: LibraryName,
    normal_index: i32,
    hover_index: i32,
    pressed_index: i32,
    position: Position,
    size: Option<Size>,
    action: ButtonAction,
    enabled: bool,
}

impl ButtonBuilder {
    pub fn new(library: LibraryName, normal_index: i32, action: ButtonAction) -> Self {
        Self {
            library,
            normal_index,
            hover_index: normal_index + 1,
            pressed_index: normal_index + 2,
            position: Position { x: 0.0, y: 0.0 },
            size: None,
            action,
            enabled: true,
        }
    }

    pub fn hover_index(mut self, index: i32) -> Self {
        self.hover_index = index;
        self
    }

    pub fn pressed_index(mut self, index: i32) -> Self {
        self.pressed_index = index;
        self
    }

    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.position = Position { x, y };
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Some(Size { width, height });
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 创建按钮实体
    pub fn build(self, world: &mut World) -> Entity {
        // 如果没有指定尺寸，使用默认值
        let size = self.size.unwrap_or(Size { width: 80.0, height: 30.0 });
        
        let bounds = Bounds {
            x: self.position.x,
            y: self.position.y,
            width: size.width,
            height: size.height,
        };

        let button = Button {
            normal_index: self.normal_index,
            hover_index: self.hover_index,
            pressed_index: self.pressed_index,
            enabled: self.enabled,
            hovered: false,
            pressed: false,
            action: self.action,
        };

        let sprite = Sprite {
            library: self.library,
            index: self.normal_index,
            visible: true,
        };

        world.spawn((
            ButtonEntity,
            self.position,
            size,
            bounds,
            button,
            sprite,
            Clickable { enabled: self.enabled },
            HoverState { hovered: false },
            Visible(true),
        ))
    }
}

/// 按钮辅助函数
pub mod button_helpers {
    use super::*;

    /// 更新按钮悬停状态
    pub fn update_hover(world: &mut World, mouse_x: f32, mouse_y: f32) {
        for (_entity, (bounds, mut button, mut hover, mut sprite)) in world.query_mut::<(&Bounds, &mut Button, &mut HoverState, &mut Sprite)>() {
            let was_hovered = hover.hovered;
            hover.hovered = button.enabled && bounds.contains(mouse_x, mouse_y);
            button.hovered = hover.hovered;

            // 更新精灵索引
            sprite.index = button.current_index();

            // 调试日志
            if was_hovered != hover.hovered {
                tracing::debug!("🖱️ 按钮悬停变化: {:?}, hovered={}", button.action, hover.hovered);
            }
        }
    }

    /// 处理按钮点击
    pub fn handle_click(world: &World, mouse_x: f32, mouse_y: f32) -> Option<ButtonAction> {
        for (_entity, (bounds, button, clickable)) in world.query::<(&Bounds, &Button, &Clickable)>().iter() {
            if clickable.enabled && button.enabled && bounds.contains(mouse_x, mouse_y) {
                tracing::info!("🔘 按钮被点击: {:?}", button.action);
                return Some(button.action);
            }
        }
        None
    }

    /// 启用/禁用按钮
    pub fn set_enabled(world: &mut World, action: ButtonAction, enabled: bool) {
        for (_entity, (mut button, mut clickable, mut sprite)) in world.query_mut::<(&mut Button, &mut Clickable, &mut Sprite)>() {
            if button.action == action {
                button.enabled = enabled;
                clickable.enabled = enabled;
                sprite.index = button.current_index();
                tracing::debug!("🔧 按钮状态更新: {:?}, enabled={}", action, enabled);
            }
        }
    }
}
