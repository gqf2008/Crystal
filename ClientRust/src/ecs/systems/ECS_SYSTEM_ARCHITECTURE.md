# 熱血傳奇完整 ECS 架構表

## 🏗️ 總體架構分層

| 層級 | 優先級範圍 | 主要職責 | 系統數量 |
|------|------------|----------|----------|
| 第0層：基礎設施 | 0-99 | 資源管理、存檔、配置 | 6個 |
| 第1層：輸入網絡 | 100-299 | 輸入處理、網絡通信 | 8個 |
| 第2層：遊戲邏輯 | 300-599 | 核心遊戲玩法、AI、經濟 | 26個 |
| 第3層：表現層 | 600-899 | 動畫、特效、UI、攝像機 | 12個 |
| 第4層：渲染層 | 900-1999 | 圖形渲染、後處理 | 8個 |
| 第5層：調試工具 | 9000+ | 開發調試、性能分析 | 4個 |

## 📋 完整系統詳細表

### 第0層：基礎設施系統 (0-99)
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `SceneSystem` | 10 | 全部 | 場景狀態管理、場景切換邏輯 |
| `ResourcePreloadSystem` | 20 | 全部 | 資源預加載、內存管理 |
| `SaveSystem` | 30 | 遊戲 | 自動存檔、數據序列化 |
| `ConfigSystem` | 40 | 全部 | 配置加載、熱更新管理 |
| `LocalizationSystem` | 50 | 全部 | 多語言支持、文本本地化 |
| `PerformanceMonitorSystem` | 60 | 全部 | 性能監控、幀率統計 |

### 第1層：輸入和網絡系統 (100-299)
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `InputCollectSystem` | 100 | 全部 | 收集鍵盤、鼠標、觸控輸入 |
| `NetworkReceiveSystem` | 110 | 遊戲 | 接收和解析網絡數據包 |
| `PlayerControlSystem` | 120 | 遊戲 | 玩家控制邏輯、指令轉換 |
| `ChatSystem` | 130 | 遊戲 | 聊天消息處理、過濾 |
| `ReconnectionSystem` | 140 | 全部 | 網絡重連、數據同步 |
| `HeartbeatSystem` | 150 | 遊戲 | 心跳包發送、延遲檢測 |
| `PacketCompressSystem` | 160 | 遊戲 | 數據包壓縮、加密 |
| `NetworkSendSystem` | 200 | 遊戲 | 發送網絡數據到服務器 |

### 第2層：遊戲邏輯系統 (300-599)

#### AI 和行為系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `MonsterAISystem` | 300 | 遊戲 | 怪物AI、行為決策、狀態切換 |
| `NPCInteractionSystem` | 310 | 遊戲 | NPC對話、任務觸發、商店交互 |
| `PetAISystem` | 320 | 遊戲 | 寵物跟隨、自動戰鬥、忠誠度 |

#### 社交系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `GuildSystem` | 330 | 遊戲 | 行會管理、行會戰爭、技能 |
| `PartySystem` | 340 | 遊戲 | 組隊管理、經驗分配、隊伍BUFF |
| `FriendSystem` | 350 | 遊戲 | 好友列表、狀態同步、私聊 |

#### 傳奇特色系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `PKSystem` | 355 | 遊戲 | PK模式、罪惡值、紅名懲罰 |
| `DungeonSystem` | 360 | 遊戲 | 副本進入、進度保存、獎勵 |
| `BossSystem` | 365 | 遊戲 | BOSS刷新、歸屬判斷、掉落 |
| `SiegeWarSystem` | 368 | 遊戲 | 沙巴克攻城、佔領獎勵 |

#### 任務系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `QuestSystem` | 370 | 遊戲 | 任務接取、進度跟踪、獎勵 |
| `DailySystem` | 375 | 遊戲 | 每日任務、簽到、活躍度 |
| `AchievementSystem` | 380 | 遊戲 | 成就追踪、獎勵發放 |

