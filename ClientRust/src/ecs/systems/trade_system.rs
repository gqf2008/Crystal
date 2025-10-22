// ============================================================================
// 交易系统 - 处理玩家交易和商店
// ============================================================================

use hecs::World;
use crate::ecs::components::{LocalPlayer, Inventory};
use crate::network::NetworkCommand;
use tokio::sync::mpsc;
use mir2_shared::data::item::UserItem;

/// 交易状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TradeState {
    Idle,           // 空闲
    Requesting,     // 发起请求
    Accepting,      // 等待接受
    Trading,        // 交易中
    Locked,         // 已锁定
    Confirming,     // 等待确认
}

/// 交易数据
#[derive(Debug, Clone)]
pub struct TradeData {
    pub partner_id: u32,
    pub partner_name: String,
    pub state: TradeState,
    pub my_items: Vec<(u8, UserItem)>,  // (格子位置, 物品)
    pub my_gold: u32,
    pub partner_items: Vec<UserItem>,
    pub partner_gold: u32,
    pub my_locked: bool,
    pub partner_locked: bool,
}

impl TradeData {
    pub fn new(partner_id: u32, partner_name: String) -> Self {
        Self {
            partner_id,
            partner_name,
            state: TradeState::Trading,
            my_items: Vec::new(),
            my_gold: 0,
            partner_items: Vec::new(),
            partner_gold: 0,
            my_locked: false,
            partner_locked: false,
        }
    }
    
    /// 添加我的物品
    pub fn add_my_item(&mut self, slot: u8, item: UserItem) -> bool {
        if self.my_locked {
            return false;
        }
        
        // 检查是否已经添加
        if self.my_items.iter().any(|(s, _)| *s == slot) {
            return false;
        }
        
        self.my_items.push((slot, item));
        true
    }
    
    /// 移除我的物品
    pub fn remove_my_item(&mut self, slot: u8) -> bool {
        if self.my_locked {
            return false;
        }
        
        if let Some(index) = self.my_items.iter().position(|(s, _)| *s == slot) {
            self.my_items.remove(index);
            true
        } else {
            false
        }
    }
    
    /// 设置我的金币
    pub fn set_my_gold(&mut self, gold: u32) -> bool {
        if self.my_locked {
            return false;
        }
        
        self.my_gold = gold;
        true
    }
}

/// 交易组件 (挂载到玩家实体上)
#[derive(Debug, Clone)]
pub struct TradeWindow {
    pub active_trade: Option<TradeData>,
}

impl TradeWindow {
    pub fn new() -> Self {
        Self {
            active_trade: None,
        }
    }
    
    /// 开始交易
    pub fn start_trade(&mut self, partner_id: u32, partner_name: String) {
        self.active_trade = Some(TradeData::new(partner_id, partner_name));
    }
    
    /// 关闭交易
    pub fn close_trade(&mut self) {
        self.active_trade = None;
    }
    
    /// 是否在交易中
    pub fn is_trading(&self) -> bool {
        self.active_trade.is_some()
    }
}

/// 商店物品
#[derive(Debug, Clone)]
pub struct ShopItem {
    pub item_id: u32,
    pub item_name: String,
    pub price: u32,
    pub stock: Option<u32>,  // None = 无限库存
}

/// 商店数据
#[derive(Debug, Clone)]
pub struct ShopData {
    pub npc_id: u32,
    pub shop_name: String,
    pub items: Vec<ShopItem>,
    pub buy_rate: f32,   // 购买倍率
    pub sell_rate: f32,  // 出售倍率
}

/// 交易系统
pub struct TradeSystem;

