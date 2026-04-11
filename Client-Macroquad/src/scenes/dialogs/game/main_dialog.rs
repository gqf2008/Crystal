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
    RankingDialogHybrid,
    HelpDialogHybrid,
    InspectDialogHybrid,
};
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use std::time::{Duration, Instant};

/// 文本输入请求类型（用于区分不同来源的 TextInputDialog 输入）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputKind {
    None,
    GroupInvite,
    AddFriend,
    AddMentor,
    GuildNotice,
    WhisperChat { target: String },
}

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
    Group,
    Friend,
    Guild,
    Mentor,
    Relationship,
    Trade,
    Mount,
    Hero,
    Buff,
    Fishing,
    IntelligentCreature,
    Compass,
    Socket,
    Mail,
    Ranking,
    Help,
    Inspect,
}

/// 主界面底部工具栏
pub struct MainDialog {
    /// 当前分辨率索引 (0=800, 1=1024, 2=1280+)
    resolution_index: usize,
    /// UI 缓存 - 当前生命值（每帧由 UIRenderSystem 从 ECS Health 组件同步）
    hp: i32,
    /// UI 缓存 - 最大生命值
    max_hp: i32,
    /// UI 缓存 - 当前魔法值（每帧由 UIRenderSystem 从 ECS Mana 组件同步）
    mp: i32,
    /// UI 缓存 - 最大魔法值
    max_mp: i32,
    /// UI 缓存 - 经验值百分比（每帧从 ECS Experience 组件同步）
    exp_percent: f32,
    /// UI 缓存 - 等级（每帧从 ECS CombatStats 组件同步）
    level: u32,
    /// UI 缓存 - 角色名（由 UserInformation 事件设置）
    character_name: String,
    /// UI 缓存 - 金币（每帧从 ECS Currency 组件同步）
    gold: u32,
    /// UI 缓存 - 当前负重（每帧从 ECS Inventory 组件同步）
    weight: u32,
    /// UI 缓存 - 最大负重
    max_weight: u32,
    /// UI 缓存 - 背包空格数
    bag_space: u32,
    /// UI 缓存 - 背包总容量
    bag_capacity: u32,

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
    group_dialog: crate::scenes::dialogs::game::group_dialog::GroupDialogHybrid,
    friend_dialog: crate::scenes::dialogs::game::friend_dialog::FriendDialogHybrid,
    guild_dialog: crate::scenes::dialogs::game::guild_dialog::GuildDialogHybrid,
    mentor_dialog: crate::scenes::dialogs::game::mentor_dialog::MentorDialogHybrid,
    relationship_dialog: crate::scenes::dialogs::game::relationship_dialog::RelationshipDialogHybrid,
    trade_dialog: crate::scenes::dialogs::game::trade_dialog::TradeDialogHybrid,
    mount_dialog: crate::scenes::dialogs::game::mount_dialog::MountDialogHybrid,
    hero_dialog: crate::scenes::dialogs::game::hero_dialog::HeroDialogHybrid,
    buff_dialog: crate::scenes::dialogs::game::buff_dialog::BuffDialogHybrid,
    fishing_dialog: crate::scenes::dialogs::game::fishing_dialog::FishingDialogHybrid,
    intelligent_creature_dialog: crate::scenes::dialogs::game::intelligent_creature_dialog::IntelligentCreatureDialogHybrid,
    compass_dialog: crate::scenes::dialogs::game::compass_dialog::CompassDialogHybrid,
    socket_dialog: crate::scenes::dialogs::game::socket_dialog::SocketDialogHybrid,
    /// 邮件对话框
    mail_dialog: crate::scenes::dialogs::game::mail_dialog::MailDialogHybrid,
    /// 设置对话框
    option_dialog: OptionDialogHybrid,
    /// 游戏商城对话框
    game_shop_dialog: GameShopDialog,
    /// 菜单对话框
    menu_dialog: MenuDialogHybrid,
    /// 小地图对话框
    minimap_dialog: MiniMapDialogHybrid,

    /// 大地图对话框（全屏查看当前地图）
    big_map_dialog: crate::scenes::dialogs::game::big_map_dialog::BigMapDialogHybrid,

    /// 排行榜对话框
    ranking_dialog: RankingDialogHybrid,
    /// 帮助对话框
    help_dialog: HelpDialogHybrid,
    /// 查看装备对话框
    inspect_dialog: InspectDialogHybrid,

    /// 通用文本输入对话框（组队邀请/添加好友/拜师等）
    text_input_dialog: crate::scenes::dialogs::game::text_input_dialog::TextInputDialogHybrid,
    /// 当前输入请求类型（用于区分不同来源的文本输入）
    pending_text_input_kind: TextInputKind,

    // 对话框打开状态
    belt_dialog_open: bool,
    chat_dialog_open: bool,
    chat_control_bar_open: bool,
    chat_option_dialog_open: bool,
    inventory_dialog_open: bool,
    character_dialog_open: bool,
    quest_log_dialog_open: bool,
    group_dialog_open: bool,
    friend_dialog_open: bool,
    guild_dialog_open: bool,
    mentor_dialog_open: bool,
    relationship_dialog_open: bool,
    trade_dialog_open: bool,
    mount_dialog_open: bool,
    hero_dialog_open: bool,
    buff_dialog_open: bool,
    fishing_dialog_open: bool,
    intelligent_creature_dialog_open: bool,
    compass_dialog_open: bool,
    socket_dialog_open: bool,
    mail_dialog_open: bool,
    option_dialog_open: bool,
    game_shop_dialog_open: bool,
    menu_dialog_open: bool,
    minimap_dialog_open: bool,
    ranking_dialog_open: bool,
    help_dialog_open: bool,
    inspect_dialog_open: bool,

    /// 待处理的安全下线请求（由 ui_system 消费）
    pending_logout_request: bool,

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

    /// UI 产生的”自动寻路目标”（世界坐标像素）；由 GameScene 在 update 阶段消费
    pending_auto_path_target: Option<(f32, f32, bool)>,

    /// 小地图左键双击检测
    minimap_left_last_click_time: Option<Instant>,
    /// 小地图右键双击检测
    minimap_right_last_click_time: Option<Instant>,
    minimap_double_click_threshold: Duration,

    /// 暂存的交易对话框动作（由 show_dialogs 产出，由 ui_system.rs 消费发包）
    pending_trade_action: Option<crate::scenes::dialogs::game::trade_dialog::TradeAction>,

    /// 暂存的排行榜刷新请求（由 show_dialogs 产出，由 ui_system.rs 消费发包）
    pending_ranking_refresh_tab: Option<u8>,

    /// 暂存的装备请求（Inventory 拖到 Character 面板时触发）
    pending_equip_request: Option<u64>, // unique_id

