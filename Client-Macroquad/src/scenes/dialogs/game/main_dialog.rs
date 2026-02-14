// ============================================================================
// MainDialog - 游戏主界面底部工具栏（纯 Hybrid 版本）
// ============================================================================
//
// 【功能说明】
// 1. 底部工具栏背景（根据分辨率适配）
// 2. 生命值/魔法值球显示
// 3. 经验条和负重条
// 4. 功能按钮组（背包、角色、技能、任务、选项、菜单、商城）
// 5. 角色信息显示（等级、金币、负重等）
//
// 【实现方式】
// - 使用 macroquad 原生绘制，完全移除 egui 依赖
// - 所有子对话框使用 hybrid 版本
//
// ============================================================================

use macroquad::prelude::*;
use super::{
    BeltItemHybrid,
    BeltDialogHybrid,
    CharacterDialogHybrid,
    CharacterTabHybrid,
    ChatControlBarHybrid,
    ChatDialogHybrid,
    ChatOptionDialogHybrid,
    GameShopDialog,
    InventoryDialogHybrid,
    ItemSlotHybrid,
    MenuDialogHybrid,
    MiniMapDialogHybrid,
    OptionDialogHybrid,
    QuestLogDialogHybrid,
};
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use std::time::{Duration, Instant};

/// 对话框类型枚举，用于 z-order 管理
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DialogType {
    Belt,
    Chat,
    ChatControlBar,
    ChatOption,
    Inventory,
    Character,
    QuestLog,
    Option,
    GameShop,
    Menu,
    MiniMap,
}

/// 主界面底部工具栏
pub struct MainDialog {
    /// 当前分辨率索引 (0=800, 1=1024, 2=1280+)
    resolution_index: usize,
    /// 模拟数据 - 当前生命值
    hp: i32,
    /// 模拟数据 - 最大生命值
    max_hp: i32,
    /// 模拟数据 - 当前魔法值
    mp: i32,
    /// 模拟数据 - 最大魔法值
    max_mp: i32,
    /// 模拟数据 - 经验值百分比
    exp_percent: f32,
    /// 模拟数据 - 等级
    level: u32,
    /// 模拟数据 - 角色名
    character_name: String,
    /// 模拟数据 - 金币
    gold: u32,
    /// 模拟数据 - 当前负重
    weight: u32,
    /// 模拟数据 - 最大负重
    max_weight: u32,
    /// 模拟数据 - 背包空格数
    bag_space: u32,

    // 子对话框（全部使用 Hybrid 版本）
    /// 快捷栏
    belt_dialog: BeltDialogHybrid,
    /// 聊天窗口
    chat_dialog: ChatDialogHybrid,
    /// 聊天控制栏
    chat_control_bar: ChatControlBarHybrid,
    /// 聊天设置/过滤
    chat_option_dialog: ChatOptionDialogHybrid,
    /// 背包
    inventory_dialog: InventoryDialogHybrid,
    /// 角色对话框
    character_dialog: CharacterDialogHybrid,
    /// 任务日志对话框
    quest_log_dialog: QuestLogDialogHybrid,
    /// 设置对话框
    option_dialog: OptionDialogHybrid,
    /// 游戏商城对话框
    game_shop_dialog: GameShopDialog,
    /// 菜单对话框
    menu_dialog: MenuDialogHybrid,
    /// 小地图对话框
    minimap_dialog: MiniMapDialogHybrid,

    // 对话框打开状态
    belt_dialog_open: bool,
    chat_dialog_open: bool,
    chat_control_bar_open: bool,
    chat_option_dialog_open: bool,
    inventory_dialog_open: bool,
    character_dialog_open: bool,
    quest_log_dialog_open: bool,
    option_dialog_open: bool,
    game_shop_dialog_open: bool,
    menu_dialog_open: bool,
    minimap_dialog_open: bool,

    /// 对话框 z-order 列表（从后到前，最后一个在最上层）
    dialog_z_order: Vec<DialogType>,

    /// 背景纹理
    bg_texture: Option<Texture2D>,
    /// 背景尺寸
    bg_size: Vec2,
    /// 位置
    position: Vec2,

    // === Resize/Layout ===
    /// 上一帧屏幕宽度（dpi 修正后的逻辑坐标）
    last_screen_w: f32,
    /// 上一帧屏幕高度（dpi 修正后的逻辑坐标）
    last_screen_h: f32,
    /// 上一帧 MainDialog 的 X（用于子对话框相对布局）
    last_main_dialog_x: f32,

    /// UI 产生的“自动寻路目标”（世界坐标像素）；由 GameScene 在 update 阶段消费
    pending_auto_path_target: Option<(f32, f32, bool)>,

    /// 小地图左键双击检测
    minimap_left_last_click_time: Option<Instant>,
    /// 小地图右键双击检测
    minimap_right_last_click_time: Option<Instant>,
    minimap_double_click_threshold: Duration,
}

