// ============================================================================
// 对话框系统（M9）
// 交互参考：Client/MirScenes/Dialogs/*.cs（原版 C#）
// 绘制参考：Client-Macroquad/src/scenes/dialogs/game/*.rs
// 框架：DialogManager 维护打开栈（z 序），每个对话框一个插件子模块
// ============================================================================

pub mod character;
pub mod inventory;
pub mod menu;
pub mod minimap;

use bevy::prelude::*;

/// 对话框类型
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum DialogKind {
    Inventory,
    Character,
    QuestLog,
    Option,
    Menu,
    GameShop,
    Minimap,
}

/// 对话框管理（打开栈，栈顶在最前）
#[derive(Resource, Default)]
pub struct DialogManager {
    pub open: Vec<DialogKind>,
}

impl DialogManager {
    pub fn is_open(&self, kind: DialogKind) -> bool {
        self.open.contains(&kind)
    }
    pub fn toggle(&mut self, kind: DialogKind) {
        if let Some(pos) = self.open.iter().position(|k| *k == kind) {
            self.open.remove(pos);
        } else {
            self.open.push(kind);
        }
    }
    pub fn close(&mut self, kind: DialogKind) {
        self.open.retain(|k| *k != kind);
    }
}

/// 对话框根标记（OnExit(Game) 统一清理）
#[derive(Component)]
pub struct DialogRoot(pub DialogKind);

pub struct DialogsPlugin;

impl Plugin for DialogsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogManager>();
        app.init_resource::<inventory::InventoryState>();
        app.init_resource::<character::CharacterState>();
        app.add_plugins((
            inventory::InventoryDialogPlugin,
            character::CharacterDialogPlugin,
            menu::MenuDialogPlugin,
            minimap::MiniMapPlugin,
        ));
    }
}