    // === 攻击模式显示 ===
    /// 攻击模式 (0=Peace, 1=Group, 2=Guild, 3=EnemyGuild, 4=RedBrown, 5=All)
    attack_mode: u8,
    /// 宠物模式 (0=Both, 1=MoveOnly, 2=AttackOnly, 3=None, 4=FocusMasterTarget)
    pet_mode: u8,
    /// 技能模式 (true=~, false=Ctrl)
    skill_mode: bool,
    /// 是否显示模式标签（默认隐藏，按 H 切换）
    mode_view: bool,

    // === 角色状态图标 ===
    /// 中毒状态 (0=无, 1=普通中毒, 2=重度中毒, 3=麻痹)
    poison_level: u8,

    // === 快捷技能栏 ===
    /// 快捷技能索引 (F1-F8 对应的技能 ID, 0 表示未绑定)
    quick_skills: [u8; 8],
}

impl Default for MainDialog {
    fn default() -> Self {
        Self::new()
    }
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
            bag_capacity: 40,

            // 子对话框
            belt_dialog: BeltDialogHybrid::new(),
            chat_dialog: ChatDialogHybrid::new(main_dialog_x, screen_h, resolution_index),
            chat_control_bar: ChatControlBarHybrid::new(main_dialog_x, screen_h, resolution_index),
            chat_option_dialog: ChatOptionDialogHybrid::new(),
            inventory_dialog: InventoryDialogHybrid::new(),
            character_dialog: CharacterDialogHybrid::new(),
            quest_log_dialog: QuestLogDialogHybrid::new(),
            group_dialog: crate::scenes::dialogs::game::group_dialog::GroupDialogHybrid::new(),
            friend_dialog: crate::scenes::dialogs::game::friend_dialog::FriendDialogHybrid::new(),
            guild_dialog: crate::scenes::dialogs::game::guild_dialog::GuildDialogHybrid::new(),
            mentor_dialog: crate::scenes::dialogs::game::mentor_dialog::MentorDialogHybrid::new(),
            relationship_dialog: crate::scenes::dialogs::game::relationship_dialog::RelationshipDialogHybrid::new(),
            trade_dialog: crate::scenes::dialogs::game::trade_dialog::TradeDialogHybrid::new(),
            mount_dialog: crate::scenes::dialogs::game::mount_dialog::MountDialogHybrid::new(),
            hero_dialog: crate::scenes::dialogs::game::hero_dialog::HeroDialogHybrid::new(),
            buff_dialog: crate::scenes::dialogs::game::buff_dialog::BuffDialogHybrid::new(),
            fishing_dialog: crate::scenes::dialogs::game::fishing_dialog::FishingDialogHybrid::new(),
            intelligent_creature_dialog: crate::scenes::dialogs::game::intelligent_creature_dialog::IntelligentCreatureDialogHybrid::new(),
            compass_dialog: crate::scenes::dialogs::game::compass_dialog::CompassDialogHybrid::new(),
            socket_dialog: crate::scenes::dialogs::game::socket_dialog::SocketDialogHybrid::new(),
            mail_dialog: crate::scenes::dialogs::game::mail_dialog::MailDialogHybrid::new(),
            option_dialog: OptionDialogHybrid::new(),
            game_shop_dialog: GameShopDialog::new(),
            menu_dialog: MenuDialogHybrid::new(),
            minimap_dialog: MiniMapDialogHybrid::new(),
            big_map_dialog: crate::scenes::dialogs::game::big_map_dialog::BigMapDialogHybrid::new(),
            ranking_dialog: RankingDialogHybrid::new(),
            help_dialog: HelpDialogHybrid::new(),
            inspect_dialog: InspectDialogHybrid::new(),

            // 对话框打开状态
            belt_dialog_open: true,
            chat_dialog_open: true,
            chat_control_bar_open: true,
            chat_option_dialog_open: false,
            inventory_dialog_open: false,
            character_dialog_open: false,
            quest_log_dialog_open: false,
            group_dialog_open: false,
            friend_dialog_open: false,
            guild_dialog_open: false,
            mentor_dialog_open: false,
            relationship_dialog_open: false,
            trade_dialog_open: false,
            mount_dialog_open: false,
            hero_dialog_open: false,
            buff_dialog_open: false,
            fishing_dialog_open: false,
            intelligent_creature_dialog_open: false,
            compass_dialog_open: false,
            socket_dialog_open: false,
            mail_dialog_open: false,
            option_dialog_open: false,
            game_shop_dialog_open: false,
            menu_dialog_open: false,
            minimap_dialog_open: true,
            ranking_dialog_open: false,
            help_dialog_open: false,
            inspect_dialog_open: false,
            pending_logout_request: false,

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
                DialogType::Group,
                DialogType::Friend,
                DialogType::Guild,
                DialogType::Mentor,
                DialogType::Relationship,
                DialogType::Trade,
                DialogType::Mount,
                DialogType::Hero,
                DialogType::Buff,
                DialogType::Fishing,
                DialogType::IntelligentCreature,
                DialogType::Compass,
                DialogType::Socket,
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
            pending_trade_action: None,
            pending_ranking_refresh_tab: None,
            pending_equip_request: None,

            // 攻击模式显示
            attack_mode: 0,
            pet_mode: 0,
            skill_mode: false,
            mode_view: false,

            // 角色状态图标
            poison_level: 0,

            // 快捷技能栏
            quick_skills: [0; 8],

            // 文本输入对话框
            text_input_dialog: crate::scenes::dialogs::game::text_input_dialog::TextInputDialogHybrid::new(),
            pending_text_input_kind: TextInputKind::None,
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

    /// 同步玩家基础属性（等级/金币/经验/负重/背包空间/角色名）
    pub fn set_player_stats(
        &mut self,
        level: u16,
        gold: u32,
        exp_percent: f32,
        weight: u16,
        max_weight: u16,
        bag_space: u32,
        bag_capacity: u32,
        character_name: Option<String>,
    ) {
        self.level = level as u32;
        self.gold = gold;
        self.exp_percent = exp_percent.clamp(0.0, 1.0);
        self.weight = weight as u32;
        self.max_weight = max_weight as u32;
        self.bag_space = bag_space;
        self.bag_capacity = bag_capacity;
        if let Some(name) = character_name {
            self.character_name = name;
        }
    }

    /// 获取角色名称
    pub fn character_name(&self) -> &str {
        &self.character_name
    }

    /// 同步攻击模式（服务器推送 ChangeAMode）
    pub fn set_attack_mode(&mut self, mode: u8) {
        self.attack_mode = mode;
    }

    /// 同步宠物模式（服务器推送 ChangePMode / HeroBehaviour）
    pub fn set_pet_mode(&mut self, mode: u8) {
        self.pet_mode = mode;
    }

    /// 切换技能模式（Ctrl/~ 切换）
    pub fn toggle_skill_mode(&mut self) {
        self.skill_mode = !self.skill_mode;
    }

    /// 切换模式标签显示（H 键）
    pub fn toggle_mode_view(&mut self) {
        self.mode_view = !self.mode_view;
    }

    /// 设置中毒状态 (0=无, 1=普通中毒, 2=重度中毒, 3=麻痹)
    pub fn set_poison_level(&mut self, level: u8) {
        self.poison_level = level;
    }