impl MainDialog {
    pub fn new() -> Self {
        // 根据屏幕宽度决定分辨率索引
        let screen_width = screen_width();
        let screen_height = screen_height();
        let dpi_scale = screen_dpi_scale();

        let screen_w = screen_width / dpi_scale;
        let screen_h = screen_height / dpi_scale;

        let resolution_index = if screen_w <= 800.0 {
            0
        } else if screen_w <= 1024.0 {
            1
        } else {
            2
        };

        // MainDialog 的 X 坐标（底部居中）
        let bg_info = LibraryName::Prguse
            .get_size(resolution_index)
            .unwrap_or((1024, 150));
        let bg_width = bg_info.0 as f32;
        let bg_height = bg_info.1 as f32;
        let main_dialog_x = (screen_w - bg_width) / 2.0;

        Self {
            resolution_index,
            // 模拟数据
            hp: 850,
            max_hp: 1000,
            mp: 450,
            max_mp: 600,
            exp_percent: 0.65,
            level: 45,
            character_name: "测试角色".to_string(),
            gold: 123456,
            weight: 75,
            max_weight: 100,
            bag_space: 28,

            // 子对话框
            belt_dialog: BeltDialogHybrid::new(),
            chat_dialog: ChatDialogHybrid::new(main_dialog_x, screen_h, resolution_index),
            chat_control_bar: ChatControlBarHybrid::new(main_dialog_x, screen_h, resolution_index),
            chat_option_dialog: ChatOptionDialogHybrid::new(),
            inventory_dialog: InventoryDialogHybrid::new(),
            character_dialog: CharacterDialogHybrid::new(),
            quest_log_dialog: QuestLogDialogHybrid::new(),
            option_dialog: OptionDialogHybrid::new(),
            game_shop_dialog: GameShopDialog::new(),
            menu_dialog: MenuDialogHybrid::new(),
            minimap_dialog: MiniMapDialogHybrid::new(),

            // 对话框打开状态
            belt_dialog_open: true,
            chat_dialog_open: true,
            chat_control_bar_open: true,
            chat_option_dialog_open: false,
            inventory_dialog_open: false,
            character_dialog_open: false,
            quest_log_dialog_open: false,
            option_dialog_open: false,
            game_shop_dialog_open: false,
            menu_dialog_open: false,
            minimap_dialog_open: true,

            // 对话框 z-order（从后到前）
            dialog_z_order: vec![
                DialogType::Chat,
                DialogType::ChatControlBar,
                DialogType::ChatOption,
                DialogType::Belt,
                DialogType::MiniMap,
                DialogType::Inventory,
                DialogType::Character,
                DialogType::QuestLog,
                DialogType::Option,
                DialogType::GameShop,
                DialogType::Menu,
            ],

            bg_texture: None,
            bg_size: vec2(bg_width, bg_height),
            position: vec2(main_dialog_x, screen_h - bg_height),

            last_screen_w: screen_w,
            last_screen_h: screen_h,
            last_main_dialog_x: main_dialog_x,

            pending_auto_path_target: None,
            minimap_left_last_click_time: None,
            minimap_right_last_click_time: None,
            minimap_double_click_threshold: Duration::from_millis(260),
        }
    }

    /// 同步主面板生命/魔法显示（红/蓝球）。
    ///
    /// 说明：MainDialog 早期使用模拟字段；现在由渲染系统从 ECS 每帧推送真实值。
    pub fn set_vitals(&mut self, hp: i32, max_hp: i32, mp: i32, max_mp: i32) {
        self.hp = hp.max(0);
        self.max_hp = max_hp.max(1);
        self.mp = mp.max(0);
        self.max_mp = max_mp.max(1);
    }

    /// 设置小地图对应的地图尺寸（单位：地图格子数 width/height）
    ///
    /// 备注：小地图点击反算会先算出格子坐标，再用 `Coord::grid_to_world_center()` 转为世界像素。
    pub fn set_minimap_world_size(&mut self, grid_w: f32, grid_h: f32) {
        self.minimap_dialog.set_world_size(grid_w, grid_h);
    }

    /// 同步小地图上的玩家点（世界坐标像素 + 朝向弧度）
    pub fn update_minimap_player_position(&mut self, world_x: f32, world_y: f32, direction_rad: f32) {
        self.minimap_dialog
            .update_player_position(world_x, world_y, direction_rad);
    }

    /// 取出（并清空）一次 UI 产生的自动寻路目标
    pub fn take_pending_auto_path_target(&mut self) -> Option<(f32, f32, bool)> {
        self.pending_auto_path_target.take()
    }

    fn apply_resize_layout(&mut self, old_screen_w: f32, old_screen_h: f32, old_main_x: f32, new_screen_w: f32, new_screen_h: f32, new_main_x: f32) {
        // 只处理“窗口尺寸变化”导致的布局漂移；拖拽后的相对偏移应保留。
        // 这里使用“相对 MainDialog 的 X 偏移 + 相对屏幕底部的 Y 偏移”来迁移位置。
        let moved = (old_screen_w - new_screen_w).abs() > 0.5 || (old_screen_h - new_screen_h).abs() > 0.5;
        if !moved {
            return;
        }

        // Belt: 以 MainDialog 为锚点 + 底部距离保持
        {
            let old_pos = self.belt_dialog.get_position();
            let dx = old_pos.x - old_main_x;
            let bottom_gap = old_screen_h - old_pos.y;
            self.belt_dialog
                .set_position(vec2(new_main_x + dx, new_screen_h - bottom_gap));
        }

        // Chat: 以 MainDialog 为锚点 + 底部距离保持
        {
            let old_pos = self.chat_dialog.get_position();
            let dx = old_pos.x - old_main_x;
            let bottom_gap = old_screen_h - old_pos.y;
            self.chat_dialog
                .set_position(vec2(new_main_x + dx, new_screen_h - bottom_gap));
        }

        // ChatControlBar: 通常每帧会锚定到 ChatDialog 上方；这里先随 ChatDialog 做一次迁移，避免一帧错位。
        {
            let old_pos = self.chat_control_bar.get_position();
            let dx = old_pos.x - old_main_x;
            let bottom_gap = old_screen_h - old_pos.y;
            self.chat_control_bar
                .set_position(vec2(new_main_x + dx, new_screen_h - bottom_gap));
        }

        // MiniMap: 默认右上角；用“左边缘到右侧的距离”保持（无需访问对话框内部 size）
        {
            let old_pos = self.minimap_dialog.get_position();
            let right_gap_from_left = old_screen_w - old_pos.x;
            self.minimap_dialog
                .set_position(vec2(new_screen_w - right_gap_from_left, old_pos.y));
        }
    }

