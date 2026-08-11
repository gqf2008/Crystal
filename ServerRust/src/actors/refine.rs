// 精炼系统（Refining）
// 纯数据结构，由 WorldActor 调用

/// 精炼状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefineStatus {
    None = 0,
    Pending = 1,    // 等待精炼完成
    Ready = 2,      // 精炼完成，可取回
    Failed = 3,     // 精炼失败
}

/// 正在精炼的物品
#[derive(Debug, Clone)]
pub struct RefiningItem {
    /// 原始物品唯一 ID（背包中的 unique_id）
    pub original_uid: u64,
    /// 精炼物品索引
    pub item_index: u32,
    /// 开始时间（秒）
    pub start_time: u64,
    /// 预计完成时间（秒）
    pub finish_time: u64,
    /// 状态
    pub status: RefineStatus,
    /// 精炼成功率（0-100）
    pub success_chance: u8,
    /// 精炼物品完整数据（C# Info.CurrentRefine；存入时克隆，取回/取消时返还）
    pub item: Option<mir2_shared::data::item::UserItem>,
}

/// 精炼日志（每个玩家一个）
#[derive(Debug, Clone, Default)]
pub struct RefineLog {
    /// 当前正在精炼的物品
    pub active_refine: Option<RefiningItem>,
    /// 精炼历史计数
    pub total_refines: u32,
    /// 成功计数
    pub successful_refines: u32,
}

impl RefineLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// 开始精炼
    pub fn start_refine(&mut self, item_index: u32, current_time: u64, duration_seconds: u64, success_chance: u8) -> RefiningItem {
        let item = RefiningItem {
            original_uid: 0, // 由 WorldActor 设置
            item_index,
            start_time: current_time,
            finish_time: current_time + duration_seconds,
            status: RefineStatus::Pending,
            success_chance,
            item: None,
        };
        self.active_refine = Some(item.clone());
        item
    }

    /// 开始精炼（C# RefineItem：已存入物品转 Pending + finish_time + 成功率，保留 item）
    pub fn begin_refine(&mut self, current_time: u64, duration_seconds: u64, success_chance: u8) -> bool {
        let Some(item) = self.active_refine.as_mut() else { return false };
        if item.status != RefineStatus::None { return false; }
        item.start_time = current_time;
        item.finish_time = current_time + duration_seconds;
        item.status = RefineStatus::Pending;
        item.success_chance = success_chance;
        true
    }

    /// 存入精炼物品（C# RefineItem：完整物品克隆进 CurrentRefine，WorldActor 负责从背包移除）
    pub fn deposit_item(&mut self, item: mir2_shared::data::item::UserItem) -> bool {
        if self.active_refine.is_some() {
            return false; // 已有精炼进行中
        }
        let uid = item.unique_id;
        let item_index = item.item_index as u32;
        self.active_refine = Some(RefiningItem {
            original_uid: uid,
            item_index,
            start_time: 0,
            finish_time: 0,
            status: RefineStatus::None,
            success_chance: 0,
            item: Some(item),
        });
        true
    }

    /// 取消精炼
    pub fn cancel(&mut self) -> Option<RefiningItem> {
        self.active_refine.take()
    }

    /// 完成精炼（返回是否成功）
    pub fn finish(&mut self) -> bool {
        if let Some(ref mut item) = self.active_refine {
            item.status = RefineStatus::Ready;
            self.total_refines += 1;
            // 随机判定：success_chance% 概率成功
            let success = fastrand::u16(0..100) < item.success_chance as u16;
            if success {
                self.successful_refines += 1;
            }
            success
        } else {
            false
        }
    }

    /// 取回精炼物品
    pub fn retrieve(&mut self) -> Option<RefiningItem> {
        self.active_refine.take()
    }

    /// 检查精炼是否完成
    pub fn is_ready(&self, current_time: u64) -> bool {
        if let Some(ref item) = self.active_refine {
            item.status == RefineStatus::Pending && current_time >= item.finish_time
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_refine() {
        let mut log = RefineLog::new();
        let item = log.start_refine(100, 0, 3600, 80);
        assert_eq!(item.item_index, 100);
        assert_eq!(item.status, RefineStatus::Pending);
        assert_eq!(item.success_chance, 80);
        assert!(log.active_refine.is_some());
    }

    #[test]
    fn test_deposit_and_cancel() {
        let mut log = RefineLog::new();
        let mut it = mir2_shared::data::item::UserItem::default();
        it.unique_id = 12345;
        assert!(log.deposit_item(it));
        assert!(log.active_refine.is_some());
        let retrieved = log.cancel();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().original_uid, 12345);
        assert!(log.active_refine.is_none());
    }

    #[test]
    fn test_deposit_twice_fails() {
        let mut log = RefineLog::new();
        let mut it = mir2_shared::data::item::UserItem::default();
        it.unique_id = 1;
        assert!(log.deposit_item(it));
        let mut it2 = mir2_shared::data::item::UserItem::default();
        it2.unique_id = 2;
        assert!(!log.deposit_item(it2)); // 已有物品
    }

    #[test]
    fn test_finish_success_or_fail() {
        // Just test that it doesn't panic and updates status
        let mut log = RefineLog::new();
        log.start_refine(100, 0, 3600, 80);
        let _ = log.finish();
        assert_eq!(log.total_refines, 1);
        if let Some(item) = &log.active_refine {
            assert_eq!(item.status, RefineStatus::Ready);
        }
    }

    #[test]
    fn test_deposit_retrieve_roundtrip_keeps_item() {
        // C# CollectRefine：返还完整物品（唯一 ID / 索引一致）
        let mut log = RefineLog::new();
        let mut it = mir2_shared::data::item::UserItem::default();
        it.unique_id = 77;
        it.item_index = 5;
        assert!(log.deposit_item(it.clone()));
        assert!(log.begin_refine(0, 3600, 80));
        let retrieved = log.retrieve().unwrap();
        assert_eq!(retrieved.original_uid, 77);
        assert_eq!(retrieved.item.as_ref().unwrap().unique_id, 77);
        assert_eq!(retrieved.item.as_ref().unwrap().item_index, 5);
    }

    #[test]
    fn test_begin_refine_requires_deposit() {
        let mut log = RefineLog::new();
        assert!(!log.begin_refine(0, 3600, 80));
    }

    #[test]
    fn test_is_ready() {
        let mut log = RefineLog::new();
        assert!(!log.is_ready(0));

        log.start_refine(100, 0, 3600, 80);
        assert!(!log.is_ready(1000)); // Not ready yet
        assert!(log.is_ready(3600));  // Ready now
    }

    #[test]
    fn test_retrieve() {
        let mut log = RefineLog::new();
        log.start_refine(100, 0, 3600, 80);
        let item = log.retrieve();
        assert!(item.is_some());
        assert!(log.active_refine.is_none());
    }
}