    /// 设置快捷技能 (slot 0-7, skill_id 0=未绑定)
    pub fn set_quick_skill(&mut self, slot: usize, skill_id: u8) {
        if slot < 8 {
            self.quick_skills[slot] = skill_id;
        }
    }

    /// 设置小地图对应的地图尺寸（单位：地图格子数 width/height）
    ///
    /// 备注：小地图点击反算会先算出格子坐标，再用 `Coord::grid_to_world_center()` 转为世界像素。
    pub fn set_minimap_world_size(&mut self, grid_w: f32, grid_h: f32) {
        self.minimap_dialog.set_world_size(grid_w, grid_h);
    }

    /// 同步背包数据到 InventoryDialog
    pub fn sync_inventory(&mut self, inv: &crate::components::Inventory) {
        self.inventory_dialog.sync_from_ecs_inventory(inv);
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

    /// 确保纹理已加载（惰性加载，避免在 data_path 设置前调用）
    pub fn ensure_textures_loaded(&mut self) {
        if self.bg_texture.is_some() {
            return;
        }

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
        self.group_dialog.load_textures();
        self.friend_dialog.load_textures();
        self.guild_dialog.load_textures();
        self.mentor_dialog.load_textures();
        self.relationship_dialog.load_textures();
        self.trade_dialog.load_textures();
        self.mount_dialog.load_textures();
        self.hero_dialog.load_textures();
        self.buff_dialog.load_textures();
        self.fishing_dialog.load_textures();
        self.intelligent_creature_dialog.load_textures();
        self.compass_dialog.load_textures();
        self.socket_dialog.load_textures();
        self.mail_dialog.load_textures();
        self.ranking_dialog.load_textures();
        self.help_dialog.load_textures();
        self.inspect_dialog.load_textures();
        self.option_dialog.load_textures();
        self.game_shop_dialog.load_textures();
        self.menu_dialog.load_textures();
        self.minimap_dialog.load_textures();
    }

    /// 异步加载纹理
    pub fn load_native_textures(&mut self) {
        self.ensure_textures_loaded();
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
        self.group_dialog.load_textures();
        self.friend_dialog.load_textures();
        self.guild_dialog.load_textures();
        self.mentor_dialog.load_textures();
        self.relationship_dialog.load_textures();
        self.trade_dialog.load_textures();
        self.mount_dialog.load_textures();
        self.hero_dialog.load_textures();
        self.buff_dialog.load_textures();
        self.fishing_dialog.load_textures();
        self.intelligent_creature_dialog.load_textures();
        self.compass_dialog.load_textures();
        self.socket_dialog.load_textures();
        self.mail_dialog.load_textures();
        self.ranking_dialog.load_textures();
        self.help_dialog.load_textures();
        self.inspect_dialog.load_textures();
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

    /// 往聊天窗口追加一条私聊消息
    pub fn push_whisper_line(&mut self, text: impl Into<String>) {
        self.chat_dialog
            .add_message(text, Color::from_rgba(255, 150, 100, 255));
    }

    /// 是否有任何“弹窗类”对话框打开（用于 ESC 逻辑）
    /// 说明：不包含 Belt/Chat/ChatControlBar/MiniMap 这些常驻 UI。
    pub fn any_popup_open(&self) -> bool {
        self.inventory_dialog_open
            || self.character_dialog_open
            || self.quest_log_dialog_open
            || self.group_dialog_open
            || self.friend_dialog_open
            || self.guild_dialog_open
            || self.mentor_dialog_open
            || self.relationship_dialog_open
            || self.trade_dialog_open
            || self.mount_dialog_open
            || self.hero_dialog_open
            || self.buff_dialog_open
            || self.fishing_dialog_open
            || self.intelligent_creature_dialog_open
            || self.compass_dialog_open
            || self.socket_dialog_open
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
        self.group_dialog_open = false;
        self.group_dialog.close();
        self.friend_dialog_open = false;
        self.friend_dialog.close();
        self.guild_dialog_open = false;
        self.guild_dialog.close();
        self.mentor_dialog_open = false;
        self.mentor_dialog.close();
        self.relationship_dialog_open = false;
        self.relationship_dialog.close();
        self.trade_dialog_open = false;
        self.trade_dialog.close();
        self.mount_dialog_open = false;
        self.mount_dialog.close();
        self.hero_dialog_open = false;
        self.hero_dialog.close();
        self.buff_dialog_open = false;
        self.buff_dialog.close();
        self.fishing_dialog_open = false;
        self.fishing_dialog.close();
        self.intelligent_creature_dialog_open = false;
        self.intelligent_creature_dialog.close();
        self.compass_dialog_open = false;
        self.compass_dialog.close();
        self.socket_dialog_open = false;
        self.socket_dialog.close();
        self.mail_dialog_open = false;
        self.mail_dialog.close();
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

    /// 处理全局快捷键（攻击模式/宠物模式/技能模式切换）
    /// 调用方：UISystem 在检测到非聊天输入时调用
    pub fn handle_mode_shortcuts(&mut self) {
        use macroquad::prelude::is_key_pressed;
        use macroquad::prelude::KeyCode;

        // H = 切换模式标签显示
        if is_key_pressed(KeyCode::H) {
            self.toggle_mode_view();
        }

        // G = 切换组队对话框
        if is_key_pressed(KeyCode::G) && is_key_down(KeyCode::LeftAlt) {
            self.group_dialog_open = !self.group_dialog_open;
            if self.group_dialog_open {
                self.bring_to_front(DialogType::Group);
            }
        }

        // F = 切换好友对话框
        if is_key_pressed(KeyCode::F) && is_key_down(KeyCode::LeftAlt) {
            self.friend_dialog_open = !self.friend_dialog_open;
            if self.friend_dialog_open {
                self.bring_to_front(DialogType::Friend);
            }
        }

        // H = 切换行会对话框
        if is_key_pressed(KeyCode::H) && is_key_down(KeyCode::LeftAlt) {
            self.guild_dialog_open = !self.guild_dialog_open;
            if self.guild_dialog_open {
                self.bring_to_front(DialogType::Guild);
            }
        }

        // M = 切换师徒对话框
        if is_key_pressed(KeyCode::M) && is_key_down(KeyCode::LeftAlt) {
            self.mentor_dialog_open = !self.mentor_dialog_open;
            if self.mentor_dialog_open {
                self.bring_to_front(DialogType::Mentor);
            }
        }

        // R = 切换婚姻对话框
        if is_key_pressed(KeyCode::R) && is_key_down(KeyCode::LeftAlt) {
            self.relationship_dialog_open = !self.relationship_dialog_open;
            if self.relationship_dialog_open {
                self.bring_to_front(DialogType::Relationship);
            }
        }

        // T = 切换交易对话框
        if is_key_pressed(KeyCode::T) && is_key_down(KeyCode::LeftAlt) {
            self.trade_dialog_open = !self.trade_dialog_open;
            if self.trade_dialog_open {
                self.bring_to_front(DialogType::Trade);
            }
        }

        // B = 切换坐骑对话框
        if is_key_pressed(KeyCode::B) && is_key_down(KeyCode::LeftAlt) {
            self.mount_dialog_open = !self.mount_dialog_open;
            if self.mount_dialog_open {
                self.bring_to_front(DialogType::Mount);
            }
        }

        // K = 切换英雄对话框
        if is_key_pressed(KeyCode::K) && is_key_down(KeyCode::LeftAlt) {
            self.hero_dialog_open = !self.hero_dialog_open;
            if self.hero_dialog_open {
                self.bring_to_front(DialogType::Hero);
            }
        }

        // Q = 切换钓鱼对话框
        if is_key_pressed(KeyCode::Q) && is_key_down(KeyCode::LeftAlt) {
            self.fishing_dialog_open = !self.fishing_dialog_open;
            if self.fishing_dialog_open {
                self.bring_to_front(DialogType::Fishing);
            }
        }

        // W = 切换智能宠物对话框
        if is_key_pressed(KeyCode::W) && is_key_down(KeyCode::LeftAlt) {
            self.intelligent_creature_dialog_open = !self.intelligent_creature_dialog_open;
            if self.intelligent_creature_dialog_open {
                self.bring_to_front(DialogType::IntelligentCreature);
            }
        }

        // C = 切换罗盘对话框
        if is_key_pressed(KeyCode::C) && is_key_down(KeyCode::LeftAlt) {
            self.compass_dialog_open = !self.compass_dialog_open;
            if self.compass_dialog_open {
                self.bring_to_front(DialogType::Compass);
            }
        }

        // S = 切换宝石镶嵌对话框
        if is_key_pressed(KeyCode::S) && is_key_down(KeyCode::LeftAlt) {
            self.socket_dialog_open = !self.socket_dialog_open;
            if self.socket_dialog_open {
                self.bring_to_front(DialogType::Socket);
            }
        }

        // Tab = 切换技能模式 (Ctrl+~ 的替代)
        // 实际应该检测 Ctrl+~，这里简化为 Tab
        if is_key_pressed(KeyCode::Tab) {
            // 只在非聊天焦点时切换
            if !self.is_any_input_active() {
                self.toggle_skill_mode();
            }
        }

        // Alt+A = 循环攻击模式
        if is_key_pressed(KeyCode::A) && is_key_down(KeyCode::LeftAlt) {
            self.cycle_attack_mode();
        }

        // Alt+P = 循环宠物模式
        if is_key_pressed(KeyCode::P) && is_key_down(KeyCode::LeftAlt) {
            self.cycle_pet_mode();
        }
    }

    /// 循环攻击模式：Peace -> Group -> Guild -> EnemyGuild -> RedBrown -> All -> Peace
    pub fn cycle_attack_mode(&mut self) {
        self.attack_mode = (self.attack_mode + 1) % 6;
    }

    /// 循环宠物模式：Both -> MoveOnly -> AttackOnly -> None -> FocusMasterTarget -> Both
    pub fn cycle_pet_mode(&mut self) {
        self.pet_mode = (self.pet_mode + 1) % 5;
    }

    /// 更新和绘制主界面
    pub fn update_and_draw(&mut self) {
        let screen_w = screen_width() / screen_dpi_scale();
        let screen_h = screen_height() / screen_dpi_scale();

        // 先计算新的 MainDialog 位置，再基于”旧布局”迁移子对话框
        let old_screen_w = self.last_screen_w;
        let old_screen_h = self.last_screen_h;
        let old_main_x = self.last_main_dialog_x;

        self.position = vec2((screen_w - self.bg_size.x) / 2.0, screen_h - self.bg_size.y);
        let new_main_x = self.position.x;

        self.apply_resize_layout(old_screen_w, old_screen_h, old_main_x, screen_w, screen_h, new_main_x);

        self.last_screen_w = screen_w;
        self.last_screen_h = screen_h;
        self.last_main_dialog_x = new_main_x;

        // 处理全局快捷键（攻击模式/宠物模式/技能模式/快捷栏使用）
        if !self.is_any_input_active() && !self.any_popup_open() {
            self.handle_mode_shortcuts();
            self.belt_dialog.handle_number_keys();
        }

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

        // 绘制模式标签（攻击模式/宠物模式/技能模式）
        if self.mode_view {
            self.draw_mode_labels();
        }

        // 绘制角色状态图标（中毒等）
        self.draw_status_icons();

        // 绘制快捷技能栏
        self.draw_quick_skill_bar();
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
                DialogType::Group => self.sync_and_draw_group(&mut consumed, mouse_pos),
                DialogType::Friend => self.sync_and_draw_friend(&mut consumed, mouse_pos),
                DialogType::Guild => self.sync_and_draw_guild(&mut consumed, mouse_pos),
                DialogType::Mentor => self.sync_and_draw_mentor(&mut consumed, mouse_pos),
                DialogType::Relationship => self.sync_and_draw_relationship(&mut consumed, mouse_pos),
                DialogType::Trade => {
                    let trade_action = self.sync_and_draw_trade(&mut consumed, mouse_pos);
                    if !matches!(trade_action, crate::scenes::dialogs::game::trade_dialog::TradeAction::None) {
                        self.pending_trade_action = Some(trade_action);
                    }
                }
                DialogType::Mount => self.sync_and_draw_mount(&mut consumed, mouse_pos),
                DialogType::Hero => self.sync_and_draw_hero(&mut consumed, mouse_pos),
                DialogType::Buff => self.sync_and_draw_buff(&mut consumed, mouse_pos),
                DialogType::Fishing => self.sync_and_draw_fishing(&mut consumed, mouse_pos),
                DialogType::IntelligentCreature => self.sync_and_draw_intelligent_creature(&mut consumed, mouse_pos),
                DialogType::Compass => self.sync_and_draw_compass(&mut consumed, mouse_pos),
                DialogType::Socket => self.sync_and_draw_socket(&mut consumed, mouse_pos),
                DialogType::Mail => self.sync_and_draw_mail(&mut consumed, mouse_pos),
                DialogType::Ranking => self.sync_and_draw_ranking(&mut consumed, mouse_pos),
                DialogType::Help => self.sync_and_draw_help(&mut consumed, mouse_pos),
                DialogType::Inspect => self.sync_and_draw_inspect(&mut consumed, mouse_pos),
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
            DialogType::Group => (self.group_dialog_open, self.group_dialog.contains(mouse_pos)),
            DialogType::Friend => (self.friend_dialog_open, self.friend_dialog.contains(mouse_pos)),
            DialogType::Guild => (self.guild_dialog_open, self.guild_dialog.contains(mouse_pos)),
            DialogType::Mentor => (self.mentor_dialog_open, self.mentor_dialog.contains(mouse_pos)),
            DialogType::Relationship => (self.relationship_dialog_open, self.relationship_dialog.contains(mouse_pos)),
            DialogType::Trade => (self.trade_dialog_open, self.trade_dialog.contains(mouse_pos)),
            DialogType::Mount => (self.mount_dialog_open, self.mount_dialog.contains(mouse_pos)),
            DialogType::Hero => (self.hero_dialog_open, self.hero_dialog.contains(mouse_pos)),
            DialogType::Buff => (self.buff_dialog_open, self.buff_dialog.contains(mouse_pos)),
            DialogType::Fishing => (self.fishing_dialog_open, self.fishing_dialog.contains(mouse_pos)),
            DialogType::IntelligentCreature => (self.intelligent_creature_dialog_open, self.intelligent_creature_dialog.contains(mouse_pos)),
            DialogType::Compass => (self.compass_dialog_open, self.compass_dialog.contains(mouse_pos)),
            DialogType::Socket => (self.socket_dialog_open, self.socket_dialog.contains(mouse_pos)),
            DialogType::Mail => (self.mail_dialog_open, self.mail_dialog.contains(mouse_pos)),
            DialogType::Option => (self.option_dialog_open, self.option_dialog.contains(mouse_pos)),
            DialogType::GameShop => (self.game_shop_dialog_open, self.game_shop_dialog.contains(mouse_pos)),
            DialogType::Menu => (self.menu_dialog_open, self.menu_dialog.contains(mouse_pos)),
            DialogType::MiniMap => (self.minimap_dialog_open, self.minimap_dialog.contains(mouse_pos)),
            DialogType::Ranking => (self.ranking_dialog_open, self.ranking_dialog.contains(mouse_pos)),
            DialogType::Help => (self.help_dialog_open, self.help_dialog.contains(mouse_pos)),
            DialogType::Inspect => (self.inspect_dialog_open, self.inspect_dialog.contains(mouse_pos)),
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
    // 模式标签（攻击模式/宠物模式/技能模式）
    // ========================================================================

    fn draw_mode_labels(&self) {
        // 位置：小地图左侧，垂直排列
        // C#: X = MiniMapDialog.Location.X - 3 - MainDialog.Location.X
        //     Y = MiniMapDialog.Size.Height + 150/165/180 - ScreenHeight
        // 简化：使用固定相对位置（在 MainDialog 右上方）
        let base_x = self.position.x + self.bg_size.x - 60.0;
        let base_y = self.position.y - 60.0; // 在工具栏上方

        // SMode (技能模式) - 最上
        let skill_mode_text = if self.skill_mode { "~" } else { "Ctrl" };
        let s_label = format!("[Skill: {}]", skill_mode_text);
        draw_text_cn(&s_label, base_x, base_y, 10.0, Color::from_rgba(50, 255, 50, 255));

        // AMode (攻击模式) - 中间
        let a_label = self.attack_mode_label();
        let a_color = match self.attack_mode {
            0 => Color::from_rgba(255, 255, 0, 255),    // Peace - Yellow
            1 => Color::from_rgba(100, 255, 100, 255),  // Group - Light green
            2 => Color::from_rgba(100, 150, 255, 255),  // Guild - Blue
            3 => Color::from_rgba(255, 100, 100, 255),  // EnemyGuild - Red
            4 => Color::from_rgba(255, 80, 80, 255),    // RedBrown - Red
            5 => Color::from_rgba(255, 50, 50, 255),    // All - Red
            _ => Color::from_rgba(255, 255, 0, 255),
        };
        draw_text_cn(&a_label, base_x, base_y + 15.0, 10.0, a_color);

        // PMode (宠物模式) - 最下
        if self.pet_mode > 0 || self.has_pet() {
            let p_label = self.pet_mode_label();
            draw_text_cn(&p_label, base_x, base_y + 30.0, 10.0, Color::from_rgba(255, 165, 0, 255));
        }
    }

    fn attack_mode_label(&self) -> String {
        match self.attack_mode {
            0 => "[Mode: Peace]".to_string(),
            1 => "[Mode: Group]".to_string(),
            2 => "[Mode: Guild]".to_string(),
            3 => "[Mode: Enemy]".to_string(),
            4 => "[Mode: PK]".to_string(),
            5 => "[Mode: All]".to_string(),
            _ => format!("[Mode: {}]", self.attack_mode),
        }
    }

    fn pet_mode_label(&self) -> String {
        match self.pet_mode {
            0 => "[Pet: Move+Atk]".to_string(),
            1 => "[Pet: No Atk]".to_string(),
            2 => "[Pet: No Move]".to_string(),
            3 => "[Pet: Idle]".to_string(),
            4 => "[Pet: Focus]".to_string(),
            _ => format!("[Pet: {}]", self.pet_mode),
        }
    }

    fn has_pet(&self) -> bool {
        // 简单判断：如果有宠物实体存在则返回 true
        // 实际应该从 ECS 查询 Pet 组件
        false
    }

    // ========================================================================
    // 角色状态图标（中毒等）
    // ========================================================================

    fn draw_status_icons(&self) {
        if self.poison_level == 0 {
            return;
        }

        // 位置：主工具栏左上方，靠近血球
        let icon_x = self.position.x + 80.0;
        let icon_y = self.position.y + 10.0;

        // 中毒图标：使用 Prguse 纹理库的中毒图标
        let poison_tex_idx = match self.poison_level {
            1 => 1950, // 普通中毒
            2 => 1951, // 重度中毒
            3 => 1952, // 麻痹
            _ => 0,
        };

        if poison_tex_idx > 0 {
            if let Some(info) = LibraryName::Prguse.get_texture(poison_tex_idx) {
                if let Some(tex) = info.image {
                    draw_texture(&tex, icon_x, icon_y, WHITE);
                }
            } else {
                // 降级：用彩色方块表示
                let poison_color = match self.poison_level {
                    1 => Color::from_rgba(150, 255, 0, 200),   // 绿色（轻度中毒）
                    2 => Color::from_rgba(255, 150, 0, 200),   // 橙色（重度中毒）
                    3 => Color::from_rgba(200, 200, 200, 200), // 灰色（麻痹）
                    _ => WHITE,
                };
                draw_rectangle(icon_x, icon_y, 20.0, 20.0, poison_color);
            }
        }
    }

    // ========================================================================
    // 快捷技能栏（功能按钮右侧）
    // ========================================================================

    fn draw_quick_skill_bar(&mut self) {
        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        let bar_y = self.position.y + 76.0; // 与功能按钮同一行
        let icon_size = vec2(20.0, 20.0);

        // 8 个快捷栏位（F1-F8），从功能按钮左侧开始
        for (i, &skill_id) in self.quick_skills.iter().enumerate() {
            let x = self.position.x + self.bg_size.x - 230.0 + i as f32 * 22.0;
            let slot_rect = Rect::new(x, bar_y, icon_size.x, icon_size.y);

            // 背景
            draw_rectangle(x, bar_y, icon_size.x, icon_size.y, Color::from_rgba(40, 40, 50, 200));
            draw_rectangle_lines(x, bar_y, icon_size.x, icon_size.y, 1.0, Color::from_rgba(100, 100, 120, 150));

            // 如果绑定了技能，显示技能图标
            if skill_id > 0 {
                // 技能图标使用 Prguse 纹理库（1800+skill_id 作为索引）
                let tex_idx = 1800 + skill_id as usize;
                if let Some(info) = LibraryName::Prguse.get_texture(tex_idx) {
                    if let Some(tex) = info.image {
                        draw_texture(&tex, x, bar_y, WHITE);
                    }
                } else {
                    // 降级：显示技能 ID
                    draw_text_cn(&format!("{}", skill_id), x + 6.0, bar_y + 6.0, 10.0, WHITE);
                }
            }

            // F1-F8 快捷键提示
            let key_hint = format!("F{}", i + 1);
            draw_text_cn(&key_hint, x + 1.0, bar_y + 1.0, 8.0, Color::from_rgba(200, 200, 200, 180));

            // 悬停高亮
            if slot_rect.contains(mouse_pos) {
                draw_rectangle_lines(x, bar_y, icon_size.x, icon_size.y, 1.0, Color::from_rgba(255, 255, 100, 255));
            }
        }
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
        // ===== Inventory 拖出窗口（跨对话框拖拽） =====
        if let Some((tab, slot, drop_pos)) = self.inventory_dialog.take_drag_out_request() {
            // 检查是否落在 Belt 对话框上
            if self.belt_dialog.is_visible() && self.belt_dialog.contains(drop_pos) {
                if let Some(item) = self.inventory_dialog.take_item_from_slot(tab, slot) {
                    if let Some(icon_index) = item.icon_index {
                        let belt_item = BeltItemHybrid::with_id(icon_index, item.name.clone(), item.count, item.unique_id);
                        if let Err(rollback_item) = self.belt_dialog.try_insert_item(belt_item) {
                            if !self.inventory_dialog.restore_item_to_slot(
                                tab, slot,
                                ItemSlotHybrid::new(rollback_item.icon_index, rollback_item.name.unwrap_or_default(), rollback_item.count),
                            ) {
                                eprintln!("⚠️ Inventory→Belt drag rollback failed: tab={tab:?}, slot={slot}");
                            }
                        }
                    }
                }
            }
            // 检查是否落在 Character 对话框上（穿戴装备）
            else if self.character_dialog_open && self.character_dialog.contains(drop_pos) {
                if let Some(item) = self.inventory_dialog.peek_item_from_slot(tab, slot) {
                    if item.unique_id > 0 {
                        // 暂存装备请求，由 ui_system 发包
                        self.pending_equip_request = Some(item.unique_id);
                    }
                }
            }
            // 拖出到屏幕外 → 物品留在原处（不做任何操作）
            // 未来可在此处添加丢弃确认逻辑
        }

        // ===== Belt 拖出窗口（跨对话框拖拽） =====
        if let Some((slot, drop_pos)) = self.belt_dialog.take_drag_out_request() {
            // 检查是否落在 Inventory 对话框上
            if self.inventory_dialog.is_visible() && self.inventory_dialog.contains(drop_pos) {
                if let Some(item) = self.belt_dialog.take_item_from_slot(slot) {
                    let inventory_item = ItemSlotHybrid::with_id(item.icon_index, item.name.unwrap_or_default(), item.count, item.unique_id);
                    if let Err(rollback_item) = self.inventory_dialog.try_insert_item(inventory_item) {
                        if let Some(icon_index) = rollback_item.icon_index {
                            if !self.belt_dialog.restore_item_to_slot(
                                slot,
                                BeltItemHybrid::with_id(icon_index, rollback_item.name, rollback_item.count, rollback_item.unique_id),
                            ) {
                                eprintln!("⚠️ Belt→Inventory drag rollback failed: slot={slot}");
                            }
                        }
                    }
                }
            }
            // 拖出到屏幕外 → 物品留在原处（不做任何操作）
            // 未来可在此处添加丢弃确认逻辑
        }

        // Inventory → Belt: 右键转移（现有逻辑）
        if self.belt_dialog.is_visible() {
            if let Some((tab, slot)) = self.inventory_dialog.take_transfer_to_belt_request() {
                if let Some(item) = self.inventory_dialog.take_item_from_slot(tab, slot) {
                    if let Some(icon_index) = item.icon_index {
                        let belt_item = BeltItemHybrid::with_id(icon_index, item.name.clone(), item.count, item.unique_id);
                        if let Err(rollback_item) = self.belt_dialog.try_insert_item(belt_item) {
                            if !self.inventory_dialog.restore_item_to_slot(
                                tab,
                                slot,
                                ItemSlotHybrid::new(rollback_item.icon_index, rollback_item.name.unwrap_or_default(), rollback_item.count),
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
        } else {
            let _ = self.inventory_dialog.take_transfer_to_belt_request();
        }

        // Belt → Inventory: 右键转移（现有逻辑）
        if self.inventory_dialog_open {
            if let Some(slot) = self.belt_dialog.take_transfer_to_inventory_request() {
                if let Some(item) = self.belt_dialog.take_item_from_slot(slot) {
                    let inventory_item = ItemSlotHybrid::with_id(item.icon_index, item.name.unwrap_or_default(), item.count, item.unique_id);
                    if let Err(rollback_item) = self.inventory_dialog.try_insert_item(inventory_item) {
                        if let Some(icon_index) = rollback_item.icon_index {
                            if !self.belt_dialog.restore_item_to_slot(
                                slot,
                                BeltItemHybrid::with_id(icon_index, rollback_item.name, rollback_item.count, rollback_item.unique_id),
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
        } else {
            let _ = self.belt_dialog.take_transfer_to_inventory_request();
        }

        // ===== Character 拖出窗口（卸下装备 → 背包） =====
        if let Some((slot, drop_pos)) = self.character_dialog.take_drag_out_request() {
            // 检查是否落在 Inventory 对话框上
            if self.inventory_dialog.is_visible() && self.inventory_dialog.contains(drop_pos) {
                if let Some(equip) = self.character_dialog.equipment[slot].take() {
                    let inv_item = ItemSlotHybrid::with_id(equip.icon_index, equip.name.clone(), 1, equip.unique_id);
                    if let Err(_rollback) = self.inventory_dialog.try_insert_item(inv_item) {
                        // 背包已满，恢复装备
                        self.character_dialog.equipment[slot] = Some(equip);
                        eprintln!("⚠️ Character→Inventory drag failed: inventory full");
                    } else {
                        println!("🎒 卸下装备: {} (槽位{}) → 背包", equip.name, slot);
                    }
                }
            }
            // 拖出到屏幕外 → 装备留在原处（不做任何操作）
            // 未来可在此处添加卸下装备网络请求
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
            self.handle_menu_action(action);
        }
        if !self.menu_dialog.is_visible() {
            self.menu_dialog_open = false;
        }
        if self.menu_dialog_open && self.menu_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 处理菜单对话框触发的动作
    fn handle_menu_action(&mut self, action: crate::scenes::dialogs::game::menu_dialog::MenuAction) {
        use crate::scenes::dialogs::game::menu_dialog::MenuAction;
        match action {
            MenuAction::Mount => {
                self.mount_dialog_open = true;
                self.bring_to_front(DialogType::Mount);
            }
            MenuAction::Friends => {
                self.friend_dialog_open = true;
                self.bring_to_front(DialogType::Friend);
            }
            MenuAction::Mentor => {
                self.mentor_dialog_open = true;
                self.bring_to_front(DialogType::Mentor);
            }
            MenuAction::Group => {
                self.group_dialog_open = true;
                self.bring_to_front(DialogType::Group);
            }
            MenuAction::Guild => {
                self.guild_dialog_open = true;
                self.bring_to_front(DialogType::Guild);
            }
            MenuAction::Relationship => {
                self.relationship_dialog_open = true;
                self.bring_to_front(DialogType::Relationship);
            }
            MenuAction::Exit => {
                println!("❌ 退出游戏");
                std::process::exit(0);
            }
            MenuAction::Logout => {
                self.pending_logout_request = true;
                self.push_system_chat_line("正在安全下线...".to_string());
            }
            MenuAction::Help => {
                self.help_dialog_open = true;
                self.bring_to_front(DialogType::Help);
            }
            MenuAction::Keyboard => {
                println!("⌨️ 键盘设置");
            }
            MenuAction::Ranking => {
                self.ranking_dialog_open = true;
                self.bring_to_front(DialogType::Ranking);
            }
            MenuAction::Creature => {
                self.intelligent_creature_dialog_open = true;
                self.bring_to_front(DialogType::IntelligentCreature);
            }
            MenuAction::Fishing => {
                self.fishing_dialog_open = true;
                self.bring_to_front(DialogType::Fishing);
            }
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

    /// 绘制任务追踪面板（游戏屏幕右侧）
    pub fn draw_quest_tracker(&self) {
        self.quest_log_dialog.draw_quest_tracker(
            screen_width() / screen_dpi_scale() - 230.0,
            60.0,
        );
    }

    /// 绘制任务完成通知
    pub fn draw_quest_notifications(&self) {
        self.quest_log_dialog.draw_completion_notifications();
    }

    /// 获取任务日志对话框的可变引用（用于更新进度等）
    pub fn quest_log_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::quest_log_dialog::QuestLogDialogHybrid {
        &mut self.quest_log_dialog
    }

    fn sync_and_draw_group(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.group_dialog_open {
            self.group_dialog.open();
        } else {
            self.group_dialog.close();
        }
        self.group_dialog.update_and_draw();
        if !self.group_dialog.is_visible() {
            self.group_dialog_open = false;
        }
        if self.group_dialog_open && self.group_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取组队对话框的可变引用
    pub fn group_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::group_dialog::GroupDialogHybrid {
        &mut self.group_dialog
    }

    fn sync_and_draw_friend(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.friend_dialog_open {
            self.friend_dialog.open();
        } else {
            self.friend_dialog.close();
        }
        self.friend_dialog.update_and_draw();
        if !self.friend_dialog.is_visible() {
            self.friend_dialog_open = false;
        }
        if self.friend_dialog_open && self.friend_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取好友对话框的可变引用
    pub fn friend_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::friend_dialog::FriendDialogHybrid {
        &mut self.friend_dialog
    }

    fn sync_and_draw_guild(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.guild_dialog_open {
            self.guild_dialog.open();
        } else {
            self.guild_dialog.close();
        }
        self.guild_dialog.update_and_draw();
        if !self.guild_dialog.is_visible() {
            self.guild_dialog_open = false;
        }
        if self.guild_dialog_open && self.guild_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取行会对话框的可变引用
    pub fn guild_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::guild_dialog::GuildDialogHybrid {
        &mut self.guild_dialog
    }

    fn sync_and_draw_mentor(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.mentor_dialog_open {
            self.mentor_dialog.open();
        } else {
            self.mentor_dialog.close();
        }
        self.mentor_dialog.update_and_draw();
        if !self.mentor_dialog.is_visible() {
            self.mentor_dialog_open = false;
        }
        if self.mentor_dialog_open && self.mentor_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取师徒对话框的可变引用
    pub fn mentor_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::mentor_dialog::MentorDialogHybrid {
        &mut self.mentor_dialog
    }

    fn sync_and_draw_relationship(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.relationship_dialog_open {
            self.relationship_dialog.open();
        } else {
            self.relationship_dialog.close();
        }
        self.relationship_dialog.update_and_draw();
        if !self.relationship_dialog.is_visible() {
            self.relationship_dialog_open = false;
        }
        if self.relationship_dialog_open && self.relationship_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取婚姻对话框的可变引用
    pub fn relationship_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::relationship_dialog::RelationshipDialogHybrid {
        &mut self.relationship_dialog
    }

    fn sync_and_draw_trade(&mut self, consumed: &mut bool, mouse_pos: Vec2) -> crate::scenes::dialogs::game::trade_dialog::TradeAction {
        if self.trade_dialog_open {
            if !self.trade_dialog.is_visible() {
                // 只在从隐藏→可见时初始化，不重置已有的交易内容
                self.trade_dialog.open_trade("");
            }
        } else {
            self.trade_dialog.close();
        }
        let action = self.trade_dialog.update_and_draw();
        if !self.trade_dialog.is_visible() {
            self.trade_dialog_open = false;
        }
        if self.trade_dialog_open && self.trade_dialog.contains(mouse_pos) {
            *consumed = true;
        }
        action
    }

    /// 获取交易对话框的可变引用
    pub fn trade_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::trade_dialog::TradeDialogHybrid {
        &mut self.trade_dialog
    }

    /// 打开交易对话框（由服务器 TradeStarted 事件驱动）
    pub fn open_trade_dialog(&mut self, partner: &str) {
        self.trade_dialog_open = true;
        self.trade_dialog.open_trade(partner);
        self.bring_to_front(DialogType::Trade);
    }

    /// 取出暂存的交易动作（由 ui_system.rs 消费发包）
    pub fn take_pending_trade_action(&mut self) -> Option<crate::scenes::dialogs::game::trade_dialog::TradeAction> {
        self.pending_trade_action.take()
    }

    /// 取出暂存的排行榜刷新请求（由 ui_system.rs 消费发包）
    pub fn take_pending_ranking_refresh_tab(&mut self) -> Option<u8> {
        self.pending_ranking_refresh_tab.take()
    }

    /// 取出暂存的装备请求（由 ui_system.rs 消费发包）
    pub fn take_pending_equip_request(&mut self) -> Option<u64> {
        self.pending_equip_request.take()
    }

    fn sync_and_draw_mount(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.mount_dialog_open {
            self.mount_dialog.open();
        } else {
            self.mount_dialog.close();
        }
        self.mount_dialog.update_and_draw();
        if !self.mount_dialog.is_visible() {
            self.mount_dialog_open = false;
        }
        if self.mount_dialog_open && self.mount_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取坐骑对话框的可变引用
    pub fn mount_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::mount_dialog::MountDialogHybrid {
        &mut self.mount_dialog
    }

    fn sync_and_draw_hero(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.hero_dialog_open {
            self.hero_dialog.open();
        } else {
            self.hero_dialog.close();
        }
        self.hero_dialog.update_and_draw();
        if !self.hero_dialog.is_visible() {
            self.hero_dialog_open = false;
        }
        if self.hero_dialog_open && self.hero_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取英雄对话框的可变引用
    pub fn hero_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::hero_dialog::HeroDialogHybrid {
        &mut self.hero_dialog
    }

    /// 获取增益对话框的可变引用
    pub fn buff_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::buff_dialog::BuffDialogHybrid {
        &mut self.buff_dialog
    }

    fn sync_and_draw_buff(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        let dt = 0.016; // ~60fps
        let screen_w = screen_width() / screen_dpi_scale();
        self.buff_dialog.update_and_draw(dt, screen_w);
        if self.buff_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    fn sync_and_draw_fishing(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.fishing_dialog_open {
            self.fishing_dialog.open();
        } else {
            self.fishing_dialog.close();
        }
        self.fishing_dialog.update_and_draw();
        if !self.fishing_dialog.is_visible() {
            self.fishing_dialog_open = false;
        }
        if self.fishing_dialog_open && self.fishing_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取钓鱼对话框的可变引用
    pub fn fishing_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::fishing_dialog::FishingDialogHybrid {
        &mut self.fishing_dialog
    }

    fn sync_and_draw_intelligent_creature(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.intelligent_creature_dialog_open {
            self.intelligent_creature_dialog.open();
        } else {
            self.intelligent_creature_dialog.close();
        }
        self.intelligent_creature_dialog.update_and_draw();
        if !self.intelligent_creature_dialog.is_visible() {
            self.intelligent_creature_dialog_open = false;
        }
        if self.intelligent_creature_dialog_open && self.intelligent_creature_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取智能宠物对话框的可变引用
    pub fn intelligent_creature_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::intelligent_creature_dialog::IntelligentCreatureDialogHybrid {
        &mut self.intelligent_creature_dialog
    }

    fn sync_and_draw_compass(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.compass_dialog_open {
            self.compass_dialog.open();
        } else {
            self.compass_dialog.close();
        }
        self.compass_dialog.update_and_draw();
        if !self.compass_dialog.is_visible() {
            self.compass_dialog_open = false;
        }
        if self.compass_dialog_open && self.compass_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取罗盘对话框的可变引用
    pub fn compass_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::compass_dialog::CompassDialogHybrid {
        &mut self.compass_dialog
    }

    fn sync_and_draw_socket(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.socket_dialog_open {
            self.socket_dialog.open();
        } else {
            self.socket_dialog.close();
        }
        self.socket_dialog.update_and_draw();
        if !self.socket_dialog.is_visible() {
            self.socket_dialog_open = false;
        }
        if self.socket_dialog_open && self.socket_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取宝石镶嵌对话框的可变引用
    pub fn socket_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::socket_dialog::SocketDialogHybrid {
        &mut self.socket_dialog
    }

    fn sync_and_draw_mail(&mut self, consumed: &mut bool, mouse_pos: Vec2) {
        if self.mail_dialog_open {
            self.mail_dialog.open();
        } else {
            self.mail_dialog.close();
        }
        self.mail_dialog.update_and_draw();
        if !self.mail_dialog.is_visible() {
            self.mail_dialog_open = false;
        }
        if self.mail_dialog_open && self.mail_dialog.contains(mouse_pos) {
            *consumed = true;
        }
    }

    /// 获取邮件对话框的可变引用
    pub fn mail_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::mail_dialog::MailDialogHybrid {
        &mut self.mail_dialog
    }

    /// 获取小地图对话框的可变引用
    pub fn minimap_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::minimap_dialog::MiniMapDialogHybrid {
        &mut self.minimap_dialog
    }

    /// 获取大地图对话框的可变引用
    pub fn big_map_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::big_map_dialog::BigMapDialogHybrid {
        &mut self.big_map_dialog
    }

    /// 获取文本输入对话框的可变引用
    pub fn text_input_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::text_input_dialog::TextInputDialogHybrid {
        &mut self.text_input_dialog
    }

    /// 设置当前文本输入类型
    pub fn set_pending_text_input_kind(&mut self, kind: TextInputKind) {
        self.pending_text_input_kind = kind;
    }

    /// 重置文本输入类型
    pub fn reset_pending_text_input_kind(&mut self) {
        self.pending_text_input_kind = TextInputKind::None;
    }

    /// 获取当前文本输入类型
    pub fn pending_text_input_kind(&self) -> TextInputKind {
        self.pending_text_input_kind.clone()
    }

    /// 文本输入对话框是否可见
    pub fn text_input_is_visible(&self) -> bool {
        self.text_input_dialog.is_visible()
    }

    /// 取出待处理的安全下线请求
    pub fn take_pending_logout(&mut self) -> bool {
        std::mem::replace(&mut self.pending_logout_request, false)
    }

    fn sync_and_draw_ranking(&mut self, _consumed: &mut bool, _mouse_pos: Vec2) {
        if self.ranking_dialog_open {
            self.ranking_dialog.open();
        } else {
            self.ranking_dialog.close();
        }
        self.ranking_dialog.update_and_draw();
        if !self.ranking_dialog.is_visible() {
            self.ranking_dialog_open = false;
        }
        // 处理刷新动作 → 暂存，由 ui_system 消费发包
        match self.ranking_dialog.take_action() {
            crate::scenes::dialogs::game::RankingDialogAction::Refresh { tab } => {
                self.pending_ranking_refresh_tab = Some(tab);
            }
            crate::scenes::dialogs::game::RankingDialogAction::None => {}
        }
    }

    /// 获取排行榜对话框的可变引用
    pub fn ranking_dialog_mut(&mut self) -> &mut crate::scenes::dialogs::game::ranking_dialog::RankingDialogHybrid {
        &mut self.ranking_dialog
    }

    fn sync_and_draw_help(&mut self, _consumed: &mut bool, _mouse_pos: Vec2) {
        if self.help_dialog_open {
            self.help_dialog.open();
        } else {
            self.help_dialog.close();
        }
        self.help_dialog.update_and_draw();
        if !self.help_dialog.is_visible() {
            self.help_dialog_open = false;
        }
    }

    fn sync_and_draw_inspect(&mut self, _consumed: &mut bool, _mouse_pos: Vec2) {
        if self.inspect_dialog_open {
            self.inspect_dialog.open("玩家");
        } else {
            self.inspect_dialog.close();
        }
        self.inspect_dialog.update_and_draw();
        if !self.inspect_dialog.is_visible() {
            self.inspect_dialog_open = false;
        }
    }
}