    /// 异步加载纹理
    pub fn load_native_textures(&mut self) {
        // 加载主背景纹理
        if let Some(texture) = LibraryName::Prguse.get_texture(self.resolution_index) {
            self.bg_size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 加载所有子对话框纹理
        self.belt_dialog.load_textures();
        self.chat_dialog.load_textures();
        self.chat_control_bar.load_textures();
        self.chat_option_dialog.load_textures();
        self.inventory_dialog.load_textures();
        self.character_dialog.load_textures();
        self.quest_log_dialog.load_textures();
        self.option_dialog.load_textures();
        self.game_shop_dialog.load_textures();
        self.menu_dialog.load_textures();
        self.minimap_dialog.load_textures();
        // 设置快捷栏初始位置
        let screen_h = screen_height() / screen_dpi_scale();
        let screen_w = screen_width() / screen_dpi_scale();
        let bg_info = LibraryName::Prguse.get_size(self.resolution_index).unwrap_or((1024, 150));
        let bg_width = bg_info.0 as f32;
        let main_dialog_x = (screen_w - bg_width) / 2.0;
        self.belt_dialog.set_position(vec2(main_dialog_x + 230.0, screen_h - 150.0));
    }

    /// 检查是否有任何输入框正在接收输入（用于判断是否应该消耗键盘输入）
    pub fn is_any_input_active(&self) -> bool {
        self.chat_dialog.is_input_active()
    }

    /// 激活聊天输入框（对应原版 Enter 打开输入）
    pub fn activate_chat_input(&mut self) {
        self.chat_dialog.activate_input();
    }

    /// 禁用聊天输入框（离开场景时确保 IME 关闭）
    pub fn deactivate_chat_input(&mut self) {
        self.chat_dialog.deactivate_input();
    }

    /// 往聊天窗口追加一条系统提示（用于网络/脚本调试输出）
    pub fn push_system_chat_line(&mut self, text: impl Into<String>) {
        self.chat_dialog
            .add_message(text, Color::from_rgba(100, 150, 255, 255));
    }

    /// 往聊天窗口追加一条普通消息
    pub fn push_chat_line(&mut self, text: impl Into<String>) {
        self.chat_dialog
            .add_message(text, Color::from_rgba(100, 150, 255, 255));
    }

    /// 是否有任何“弹窗类”对话框打开（用于 ESC 逻辑）
    /// 说明：不包含 Belt/Chat/ChatControlBar/MiniMap 这些常驻 UI。
    pub fn any_popup_open(&self) -> bool {
        self.inventory_dialog_open
            || self.character_dialog_open
            || self.quest_log_dialog_open
            || self.option_dialog_open
            || self.game_shop_dialog_open
            || self.menu_dialog_open
            || self.chat_option_dialog_open
    }

    /// 关闭所有“弹窗类”对话框（用于 ESC 一键收起）
    pub fn close_all_popups(&mut self) {
        self.inventory_dialog_open = false;
        self.character_dialog_open = false;
        self.quest_log_dialog_open = false;
        self.option_dialog_open = false;
        self.game_shop_dialog_open = false;
        self.menu_dialog_open = false;
        self.chat_option_dialog_open = false;
    }

    /// 打开背包对话框（用于对齐原版：打开 NPC 商店时自动弹出背包）
    pub fn open_inventory(&mut self) {
        self.inventory_dialog_open = true;
        self.bring_to_front(DialogType::Inventory);
    }

    /// 切换小地图显示（快捷键M）
    pub fn toggle_minimap(&mut self) {
        self.minimap_dialog_open = !self.minimap_dialog_open;
    }

    /// 切换小地图大小模式（快捷键TAB）
    pub fn toggle_minimap_size(&mut self) {
        self.minimap_dialog.toggle_size();
    }

    /// 更新和绘制主界面
    pub fn update_and_draw(&mut self) {
        let screen_w = screen_width() / screen_dpi_scale();
        let screen_h = screen_height() / screen_dpi_scale();

        // 先计算新的 MainDialog 位置，再基于“旧布局”迁移子对话框
        let old_screen_w = self.last_screen_w;
        let old_screen_h = self.last_screen_h;
        let old_main_x = self.last_main_dialog_x;

        self.position = vec2((screen_w - self.bg_size.x) / 2.0, screen_h - self.bg_size.y);
        let new_main_x = self.position.x;

        self.apply_resize_layout(old_screen_w, old_screen_h, old_main_x, screen_w, screen_h, new_main_x);

        self.last_screen_w = screen_w;
        self.last_screen_h = screen_h;
        self.last_main_dialog_x = new_main_x;

        // 绘制主背景
        self.draw_background();

        // 绘制生命值/魔法值球
        self.draw_health_mana_orbs();

        // 绘制经验条
        self.draw_exp_bar();

        // 绘制负重条
        self.draw_weight_bar();

        // 绘制角色信息
        self.draw_character_info();

        // 绘制功能按钮组
        self.draw_buttons();
    }

    /// 显示所有子对话框
    /// 返回 true 表示UI消耗了鼠标事件
    pub fn show_dialogs(&mut self) -> bool {
        let mut consumed = false;
        let (mx, my) = mouse_position();
        let mouse_pos = vec2(mx, my);
        let left_clicked = is_mouse_button_pressed(MouseButton::Left);
        let right_clicked = is_mouse_button_pressed(MouseButton::Right);
        let any_clicked = left_clicked || right_clicked;

        // 检测哪个对话框被点击，用于置顶
        // 从 z-order 最高（最上层）开始检测，找到第一个被点击的对话框
        let mut clicked_dialog: Option<DialogType> = None;
        
        if any_clicked {
            for dialog_type in self.dialog_z_order.iter().rev() {
                let (is_open, contains) = self.check_dialog_contains(*dialog_type, mouse_pos);
                if is_open && contains {
                    clicked_dialog = Some(*dialog_type);
                    break;
                }
            }
        }

        // 如果有对话框被点击，将其移到最前面
        if let Some(dialog_type) = clicked_dialog {
            self.bring_to_front(dialog_type);

            // 小地图点击：在地图区域内则触发自动寻路（由 GameScene 消费）
            if dialog_type == DialogType::MiniMap {
                if left_clicked {
                    // 左键：双击触发“奔跑寻路”，单击只记录时间
                    if let Some((wx, wy)) = self.minimap_dialog.pick_world_target_from_mouse(mouse_pos) {
                        let now = Instant::now();
                        let is_double = self
                            .minimap_left_last_click_time
                            .is_some_and(|t| now.duration_since(t) <= self.minimap_double_click_threshold);

                        if is_double {
                            self.pending_auto_path_target = Some((wx, wy, true));
                            self.minimap_left_last_click_time = None;
                            consumed = true;
                        } else {
                            self.minimap_left_last_click_time = Some(now);
                        }
                    }
                }

                if right_clicked {
                    // 右键：仅双击触发“奔跑寻路”，单击只记录时间
                    if let Some((wx, wy)) = self.minimap_dialog.pick_world_target_from_mouse(mouse_pos) {
                        let now = Instant::now();
                        let is_double = self
                            .minimap_right_last_click_time
                            .is_some_and(|t| now.duration_since(t) <= self.minimap_double_click_threshold);

                        if is_double {
                            self.pending_auto_path_target = Some((wx, wy, true));
                            self.minimap_right_last_click_time = None;
                            consumed = true;
                        } else {
                            self.minimap_right_last_click_time = Some(now);
                        }
                    }
                }
            }
        }

        // 按 z-order 顺序绘制所有对话框（从后到前）
        for dialog_type in self.dialog_z_order.clone().iter() {
            match dialog_type {
                DialogType::Belt => self.sync_and_draw_belt(&mut consumed, mouse_pos),
                DialogType::Chat => self.sync_and_draw_chat(&mut consumed, mouse_pos),
                DialogType::ChatControlBar => self.sync_and_draw_chat_control_bar(&mut consumed, mouse_pos),
                DialogType::ChatOption => self.sync_and_draw_chat_option(&mut consumed, mouse_pos),
                DialogType::Inventory => self.sync_and_draw_inventory(&mut consumed, mouse_pos),
                DialogType::Character => self.sync_and_draw_character(&mut consumed, mouse_pos),
                DialogType::QuestLog => self.sync_and_draw_quest_log(&mut consumed, mouse_pos),
                DialogType::Option => self.sync_and_draw_option(&mut consumed, mouse_pos),
                DialogType::GameShop => self.sync_and_draw_shop(&mut consumed, mouse_pos),
                DialogType::Menu => self.sync_and_draw_menu(&mut consumed, mouse_pos),
                DialogType::MiniMap => self.sync_and_draw_minimap(&mut consumed, mouse_pos),
            }
        }
        self.process_inventory_belt_interop();

        consumed
    }

    /// 当前鼠标位置是否位于 UI 区域之上（用于屏蔽游戏内点击/移动）
    ///
    /// 说明：这是“命中检测”接口，不会触发绘制。
    pub fn is_mouse_over_ui(&self, mouse_pos: Vec2) -> bool {
        // 主底部界面背景区域
        let main_rect = Rect::new(self.position.x, self.position.y, self.bg_size.x, self.bg_size.y);
        if mouse_pos.x >= main_rect.x
            && mouse_pos.x <= main_rect.x + main_rect.w
            && mouse_pos.y >= main_rect.y
            && mouse_pos.y <= main_rect.y + main_rect.h
        {
            return true;
        }

        // 子对话框区域（按 z-order 从上到下检测）
        for dialog_type in self.dialog_z_order.iter().rev() {
            let (is_open, contains) = self.check_dialog_contains(*dialog_type, mouse_pos);
            if is_open && contains {
                return true;
            }
        }

        false
    }

    /// 检查指定对话框是否打开且包含指定位置
    fn check_dialog_contains(&self, dialog_type: DialogType, mouse_pos: Vec2) -> (bool, bool) {
        match dialog_type {
            DialogType::Belt => (self.belt_dialog_open, self.belt_dialog.contains(mouse_pos)),
            DialogType::Chat => (self.chat_dialog_open, self.chat_dialog.contains(mouse_pos)),
            DialogType::ChatControlBar => (self.chat_control_bar_open, self.chat_control_bar.contains(mouse_pos)),
            DialogType::ChatOption => (
                self.chat_option_dialog_open,
                self.chat_option_dialog.contains(mouse_pos),
            ),
            DialogType::Inventory => (self.inventory_dialog_open, self.inventory_dialog.contains(mouse_pos)),
            DialogType::Character => (self.character_dialog_open, self.character_dialog.contains(mouse_pos)),
            DialogType::QuestLog => (self.quest_log_dialog_open, self.quest_log_dialog.contains(mouse_pos)),
            DialogType::Option => (self.option_dialog_open, self.option_dialog.contains(mouse_pos)),
            DialogType::GameShop => (self.game_shop_dialog_open, self.game_shop_dialog.contains(mouse_pos)),
            DialogType::Menu => (self.menu_dialog_open, self.menu_dialog.contains(mouse_pos)),
            DialogType::MiniMap => (self.minimap_dialog_open, self.minimap_dialog.contains(mouse_pos)),
        }
    }

    /// 将指定对话框移到最前面（z-order 最高）
    fn bring_to_front(&mut self, dialog_type: DialogType) {
        if let Some(pos) = self.dialog_z_order.iter().position(|&d| d == dialog_type) {
            // 如果已经在最前面，不需要移动
            if pos == self.dialog_z_order.len() - 1 {
                return;
            }
            self.dialog_z_order.remove(pos);
            self.dialog_z_order.push(dialog_type);
        }
    }

    // ========================================================================
    // 私有绘制方法
    // ========================================================================

    /// 绘制主背景
    fn draw_background(&self) {
        if let Some(ref texture) = self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        } else {
            // 降级：绘制临时背景
            draw_rectangle(
                self.position.x,
                self.position.y,
                self.bg_size.x,
                self.bg_size.y,
                Color::from_rgba(30, 30, 40, 240),
            );
        }
    }

