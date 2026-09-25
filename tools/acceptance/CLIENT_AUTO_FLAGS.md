# 客户端自动化开关（`--xxx-test`）覆盖清单

> **这份清单是机器对账的**：`tools/acceptance/flag_coverage_check.ps1` 会从
> `Client-Bevy/src/auto/mod.rs` 重新提取开关并与下面表格逐一比对（多一个/少一个都红），
> 同时校验「门禁桶」的开关确实还写在 `scripts/run_real_e2e.ps1` 里、
> 「夹具桶」列的 runner 文件确实引用了它。**新增/删除开关时必须同步这张表**，否则门禁红。

## 为什么需要它

`Client-Bevy/src/auto/` 里累积了 **82** 个 `--xxx-test` 开关，但发版门禁
（`scripts/run_real_e2e.ps1`）只跑 **12** 个、另有 **2** 个被别的实机夹具引用、
其余 **68** 个**没有任何 runner**（当年为复现/定位某个缺陷写的一次性探针）。
探针写完就该退休，比例本身不是问题——**问题是此前没有任何地方写着「哪些是门禁、哪些是探针」**：
「覆盖了什么」不可对账，新人容易把探针当门禁、或把门禁当探针。

三个桶的定义：

| 桶 | 含义 | 维护要求 |
|---|---|---|
| `gate` | 发版前门禁**每次都要跑** | 改了相关功能必须让它保持绿；红了按产品缺陷处理（先定性再改，见 LESSON） |
| `fixture` | 由某条实机夹具专项跑（不在默认门禁里） | 改动相关功能时跑对应夹具；夹具本身也是交付物 |
| `probe` | 历史探针，没有 runner | **不当作覆盖证据**；要重新用它先读对应的 `auto_*` 系统函数确认判据仍成立 |

重新生成这份表的方式（只读分析，不写库）：
从 `auto/mod.rs` 里 `grep -o '"--[a-z0-9-]*-test"' | sort -u` 取开关全集，
再在 `scripts/` 与 `tools/acceptance/` 里 `grep` 每个开关，按上表分桶。

## 开关清单

<!-- 下面的表格由 flag_coverage_check.ps1 校验：开关集合必须与源码一致 -->

