//! 商城示例物品数据

use super::types::{ShopItemHybrid, ShopCategoryHybrid, ShopClassHybrid};

/// 创建示例商品数据
pub fn create_sample_items() -> Vec<ShopItemHybrid> {
    vec![
        ShopItemHybrid {
            id: 1, name: "龙纹剑".into(), description: "攻击力+50".into(),
            icon_index: 1, price_gold: 100000, price_ingot: 500,
            class: ShopClassHybrid::Warrior,
            category: ShopCategoryHybrid::Weapon, in_stock: true,
            hot: true, new: false, deal: false, days_ago: 30,
            stock: 10, count: 1,
        },
        ShopItemHybrid {
            id: 2, name: "天师道袍".into(), description: "魔防+30".into(),
            icon_index: 20, price_gold: 80000, price_ingot: 400,
            class: ShopClassHybrid::Taoist,
            category: ShopCategoryHybrid::Armor, in_stock: true,
            hot: false, new: true, deal: false, days_ago: 2,
            stock: 5, count: 1,
        },
        ShopItemHybrid {
            id: 3, name: "强效金疮药".into(), description: "恢复500HP".into(),
            icon_index: 40, price_gold: 1000, price_ingot: 5,
            class: ShopClassHybrid::All,
            category: ShopCategoryHybrid::Potion, in_stock: true,
            hot: false, new: false, deal: true, days_ago: 90,
            stock: 0, count: 10,
        },
        ShopItemHybrid {
            id: 4, name: "传送戒指".into(), description: "随机传送".into(),
            icon_index: 60, price_gold: 500000, price_ingot: 2000,
            class: ShopClassHybrid::All,
            category: ShopCategoryHybrid::Special, in_stock: false,
            hot: true, new: true, deal: true, days_ago: 1,
            stock: 0, count: 1,
        },
        ShopItemHybrid {
            id: 5, name: "华丽时装".into(), description: "外观装饰".into(),
            icon_index: 80, price_gold: 0, price_ingot: 1000,
            class: ShopClassHybrid::Archer,
            category: ShopCategoryHybrid::Fashion, in_stock: true,
            hot: false, new: true, deal: false, days_ago: 5,
            stock: 20, count: 1,
        },
        ShopItemHybrid {
            id: 6, name: "裁决之杖".into(), description: "攻击力+80".into(),
            icon_index: 5, price_gold: 200000, price_ingot: 1000,
            class: ShopClassHybrid::Warrior,
            category: ShopCategoryHybrid::Weapon, in_stock: true,
            hot: true, new: false, deal: true, days_ago: 14,
            stock: 3, count: 1,
        },
        ShopItemHybrid {
            id: 7, name: "法神披风".into(), description: "魔攻+40".into(),
            icon_index: 25, price_gold: 150000, price_ingot: 750,
            class: ShopClassHybrid::Wizard,
            category: ShopCategoryHybrid::Armor, in_stock: true,
            hot: false, new: false, deal: false, days_ago: 60,
            stock: 8, count: 1,
        },
        ShopItemHybrid {
            id: 8, name: "太阳水".into(), description: "恢复300MP".into(),
            icon_index: 45, price_gold: 800, price_ingot: 4,
            class: ShopClassHybrid::All,
            category: ShopCategoryHybrid::Potion, in_stock: true,
            hot: false, new: false, deal: false, days_ago: 120,
            stock: 0, count: 10,
        },
        ShopItemHybrid {
            id: 9, name: "复活戒指".into(), description: "死亡复活".into(),
            icon_index: 65, price_gold: 1000000, price_ingot: 5000,
            class: ShopClassHybrid::Assassin,
            category: ShopCategoryHybrid::Special, in_stock: true,
            hot: true, new: true, deal: false, days_ago: 3,
            stock: 1, count: 1,
        },
        ShopItemHybrid {
            id: 10, name: "新年套装".into(), description: "限定外观".into(),
            icon_index: 85, price_gold: 0, price_ingot: 2000,
            class: ShopClassHybrid::Archer,
            category: ShopCategoryHybrid::Fashion, in_stock: true,
            hot: true, new: true, deal: true, days_ago: 6,
            stock: 50, count: 1,
        },
    ]
}
