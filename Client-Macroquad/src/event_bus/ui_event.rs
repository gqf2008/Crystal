// ============================================================================
// UIEvent - UI事件定义
// ============================================================================
//
// 职责：
// - 定义所有UI交互事件
// - 由 UI系统 产生，被 逻辑系统/网络系统 消费
//
// 设计原则：
// - 用户导向：描述用户在UI上的操作
// - 解耦UI实现：不暴露UI内部细节

use mir2_shared::enums::ChatType;

// ============================================================================
// UI事件枚举
// ============================================================================

#[derive(Debug, Clone)]
pub enum UIEvent {
    // ========================================================================
    // 按钮点击事件
    // ========================================================================
    /// 通用按钮点击
    ButtonClicked { button_id: String },

    /// 技能按钮点击
    SkillButtonClicked { slot: u32, spell_id: u8 },

    /// 快捷栏按钮点击
    QuickSlotClicked { slot: u32 },

    // ========================================================================
    // 对话框事件
    // ========================================================================
    /// 对话框确认
    DialogConfirmed { dialog_id: String, choice: i32 },

    /// 对话框取消
    DialogCancelled { dialog_id: String },

    /// NPC对话选项
    NpcDialogueChoice { npc_id: u32, choice_index: i32 },

    // ========================================================================
    // 物品栏事件
    // ========================================================================
    /// 物品栏格子点击
    InventorySlotClicked {
        slot: u32,
        item_id: Option<u64>,
        button: ClickType,
    },

    /// 物品拖拽开始
    ItemDragStart {
        from_grid: GridType,
        from_slot: u32,
        item_id: u64,
    },

    /// 物品拖拽结束
    ItemDragEnd { to_grid: GridType, to_slot: u32 },

    /// 物品拖拽取消
    ItemDragCancel { item_id: u64 },

    /// 物品右键菜单
    ItemContextMenu { item_id: u64, slot: u32 },

    /// 物品使用
    ItemUseRequest { item_id: u64, slot: u32 },

    /// 物品丢弃
    ItemDropRequest { item_id: u64, count: u32 },

    // ========================================================================
    // 装备事件
    // ========================================================================
    /// 装备物品
    EquipItemRequest {
        item_id: u64,
        from_slot: u32,
        to_slot: u8,
    },

    /// 卸下装备
    UnequipItemRequest { item_id: u64, from_slot: u8 },

    // ========================================================================
    // 聊天事件
    // ========================================================================
    /// 发送聊天消息
    ChatMessageSent {
        message: String,
        chat_type: ChatType,
    },

    /// 切换聊天频道
    ChatChannelChanged { new_channel: ChatType },

    /// 私聊目标选择
    WhisperTargetSelected { target_name: String },

    // ========================================================================
    // 窗口事件
    // ========================================================================
    /// 打开窗口
    WindowOpened { window_type: WindowType },

    /// 关闭窗口
    WindowClosed { window_type: WindowType },

    /// 窗口标签切换
    WindowTabChanged {
        window_type: WindowType,
        tab_index: u32,
    },

    // ========================================================================
    // 角色事件
    // ========================================================================
    /// 打开角色面板
    CharacterPanelOpened,

    /// 关闭角色面板
    CharacterPanelClosed,

    /// 属性点分配
    AttributePointAllocated {
        attribute: AttributeType,
        points: i32,
    },

    // ========================================================================
    // 地图事件
    // ========================================================================
    /// 小地图点击
    MinimapClicked { world_x: i32, world_y: i32 },

    /// 大地图打开
    WorldMapOpened,

    /// 大地图关闭
    WorldMapClosed,

    // ========================================================================
    // 交易事件
    // ========================================================================
    /// 发起交易请求
    TradeRequestSent { target_player: String },

    /// 接受交易
    TradeAccepted,

    /// 拒绝交易
    TradeDeclined,

    /// 交易确认
    TradeConfirmed,

    /// 交易取消
    TradeCancelled,

    /// 放入交易物品
    TradeItemAdded { item_id: u64 },

    /// 设置交易金币
    TradeGoldSet { amount: u32 },

    // ========================================================================
    // 商店事件
    // ========================================================================
    /// 购买物品
    ShopBuyRequest { item_index: u32, count: u32 },

    /// 出售物品
    ShopSellRequest { item_id: u64, count: u32 },

    /// 修理物品
    ShopRepairRequest { item_id: u64 },

    // ========================================================================
    // 任务事件
    // ========================================================================
    /// 接受任务
    QuestAccepted { quest_id: u32 },

    /// 完成任务
    QuestCompleted {
        quest_id: u32,
        reward_choice: Option<u32>,
    },

    /// 放弃任务
    QuestAbandoned { quest_id: u32 },

    /// 打开任务日志
    QuestLogOpened,

    // ========================================================================
    // 社交事件
    // ========================================================================
    /// 添加好友
    FriendAddRequest { player_name: String },

    /// 删除好友
    FriendRemoveRequest { player_name: String },

    /// 组队邀请
    PartyInviteSent { player_name: String },

    /// 公会邀请
    GuildInviteSent { player_name: String },

    // ========================================================================
    // 系统事件
    // ========================================================================
    /// 打开设置面板
    SettingsOpened,

    /// 关闭设置面板
    SettingsClosed,

    /// 设置项改变
    SettingChanged { key: String, value: String },

    /// 登出请求
    LogoutRequest,

    /// 退出游戏请求
    QuitGameRequest,
}

// ============================================================================
// 辅助枚举
// ============================================================================

/// 点击类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickType {
    Left,
    Right,
    Middle,
}

/// 网格类型（对应不同的物品容器）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridType {
    Inventory, // 背包
    Equipment, // 装备栏
    Storage,   // 仓库
    Trade,     // 交易栏
    Shop,      // 商店
    Quest,     // 任务物品
}

/// 窗口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    Character, // 角色面板
    Inventory, // 背包
    Skills,    // 技能面板
    Quest,     // 任务日志
    Map,       // 地图
    Social,    // 社交面板
    Guild,     // 公会面板
    Trade,     // 交易窗口
    Shop,      // 商店窗口
    Settings,  // 设置面板
}

/// 属性类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeType {
    Strength,     // 力量
    Agility,      // 敏捷
    Intelligence, // 智力
    Vitality,     // 体质
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_types() {
        let inv = GridType::Inventory;
        let equip = GridType::Equipment;

        assert_ne!(inv, equip);
        assert_eq!(inv, GridType::Inventory);
    }
}
