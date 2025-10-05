# MirScenes Module Migration Plan

## Overview
Complete migration of Client/MirScenes module to ClientRust/src/scenes

## Current Status

### ✅ Already Implemented
- **Core Infrastructure**:
  - `scene_trait.rs` - Scene trait and types
  - `state.rs` - ClientState and game events
  - `mod.rs` - Module exports

- **Main Scenes** (Partial):
  - `login_scene.rs` - Login scene (needs completion)
  - `select_scene.rs` - Character selection scene (needs completion)
  - `game_scene.rs` - Main game scene (framework only)

- **Dialogs** (Partial - 33 files):
  - Belt, BigMap, Buff, Character, Chat, Craft, Fishing, Friend
  - Group, Guild, Help, Inspect, Inventory, Keyboard Layout, Mail
  - Main, Menu, Mount, Notice, NPC, Option, Quest, Refine
  - Report, Skillbar, Socket, Storage, Timer, Trade
  - Character Creation/Deletion, Dialog Manager

### ❌ Missing/Incomplete

#### 1. Main Scenes Need Completion
- **LoginScene** (currently 414 lines vs C# 1376 lines):
  - Missing: LoginDialog, NewAccountDialog, ChangePasswordDialog
  - Missing: InputKeyDialog (virtual keyboard)
  - Missing: UI rendering implementation
  - Missing: Animation system integration
  - Missing: Sound system integration

- **SelectScene** (currently 220 lines vs C# 637 lines):
  - Missing: CharacterButton components
  - Missing: Character display animation
  - Missing: Delete character confirmation
  - Missing: Credits screen integration

- **GameScene** (currently 552 lines vs C# 12,294 lines):
  - Missing: MapControl integration
  - Missing: All dialog instances
  - Missing: Output message system
  - Missing: Attack/Pet mode switching
  - Missing: Spell casting system
  - Missing: Item system integration
  - Missing: Storage arrays
  - Missing: Gold/Credit tracking

#### 2. Dialog Components Need Implementation
Based on C# Dialogs/ folder (35 files):

**High Priority** (Core gameplay):
- ✅ MainDialog (has main_dialog.rs)
- ✅ ChatDialog (has chat_dialog.rs)
- ✅ InventoryDialog (has inventory_dialog.rs)
- ✅ CharacterDialog (has character_dialog.rs)
- ⚠️ MiniMapDialog (missing)
- ⚠️ SkillBarDialog (has skillbar_dialog.rs, may be incomplete)

**Medium Priority** (Common features):
- ✅ NPCDialog (has npc_dialog.rs)
- ✅ StorageDialog (has storage_dialog.rs)
- ✅ TradeDialog (has trade_dialog.rs)
- ✅ GroupDialog (has group_dialog.rs)
- ✅ GuildDialog (has guild_dialog.rs)
- ⚠️ DuraStatusDialog (missing)
- ⚠️ TrustMerchantDialog (missing)

**Low Priority** (Advanced features):
- ⚠️ IntelligentCreatureDialog (missing)
- ⚠️ IntelligentCreatureOptionsDialog (missing)
- ⚠️ ItemRentalDialog (has item_rental stub)
- ⚠️ GameShopDialog (missing)
- ⚠️ RankingDialog (missing)
- ⚠️ MentorDialog (missing)
- ⚠️ RelationshipDialog (missing)
- ⚠️ GuildTerritoryDialog (missing)

#### 3. Nested Dialog Classes in C#
Many C# scene files contain nested dialog classes:

**LoginScene.cs** contains:
- LoginDialog (lines 323-550)
- InputKeyDialog (lines 552-744) - Virtual keyboard
- NewAccountDialog (lines 746-1142)
- ChangePasswordDialog (lines 1144-1354)

**SelectScene.cs** contains:
- CharacterButton (lines 300-400 approx)

**GameScene.cs** contains:
- MapControl (lines 10062-12294) - MASSIVE nested class

## Migration Strategy

### Phase 1: Complete Core Scenes ⚠️ CURRENT PHASE
1. **LoginScene Enhancement**:
   - Extract LoginDialog as separate component
   - Add NewAccountDialog
   - Add ChangePasswordDialog
   - Add InputKeyDialog (virtual keyboard)
   - Integrate with sound system
   - Add UI rendering

2. **SelectScene Enhancement**:
   - Add CharacterButton component
   - Complete character display system
   - Add delete character flow
   - Integrate character sorting

3. **GameScene Foundation**:
   - Add MapControl struct (as separate file)
   - Add static field equivalents
   - Implement output message system
   - Add mode switching logic

### Phase 2: Dialog System Completion
1. Create missing dialog files:
   - `minimap_dialog.rs`
   - `dura_status_dialog.rs`
   - `trust_merchant_dialog.rs`
   - `intelligent_creature_dialog.rs`
   - `gameshop_dialog.rs`
   - `ranking_dialog.rs`
   - `mentor_dialog.rs`
   - `relationship_dialog.rs`
   - `guild_territory_dialog.rs`

2. Complete existing partial dialogs
3. Add dialog manager integration

### Phase 3: MapControl Implementation
1. Extract MapControl to `map_control.rs` (~2000 lines estimated)
2. Implement cell grid system
3. Add pathfinding integration
4. Add door system
5. Add lighting/weather effects

### Phase 4: Integration & Testing
1. Wire up all dialogs to GameScene
2. Implement item/storage systems
3. Add spell system
4. Complete event processing
5. Performance testing

## File Structure (Target)

```
ClientRust/src/scenes/
├── mod.rs
├── scene_trait.rs
├── state.rs
├── login_scene.rs (complete)
├── select_scene.rs (complete)
├── game_scene.rs (complete)
├── map_control.rs (NEW - extracted from GameScene)
│
├── dialogs/
│   ├── mod.rs
│   ├── dialog_manager.rs
│   │
│   ├── login_dialog.rs (NEW - extracted from LoginScene)
│   ├── new_account_dialog.rs (NEW)
│   ├── change_password_dialog.rs (NEW)
│   ├── input_key_dialog.rs (NEW - virtual keyboard)
│   │
│   ├── character_button.rs (NEW - extracted from SelectScene)
│   │
│   ├── main_dialog.rs ✓
│   ├── chat_dialog.rs ✓
│   ├── inventory_dialog.rs ✓
│   ├── character_dialog.rs ✓
│   ├── minimap_dialog.rs (NEW)
│   ├── skillbar_dialog.rs ✓
│   ├── belt_dialog.rs ✓
│   │
│   ├── npc_dialog.rs ✓
│   ├── storage_dialog.rs ✓
│   ├── trade_dialog.rs ✓
│   ├── group_dialog.rs ✓
│   ├── guild_dialog.rs ✓
│   ├── guild_territory_dialog.rs (NEW)
│   │
│   ├── quest_list_dialog.rs ✓
│   ├── mail_dialog.rs ✓
│   ├── friend_dialog.rs ✓
│   ├── mentor_dialog.rs (NEW)
│   ├── relationship_dialog.rs (NEW)
│   │
│   ├── dura_status_dialog.rs (NEW)
│   ├── trust_merchant_dialog.rs (NEW)
│   ├── intelligent_creature_dialog.rs (NEW)
│   ├── gameshop_dialog.rs (NEW)
│   ├── ranking_dialog.rs (NEW)
│   │
│   └── ... (other existing dialogs)
```

## Key Design Decisions

### 1. Nested Classes → Separate Files
C# nested classes will be extracted to separate Rust files:
- Better module organization
- Easier testing
- Better IDE support
- Rust doesn't encourage deep nesting

### 2. Static Fields → ClientState
C# static fields in GameScene become ClientState fields:
- `GameScene.User` → `ClientState.user`
- `GameScene.Storage` → `ClientState.storage`
- Enables proper ownership/borrowing

### 3. Event Handling
C# events/delegates → Rust enum-based events:
- `GameEvent` enum in state.rs
- Pattern matching in `process_event`

### 4. UI Framework
Need to choose Rust UI approach:
- Option A: Direct rendering (current approach)
- Option B: egui integration
- Option C: Custom immediate-mode GUI

## Next Steps

1. ✅ Create this migration plan
2. ⚠️ Complete LoginScene with all nested dialogs
3. ⚠️ Complete SelectScene with CharacterButton
4. ⚠️ Extract MapControl from GameScene
5. ⚠️ Implement missing high-priority dialogs
6. ⚠️ Complete GameScene integration
7. ⚠️ Add comprehensive tests

## Estimated Effort

- Phase 1 (Core Scenes): ~1500 lines
- Phase 2 (Dialogs): ~2000 lines
- Phase 3 (MapControl): ~2000 lines
- Phase 4 (Integration): ~500 lines
- **Total: ~6000 lines of new code**

## Dependencies

External crates needed:
- ✅ mir2_shared (shared data structures)
- ⚠️ UI framework (TBD)
- ⚠️ Audio system integration
- ⚠️ Graphics/texture system
- ⚠️ Input handling

## Completion Criteria

✅ Scene complete when:
1. All C# functionality mirrored
2. All dialogs implemented
3. Event processing works
4. Tests pass
5. No compilation errors
6. Performance acceptable

---

**Status**: Phase 1 in progress - completing core scenes
**Last Updated**: 2025-10-05