#### 戰鬥系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `CombatSystem` | 400 | 遊戲 | 戰鬥邏輯、傷害計算、死亡處理 |
| `SkillSystem` | 410 | 遊戲 | 技能釋放、冷卻、效果應用 |
| `BuffDebuffSystem` | 420 | 遊戲 | BUFF/DEBUFF管理、定時效果 |
| `RegenSystem` | 430 | 遊戲 | **生命/魔法恢復、DoT傷害、Buff過期** |

#### 職業系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `ClassSystem` | 440 | 遊戲 | 職業特性、轉職任務、職業平衡 |
| `TalentSystem` | 450 | 遊戲 | 天賦樹、技能點分配 |
| `SummonSystem` | 460 | 遊戲 | 召喚物管理、寵物控制 |

#### 經濟系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `InventorySystem` | 500 | 遊戲 | 背包管理、物品整理 |
| `EquipmentSystem` | 510 | 遊戲 | 裝備穿戴、屬性計算 |
| `AuctionSystem` | 520 | 遊戲 | 拍賣行、物品競價 |
| `MarketSystem` | 530 | 遊戲 | 交易行、價格波動 |
| `ShopSystem` | 540 | 遊戲 | NPC商店、庫存刷新 |

#### 移動和物理系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `MovementSystem` | 550 | 遊戲 | 位置移動、路徑計算 |
| `CollisionSystem` | 560 | 遊戲 | 碰撞檢測、障礙物處理 |
| `TeleportSystem` | 570 | 遊戲 | 傳送點、地圖切換 |

#### 自動化系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `AutoBattleSystem` | 580 | 遊戲 | 自動戰鬥、掛機設定 |
| `AutoPathfindingSystem` | 590 | 遊戲 | 自動尋路、任務導航 |

### 第3層：表現層系統 (600-899)

#### 動畫和特效系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `AnimationSystem` | 600 | 遊戲 | 角色動畫、幀更新、**攻擊動畫** |
| `ParticleSystem` | 610 | 遊戲 | 粒子效果、生命周期管理 |
| `WeatherSystem` | 620 | 遊戲 | 天氣效果、日夜循環 |

#### 音效系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `SoundSystem` | 630 | 全部 | 3D音效、背景音樂管理 |
| `VoiceChatSystem` | 640 | 遊戲 | 語音聊天、音量控制 |

#### 攝像機系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `CameraFollowSystem` | 650 | 遊戲 | 攝像機跟隨玩家 |
| `CameraSystem` | 700 | 遊戲 | 攝像機矩陣計算、特效 |

#### UI 系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `UISystem` | 800 | 全部 | UI狀態更新、事件處理 |
| `HUDSystem` | 810 | 遊戲 | 血條、狀態欄、快捷欄 |
| `MinimapSystem` | 820 | 遊戲 | 小地圖、坐標顯示 |
| `DialogSystem` | 830 | 遊戲 | 對話框、劇情文本 |

### 第4層：渲染層系統 (900-1999)

#### 基礎渲染系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `MapRenderSystem` | 1000 | 遊戲 | 地圖渲染、**Tile動畫** |
| `SpriteRenderSystem` | 1010 | 遊戲 | 精靈渲染、排序 |
| `EffectRenderSystem` | 1020 | 遊戲 | 特效渲染、後處理 |
| `UIRenderSystem` | 1030 | 全部 | UI界面渲染 |

#### 高級渲染系統
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `LightingRenderSystem` | 1040 | 遊戲 | 動態光影、陰影 |
| `PostProcessSystem` | 1050 | 遊戲 | 顏色校正、濾鏡 |
| `TextRenderSystem` | 1060 | 全部 | 文字渲染、字體 |
| `DebugRenderSystem` | 1100 | 開發 | 調試信息渲染 |

### 第5層：調試工具系統 (9000+)
| 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
|---------|--------|----------|----------|
| `CheatSystem` | 9000 | 開發 | 開發者指令、測試功能 |
| `ProfileSystem` | 9010 | 開發 | 性能分析、內存監控 |
| `DebugSystem` | 9020 | 開發 | 調試信息、可視化 |
| `RecordSystem` | 9030 | 開發 | 操作錄製、回放 |

