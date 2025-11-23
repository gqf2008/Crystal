/// 角色对话框 - 包含装备、技能、状态页面
/// 对应原工程 CharacterDialog.cs
/// 
/// 功能：
/// - 角色装备显示和管理
/// - 技能树显示和升级
/// - 角色属性和状态显示
/// - 支持标签页切换

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;

/// 角色对话框标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterTab {
    Character,  // 角色装备页 (Char)
    Status,     // 状态页I (Stats I)
    State,      // 状态页II (Stats II)
    Skills,     // 技能页 (Spells)
}

/// 装备栏位类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentSlot {
    Weapon,     // 武器
    Armor,      // 衣服
    Helmet,     // 头盔
    Torch,      // 照明物
    Necklace,   // 项链
    BraceletL,  // 左手镯
    BraceletR,  // 右手镯
    RingL,      // 左戒指
    RingR,      // 右戒指
    Amulet,     // 护身符
    Belt,       // 腰带
    Boots,      // 鞋子
    Stone,      // 宝石
    Mount,      // 坐骑
}

/// 装备物品数据
#[derive(Debug, Clone, Copy)]
pub struct EquipmentItem {
    pub icon_index: usize,      // Items库中的图标索引
    pub image_index: usize,     // StateItems库中的外观索引
    pub durability: (u32, u32), // (当前, 最大)
    pub upgraded: bool,         // 是否强化
}

/// 技能数据
#[derive(Debug, Clone)]  
pub struct SkillInfo {
    pub id: u32,
    pub name: String,
    pub level: u32,
    pub max_level: u32,
    pub icon_index: usize,
    pub experience: u64,
    pub next_exp: u64,
}

/// 角色对话框
pub struct CharacterDialog {
    pub position: egui::Pos2,
    
    /// 当前标签页
    pub active_tab: CharacterTab,
    
    /// 窗口拖拽状态
    dragging: bool,
    drag_offset: egui::Vec2,
    
    /// 装备数据 (14个装备栏位)
    pub equipment: [Option<EquipmentItem>; 14],
    
    /// 技能数据
    pub skills: Vec<SkillInfo>,
    
    /// 角色属性
    pub character_stats: CharacterStats,
    
    /// 角色名字
    pub character_name: String,
    
    /// 公会名字
    pub guild_name: Option<String>,
}

/// 角色属性数据
#[derive(Debug, Clone)]
pub struct CharacterStats {
    pub level: u32,
    pub experience: u64,
    pub next_exp: u64,
    pub health: (u32, u32),     // (当前, 最大)
    pub mana: (u32, u32),       // (当前, 最大)
    pub dc: (u32, u32),         // 攻击力 (最小, 最大)
    pub mc: (u32, u32),         // 魔法 (最小, 最大)
    pub sc: (u32, u32),         // 道术 (最小, 最大)
    pub ac: (u32, u32),         // 防御 (最小, 最大)
    pub mac: (u32, u32),        // 魔防 (最小, 最大)
    pub accuracy: u32,          // 准确
    pub agility: u32,           // 敏捷
    pub luck: u32,              // 幸运
}