impl TradeSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 发起交易请求 (匹配C# Network.Enqueue(new C.TradeRequest()))
    pub fn request_trade(
        world: &World,
        target_name: String,  // 仅用于UI显示
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        println!("🤝 发起交易请求: {}", target_name);
        
        // ✅ 发送空命令，服务器从点击事件获取目标
        network_tx.send(NetworkCommand::TradeRequest)
            .map_err(|e| format!("发送TradeRequest失败: {}", e))?;
        Ok(())
    }
    
    /// 接受交易请求
    pub fn accept_trade(
        world: &mut World,
        partner_id: u32,
        partner_name: String,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        println!("✅ 接受交易: {}", partner_name);
        
        // 创建交易窗口
        for (_, (_, trade_window)) in world.query_mut::<(&LocalPlayer, &mut TradeWindow)>() {
            trade_window.start_trade(partner_id, partner_name.clone());
            break;
        }
        
        // 发送网络命令
        network_tx.send(NetworkCommand::TradeReply { accept_invite: true })
            .map_err(|e| format!("发送TradeReply失败: {}", e))?;
        Ok(())
    }
    
    /// 拒绝交易请求
    pub fn decline_trade(
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        println!("❌ 拒绝交易");
        
        // 发送网络命令
        network_tx.send(NetworkCommand::TradeReply { accept_invite: false })
            .map_err(|e| format!("发送TradeReply失败: {}", e))?;
        Ok(())
    }
    
    /// 添加交易物品
    pub fn add_trade_item(
        world: &mut World,
        slot: u8,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        for (_, (_, inventory, trade_window)) in world.query_mut::<(&LocalPlayer, &Inventory, &mut TradeWindow)>() {
            if let Some(ref mut trade) = trade_window.active_trade {
                if let Some(item) = &inventory.items[slot as usize] {
                    if trade.add_my_item(slot, item.clone()) {
                        println!("➕ 添加交易物品: 格子 {}", slot);
                        
                        // 使用MoveItem命令，grid为Trade，from为背包格子，to为交易窗口格子
                        // 注意：这里简化处理，实际可能需要trade窗口的具体格子索引
                        network_tx.send(NetworkCommand::MoveItem { 
                            grid: 3, // MirGridType::Trade
                            from: slot as i32, 
                            to: 0 // 交易窗口第一个空格子，实际应该由服务器分配
                        }).map_err(|e| format!("发送MoveItem失败: {}", e))?;
                    }
                }
            }
            break;
        }
        Ok(())
    }
    
    /// 移除交易物品
    pub fn remove_trade_item(
        world: &mut World,
        slot: u8,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        for (_, (_, trade_window)) in world.query_mut::<(&LocalPlayer, &mut TradeWindow)>() {
            if let Some(ref mut trade) = trade_window.active_trade {
                if trade.remove_my_item(slot) {
                    println!("➖ 移除交易物品: 格子 {}", slot);
                    
                    // 使用MoveItem将物品从交易窗口移回背包
                    network_tx.send(NetworkCommand::MoveItem { 
                        grid: 3, // MirGridType::Trade
                        from: slot as i32, 
                        to: -1 // -1 表示移回背包，让服务器自动分配格子
                    }).map_err(|e| format!("发送MoveItem失败: {}", e))?;
                }
            }
            break;
        }
        Ok(())
    }
    
    /// 设置交易金币
    pub fn set_trade_gold(
        world: &mut World,
        gold: u32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        // 先检查玩家金币是否足够
        let mut player_gold = 0;
        for (_, (_, inventory)) in world.query::<(&LocalPlayer, &Inventory)>().iter() {
            player_gold = inventory.gold;
            break;
        }
        
        if gold > player_gold {
            return Err(format!("金币不足: 尝试交易{}金币,但只有{}金币", gold, player_gold));
        }
        
        // 设置交易金币
        for (_, (_, trade_window)) in world.query_mut::<(&LocalPlayer, &mut TradeWindow)>() {
            if let Some(ref mut trade) = trade_window.active_trade {
                if trade.set_my_gold(gold) {
                    println!("💰 设置交易金币: {}", gold);
                    
                    network_tx.send(NetworkCommand::TradeGold { amount: gold })
                        .map_err(|e| format!("发送TradeGold失败: {}", e))?;
                }
            }
            break;
        }
        Ok(())
    }
    
    /// 切换交易锁定状态 (匹配C# ChangeLockState(!GameScene.User.TradeLocked))
    pub fn toggle_trade_lock(
        world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<bool, String> {
        for (_, (_, trade_window)) in world.query_mut::<(&LocalPlayer, &mut TradeWindow)>() {
            if let Some(ref mut trade) = trade_window.active_trade {
                // ✅ 切换锁定状态而非单向设置
                trade.my_locked = !trade.my_locked;
                
                if trade.my_locked {
                    trade.state = TradeState::Locked;
                    println!("🔒 锁定交易");
                } else {
                    trade.state = TradeState::Trading;
                    println!("🔓 解锁交易");
                }
                
                // 发送当前锁定状态到服务器
                network_tx.send(NetworkCommand::TradeConfirm { 
                    locked: trade.my_locked 
                }).map_err(|e| format!("发送TradeConfirm失败: {}", e))?;
                
                return Ok(trade.my_locked);
            }
            break;
        }
        Ok(false)
    }
    
    /// 确认交易
    pub fn confirm_trade(
        world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        for (_, (_, trade_window)) in world.query_mut::<(&LocalPlayer, &mut TradeWindow)>() {
            if let Some(ref trade) = trade_window.active_trade {
                if trade.my_locked && trade.partner_locked {
                    println!("✔️ 确认交易");
                    
                    // 最终确认，完成交易
                    network_tx.send(NetworkCommand::TradeConfirm { locked: true })
                        .map_err(|e| format!("发送TradeConfirm失败: {}", e))?;
                }
            }
            break;
        }
        Ok(())
    }
    
    /// 取消交易
    pub fn cancel_trade(
        world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        for (_, (_, trade_window)) in world.query_mut::<(&LocalPlayer, &mut TradeWindow)>() {
            if trade_window.active_trade.is_some() {
                println!("🚫 取消交易");
                trade_window.close_trade();
                
                network_tx.send(NetworkCommand::TradeCancel)
                    .map_err(|e| format!("发送TradeCancel失败: {}", e))?;
            }
            break;
        }
        Ok(())
    }
}

/// 商店系统
pub struct ShopSystem;

impl ShopSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 购买物品
    pub fn buy_item(
        item_index: u64,
        quantity: u16,
        panel_type: u8,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        println!("🛒 购买物品: ID={}, 数量={}", item_index, quantity);
        
        network_tx.send(NetworkCommand::BuyItem { 
            item_index, 
            count: quantity,
            panel_type,
        }).map_err(|e| format!("发送BuyItem失败: {}", e))?;
        Ok(())
    }
    
    /// 出售物品
    pub fn sell_item(
        unique_id: u64,
        quantity: u16,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        println!("💵 出售物品: unique_id={}, 数量={}", unique_id, quantity);
        
        network_tx.send(NetworkCommand::SellItem { 
            unique_id, 
            count: quantity,
        }).map_err(|e| format!("发送SellItem失败: {}", e))?;
        Ok(())
    }
    
    /// 修理物品
    pub fn repair_item(
        unique_id: u64,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        println!("� 修理物品: unique_id={}", unique_id);
        
        network_tx.send(NetworkCommand::RepairItem { unique_id })
            .map_err(|e| format!("发送RepairItem失败: {}", e))?;
        Ok(())
    }
}