    /// 绘制生命值/魔法值球
    fn draw_health_mana_orbs(&self) {
        // 根据C#源码：HealthOrb.Location = (0, 30)，orb高度80px
        let orb_x = self.position.x;
        let orb_y = self.position.y + 30.0;
        let orb_height = 80.0;

        // 计算百分比
        let hp_percent = (self.hp as f32 / self.max_hp as f32).clamp(0.0, 1.0);
        let mp_percent = (self.mp as f32 / self.max_mp as f32).clamp(0.0, 1.0);

        // 绘制球纹理 Prguse[4] - 填充球
        if let Some(texture) = LibraryName::Prguse.get_texture(4) {
            if let Some(ref tex) = texture.image {
                // HP 球（左半部分，红色）
                // C#: height = (int)(80 * User.HP / (float)User.Stats[Stat.HP])
                // C#: r = new Rectangle(0, 80 - height, 50, height)
                // C#: Draw位置 = (orb_x, orb_y + 80 - height)
                let hp_height = orb_height * hp_percent;
                let hp_src_y = orb_height - hp_height;

                draw_texture_ex(
                    tex,
                    orb_x,
                    orb_y + hp_src_y,
                    WHITE,
                    DrawTextureParams {
                        source: Some(Rect::new(0.0, hp_src_y, 50.0, hp_height)),
                        ..Default::default()
                    },
                );

                // MP 球（右半部分，蓝色）
                // C#: r = new Rectangle(51, 80 - height, 50, height)
                // C#: Draw位置 = (orb_x + 51, orb_y + 80 - height)
                let mp_height = orb_height * mp_percent;
                let mp_src_y = orb_height - mp_height;

                draw_texture_ex(
                    tex,
                    orb_x + 51.0,
                    orb_y + mp_src_y,
                    WHITE,
                    DrawTextureParams {
                        source: Some(Rect::new(51.0, mp_src_y, 50.0, mp_height)),
                        ..Default::default()
                    },
                );
            }
        } else {
            // 降级绘制
            // HP球
            draw_rectangle(orb_x, orb_y + orb_height * (1.0 - hp_percent), 50.0, orb_height * hp_percent, RED);
            // MP球 (X偏移51)
            draw_rectangle(orb_x + 51.0, orb_y + orb_height * (1.0 - mp_percent), 50.0, orb_height * mp_percent, BLUE);
        }

        // 注意：球框已经是 MainDialog 背景纹理 (Prguse[0/1/2]) 的一部分，不需要单独绘制

        // 绘制数值文字 - 使用 TopLabel/BottomLabel 样式（居中显示）
        // C#: TopLabel.Location = (9, 20) 相对于 HealthOrb, Size = (85, 30)
        // C#: BottomLabel.Location = (9, 50) 相对于 HealthOrb, Size = (85, 30)
        let label_width = 85.0;
        
        // HP 显示在上方
        let hp_text = format!("{}", self.hp);
        let hp_text_width = hp_text.len() as f32 * 6.0; // 估算文本宽度
        draw_text_cn(&hp_text, orb_x + 9.0 + (label_width - hp_text_width) / 2.0, orb_y + 20.0 + 12.0, 11.0, WHITE);
        
        // 分隔线
        draw_text_cn("--", orb_x + 9.0 + label_width / 2.0 - 6.0, orb_y + 35.0, 11.0, WHITE);
        
        // MaxHP 显示在下方
        let max_hp_text = format!("{}", self.max_hp);
        let max_hp_text_width = max_hp_text.len() as f32 * 6.0;
        draw_text_cn(&max_hp_text, orb_x + 9.0 + (label_width - max_hp_text_width) / 2.0, orb_y + 50.0 + 12.0, 11.0, WHITE);
    }

