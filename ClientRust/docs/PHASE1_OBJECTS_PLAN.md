# Phase 1: MirObjects Module - Implementation Plan

**Date**: 2025-01-03  
**Status**: 🚧 IN PROGRESS  
**Priority**: P0 (Blocking compilation)

---

## 📋 Overview

Phase 1 focuses on fixing and completing the **MirObjects** module to enable game object management. The module already exists but has ~51 compilation errors due to field mismatches between ClientRust code and SharedRust packet definitions.

---

## 🎯 Goals

1. ✅ Fix all field access errors (location → location_x/y)
2. ✅ Fix name_colour_argb → name_colour
3. ✅ Complete MapObject public API
4. ✅ Fix MonsterObject implementation
5. ✅ Fix NPCObject implementation
6. ✅ Add unit tests for all objects
7. ✅ Update documentation

---

## 🔍 Current State

### Existing Files

```
src/objects/
├── mod.rs              - Module exports ✅
├── map_object.rs       - Base object (❌ 15 errors)
├── monster_object.rs   - Monster (❌ 26 errors)
├── npc_object.rs       - NPC (❌ 10 errors)
├── user_object.rs      - Player (unknown status)
├── hero_object.rs      - Hero (unknown status)
├── item_object.rs      - Items
├── spell_object.rs     - Spells
├── effect.rs           - Visual effects
├── damage.rs           - Damage display
├── frames.rs           - Animation frames
└── pathfinder.rs       - Pathfinding
```

### Error Categories

**Category 1: Field Name Mismatches (35 errors)**
- `packet.location` → `Point::new(packet.location_x, packet.location_y)`
- `packet.name_colour_argb` → `packet.name_colour`

**Category 2: Missing MapObject Methods (10 errors)**
- `MapObject::new_monster()` - doesn't exist
- `MapObject::new_npc()` - doesn't exist
- Direct field access to private fields

**Category 3: API Mismatches (6 errors)**
- `map_object.object_id` should be method call
- Private fields accessed directly
- Type mismatches (i32 vs u32)

---

## 🛠️ Implementation Plan

### Step 1: Fix SharedRust Packet Field Access ⏱️ 30 mins

**File**: `src/objects/map_object.rs`

**Changes**:
```rust
// BEFORE (wrong)
let location = player.location;
let colour = player.name_colour_argb;

// AFTER (correct)
let location = Point::new(player.location_x, player.location_y);
let colour = player.name_colour;
```

**Locations**:
- Line 112: `player.location` → `Point::new(...)`
- Line 138: `hero.player.location` → `Point::new(...)`
- Line 168: `monster.location` → `Point::new(...)`
- Lines 236-247: `name_colour_argb` → `name_colour` (6 occurrences)
- Lines 301, 414, 418, 428, 432, 436: More location fixes

**Estimated fixes**: ~15 replacements

---

### Step 2: Add MapObject Constructor Methods ⏱️ 20 mins

**File**: `src/objects/map_object.rs`

**Add these methods**:
```rust
impl MapObject {
    /// Create a new monster object
    pub fn new_monster(object_id: u32) -> Self {
        Self {
            kind: MapObjectKind::Monster(ObjectMonster {
                object_id,
                ..Default::default()
            }),
            buffs: BuffState::default(),
            animation: AnimationState::default(),
            location: Point::new(0, 0),
            direction: MirDirection::Up,
        }
    }
    
    /// Create a new NPC object  
    pub fn new_npc(object_id: u32) -> Self {
        // Similar implementation
    }
    
    /// Create a new player object
    pub fn new_player(object_id: u32) -> Self {
        // Similar implementation
    }
}
```

---

### Step 3: Fix MonsterObject Field Access ⏱️ 30 mins

**File**: `src/objects/monster_object.rs`

**Changes** (26 errors):
1. Line 127: Add `new_monster()` method to MapObject
2. Line 149-177: Fix field access (use getters/setters)
3. Line 197-204: Fix AI and dead field access
4. Line 268: `object_id` is method not field
5. Line 284: Fix dead field access
6. Lines 213, 237: Remove unused variable warnings

**Pattern**:
```rust
// BEFORE
self.map_object.name = info.name.clone();
self.map_object.current_location = info.location;

// AFTER  
self.map_object.set_name(info.name.clone());
self.map_object.set_location(Point::new(info.location_x, info.location_y));
```

---

### Step 4: Fix NPCObject Field Access ⏱️ 20 mins

**File**: `src/objects/npc_object.rs`

**Changes** (10 errors):
1. Line 43: Add `new_npc()` method
2. Lines 51-59: Fix field access patterns
3. Remove unused Point import

---

### Step 5: Make MapObject Fields Public/Add Getters ⏱️ 30 mins

**File**: `src/objects/map_object.rs`

