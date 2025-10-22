// 网络命令 - UI线程发送给网络线程的命令

use mir2_shared::packets::client;

/// Commands that can be sent from UI thread to network thread
#[derive(Debug, Clone)]
pub enum NetworkCommand {
    /// Send login packet
    Login {
        username: String,
        password: String,
    },
    
    /// Create new account
    NewAccount {
        account_id: String,
        password: String,
        birth_date: i64,  // Unix timestamp (C# DateTime.ToBinary())
        username: String,
        secret_question: String,
        secret_answer: String,
        email: String,
    },
    
    /// Change password
    ChangePassword {
        account_id: String,
        current_password: String,
        new_password: String,
    },
    
    /// Select character
    SelectCharacter {
        index: i32,
    },
    
    /// Create new character
    NewCharacter {
        name: String,
        class: u8,
        gender: u8,
    },
    
    /// Delete character
    DeleteCharacter {
        index: i32,
    },
    
    /// Start game with selected character
    StartGame {
        character_index: i32,
    },
    
    /// Walk in direction
    Walk {
        direction: mir2_shared::enums::MirDirection,
    },
    
    /// Run in direction
    Run {
        direction: mir2_shared::enums::MirDirection,
    },
    
    /// Turn to direction (without moving)
    Turn {
        direction: mir2_shared::enums::MirDirection,
    },
    
    /// Send movement command (Walk/Run) - deprecated, use Walk/Run instead
    Move {
        direction: u8,  // MirDirection as u8
        location: (i32, i32),  // (x, y)
    },
    
    /// Attack in direction
    Attack {
        direction: mir2_shared::enums::MirDirection,
        spell: mir2_shared::enums::Spell,
    },
    
    /// Cast magic spell
    Magic {
        spell: u8,                    // Spell type as u8
        direction: mir2_shared::enums::MirDirection, // Spell direction
        target_id: u32,               // Target object ID (0 for no target)
        location: Option<(i32, i32)>, // Target location for ground spells
    },
    
    /// Pickup item at location
    PickupItem {
        location: (i32, i32),  // (x, y)
    },
    
    /// Move item between inventories/grids
    MoveItem {
        grid: u8,      // MirGridType as u8
        from: i32,     // Source slot
        to: i32,       // Destination slot
    },
    
    // ========================================================================
    // Quest System Commands
    // ========================================================================
    
    /// Accept quest from NPC
    AcceptQuest {
        npc_index: u32,
        quest_index: i32,
    },
    
    /// Finish (turn in) quest
    FinishQuest {
        quest_index: i32,
        selected_item_index: i32,  // For quests with item choice rewards
    },
    
    /// Abandon quest
    AbandonQuest {
        quest_index: i32,
    },
    
    /// Share quest with party members
    ShareQuest {
        quest_index: i32,
    },
    
    // ========================================================================
    // Trade System Commands
    // ========================================================================
    
    /// Request trade with another player (server gets target from click)
    /// Note: C# version is empty packet, server determines target from click event
    TradeRequest,
    
    /// Reply to trade request
    TradeReply {
        accept_invite: bool,
    },
    
    /// Add gold to trade window
    TradeGold {
        amount: u32,
    },
    
    /// Confirm/Lock trade (ready to finalize)
    TradeConfirm {
        locked: bool,
    },
    
    /// Cancel trade
    TradeCancel,
    
    // ========================================================================
    // Shop/NPC System Commands
    // ========================================================================
    
    /// Buy item from NPC shop
    BuyItem {
        item_index: u64,  // Item's unique ID in shop
        count: u16,       // Quantity to buy
        panel_type: u8,   // PanelType: Buy(0), Sell(1), etc.
    },
    
    /// Sell item to NPC
    SellItem {
        unique_id: u64,   // Item's unique ID
        count: u16,       // Quantity to sell
    },
    
    /// Repair item at NPC
    RepairItem {
        unique_id: u64,   // Item's unique ID
    },
    
    /// Disconnect from server
    Disconnect,
}