    /// 绘制经验条
    fn draw_exp_bar(&self) {
        // C#: ExperienceBar.Location = new Point(9, 143)
        let bar_x = self.position.x + 9.0;
        let bar_y = self.position.y + 143.0;

        // 根据分辨率选择纹理索引 (800用7，其他用8)
        let exp_texture_idx = if self.resolution_index == 0 { 7 } else { 8 };

        if let Some(texture) = LibraryName::Prguse.get_texture(exp_texture_idx) {
            if let Some(ref tex) = texture.image {
                let bar_width = texture.width as f32 - 3.0;
                let fill_width = bar_width * self.exp_percent;

                draw_texture_ex(
                    tex,
                    bar_x,
                    bar_y,
                    WHITE,
                    DrawTextureParams {
                        source: Some(Rect::new(0.0, 0.0, fill_width, texture.height as f32)),
                        ..Default::default()
                    },
                );
            }
        } else {
            // 降级绘制
            let bar_width = 100.0;
            draw_rectangle(bar_x, bar_y, bar_width, 5.0, Color::from_rgba(40, 40, 40, 255));
            draw_rectangle(bar_x, bar_y, bar_width * self.exp_percent, 5.0, Color::from_rgba(255, 215, 0, 255));
        }

        // 经验百分比文字
        // C#: ExperienceLabel.Location = ((ExperienceBar.Size.Width / 2) - 20, -10) 相对于 ExperienceBar
        let bar_width = if let Some(texture) = LibraryName::Prguse.get_texture(exp_texture_idx) {
            texture.width as f32
        } else {
            100.0
        };
        let exp_text = format!("{:.1}%", self.exp_percent * 100.0);
        draw_text_cn(&exp_text, bar_x + bar_width / 2.0 - 20.0, bar_y - 10.0 + 10.0, 9.0, WHITE);
    }