impl CharacterDialog {
    pub fn new() -> Self {
        // DEBUG: 检查 StateItems 库
        if let Some(lib) = LibraryName::StateItems.get_library() {
            let mut lib_ref = lib.borrow_mut();
            let count = lib_ref.count();
            println!("📚 StateItems 库信息:");
            println!("  - 图像数量: {}", count);
            // 测试一些可能的索引范围
            println!("  - 测试索引 0-20:");
            for i in 0..20.min(count) {
                let has_texture = lib_ref.get_size(i).is_ok();
                if has_texture {
                    println!("    索引 {}: ✅", i);
                }
            }
        } else {
            println!("❌ StateItems 库未加载！");
        }
        
        // 满配装备数据
        let mut equipment = [None; 14];
        
        // 武器 - 屠龙刀  
        equipment[0] = Some(EquipmentItem {
            icon_index: 4,
            image_index: 10,  // 使用确认可以显示的索引
            durability: (100, 100),
            upgraded: true,
        });
        
        // 衣服 - 天魔神甲
        equipment[1] = Some(EquipmentItem {
            icon_index: 24,
            image_index: 10,  // 也使用索引10测试
            durability: (100, 100),
            upgraded: true,
        });
        
        // 头盔 - 龙之盔
        equipment[2] = Some(EquipmentItem {
            icon_index: 44,
            image_index: 10,  // 也使用索引10测试
            durability: (100, 100),
            upgraded: true,
        });
        
        // 照明物 - 火把
        equipment[3] = Some(EquipmentItem {
            icon_index: 64,
            image_index: 0,
            durability: (100, 100),
            upgraded: false,
        });
        
        // 项链 - 绿色项链
        equipment[4] = Some(EquipmentItem {
            icon_index: 84,
            image_index: 0,
            durability: (100, 100),
            upgraded: true,
        });
        
        // 左手镯 - 幽灵手镯
        equipment[5] = Some(EquipmentItem {
            icon_index: 104,
            image_index: 0,
            durability: (100, 100),
            upgraded: true,
        });
        
        // 右手镯 - 幽灵手镯
        equipment[6] = Some(EquipmentItem {
            icon_index: 104,
            image_index: 0,
            durability: (100, 100),
            upgraded: true,
        });
        
        // 左戒指 - 龙之戒指
        equipment[7] = Some(EquipmentItem {
            icon_index: 124,
            image_index: 0,
            durability: (100, 100),
            upgraded: true,
        });
        
        // 右戒指 - 龙之戒指
        equipment[8] = Some(EquipmentItem {
            icon_index: 124,
            image_index: 0,
            durability: (100, 100),
            upgraded: true,
        });
        
        // 护身符 - 高级护身符
        equipment[9] = Some(EquipmentItem {
            icon_index: 144,
            image_index: 0,
            durability: (100, 100),
            upgraded: true,
        });
        
        // 腰带 - 龙纹腰带
        equipment[10] = Some(EquipmentItem {
            icon_index: 164,
            image_index: 0,
            durability: (100, 100),
            upgraded: true,
        });
        
        // 鞋子 - 战神之靴
        equipment[11] = Some(EquipmentItem {
            icon_index: 184,
            image_index: 0,
            durability: (100, 100),
            upgraded: true,
        });
        
        // 宝石
        equipment[12] = Some(EquipmentItem {
            icon_index: 204,
            image_index: 0,
            durability: (100, 100),
            upgraded: true,
        });
        
        // 坐骑
        equipment[13] = Some(EquipmentItem {
            icon_index: 224,
            image_index: 0,
            durability: (100, 100),
            upgraded: false,
        });
        
        // 满级技能数据
        let skills = vec![
            SkillInfo {
                id: 1,
                name: "基本剑术".to_string(),
                level: 3,
                max_level: 3,
                icon_index: 1,
                experience: 0,
                next_exp: 0,
            },
            SkillInfo {
                id: 2,
                name: "攻杀剑术".to_string(),
                level: 3,
                max_level: 3,
                icon_index: 2,
                experience: 0,
                next_exp: 0,
            },
            SkillInfo {
                id: 3,
                name: "刺杀剑术".to_string(),
                level: 3,
                max_level: 3,
                icon_index: 3,
                experience: 0,
                next_exp: 0,
            },
            SkillInfo {
                id: 4,
                name: "半月弯刀".to_string(),
                level: 3,
                max_level: 3,
                icon_index: 4,
                experience: 0,
                next_exp: 0,
            },
            SkillInfo {
                id: 5,
                name: "野蛮冲撞".to_string(),
                level: 3,
                max_level: 3,
                icon_index: 5,
                experience: 0,
                next_exp: 0,
            },
            SkillInfo {
                id: 6,
                name: "烈火剑法".to_string(),
                level: 3,
                max_level: 3,
                icon_index: 6,
                experience: 0,
                next_exp: 0,
            },
            SkillInfo {
                id: 7,
                name: "逐日剑法".to_string(),
                level: 3,
                max_level: 3,
                icon_index: 7,
                experience: 0,
                next_exp: 0,
            },
        ];
        
        let character_stats = CharacterStats {
            level: 60,
            experience: 980000,
            next_exp: 1000000,
            health: (850, 850),
            mana: (450, 450),
            dc: (45, 80),
            mc: (0, 0),
            sc: (0, 0),
            ac: (35, 55),
            mac: (18, 35),
            accuracy: 25,
            agility: 30,
            luck: 5,
        };
        
        Self {
            position: egui::pos2(100.0, 100.0),
            active_tab: CharacterTab::Character,
            dragging: false,
            drag_offset: egui::vec2(0.0, 0.0),
            equipment,
            skills,
            character_stats,
            character_name: "测试角色".to_string(),
            guild_name: Some("传奇公会".to_string()),
        }
    }
    
    
    /// 显示角色页
    pub fn show_character_page(&mut self) {
        self.active_tab = CharacterTab::Character;
    }
    
    /// 显示状态页I
    pub fn show_status_page(&mut self) {
        self.active_tab = CharacterTab::Status;
    }
    
    /// 显示状态页II
    pub fn show_state_page(&mut self) {
        self.active_tab = CharacterTab::State;
    }
    
    /// 显示技能页
    pub fn show_skill_page(&mut self) {
        self.active_tab = CharacterTab::Skills;
    }
    