## 🔄 完整執行流程

```
幀開始
↓
第0層：基礎設施 (0-99)
├── SceneSystem → ResourcePreloadSystem → SaveSystem
↓
第1層：輸入網絡 (100-299) 
├── InputCollectSystem → NetworkReceiveSystem → PlayerControlSystem
↓
第2層：遊戲邏輯 (300-599)
├── AI系統: MonsterAISystem → NPCInteractionSystem → PetAISystem
├── 社交系統: GuildSystem → PartySystem → FriendSystem
├── 傳奇特色: PKSystem → DungeonSystem → BossSystem → SiegeWarSystem
├── 任務系統: QuestSystem → DailySystem → AchievementSystem
├── 戰鬥系統: CombatSystem → SkillSystem → BuffDebuffSystem → RegenSystem
├── 職業系統: ClassSystem → TalentSystem → SummonSystem
├── 經濟系統: InventorySystem → EquipmentSystem → AuctionSystem → MarketSystem
├── 移動系統: MovementSystem → CollisionSystem → TeleportSystem
└── 自動化: AutoBattleSystem → AutoPathfindingSystem
↓
第1層：網絡發送 (200)
├── NetworkSendSystem (數據發送到服務器)
↓
第3層：表現層 (600-899)
├── 動畫特效: AnimationSystem → ParticleSystem → WeatherSystem
├── 音效系統: SoundSystem → VoiceChatSystem
├── 攝像機系統: CameraFollowSystem → CameraSystem
└── UI系統: UISystem → HUDSystem → MinimapSystem → DialogSystem
↓
第4層：渲染層 (900-1999)
├── 基礎渲染: MapRenderSystem → SpriteRenderSystem → EffectRenderSystem → UIRenderSystem
└── 高級渲染: LightingRenderSystem → PostProcessSystem → TextRenderSystem
↓
第5層：調試工具 (9000+)
├── CheatSystem → ProfileSystem → DebugSystem → RecordSystem
↓
幀結束
```

## 🎯 關鍵特性說明

### 新增加的重要系統：
1. **`RegenSystem` (430)** - 專門處理：
   - HP恢復: MaxHP * 3% + 1 (每10秒)
   - MP恢復: MaxMP * 3% + 1 (每10秒) 
   - Buff/Debuff過期清理
   - DoT傷害計算 (毒、流血等)

2. **完整的攻擊動畫流程**：
   - `InputCollectSystem` → 攻擊輸入檢測
   - `CombatSystem` → 攻擊邏輯觸發
   - `AnimationSystem` → 攻擊動畫管理
   - `EffectSystem` → 攻擊特效播放

3. **Tile動畫管理**：
   - `MapRenderSystem` 負責地圖Tile動畫
   - 與實體動畫系統分離

## ⚙️ 場景配置示例

### 遊戲場景啟用系統 (58個)：
```rust
// 第0層: 6個系統
// 第1層: 8個系統  
// 第2層: 26個系統
// 第3層: 12個系統
// 第4層: 6個系統 (除DebugRenderSystem)
```

### 登錄場景啟用系統：
```rust
["SceneSystem", "ResourcePreloadSystem", "InputCollectSystem", 
 "UISystem", "UIRenderSystem", "SoundSystem"]
```

## 🚀 性能優化策略

### 更新頻率分級：
- **高頻** (每幀): 輸入、移動、動畫、攝像機
- **中頻** (每2-5幀): AI、粒子、小地圖
- **低頻** (每10+幀): 天氣、社交、成就、**恢復系統**

### 條件執行：
- 只在可見區域更新動畫和AI
- 根據距離決定更新精度
- 動態啟用/禁用非活躍系統

這個完整的ECS架構表涵蓋了熱血傳奇所有核心功能，確保了系統間的清晰職責分離和高效協作！