    /// 绘制负重条
    fn draw_weight_bar(&self) {
        // C#: WeightBar.Location = new Point(this.Size.Width - 105, 103)
        let bar_x = self.position.x + self.bg_size.x - 105.0;
        let bar_y = self.position.y + 103.0;

        let weight_percent = (self.weight as f32 / self.max_weight as f32).clamp(0.0, 1.0);

        if let Some(texture) = LibraryName::Prguse.get_texture(76) {
            if let Some(ref tex) = texture.image {
                let bar_width = texture.width as f32 - 2.0;
                let fill_width = bar_width * weight_percent;

                // 根据负重比例选择颜色
                let color = if weight_percent < 0.8 {
                    WHITE
                } else if weight_percent < 1.0 {
                    YELLOW
                } else {
                    Color::from_rgba(255, 100, 100, 255)
                };

                draw_texture_ex(
                    tex,
                    bar_x,
                    bar_y,
                    color,
                    DrawTextureParams {
                        source: Some(Rect::new(0.0, 0.0, fill_width, texture.height as f32)),
                        ..Default::default()
                    },
                );
            }
        } else {
            // 降级绘制
            let bar_width = 80.0;
            draw_rectangle(bar_x, bar_y, bar_width, 5.0, Color::from_rgba(40, 40, 40, 255));
            let color = if weight_percent < 0.8 { GREEN } else if weight_percent < 1.0 { YELLOW } else { RED };
            draw_rectangle(bar_x, bar_y, bar_width * weight_percent, 5.0, color);
        }

        // 负重文字
        // C#: WeightLabel.Location = (Size.Width - 105, 101)
        // C#: SpaceLabel.Location = (Size.Width - 30, 101)
        let weight_label_y = self.position.y + 101.0;
        let weight_text = format!("{}/{}", self.weight, self.max_weight);
        draw_text_cn(&weight_text, bar_x, weight_label_y + 10.0, 9.0, WHITE);
        
        // 背包空格 - 显示在右侧
        let space_x = self.position.x + self.bg_size.x - 30.0;
        let space_text = format!("{}", self.bag_space);
        draw_text_cn(&space_text, space_x, weight_label_y + 10.0, 9.0, WHITE);
    }

    /// 绘制角色信息
    fn draw_character_info(&self) {
        // C#: LevelLabel.Location = new Point(5, 108)
        let level_x = self.position.x + 5.0;
        let level_y = self.position.y + 108.0;

        // C#: CharacterName.Location = new Point(6, 120), Size = (90, 16) 居中
        let name_x = self.position.x + 6.0;
        let name_y = self.position.y + 120.0;
        let name_width = 90.0;

        // C#: GoldLabel.Location = new Point(this.Size.Width - 105, 119), Size = (99, 13)
        let gold_x = self.position.x + self.bg_size.x - 105.0;
        let gold_y = self.position.y + 119.0;

        // 等级
        let level_text = format!("{}", self.level);
        draw_text_cn(&level_text, level_x, level_y + 10.0, 11.0, WHITE);

        // 角色名（居中显示）
        let name_text_width = self.character_name.chars().count() as f32 * 8.0;
        let name_center_x = name_x + (name_width - name_text_width) / 2.0;
        draw_text_cn(&self.character_name, name_center_x, name_y + 12.0, 11.0, Color::from_rgba(255, 215, 0, 255));

        // 金币
        let gold_text = format!("{}", self.gold);
        draw_text_cn(&gold_text, gold_x, gold_y + 10.0, 9.0, Color::from_rgba(255, 215, 0, 255));
    }

    /// 绘制功能按钮组
    fn draw_buttons(&mut self) {
        let button_y = self.position.y + 76.0;
        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 按钮列表：按C#源码的精确位置
        // C#: CharacterButton = Size.Width - 119
        // C#: InventoryButton = Size.Width - 96
        // C#: SkillButton = Size.Width - 73
        // C#: QuestButton = Size.Width - 50
        // C#: OptionButton = Size.Width - 27
        // 纹理索引：(正常, 悬停, 按下, 提示, X偏移)
        let buttons: [(usize, usize, usize, &str, f32); 5] = [
            (1900, 1901, 1902, "角色", self.bg_size.x - 119.0),
            (1903, 1904, 1905, "背包", self.bg_size.x - 96.0),
            (1906, 1907, 1908, "技能", self.bg_size.x - 73.0),
            (1909, 1910, 1911, "任务", self.bg_size.x - 50.0),
            (1912, 1913, 1914, "选项", self.bg_size.x - 27.0),
        ];

        for (normal_idx, hover_idx, pressed_idx, hint, x_offset) in buttons {
            let btn_x = self.position.x + x_offset;
            if self.draw_button(mouse_pos, btn_x, button_y, normal_idx, hover_idx, pressed_idx) {
                println!("🖱️ 点击了 {} 按钮", hint);
                match hint {
                    "背包" => {
                        self.inventory_dialog_open = !self.inventory_dialog_open;
                        if self.inventory_dialog_open {
                            self.bring_to_front(DialogType::Inventory);
                        }
                    }
                    "角色" => {
                        self.character_dialog_open = !self.character_dialog_open;
                        if self.character_dialog_open {
                            self.bring_to_front(DialogType::Character);
                        }
                    }
                    "技能" => {
                        // 如果角色对话框已打开且在技能页，则关闭；否则打开并切换到技能页
                        if self.character_dialog_open && self.character_dialog.is_skills_tab() {
                            self.character_dialog_open = false;
                        } else {
                            self.character_dialog_open = true;
                            self.character_dialog.switch_tab(CharacterTabHybrid::Skills);
                            self.bring_to_front(DialogType::Character);
                        }
                    }
                    "任务" => {
                        self.quest_log_dialog_open = !self.quest_log_dialog_open;
                        if self.quest_log_dialog_open {
                            self.bring_to_front(DialogType::QuestLog);
                        }
                    }
                    "选项" => {
                        self.option_dialog_open = !self.option_dialog_open;
                        if self.option_dialog_open {
                            self.bring_to_front(DialogType::Option);
                        }
                    }
                    _ => {}
                }
            }
        }

        // 菜单按钮 C#: Size.Width - 55, 35
        let menu_x = self.position.x + self.bg_size.x - 55.0;
        let menu_y = self.position.y + 35.0;
        if self.draw_button(mouse_pos, menu_x, menu_y, 1960, 1961, 1962) {
            println!("🖱️ 点击了菜单按钮");
            self.menu_dialog_open = !self.menu_dialog_open;
            if self.menu_dialog_open {
                self.bring_to_front(DialogType::Menu);
            }
        }

        // 商城按钮 C#: Size.Width - 105, 35
        let shop_x = self.position.x + self.bg_size.x - 105.0;
        let shop_y = self.position.y + 35.0;
        if self.draw_button(mouse_pos, shop_x, shop_y, 826, 827, 828) {
            println!("🖱️ 点击了商城按钮");
            self.game_shop_dialog_open = !self.game_shop_dialog_open;
            if self.game_shop_dialog_open {
                self.bring_to_front(DialogType::GameShop);
            }
        }
    }

