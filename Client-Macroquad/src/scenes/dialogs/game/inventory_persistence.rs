// ============================================================================
// 背包数据持久化模块
// ============================================================================

use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::fs;
use super::inventory_dialog::{InventoryDialog, ItemSlot, InventoryTab};

/// 背包持久化数据（用于保存/加载）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryData {
    /// 物品格子数据（80格）
    pub item_slots: Vec<ItemSlot>,
    /// 任务物品格子数据（40格）
    pub quest_slots: Vec<ItemSlot>,
    /// 背包最大容量
    pub max_capacity: usize,
    /// 金币数量
    pub gold: u32,
    /// 当前负重 / 最大负重
    pub weight: (u32, u32),
    /// 当前活跃标签页
    pub active_tab: InventoryTab,
    /// 数据版本号（用于未来兼容性）
    pub version: u32,
}

impl InventoryData {
    /// 当前数据版本
    const CURRENT_VERSION: u32 = 1;
    
    /// 获取默认保存路径
    pub fn get_save_path() -> PathBuf {
        // 使用用户数据目录
        let mut path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        path.push("Mir2Client");
        path.push("inventory.json");
        path
    }
    
    /// 从背包对话框创建数据快照
    pub fn from_dialog(dialog: &InventoryDialog) -> Self {
        Self {
            item_slots: dialog.item_slots.clone(),
            quest_slots: dialog.quest_slots.clone(),
            max_capacity: dialog.max_capacity,
            gold: dialog.gold,
            weight: dialog.weight,
            active_tab: dialog.active_tab,
            version: Self::CURRENT_VERSION,
        }
    }
    
    /// 保存到文件
    pub fn save_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // 序列化为JSON
        let json = serde_json::to_string_pretty(self)?;
        
        // 写入文件
        fs::write(path, json)?;
        
        println!("💾 背包数据已保存到: {:?}", path);
        Ok(())
    }
    
    /// 从文件加载
    pub fn load_from_file(path: &PathBuf) -> anyhow::Result<Self> {
        // 读取文件
        let json = fs::read_to_string(path)?;
        
        // 反序列化
        let data: InventoryData = serde_json::from_str(&json)?;
        
        // 版本检查（未来可以处理版本迁移）
        if data.version != Self::CURRENT_VERSION {
            println!("⚠️ 数据版本不匹配: {} vs {}", data.version, Self::CURRENT_VERSION);
        }
        
        println!("📂 背包数据已加载自: {:?}", path);
        Ok(data)
    }
    
    /// 应用到背包对话框
    pub fn apply_to_dialog(&self, dialog: &mut InventoryDialog) {
        dialog.item_slots = self.item_slots.clone();
        dialog.quest_slots = self.quest_slots.clone();
        dialog.max_capacity = self.max_capacity;
        dialog.gold = self.gold;
        dialog.weight = self.weight;
        dialog.active_tab = self.active_tab;
        
        println!("✅ 背包数据已应用: {} 个物品格子, {} 金币", self.max_capacity, self.gold);
    }
}