**Add public accessors**:
```rust
impl MapObject {
    // Core getters
    pub fn object_id(&self) -> u32 { self.kind.object_id() }
    pub fn location(&self) -> Point { self.location }
    pub fn direction(&self) -> MirDirection { self.direction }
    pub fn buffs(&self) -> &[BuffType] { &self.buffs.active }
    
    // Core setters
    pub fn set_name(&mut self, name: String) { /* ... */ }
    pub fn set_location(&mut self, loc: Point) { self.location = loc; }
    pub fn set_direction(&mut self, dir: MirDirection) { self.direction = dir; }
    
    // State queries
    pub fn is_dead(&self) -> bool { /* ... */ }
    pub fn is_hidden(&self) -> bool { /* ... */ }
    pub fn light_level(&self) -> u8 { /* ... */ }
    pub fn ai_type(&self) -> u8 { /* ... */ }
    pub fn poison_type(&self) -> PoisonType { /* ... */ }
    
    // State mutations
    pub fn set_dead(&mut self, dead: bool) { /* ... */ }
    pub fn set_hidden(&mut self, hidden: bool) { /* ... */ }
    pub fn set_light(&mut self, light: u8) { /* ... */ }
    pub fn set_ai(&mut self, ai: u8) { /* ... */ }
    pub fn set_poison(&mut self, poison: PoisonType) { /* ... */ }
    pub fn set_buffs(&mut self, buffs: Vec<BuffType>) { /* ... */ }
}
```

---

### Step 6: Add Unit Tests ⏱️ 40 mins

**File**: `src/objects/map_object.rs` (bottom)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_monster() {
        let obj = MapObject::new_monster(123);
        assert_eq!(obj.object_id(), 123);
    }
    
    #[test]
    fn test_location_updates() {
        let mut obj = MapObject::new_player(1);
        obj.set_location(Point::new(100, 200));
        assert_eq!(obj.location(), Point::new(100, 200));
    }
    
    #[test]
    fn test_buff_management() {
        let mut obj = MapObject::new_player(1);
        obj.set_buffs(vec![BuffType::Magic]);
        assert_eq!(obj.buffs().len(), 1);
    }
    
    // Add 10-15 comprehensive tests
}
```

**File**: `src/objects/monster_object.rs` (bottom)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_monster_creation() { /* ... */ }
    
    #[test]
    fn test_monster_load_info() { /* ... */ }
    
    #[test]
    fn test_monster_animation() { /* ... */ }
    
    // 5-10 tests
}
```

---

## 📊 Progress Tracking

### Step 1: Packet Field Fixes
- [ ] map_object.rs location fixes (12 occurrences)
- [ ] map_object.rs name_colour fixes (6 occurrences)
- [ ] Verify compilation

### Step 2: Constructor Methods  
- [ ] `new_monster()` implementation
- [ ] `new_npc()` implementation
- [ ] `new_player()` implementation
- [ ] Verify compilation

### Step 3: MonsterObject Fixes
- [ ] Fix field access patterns
- [ ] Update to use getters/setters
- [ ] Remove unused imports
- [ ] Fix type mismatches
- [ ] Verify compilation

### Step 4: NPCObject Fixes
- [ ] Fix field access patterns
- [ ] Update to use getters/setters
- [ ] Remove unused imports
- [ ] Verify compilation

### Step 5: MapObject Public API
- [ ] Add getter methods (10+ methods)
- [ ] Add setter methods (10+ methods)
- [ ] Add state query methods
- [ ] Document all methods
- [ ] Verify compilation

### Step 6: Unit Tests
- [ ] MapObject tests (10 tests)
- [ ] MonsterObject tests (5 tests)
- [ ] NPCObject tests (5 tests)
- [ ] Run all tests
- [ ] Verify 100% pass rate

---

## ⏱️ Time Estimates

| Task | Estimated Time | Status |
|------|---------------|--------|
| Step 1: Packet fixes | 30 mins | 🔲 Todo |
| Step 2: Constructors | 20 mins | 🔲 Todo |
| Step 3: MonsterObject | 30 mins | 🔲 Todo |
| Step 4: NPCObject | 20 mins | 🔲 Todo |
| Step 5: Public API | 30 mins | 🔲 Todo |
| Step 6: Unit tests | 40 mins | 🔲 Todo |
| **Total** | **~2.5 hours** | **0% Complete** |

---

## 🎯 Success Criteria

- [ ] ✅ **0 compilation errors** in objects module
- [ ] ✅ **0 compiler warnings** in objects module
- [ ] ✅ **20+ unit tests** passing
- [ ] ✅ **Full public API** documented
- [ ] ✅ **MapObject, MonsterObject, NPCObject** fully functional
- [ ] ✅ **Integration tests** with network module

---

## 🚀 Next Steps After Phase 1

**Phase 2: UserObject & HeroObject**
- Complete user_object.rs implementation
- Complete hero_object.rs implementation
- Add player-specific logic
- Inventory management
- Skill/magic system integration

**Phase 3: Rendering Pipeline**
- Connect objects to graphics module
- Implement sprite rendering
- Animation frame updates
- Effect rendering

**Phase 4: Integration**
- Connect network packets to object creation
- Implement object lifecycle management
- Add object pooling/caching
- Performance optimization

---

## 📚 References

### C# Source Files
- `Client/MirObjects/MapObject.cs` - Base class
- `Client/MirObjects/MonsterObject.cs` - Monster logic
- `Client/MirObjects/NPCObject.cs` - NPC logic
- `Client/MirObjects/PlayerObject.cs` - Player logic

### Rust Files
- `SharedRust/src/packets/server/objects.rs` - Packet definitions
- `ClientRust/src/objects/map_object.rs` - Current implementation
- `ClientRust/src/network/game_client.rs` - Network integration

---

**Status**: Ready to begin Step 1 🚀