    /// 绘制单个按钮（返回是否被点击）
    fn draw_button(&self, mouse_pos: Vec2, x: f32, y: f32, normal_idx: usize, hover_idx: usize, pressed_idx: usize) -> bool {
        let btn_size = if let Some(texture) = LibraryName::Prguse.get_texture(normal_idx) {
            vec2(texture.width as f32, texture.height as f32)
        } else {
            vec2(20.0, 20.0)
        };

        let btn_rect = Rect::new(x, y, btn_size.x, btn_size.y);
        let is_hovered = btn_rect.contains(mouse_pos);
        let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

        let texture_idx = if is_pressed {
            pressed_idx
        } else if is_hovered {
            hover_idx
        } else {
            normal_idx
        };

        if let Some(texture) = LibraryName::Prguse.get_texture(texture_idx) {
            if let Some(ref tex) = texture.image {
                draw_texture_ex(tex, x, y, WHITE, DrawTextureParams::default());
            }
        } else {
            // 降级绘制
            let color = if is_pressed {
                Color::from_rgba(100, 100, 150, 255)
            } else if is_hovered {
                Color::from_rgba(80, 80, 100, 255)
            } else {
                Color::from_rgba(60, 60, 70, 255)
            };
            draw_rectangle(x, y, btn_size.x, btn_size.y, color);
        }

        is_hovered && is_mouse_button_pressed(MouseButton::Left)
    }

    // ========================================================================
    // 对话框同步和绘制辅助方法
    // ========================================================================

