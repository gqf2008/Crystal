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
    BeltDialogHybrid,
    CharacterDialogHybrid,
    CharacterTabHybrid,
    ChatControlBarHybrid,
    ChatDialogHybrid,
    GameShopDialogHybrid,
    InventoryDialogHybrid,
    MenuDialogHybrid,
    MiniMapDialogHybrid,
    OptionDialogHybrid,
    QuestLogDialogHybrid,
};
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;

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
    /// 背包
    inventory_dialog: InventoryDialogHybrid,
    /// 角色对话框
    character_dialog: CharacterDialogHybrid,
    /// 任务日志对话框
    quest_log_dialog: QuestLogDialogHybrid,
    /// 设置对话框
    option_dialog: OptionDialogHybrid,
    /// 游戏商城对话框
    game_shop_dialog: GameShopDialogHybrid,
    /// 菜单对话框
    menu_dialog: MenuDialogHybrid,
    /// 小地图对话框
    minimap_dialog: MiniMapDialogHybrid,

    // 对话框打开状态
    belt_dialog_open: bool,
    chat_dialog_open: bool,
    chat_control_bar_open: bool,
    inventory_dialog_open: bool,
    character_dialog_open: bool,
    quest_log_dialog_open: bool,
    option_dialog_open: bool,
    game_shop_dialog_open: bool,
    menu_dialog_open: bool,
    minimap_dialog_open: bool,

    /// 背景纹理
    bg_texture: Option<Texture2D>,
    /// 背景尺寸
    bg_size: Vec2,
    /// 位置
    position: Vec2,
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
            inventory_dialog: InventoryDialogHybrid::new(),
            character_dialog: CharacterDialogHybrid::new(),
            quest_log_dialog: QuestLogDialogHybrid::new(),
            option_dialog: OptionDialogHybrid::new(),
            game_shop_dialog: GameShopDialogHybrid::new(),
            menu_dialog: MenuDialogHybrid::new(),
            minimap_dialog: MiniMapDialogHybrid::new(),

            // 对话框打开状态
            belt_dialog_open: true,
            chat_dialog_open: true,
            chat_control_bar_open: true,
            inventory_dialog_open: false,
            character_dialog_open: false,
            quest_log_dialog_open: false,
            option_dialog_open: false,
            game_shop_dialog_open: false,
            menu_dialog_open: false,
            minimap_dialog_open: true,

            bg_texture: None,
            bg_size: vec2(bg_width, bg_height),
            position: vec2(main_dialog_x, screen_h - bg_height),
        }
    }

    /// 异步加载纹理
    pub async fn load_native_textures(&mut self) {
        // 加载主背景纹理
        if let Some(texture) = LibraryName::Prguse.get_texture(self.resolution_index) {
            self.bg_size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 加载所有子对话框纹理
        self.belt_dialog.load_textures().await;
        self.chat_dialog.load_textures().await;
        self.chat_control_bar.load_textures().await;
        self.inventory_dialog.load_textures().await;
        self.character_dialog.load_textures().await;
        self.quest_log_dialog.load_textures().await;
        self.option_dialog.load_textures().await;
        self.game_shop_dialog.load_textures().await;
        self.menu_dialog.load_textures().await;
        self.minimap_dialog.load_textures().await;

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

        // 更新位置
        self.position = vec2((screen_w - self.bg_size.x) / 2.0, screen_h - self.bg_size.y);

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

        // 快捷栏
        self.sync_and_draw_belt(&mut consumed, mouse_pos);

        // 背包
        self.sync_and_draw_inventory(&mut consumed, mouse_pos);

        // 角色
        self.sync_and_draw_character(&mut consumed, mouse_pos);

        // 商城
        self.sync_and_draw_shop(&mut consumed, mouse_pos);

        // 菜单
        self.sync_and_draw_menu(&mut consumed, mouse_pos);

        // 小地图
        self.sync_and_draw_minimap(&mut consumed, mouse_pos);

        // 选项
        self.sync_and_draw_option(&mut consumed, mouse_pos);

        // 聊天
        self.sync_and_draw_chat(&mut consumed, mouse_pos);

        // 聊天控制栏
        self.sync_and_draw_chat_control_bar(&mut consumed, mouse_pos);

        // 任务日志
        self.sync_and_draw_quest_log(&mut consumed, mouse_pos);

        consumed
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

        // 绘制球框纹理 Prguse[5] - 左框，Prguse[6] - 右框
        if let Some(frame_left) = LibraryName::Prguse.get_texture(5) {
            if let Some(ref tex) = frame_left.image {
                draw_texture_ex(tex, orb_x - 5.0, orb_y - 5.0, WHITE, DrawTextureParams::default());
            }
        }
        if let Some(frame_right) = LibraryName::Prguse.get_texture(6) {
            if let Some(ref tex) = frame_right.image {
                let frame_left_width = LibraryName::Prguse.get_texture(5).map(|t| t.width as f32).unwrap_or(50.0);
                draw_texture_ex(tex, orb_x + frame_left_width - 5.0, orb_y - 5.0, WHITE, DrawTextureParams::default());
            }
        }

        // 绘制数值文字
        // HP标签位置：C#源码 HealthLabel.Location = (0, 27) 相对于 HealthOrb
        let hp_text = format!("{}/{}", self.hp, self.max_hp);
        draw_text_cn(&hp_text, orb_x + 9.0, orb_y + 27.0, 11.0, WHITE);

        // MP标签位置：C#源码 ManaLabel.Location = (0, 42) 相对于 HealthOrb
        let mp_text = format!("{}/{}", self.mp, self.max_mp);
        draw_text_cn(&mp_text, orb_x + 9.0, orb_y + 42.0, 11.0, WHITE);
    }

    /// 绘制经验条
    fn draw_exp_bar(&self) {
        let bar_x = self.position.x + 9.0;
        let bar_y = self.position.y + self.bg_size.y - 10.0;

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
        let exp_text = format!("{:.1}%", self.exp_percent * 100.0);
        draw_text_cn(&exp_text, bar_x + 40.0, bar_y + 4.0, 9.0, WHITE);
    }

    /// 绘制负重条
    fn draw_weight_bar(&self) {
        let bar_x = self.position.x + self.bg_size.x - 105.0;
        let bar_y = self.position.y + self.bg_size.y - 30.0;

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
        let weight_text = format!("{}/{}", self.weight, self.max_weight);
        draw_text_cn(&weight_text, bar_x + 25.0, bar_y + 4.0, 9.0, WHITE);
    }

    /// 绘制角色信息
    fn draw_character_info(&self) {
        let info_x = self.position.x + 130.0;
        let info_y = self.position.y + 15.0;

        // 角色名和等级
        let name_level = format!("{} Lv.{}", self.character_name, self.level);
        draw_text_cn(&name_level, info_x, info_y + 12.0, 16.0, Color::from_rgba(255, 215, 0, 255));

        // 金币
        let gold_text = format!("金币: {}", self.gold);
        draw_text_cn(&gold_text, info_x, info_y + 28.0, 12.0, Color::from_rgba(255, 215, 0, 255));

        // 背包空格
        let space_text = format!("空格: {}", self.bag_space);
        draw_text_cn(&space_text, info_x, info_y + 42.0, 12.0, WHITE);
    }

    /// 绘制功能按钮组
    fn draw_buttons(&mut self) {
        let button_y = self.position.y + 76.0;
        let button_start_x = self.position.x + self.bg_size.x - 125.0;
        let button_spacing = 23.0;

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 按钮列表：背包、角色、技能、任务、选项
        // 纹理索引：(正常, 悬停, 按下)
        let buttons: [(usize, usize, usize, &str, usize); 5] = [
            (1903, 1904, 1905, "背包", 0),
            (1900, 1901, 1902, "角色", 1),
            (1906, 1907, 1908, "技能", 2),
            (1909, 1910, 1911, "任务", 3),
            (1912, 1913, 1914, "选项", 4),
        ];

        for (normal_idx, hover_idx, pressed_idx, hint, i) in buttons {
            let btn_x = button_start_x + (i as f32 * button_spacing);
            if self.draw_button(mouse_pos, btn_x, button_y, normal_idx, hover_idx, pressed_idx) {
                println!("🖱️ 点击了 {} 按钮", hint);
                match hint {
                    "背包" => self.inventory_dialog_open = !self.inventory_dialog_open,
                    "角色" => self.character_dialog_open = !self.character_dialog_open,
                    "技能" => {
                        self.character_dialog_open = true;
                        self.character_dialog.switch_tab(CharacterTabHybrid::Skills);
                    }
                    "任务" => self.quest_log_dialog_open = !self.quest_log_dialog_open,
                    "选项" => self.option_dialog_open = !self.option_dialog_open,
                    _ => {}
                }
            }
        }

        // 菜单按钮（位置稍上）
        let menu_x = self.position.x + self.bg_size.x - 55.0;
        let menu_y = self.position.y + 35.0;
        if self.draw_button(mouse_pos, menu_x, menu_y, 1960, 1961, 1962) {
            println!("🖱️ 点击了菜单按钮");
            self.menu_dialog_open = !self.menu_dialog_open;
        }

        // 商城按钮
        let shop_x = self.position.x + self.bg_size.x - 105.0;
        let shop_y = self.position.y + 35.0;
        if self.draw_button(mouse_pos, shop_x, shop_y, 826, 827, 828) {
            println!("🖱️ 点击了商城按钮");
            self.game_shop_dialog_open = !self.game_shop_dialog_open;
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
        self.belt_dialog.update_and_draw();
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
        if !self.chat_dialog.is_visible() {
            self.chat_dialog_open = false;
        }
        if self.chat_dialog_open && self.chat_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn sync_and_draw_chat_control_bar(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.chat_control_bar_open {
            self.chat_control_bar.open();
        } else {
            self.chat_control_bar.close();
        }
        let (size_clicked, _settings_clicked) = self.chat_control_bar.update_and_draw();

        // 如果 Size 按钮被点击，改变 ChatDialog 大小
        if size_clicked {
            let screen_h = screen_height() / screen_dpi_scale();
            self.chat_dialog.change_size(screen_h);

            // 同步更新 ChatControlBar 位置
            let chat_pos = self.chat_dialog.get_position();
            let control_bar_y = chat_pos.y - 15.0;
            self.chat_control_bar.set_position(vec2(chat_pos.x, control_bar_y));
        }

        if !self.chat_control_bar.is_visible() {
            self.chat_control_bar_open = false;
        }
        if self.chat_control_bar_open && self.chat_control_bar.contains(mouse_pos) {
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