    /// 绘制对话框背景
    fn draw_background(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 角色对话框主窗口背景 (固定使用 504)
        let bg_index = 504;
        
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, bg_index) {
            if let Some(bg_texture) = info.egui_texture {
                let bg_size = bg_texture.size_vec2();
                let bg_rect = egui::Rect::from_min_size(self.position, bg_size);
                
                ui.painter().image(
                    bg_texture.id(),
                    bg_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                return bg_rect;
            }
        }
        
        // 降级：绘制默认背景
        let default_rect = egui::Rect::from_min_size(self.position, egui::vec2(300.0, 400.0));
        ui.painter().rect_filled(
            default_rect,
            5.0,
            egui::Color32::from_rgb(40, 40, 45),
        );
        ui.painter().rect_stroke(
            default_rect,
            5.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 100, 100)),
            egui::epaint::StrokeKind::Outside,
        );
        
        default_rect
    }
    
    /// 绘制角色名字和公会信息
    fn draw_character_info(&self, ui: &mut egui::Ui, bg_rect: &egui::Rect) {
        // NameLabel: Location = (0, 12), Size = (264, 20), 居中对齐
        let name_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x, bg_rect.min.y + 12.0),
            egui::vec2(264.0, 20.0)
        );
        
        ui.painter().text(
            name_rect.center(),
            egui::Align2::CENTER_CENTER,
            &self.character_name,
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );
        
        // GuildLabel: Location = (0, 33), Size = (264, 30), 居中对齐
        if let Some(guild) = &self.guild_name {
            let guild_rect = egui::Rect::from_min_size(
                egui::pos2(bg_rect.min.x, bg_rect.min.y + 33.0),
                egui::vec2(264.0, 30.0)
            );
            
            ui.painter().text(
                guild_rect.center(),
                egui::Align2::CENTER_CENTER,
                guild,
                egui::FontId::proportional(12.0),
                egui::Color32::from_rgb(255, 215, 0), // 金色
            );
        }
    }
    
    /// 绘制标签页按钮
    fn draw_tab_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 标签页按钮位置 (基于原工程布局)
        // CharacterButton: Location = (8, 70), Size = (64, 20), PressedIndex = 500
        // StatusButton: Location = (70, 70), Size = (64, 20), PressedIndex = 501
        // StateButton: Location = (132, 70), Size = (64, 20), PressedIndex = 502
        // SkillButton: Location = (194, 70), Size = (64, 20), PressedIndex = 503
        
        let tab_y = bg_rect.min.y + 70.0;
        let tab_buttons = [
            (CharacterTab::Character, 500, bg_rect.min.x + 8.0),
            (CharacterTab::Status, 501, bg_rect.min.x + 70.0),
            (CharacterTab::State, 502, bg_rect.min.x + 132.0),
            (CharacterTab::Skills, 503, bg_rect.min.x + 194.0),
        ];
        
        for (tab, texture_index, x) in tab_buttons {
            let is_active = self.active_tab == tab;
            let button_rect = egui::Rect::from_min_size(
                egui::pos2(x, tab_y),
                egui::vec2(64.0, 20.0)
            );
            
            let response = ui.interact(button_rect, egui::Id::new(format!("tab_{:?}", tab)), egui::Sense::click());
            
            // 使用Title库的纹理绘制按钮
            if let Some(info) = LibraryName::Title.get_egui_texture(ctx, texture_index) {
                if let Some(btn_texture) = info.egui_texture {
                    ui.painter().image(
                        btn_texture.id(),
                        button_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        if is_active { egui::Color32::WHITE } else { egui::Color32::from_rgb(180, 180, 180) },
                    );
                }
            } else {
                // 备用：绘制简单按钮
                let bg_color = if is_active {
                    egui::Color32::from_rgb(80, 120, 160)
                } else if response.hovered() {
                    egui::Color32::from_rgb(60, 60, 70)
                } else {
                    egui::Color32::from_rgb(50, 50, 55)
                };
                
                ui.painter().rect_filled(button_rect, 3.0, bg_color);
                ui.painter().rect_stroke(
                    button_rect,
                    3.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
                    egui::epaint::StrokeKind::Outside,
                );
                
                // 备用文字
                let label = match tab {
                    CharacterTab::Character => "角色",
                    CharacterTab::Status => "状态I",
                    CharacterTab::State => "状态II",
                    CharacterTab::Skills => "技能",
                };
                ui.painter().text(
                    button_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(10.0),
                    egui::Color32::WHITE,
                );
            }
            
            if response.clicked() {
                self.active_tab = tab;
                println!("🔄 切换到标签页: {:?}", tab);
            }
        }
    }
    
    /// 绘制装备页内容
    fn draw_character_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 绘制角色页背景 (340 - Prguse)
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 340) {
            if let Some(page_texture) = info.egui_texture {
                let page_base_x = bg_rect.min.x + 8.0;
                let page_base_y = bg_rect.min.y + 90.0;
                
                // CharacterPage 是 MirImageControl, UseOffSet=false, 不应用offset
                let page_pos = egui::pos2(page_base_x, page_base_y);
                
                unsafe {
                    if !DEBUG_PRINTED {
                        println!("  📄 CharacterPage[340]: offset=({}, {}), size=({}, {})",
                            info.offset_x, info.offset_y, info.width, info.height);
                        println!("     渲染位置=({:.1}, {:.1})", page_pos.x, page_pos.y);
                    }
                }
                
                let page_size = page_texture.size_vec2();
                let page_rect = egui::Rect::from_min_size(page_pos, page_size);
                
                ui.painter().image(
                    page_texture.id(),
                    page_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 绘制职业图标/角色立绘 (ClassImage: Location = (15, 33), Index = 100-104, Library = Prguse)
        // 100=战士, 101=法师, 102=道士, 103=刺客, 104=弓箭手
        // 注意：ClassImage.Parent = CharacterDialog（不是CharacterPage！）
        let class_index = 100; // 战士
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, class_index) {
            if let Some(class_texture) = info.egui_texture {
                // ClassImage相对于CharacterDialog的位置
                let class_base_x = bg_rect.min.x + 15.0;
                let class_base_y = bg_rect.min.y + 33.0;
                
                // ClassImage 是 MirImageControl, UseOffSet=false, 不应用offset
                let class_draw_x = class_base_x;
                let class_draw_y = class_base_y;
                
                unsafe {
                    if !DEBUG_PRINTED {
                        println!("  👤 ClassImage[{}]: Prguse offset=({}, {})", class_index, info.offset_x, info.offset_y);
                        println!("     渲染位置=({:.1}, {:.1}), size=({}, {})", class_draw_x, class_draw_y, info.width, info.height);
                    }
                }
                
                let class_rect = egui::Rect::from_min_size(
                    egui::pos2(class_draw_x, class_draw_y),
                    class_texture.size_vec2()
                );
                
                ui.painter().image(
                    class_texture.id(),
                    class_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 绘制角色外观（基于原工程的CharacterPage.AfterDraw逻辑）
        // 
        // 🔑 核心理解：
        // 1. CharacterDialog的DisplayLocation = CharacterDialog在屏幕上的位置
        // 2. CharacterPage.DisplayLocation = CharacterDialog.DisplayLocation + (8, 90)
        // 3. 在CharacterPage.AfterDraw中，使用的DisplayLocation就是CharacterPage的DisplayLocation
        // 4. 这个DisplayLocation就是角色的锚点（脚底位置）！
        //
        // 根据原C#代码：
        // - CharacterPage.AfterDraw中直接使用DisplayLocation作为绘制位置
        // - Libraries.StateItems.Draw(image, DisplayLocation, color, true)
        // - 第4个参数true表示应用offset
        //
        // 所以：DisplayLocation = CharacterPage的绝对位置 = 角色锚点
        
        // 🎯 关键修正：
        // C# 中 CharacterPage.AfterDraw 使用 DisplayLocation 作为锚点
        // CharacterPage 是 MirImageControl，配置：
        // - Location = (8, 90) 相对于 CharacterDialog
        // - Index = 340 (Prguse)
        // - DisplayLocation = Parent.DisplayLocation + Location + Library.GetOffset(Index)
        //
        // 所以锚点应该是 CharacterPage 的位置,不是 ClassImage!
        
        // CharacterPage 是 MirImageControl, UseOffSet=false
        // DisplayLocation = Parent.DisplayLocation + Location (不加offset)
        // = CharacterDialog位置 + (8, 90)
        let character_anchor_x = bg_rect.min.x + 8.0;
        let character_anchor_y = bg_rect.min.y + 90.0;
        
        // 只打印一次用于调试
        static mut DEBUG_PRINTED: bool = false;
        unsafe {
            if !DEBUG_PRINTED {
                println!("\n🎯 装备渲染锚点调试:");
                println!("  CharacterDialog位置: ({:.1}, {:.1})", bg_rect.min.x, bg_rect.min.y);
                println!("  CharacterPage.Location: (8, 90)");
                println!("  CharacterPage.UseOffSet: false (不应用offset)");
                println!("  最终锚点(CharacterPage.DisplayLocation): ({:.1}, {:.1})", character_anchor_x, character_anchor_y);
                DEBUG_PRINTED = true;
            }
        }
        
        // offset会自动被纹理加载器应用
        // 我们只需要使用锚点坐标 + offset
        
        // 1. 绘制衣服/盔甲外观（如果有）
        if let Some(armour) = &self.equipment[1] {  // EquipmentSlot.Armour
            println!("  🛡️ 盔甲装备: image_index={}", armour.image_index);
            if armour.image_index > 0 {
                match LibraryName::StateItems.get_egui_texture(ctx, armour.image_index) {
                    Some(info) => {
                        if let Some(texture) = info.egui_texture {
                            let pos = egui::pos2(
                                character_anchor_x + info.offset_x as f32,
                                character_anchor_y + info.offset_y as f32
                            );
                            
                            println!("     offset=({}, {}), 最终位置=({:.1}, {:.1}), size=({}, {})", 
                                info.offset_x, info.offset_y, pos.x, pos.y, info.width, info.height);
                            
                            let rect = egui::Rect::from_min_size(pos, texture.size_vec2());
                            
                            ui.painter().image(
                                texture.id(),
                                rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                egui::Color32::WHITE,
                            );
                        } else {
                            println!("     ❌ 纹理无效 (size={}x{})", info.width, info.height);
                        }
                    }
                    None => {
                        println!("     ❌ 无法获取纹理信息");
                    }
                }
            }
        }
        
        // 【超大红色测试矩形】- 在对话框中心绘制100x100的纯红色矩形
        let test_rect_center = egui::pos2(
            bg_rect.min.x + 124.0,  // 对话框中心X (248/2)
            bg_rect.min.y + 142.0   // 对话框中心Y (284/2)
        );
        ui.painter().rect_filled(
            egui::Rect::from_center_size(test_rect_center, egui::vec2(100.0, 100.0)),
            0.0,
            egui::Color32::RED  // 完全不透明的纯红色
        );
        
        // 2. 绘制武器外观（如果有）
        if let Some(weapon) = &self.equipment[0] {  // EquipmentSlot.Weapon
            // 总是尝试绘制，包括索引0
            match LibraryName::StateItems.get_egui_texture(ctx, weapon.image_index) {
                Some(info) => {
                    if let Some(texture) = info.egui_texture {
                        let pos = egui::pos2(
                            character_anchor_x + info.offset_x as f32,
                            character_anchor_y + info.offset_y as f32
                        );
                        
                        let rect = egui::Rect::from_min_size(pos, texture.size_vec2());
                        
                        ui.painter().image(
                            texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                None => {}
            }
        }
        
        // 3. 绘制头盔外观（如果有），否则绘制头发
        if let Some(helmet) = &self.equipment[2] {  // EquipmentSlot.Helmet
            if helmet.image_index > 0 {
                if let Some(info) = LibraryName::StateItems.get_egui_texture(ctx, helmet.image_index) {
                    if let Some(texture) = info.egui_texture {
                        // println!("🪖 头盔[{}] - anchor: ({:.1}, {:.1}), offset: ({}, {}), size: ({}, {})",
                        //     helmet.image_index, character_anchor_x, character_anchor_y, 
                        //     info.offset_x, info.offset_y, info.width, info.height);
                        
                        let pos = egui::pos2(
                            character_anchor_x + info.offset_x as f32,
                            character_anchor_y + info.offset_y as f32
                        );
                        
                        // println!("   最终渲染位置: ({:.1}, {:.1})", pos.x, pos.y);
                        
                        let rect = egui::Rect::from_min_size(pos, texture.size_vec2());
                        
                        ui.painter().image(
                            texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }
        } else {
            // 绘制头发 (Prguse库 441+发型索引)
            // 441 + Hair + (Class == Assassin ? 20 : 0) + (Gender == Male ? 0 : 40)
            let hair_index = 441; // 默认战士男性发型0
            if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, hair_index) {
                if let Some(texture) = info.egui_texture {
                    // 头发也需要应用offset
                    let hair_pos = egui::pos2(
                        character_anchor_x + info.offset_x as f32,
                        character_anchor_y + info.offset_y as f32
                    );
                    let hair_rect = egui::Rect::from_min_size(hair_pos, texture.size_vec2());
                    
                    ui.painter().image(
                        texture.id(),
                        hair_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
        }
        
        // 装备栏位布局 (基于原工程精确位置)
        // CharacterPage 的 Location = (8, 90)，装备栏是相对 CharacterPage 的位置
        let equipment_slots = [
            (EquipmentSlot::Weapon, egui::pos2(bg_rect.min.x + 8.0 + 123.0, bg_rect.min.y + 90.0 + 7.0)),
            (EquipmentSlot::Armor, egui::pos2(bg_rect.min.x + 8.0 + 163.0, bg_rect.min.y + 90.0 + 7.0)),
            (EquipmentSlot::Helmet, egui::pos2(bg_rect.min.x + 8.0 + 203.0, bg_rect.min.y + 90.0 + 7.0)),
            (EquipmentSlot::Torch, egui::pos2(bg_rect.min.x + 8.0 + 203.0, bg_rect.min.y + 90.0 + 134.0)),
            (EquipmentSlot::Necklace, egui::pos2(bg_rect.min.x + 8.0 + 203.0, bg_rect.min.y + 90.0 + 98.0)),
            (EquipmentSlot::BraceletL, egui::pos2(bg_rect.min.x + 8.0 + 8.0, bg_rect.min.y + 90.0 + 170.0)),
            (EquipmentSlot::BraceletR, egui::pos2(bg_rect.min.x + 8.0 + 203.0, bg_rect.min.y + 90.0 + 170.0)),
            (EquipmentSlot::RingL, egui::pos2(bg_rect.min.x + 8.0 + 8.0, bg_rect.min.y + 90.0 + 206.0)),
            (EquipmentSlot::RingR, egui::pos2(bg_rect.min.x + 8.0 + 203.0, bg_rect.min.y + 90.0 + 206.0)),
            (EquipmentSlot::Amulet, egui::pos2(bg_rect.min.x + 8.0 + 8.0, bg_rect.min.y + 90.0 + 242.0)),
            (EquipmentSlot::Belt, egui::pos2(bg_rect.min.x + 8.0 + 88.0, bg_rect.min.y + 90.0 + 242.0)),
            (EquipmentSlot::Boots, egui::pos2(bg_rect.min.x + 8.0 + 48.0, bg_rect.min.y + 90.0 + 242.0)),
            (EquipmentSlot::Stone, egui::pos2(bg_rect.min.x + 8.0 + 128.0, bg_rect.min.y + 90.0 + 242.0)),
            (EquipmentSlot::Mount, egui::pos2(bg_rect.min.x + 8.0 + 203.0, bg_rect.min.y + 90.0 + 62.0)),
        ];
        
        for (slot_index, (_slot_type, pos)) in equipment_slots.iter().enumerate() {
            self.draw_equipment_slot(ui, ctx, slot_index, *pos);
        }
        
        // Character页不显示文字属性，只显示装备和角色外观
    }
    
    /// 绘制装备栏位
    fn draw_equipment_slot(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, slot_index: usize, pos: egui::Pos2) {
        let slot_size = egui::vec2(32.0, 32.0);
        let slot_rect = egui::Rect::from_min_size(pos, slot_size);
        
        // 绘制装备图标（无背景，无边框）
        if let Some(equipment) = &self.equipment[slot_index] {
            if let Some(info) = LibraryName::Items.get_egui_texture(ctx, equipment.icon_index) {
                if let Some(item_texture) = info.egui_texture {
                    // 直接显示装备图标，不居中，使用原始位置
                    let img_size = egui::vec2(info.width as f32, info.height as f32);
                    let img_rect = egui::Rect::from_min_size(pos, img_size);
                    
                    ui.painter().image(
                        item_texture.id(),
                        img_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    
                    // 耐久度条也移除，只显示纯粹的装备图标
                }
            }
        }
        
        // 处理栏位交互
        let response = ui.interact(slot_rect, egui::Id::new(format!("equip_slot_{}", slot_index)), egui::Sense::click());
        if response.clicked() {
            println!("🎯 点击装备栏位: {}", slot_index);
        }
    }
    
    /// 绘制技能页内容
    fn draw_skills_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 绘制技能页背景 (508 - Title)
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 508) {
            if let Some(page_texture) = info.egui_texture {
                let page_pos = egui::pos2(bg_rect.min.x + 8.0, bg_rect.min.y + 90.0);
                let page_size = page_texture.size_vec2();
                let page_rect = egui::Rect::from_min_size(page_pos, page_size);
                
                ui.painter().image(
                    page_texture.id(),
                    page_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 技能栏位布局 (7个技能栏位，基于原工程位置8, 8开始，每个33像素间隔)
        let skill_start_x = bg_rect.min.x + 8.0 + 8.0;
        let skill_start_y = bg_rect.min.y + 90.0 + 8.0;
        let skill_spacing = 33.0;
        
        for i in 0..7 {
            let pos = egui::pos2(
                skill_start_x,
                skill_start_y + i as f32 * skill_spacing
            );
            
            // 如果有技能数据，显示技能信息（完全透明背景，无边框）
            if i < self.skills.len() {
                let skill = &self.skills[i];
                ui.painter().text(
                    egui::pos2(pos.x + 10.0, pos.y + 15.0),
                    egui::Align2::LEFT_CENTER,
                    &skill.name,
                    egui::FontId::proportional(11.0),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // 绘制分页按钮 (BackButton: 90, 250; NextButton: 140, 250)
        let back_btn_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 8.0 + 90.0, bg_rect.min.y + 90.0 + 250.0),
            egui::vec2(20.0, 20.0)
        );
        let next_btn_rect = egui::Rect::from_min_size(
            egui::pos2(bg_rect.min.x + 8.0 + 140.0, bg_rect.min.y + 90.0 + 250.0),
            egui::vec2(20.0, 20.0)
        );
        
        // 上一页按钮 (Prguse纹理398/399)
        let back_response = ui.interact(back_btn_rect, egui::Id::new("skill_back"), egui::Sense::click());
        let back_index = if back_response.is_pointer_button_down_on() {
            399
        } else {
            398
        };
        
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, back_index) {
            if let Some(texture) = info.egui_texture {
                ui.painter().image(
                    texture.id(),
                    back_btn_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        } else {
            ui.painter().rect_filled(back_btn_rect, 3.0, egui::Color32::from_rgb(80, 80, 120));
            ui.painter().text(
                back_btn_rect.center(),
                egui::Align2::CENTER_CENTER,
                "◀",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }
        
        if back_response.clicked() {
            println!("◀ 技能上一页");
        }
        
        // 下一页按钮 (Prguse纹理396/397)
        let next_response = ui.interact(next_btn_rect, egui::Id::new("skill_next"), egui::Sense::click());
        let next_index = if next_response.is_pointer_button_down_on() {
            397
        } else {
            396
        };
        
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, next_index) {
            if let Some(texture) = info.egui_texture {
                ui.painter().image(
                    texture.id(),
                    next_btn_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        } else {
            ui.painter().rect_filled(next_btn_rect, 3.0, egui::Color32::from_rgb(80, 80, 120));
            ui.painter().text(
                next_btn_rect.center(),
                egui::Align2::CENTER_CENTER,
                "▶",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }
        
        if next_response.clicked() {
            println!("▶ 技能下一页");
        }
    }
    
    /// 绘制状态页I内容 (Stats I)
    fn draw_status_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 绘制状态页背景 (506 - Title)
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 506) {
            if let Some(page_texture) = info.egui_texture {
                let page_pos = egui::pos2(bg_rect.min.x + 8.0, bg_rect.min.y + 90.0);
                let page_size = page_texture.size_vec2();
                let page_rect = egui::Rect::from_min_size(page_pos, page_size);
                
                ui.painter().image(
                    page_texture.id(),
                    page_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // Stats I页 - 显示战斗属性（基于原工程StatusLabel布局）
        let stats_x = bg_rect.min.x + 8.0 + 126.0;
        let stats_y = bg_rect.min.y + 90.0 + 20.0;
        let line_height = 18.0;
        
        let status_labels = [
            (format!("{}/{}", self.character_stats.health.0, self.character_stats.health.1), 0),  // HP
            (format!("{}/{}", self.character_stats.mana.0, self.character_stats.mana.1), 1),      // MP
            (format!("{}-{}", self.character_stats.ac.0, self.character_stats.ac.1), 2),          // AC
            (format!("{}-{}", self.character_stats.mac.0, self.character_stats.mac.1), 3),        // MAC
            (format!("{}-{}", self.character_stats.dc.0, self.character_stats.dc.1), 4),          // DC
            (format!("{}-{}", self.character_stats.mc.0, self.character_stats.mc.1), 5),          // MC
            (format!("{}-{}", self.character_stats.sc.0, self.character_stats.sc.1), 6),          // SC
            (format!("0-0"), 7),          // 攻击速度
            (format!("0"), 8),            // 准确
            (format!("0"), 9),            // 敏捷
            (format!("0"), 10),           // 幸运
        ];
        
        for (text, offset) in status_labels {
            ui.painter().text(
                egui::pos2(stats_x, stats_y + offset as f32 * line_height),
                egui::Align2::LEFT_TOP,
                text,
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
        }
    }
    
    /// 绘制状态页II内容 (Stats II)
    fn draw_state_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 绘制状态详情页背景 (507 - Title)
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 507) {
            if let Some(page_texture) = info.egui_texture {
                let page_pos = egui::pos2(bg_rect.min.x + 8.0, bg_rect.min.y + 90.0);
                let page_size = page_texture.size_vec2();
                let page_rect = egui::Rect::from_min_size(page_pos, page_size);
                
                ui.painter().image(
                    page_texture.id(),
                    page_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // Stats II页 - 显示详细属性（经验、负重、抗性等）
        let stats_x = bg_rect.min.x + 8.0 + 126.0;
        let stats_y = bg_rect.min.y + 90.0 + 20.0;
        let line_height = 18.0;
        
        let exp_percent = if self.character_stats.next_exp > 0 {
            (self.character_stats.experience as f64 / self.character_stats.next_exp as f64 * 100.0) as u32
        } else {
            0
        };
        
        let state_labels = [
            (format!("{}%", exp_percent), 0),  // 经验百分比
            (format!("0/100"), 1),  // 背包负重
            (format!("0/100"), 2),  // 穿戴负重
            (format!("0/100"), 3),  // 腕力负重
            (format!("+0"), 4),     // 魔法抗性
            (format!("+0"), 5),     // 毒素抗性
            (format!("+0"), 6),     // 生命恢复
            (format!("+0"), 7),     // 魔法恢复
            (format!("+0"), 8),     // 中毒恢复
            (format!("+0"), 9),     // 神圣
            (format!("+0"), 10),    // 冰冻
            (format!("+0"), 11),    // 毒素攻击
        ];
        
        for (text, offset) in state_labels {
            ui.painter().text(
                egui::pos2(stats_x, stats_y + offset as f32 * line_height),
                egui::Align2::LEFT_TOP,
                text,
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
        }
    }
    
    /// 绘制关闭按钮
    fn draw_close_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) -> bool {
        // 关闭按钮位置 (原工程: Location = new Point(241, 3))
        let close_pos = egui::pos2(bg_rect.min.x + 241.0, bg_rect.min.y + 3.0);
        let close_size = egui::vec2(20.0, 20.0);
        let close_rect = egui::Rect::from_min_size(close_pos, close_size);
        
        let response = ui.interact(close_rect, egui::Id::new("character_close"), egui::Sense::click());
        
        // 根据状态选择纹理索引 (Prguse2: 360/361/362)
        let texture_index = if response.is_pointer_button_down_on() {
            362 // pressed
        } else if response.hovered() {
            361 // hover
        } else {
            360 // normal
        };
        
        // 绘制关闭按钮纹理
        if let Some(info) = LibraryName::Prguse2.get_egui_texture(ctx, texture_index) {
            if let Some(close_texture) = info.egui_texture {
                ui.painter().image(
                    close_texture.id(),
                    close_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        } else {
            // 备用：绘制简单关闭按钮
            ui.painter().rect_filled(close_rect, 2.0, egui::Color32::from_rgb(150, 50, 50));
            ui.painter().rect_stroke(
                close_rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 100, 100)),
                egui::epaint::StrokeKind::Outside,
            );
            ui.painter().text(
                close_rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
        }
        
        let is_clicked = response.clicked();
        if response.hovered() {
            response.on_hover_text("关闭");
        }
        
        is_clicked
    }
    
    /// 处理窗口拖拽
    fn handle_window_dragging(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, bg_rect: &egui::Rect) {
        // 标题栏区域作为拖拽区域
        let title_area = egui::Rect::from_min_size(
            bg_rect.min,
            egui::vec2(bg_rect.width(), 30.0),
        );
        
        let drag_response = ui.interact(title_area, egui::Id::new("character_drag"), egui::Sense::drag());
        
        if drag_response.drag_started() {
            self.dragging = true;
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.drag_offset = self.position.to_vec2() - pointer_pos.to_vec2();
            }
        }
        
        if self.dragging {
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                self.position = (pointer_pos.to_vec2() + self.drag_offset).to_pos2();
            }
        }
        
        if drag_response.drag_stopped() {
            self.dragging = false;
        }
    }
    
    /// 设置当前标签页
    pub fn set_current_tab(&mut self, tab_index: usize) {
        match tab_index {
            0 => self.active_tab = CharacterTab::Character,
            1 => self.active_tab = CharacterTab::Status,
            2 => self.active_tab = CharacterTab::State,
            3 => self.active_tab = CharacterTab::Skills,
            _ => {}
        }
    }
}

impl Dialog for CharacterDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            return;
        }
        
        // 使用 Area 创建自由浮动窗口
        let response = egui::Area::new(egui::Id::new("character_dialog"))
            .fixed_pos(self.position)
            .movable(false)
            .constrain(true)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                // 绘制背景
                let bg_rect = self.draw_background(ui, ctx);
                
                // 绘制角色名字和公会信息
                self.draw_character_info(ui, &bg_rect);
                
                // 处理窗口拖拽
                self.handle_window_dragging(ui, ctx, &bg_rect);
                
                // 绘制标签页按钮
                self.draw_tab_buttons(ui, ctx, &bg_rect);
                
                // 根据当前标签页绘制内容
                match self.active_tab {
                    CharacterTab::Character => self.draw_character_page(ui, ctx, &bg_rect),
                    CharacterTab::Status => self.draw_status_page(ui, ctx, &bg_rect),
                    CharacterTab::State => self.draw_state_page(ui, ctx, &bg_rect),
                    CharacterTab::Skills => self.draw_skills_page(ui, ctx, &bg_rect),
                }
                
                // 绘制关闭按钮
                if self.draw_close_button(ui, ctx, &bg_rect) {
                    *open = false;
                }
            });
        
        // 简单粗暴：如果鼠标在对话框区域内点击，立即提升到最前面
        if ctx.input(|i| i.pointer.primary_clicked()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if response.response.rect.contains(pos) {
                    ctx.move_to_top(response.response.layer_id);
                }
            }
        }
    }
}