    fn sync_and_draw_belt(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.belt_dialog_open {
            self.belt_dialog.open();
        } else {
            self.belt_dialog.close();
        }

        // ChatDialog 缩放/拖动会带动 ChatControlBar 位置变化，先约束一次避免 1 帧重叠
        self.clamp_belt_above_chat_control_bar();

        self.belt_dialog.update_and_draw();

        // BeltDialog 可能被拖动/切换布局导致尺寸变化，再约束一次保证立即生效
        self.clamp_belt_above_chat_control_bar();

        if !self.belt_dialog.is_visible() {
            self.belt_dialog_open = false;
        }
        if self.belt_dialog_open && self.belt_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn sync_and_draw_inventory(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.inventory_dialog_open {
            self.inventory_dialog.open();
        } else {
            self.inventory_dialog.close();
        }
        self.inventory_dialog.update_and_draw();
        if !self.inventory_dialog.is_visible() {
            self.inventory_dialog_open = false;
        }
        if self.inventory_dialog_open && self.inventory_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn process_inventory_belt_interop(&mut self) {
        if let Some((tab, slot)) = self.inventory_dialog.take_transfer_to_belt_request() {
            if let Some(item) = self.inventory_dialog.take_item_from_slot(tab, slot) {
                if let Some(icon_index) = item.icon_index {
                    let belt_item = BeltItemHybrid::new(icon_index, item.count);
                    if let Err(rollback_item) = self.belt_dialog.try_insert_item(belt_item) {
                        if !self.inventory_dialog.restore_item_to_slot(
                            tab,
                            slot,
                            ItemSlotHybrid::new(rollback_item.icon_index, rollback_item.count),
                        ) {
                            eprintln!(
                                "⚠️ Inventory rollback failed: tab={tab:?}, slot={slot}, icon={}, count={}",
                                rollback_item.icon_index,
                                rollback_item.count
                            );
                        }
                    }
                }
            }
        }

        if let Some(slot) = self.belt_dialog.take_transfer_to_inventory_request() {
            if let Some(item) = self.belt_dialog.take_item_from_slot(slot) {
                let inventory_item = ItemSlotHybrid::new(item.icon_index, item.count);
                if let Err(rollback_item) = self.inventory_dialog.try_insert_item(inventory_item) {
                    if let Some(icon_index) = rollback_item.icon_index {
                        if !self.belt_dialog.restore_item_to_slot(
                            slot,
                            BeltItemHybrid::new(icon_index, rollback_item.count),
                        ) {
                            eprintln!(
                                "⚠️ Belt rollback failed: slot={slot}, icon={icon_index}, count={}",
                                rollback_item.count
                            );
                        }
                    }
                }
            }
        }
    }

    fn sync_and_draw_character(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.character_dialog_open {
            self.character_dialog.open();
        } else {
            self.character_dialog.close();
        }
        self.character_dialog.update_and_draw();
        if !self.character_dialog.is_visible() {
            self.character_dialog_open = false;
        }
        if self.character_dialog_open && self.character_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn sync_and_draw_shop(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.game_shop_dialog_open {
            self.game_shop_dialog.open();
        } else {
            self.game_shop_dialog.close();
        }
        self.game_shop_dialog.update_and_draw();
        if !self.game_shop_dialog.is_visible() {
            self.game_shop_dialog_open = false;
        }
        if self.game_shop_dialog_open && self.game_shop_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn sync_and_draw_menu(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.menu_dialog_open {
            self.menu_dialog.open();
        } else {
            self.menu_dialog.close();
        }
        if let Some(action) = self.menu_dialog.update_and_draw() {
            self.menu_dialog.handle_action(action);
        }
        if !self.menu_dialog.is_visible() {
            self.menu_dialog_open = false;
        }
        if self.menu_dialog_open && self.menu_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn sync_and_draw_minimap(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.minimap_dialog_open {
            self.minimap_dialog.open();
        } else {
            self.minimap_dialog.close();
        }
        self.minimap_dialog.update_and_draw();
        if !self.minimap_dialog.is_visible() {
            self.minimap_dialog_open = false;
        }
        if self.minimap_dialog_open && self.minimap_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn sync_and_draw_option(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.option_dialog_open {
            self.option_dialog.open();
        } else {
            self.option_dialog.close();
        }
        self.option_dialog.update_and_draw();
        if !self.option_dialog.is_visible() {
            self.option_dialog_open = false;
        }
        if self.option_dialog_open && self.option_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn sync_and_draw_chat(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.chat_dialog_open {
            self.chat_dialog.open();
        } else {
            self.chat_dialog.close();
        }
        self.chat_dialog.update_and_draw();

        // ChatDialog 可能被拖动/缩放，确保 ChatControlBar 始终在其上方
        self.sync_chat_control_bar_position();
        if !self.chat_dialog.is_visible() {
            self.chat_dialog_open = false;
        }
        if self.chat_dialog_open && self.chat_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn sync_chat_control_bar_position(&mut self) {
        if !self.chat_control_bar_open {
            return;
        }

        // 即使 ChatDialog 被隐藏，这里也不会强制隐藏控制栏；只负责位置锚定
        let chat_pos = self.chat_dialog.get_position();
        let bar_h = self.chat_control_bar.get_size().y;
        self.chat_control_bar
            .set_position(vec2(chat_pos.x, chat_pos.y - bar_h));
    }

    fn clamp_belt_above_chat_control_bar(&mut self) {
        if !self.belt_dialog_open || !self.chat_control_bar_open {
            return;
        }

        if !self.belt_dialog.is_visible() || !self.chat_control_bar.is_visible() {
            return;
        }

        // 对齐 C#：仅水平 Belt(Index=1932) 才跟随 ChatControlBar；垂直 Belt(Index=1944) 使用自身固定位置
        if !self.belt_dialog.is_horizontal_layout() {
            return;
        }

        let bar_top_y = self.chat_control_bar.get_position().y;
        let belt_pos = self.belt_dialog.get_position();
        let belt_size = self.belt_dialog.get_size();

        // 需求：始终贴紧在 ChatControlBar 上方
        self.belt_dialog
            .set_position(vec2(belt_pos.x, bar_top_y - belt_size.y));
    }

    fn sync_and_draw_chat_control_bar(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.chat_control_bar_open {
            self.chat_control_bar.open();
        } else {
            self.chat_control_bar.close();
        }

        // 每帧锚定位置：保证 ChatDialog 缩放/拖动后控制栏不“掉队”
        self.sync_chat_control_bar_position();

        let (size_clicked, settings_clicked) = self.chat_control_bar.update_and_draw();

        // 对齐 C#：ChatControlBar 负责驱动 ChatDialog.ChatPrefix
        self.chat_dialog
            .set_chat_prefix(self.chat_control_bar.get_chat_prefix());

        // 对齐 C#：SettingsButton 切换 ChatOptionDialog
        if settings_clicked {
            self.chat_option_dialog_open = !self.chat_option_dialog_open;
            if self.chat_option_dialog_open {
                self.chat_option_dialog.open();
                self.bring_to_front(DialogType::ChatOption);
            } else {
                self.chat_option_dialog.close();
            }
        }

        // 如果 Size 按钮被点击，改变 ChatDialog 大小
        if size_clicked {
            let screen_h = screen_height() / screen_dpi_scale();
            self.chat_dialog.change_size(screen_h);

            // 同步更新 ChatControlBar 位置
            self.sync_chat_control_bar_position();
        }

        // update_and_draw 过程中尺寸可能刷新，再同步一次避免高度变化造成 1 帧错位
        self.sync_chat_control_bar_position();

        if !self.chat_control_bar.is_visible() {
            self.chat_control_bar_open = false;
        }
        if self.chat_control_bar_open && self.chat_control_bar.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn sync_and_draw_chat_option(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.chat_option_dialog_open {
            self.chat_option_dialog.open();
        } else {
            self.chat_option_dialog.close();
        }

        // 更新与绘制
        let changed = self.chat_option_dialog.update_and_draw();
        if !self.chat_option_dialog.is_visible() {
            self.chat_option_dialog_open = false;
        }

        // 同步设置到 ChatDialog（目前先同步透明聊天；过滤项留作后续接入消息类型）
        if changed || self.chat_option_dialog_open {
            let settings = self.chat_option_dialog.get_settings();
            self.chat_dialog.apply_chat_option_settings(settings);
        }

        if self.chat_option_dialog_open && self.chat_option_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn sync_and_draw_quest_log(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.quest_log_dialog_open {
            self.quest_log_dialog.open();
        } else {
            self.quest_log_dialog.close();
        }
        self.quest_log_dialog.update_and_draw();
        if !self.quest_log_dialog.is_visible() {
            self.quest_log_dialog_open = false;
        }
        if self.quest_log_dialog_open && self.quest_log_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }
}