| 开关 | 桶 | 谁在跑 | 注册的系统函数（`auto/mod.rs`） |
|---|---|---|---|
| `--action-test` | probe | —（历史探针，无 runner） | `auto_action_test` |
| `--attack-range-test` | probe | —（历史探针，无 runner） | `auto_attack_range_test` |
| `--awake-test` | probe | —（历史探针，无 runner） | `auto_awake_test` |
| `--battle-vfx-test` | fixture | `tools/acceptance/l5v_spell_fx.ps1`、`tools/acceptance/l5w_object_fx.ps1` | `auto_battle_vfx_test` |
| `--bigmap-test` | probe | —（历史探针，无 runner） | `auto_bigmap_test` |
| `--book-test` | probe | —（历史探针，无 runner） | `auto_book_test` |
| `--buff-test` | probe | —（历史探针，无 runner） | `auto_buff_test` |
| `--chat-item-test` | probe | —（历史探针，无 runner） | `auto_chat_item_test` |
| `--combat-test` | probe | —（历史探针，无 runner） | `auto_combat_test` |
| `--compass-test` | probe | —（历史探针，无 runner） | `auto_compass_test` |
| `--craft-test` | probe | —（历史探针，无 runner） | `auto_craft_test` |
| `--creature-test` | probe | —（历史探针，无 runner） | `auto_creature_test` |
| `--creature2-test` | probe | —（历史探针，无 runner） | `auto_creature2_test` |
| `--drop-pick-test` | probe | —（历史探针，无 runner） | `auto_drop_pick_test` |
| `--dura-test` | probe | —（历史探针，无 runner） | `auto_dura_test` |
| `--final-test` | probe | —（历史探针，无 runner） | `auto_final_test` |
| `--fishing-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_fishing_test` |
| `--friend-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_friend_test` |
| `--gameshop-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_gameshop_test` |
| `--gold-test` | probe | —（历史探针，无 runner） | `auto_gold_test` |
| `--group-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_group_test` |
| `--guild-gold-test` | probe | —（历史探针，无 runner） | `auto_guild_gold_test` |
| `--guild-invite-test` | probe | —（历史探针，无 runner） | `auto_guild_invite_test` |
| `--guild-item-test` | probe | —（历史探针，无 runner） | `auto_guild_item_test` |
| `--guild-notice-test` | probe | —（历史探针，无 runner） | `auto_guild_notice_test` |
| `--guild-storage-realtime-test` | probe | —（历史探针，无 runner） | `auto_guild_storage_realtime_test` |
| `--guild-test` | probe | —（历史探针，无 runner） | `auto_guild_test` |
| `--harvest-test` | probe | —（历史探针，无 runner） | `auto_harvest_test` |
| `--hero-battle-test` | probe | —（历史探针，无 runner） | `auto_hero_battle_test` |
| `--hero-exp-test` | probe | —（历史探针，无 runner） | `auto_hero_exp_test` |
| `--hero-test` | probe | —（历史探针，无 runner） | `auto_hero_test` |
| `--hold-move-test` | probe | —（历史探针，无 runner） | `hold_move_test_system` |
| `--inspect-test` | probe | —（历史探针，无 runner） | `auto_inspect_test` |
| `--item-state-test` | probe | —（历史探针，无 runner） | `auto_item_state_test` |
| `--keyboard-test` | probe | —（历史探针，无 runner） | `auto_keyboard_test` |
| `--level-fx-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_level_fx_test` |
| `--mail-compose-test` | probe | —（历史探针，无 runner） | `auto_mail_compose_test` |
| `--mail-parcel-test` | probe | —（历史探针，无 runner） | `auto_mail_compose_test` |
| `--mail-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_mail_test` |
| `--mana-test` | probe | —（历史探针，无 runner） | `auto_mana_test` |
| `--map-fx-test` | probe | —（历史探针，无 runner） | `auto_map_fx_test` |
| `--market-test` | probe | —（历史探针，无 runner） | `auto_market_test` |
| `--marriage-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_marriage_test` |
| `--member-test` | probe | —（历史探针，无 runner） | `auto_member_test` |
| `--mentor-test` | probe | —（历史探针，无 runner） | `auto_mentor_test` |
| `--misc2-test` | probe | —（历史探针，无 runner） | `auto_misc2_test` |
| `--mount-sync-test` | probe | —（历史探针，无 runner） | `auto_mount_sync_test` |
| `--mount-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_mount_test` |
| `--name-test` | probe | —（历史探针，无 runner） | `auto_name_test` |
| `--notice-test` | probe | —（历史探针，无 runner） | `auto_notice_test` |
| `--npc-credit-test` | probe | —（历史探针，无 runner） | `auto_npc_credit_test` |
| `--npc-input-test` | probe | —（历史探针，无 runner） | `auto_npc_input_test` |
| `--object-state-test` | probe | —（历史探针，无 runner） | `auto_object_state_test` |
| `--option-test` | probe | —（历史探针，无 runner） | `auto_option_test` |
| `--poison-test` | probe | —（历史探针，无 runner） | `auto_poison_test` |
| `--quest-data-test` | probe | —（历史探针，无 runner） | `auto_quest_data_test` |
| `--quest-test` | probe | —（历史探针，无 runner） | `auto_quest_test` |
| `--ranking-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_ranking_test` |
| `--real-worldmap-test` | probe | —（历史探针，无 runner） | `auto_real_worldmap_test` |
| `--recipe-test` | probe | —（历史探针，无 runner） | `auto_recipe_test` |
| `--reconnect-test` | fixture | `tools/acceptance/l5y_reconnect.ps1` | `auto_reconnect_test` |
| `--refine-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_refine_test` |
| `--reincarnation-test` | probe | —（历史探针，无 runner） | `auto_reincarnation_test` |
| `--rental-test` | probe | —（历史探针，无 runner） | `auto_rental_test` |
| `--repair-test` | probe | —（历史探针，无 runner） | `auto_repair_test` |
| `--report-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_report_test` |
| `--resize-test` | probe | —（历史探针，无 runner） | `auto_resize_test` |
| `--roll-test` | probe | —（历史探针，无 runner） | `auto_roll_test` |
| `--session-feedback-test` | probe | —（历史探针，无 runner） | `auto_session_feedback_test` |
| `--shop-test` | probe | —（历史探针，无 runner） | `auto_shop_test` |
| `--sneak-test` | probe | —（历史探针，无 runner） | `auto_sneak_test` |
| `--socket-test` | probe | —（历史探针，无 runner） | `auto_socket_test` |
| `--storage-equip-test` | probe | —（历史探针，无 runner） | `auto_storage_equip_test` |
| `--storage-resize-test` | probe | —（历史探针，无 runner） | `auto_storage_resize_test` |
| `--storage-test` | probe | —（历史探针，无 runner） | `auto_storage_test` |
| `--storage-unlock-test` | probe | —（历史探针，无 runner） | `auto_storage_unlock_test` |
| `--territory-test` | probe | —（历史探针，无 runner） | `auto_territory_test` |
| `--toggle-test` | probe | —（历史探针，无 runner） | `auto_toggle_test` |
| `--trade-test` | gate | `scripts/run_real_e2e.ps1`（发版门禁） | `auto_trade_test` |
| `--ui-dialog-test` | probe | —（历史探针，无 runner） | `auto_ui_dialog_test` |
| `--upgrade-test` | probe | —（历史探针，无 runner） | `auto_upgrade_test` |
| `--worldmap-test` | probe | —（历史探针，无 runner） | `auto_worldmap_test` |


## 计数（本表生成时的读数）

- 开关总数 **82**
- `gate` **12** / `fixture` **2** / `probe` **68**

## 已知边界

- 本清单只覆盖**客户端的 auto 开关**这一种覆盖形式。发版前的实际覆盖还包括：
  离线门禁（三侧 `cargo test`/`fmt`/`clippy`，见 `docs/DELIVERY.md` §5.1）、
  真机 E2E 门禁（§5.2，12 个用例）、交互巡回（§5.3，40 窗）、
  以及 `tools/acceptance/l5*` 那批 control-RPC 夹具（§5 各处）。
- `probe` 桶里的开关**可能已经因为协议/界面演进而失效**（当年断言的现象可能已不存在）。
  本清单不声称它们仍然有效，只声明「它们不在覆盖证据里」。
