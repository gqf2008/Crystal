// 网络客户端 - 简单双线程实现
//
// 设计：满足 Read + Write trait 即可，内部两个线程处理读写
// - read_thread: 持续读packet → 解析为 GameEvent → 发送到游戏线程
// - write_thread: 持续recv GameEvent → 转换为packet → 发送到服务器
//
// 使用：
//   let (tx, rx) = Network::new((write_stream, read_stream));
//   tx.send(GameEvent::LoginRequest {...});  // 游戏 → 网络
//   let events = rx.try_iter().collect();    // 网络 → 游戏

use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::io::{Read, Write};

use crate::network::handlers::GameEvent;

/// 网络客户端 - 零大小类型
///
/// 此结构体本身不存储任何数据，只提供静态的 `new()` 方法来创建网络连接。
/// 实际的网络 IO 由内部启动的两个线程处理，通过 channels 与游戏线程通信。
pub struct Network;

impl Network {
    /// 创建并启动网络客户端
    ///
    /// 返回：(发送channel, 接收channel)
    /// - 发送channel: 游戏 → 网络 (GameEvent)
    /// - 接收channel: 网络 → 游戏 (GameEvent)
    pub fn new<W, R>((w, r): (W, R)) -> (Sender<GameEvent>, Receiver<GameEvent>)
    where
        W: Write + Send + 'static,
        R: Read + Send + 'static,
    {
        let (game_to_net_tx, game_to_net_rx) = unbounded();
        let (net_to_game_tx, net_to_game_rx) = unbounded();

        // 读线程：packet → GameEvent
        {
            let tx = net_to_game_tx.clone();
            let to_write = game_to_net_tx.clone(); // 用于自动发送ClientVersion等
            std::thread::Builder::new()
                .name("net-read".into())
                .spawn(move || {
                    read_loop(r, tx, to_write);
                })
                .expect("Failed to spawn read thread");
        }

        // 写线程：GameEvent → packet
        {
            let rx = game_to_net_rx;
            std::thread::Builder::new()
                .name("net-write".into())
                .spawn(move || {
                    write_loop(w, rx);
                })
                .expect("Failed to spawn write thread");
        }

        (game_to_net_tx, net_to_game_rx)
    }
}

/// 读线程：持续读取 packet 并转换为 GameEvent
fn read_loop<S: Read + Send>(mut stream: S, tx: Sender<GameEvent>, to_write: Sender<GameEvent>) {
    use mir2_shared::packets::PacketHeader;

    loop {
        // 读取 packet header (4 bytes: length + opcode)
        let header = {
            match PacketHeader::read_from(&mut stream) {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!("Read header error: {}", e);
                    let _ = tx.send(GameEvent::Disconnected {
                        reason: e.to_string(),
                    });
                    break;
                }
            }
        };

        // 读取 payload
        let payload_len = (header.length as usize).saturating_sub(PacketHeader::HEADER_SIZE);
        let mut payload = vec![0u8; payload_len];
        {
            if let Err(e) = stream.read_exact(&mut payload) {
                tracing::error!("Read payload error: {}", e);
                let _ = tx.send(GameEvent::Disconnected {
                    reason: e.to_string(),
                });
                break;
            }
        }

        // 转换为 GameEvent（使用现有的 handlers）
        let events = dispatch_packet(&header, &payload);
        for event in events {
            // 特殊处理：Connected事件后自动发送ClientVersion
            if matches!(event, GameEvent::Connected) {
                // 立即发送ClientVersion到write线程
                if let Err(e) = to_write.send(GameEvent::ClientVersionSend {
                    version_hash: vec![0u8; 16] // TODO: 计算实际MD5
                }) {
                    tracing::error!("Failed to send ClientVersionSend: {}", e);
                }
                tracing::info!("📤 Auto-sending ClientVersion to write thread");
            }
            
            if tx.send(event).is_err() {
                tracing::error!("Game thread disconnected");
                return;
            }
        }
    }
}

/// 写线程：持续接收 GameEvent 并转换为 packet
fn write_loop<S: Write + Send>(mut stream: S, rx: Receiver<GameEvent>) {
    loop {
        // 接收 GameEvent
        let event = match rx.recv() {
            Ok(e) => e,
            Err(_) => {
                tracing::info!("Game thread closed, stopping write thread");
                return;
            }
        };

        // 转换为 packet 并发送
        if let Err(e) = handle_outbound_event(&mut stream, event) {
            tracing::error!("Send packet error: {}", e);
            return;
        }
    }
}

/// 分发 packet 到对应的 handler
/// 
/// 根据 ServerPacketIds 枚举将不同类型的 packet 路由到专门的 handler 处理
fn dispatch_packet(header: &mir2_shared::packets::PacketHeader, payload: &[u8]) -> Vec<GameEvent> {
    use crate::network::handlers::*;
    use mir2_shared::enums::ServerPacketIds as SP;
    
    let opcode = header.opcode as u16;
    
    // 根据 opcode 分发到对应的 handler
    match opcode {
        // ===== Connection & Authentication =====
        x if x == SP::Connected as u16
            || x == SP::ClientVersion as u16
            || x == SP::Disconnect as u16
            || x == SP::KeepAlive as u16 => {
            ConnectionHandler.handle(header, payload)
        }
        
        // ===== Character Management =====
        x if x == SP::NewAccount as u16
            || x == SP::ChangePassword as u16
            || x == SP::ChangePasswordBanned as u16
            || x == SP::Login as u16
            || x == SP::LoginBanned as u16
            || x == SP::LoginSuccess as u16
            || x == SP::NewCharacter as u16
            || x == SP::NewCharacterSuccess as u16
            || x == SP::DeleteCharacter as u16
            || x == SP::DeleteCharacterSuccess as u16
            || x == SP::StartGame as u16
            || x == SP::StartGameBanned as u16
            || x == SP::StartGameDelay as u16
            || x == SP::UserInformation as u16
            || x == SP::UserSlotsRefresh as u16
            || x == SP::LogOutSuccess as u16
            || x == SP::LogOutFailed as u16
            || x == SP::ReturnToLogin as u16 => {
            CharacterHandler.handle(header, payload)
        }
        
        // ===== Map & World =====
        x if x == SP::MapInformation as u16
            || x == SP::NewMapInfo as u16
            || x == SP::WorldMapSetup as u16
            || x == SP::SearchMapResult as u16
            || x == SP::MapChanged as u16
            || x == SP::TimeOfDay as u16 => {
            MovementHandler.handle(header, payload)
        }
        
        // ===== Movement & Position =====
        x if x == SP::UserLocation as u16
            || x == SP::ObjectPlayer as u16
            || x == SP::ObjectHero as u16
            || x == SP::ObjectRemove as u16
            || x == SP::ObjectTurn as u16
            || x == SP::ObjectWalk as u16
            || x == SP::ObjectRun as u16
            || x == SP::ObjectMonster as u16
            || x == SP::ObjectNpc as u16
            || x == SP::ObjectHide as u16
            || x == SP::ObjectShow as u16
            || x == SP::ObjectTeleportOut as u16
            || x == SP::ObjectTeleportIn as u16
            || x == SP::TeleportIn as u16
            || x == SP::UserBackStep as u16
            || x == SP::ObjectBackStep as u16
            || x == SP::UserDash as u16
            || x == SP::ObjectDash as u16
            || x == SP::UserDashFail as u16
            || x == SP::ObjectDashFail as u16
            || x == SP::ObjectSitDown as u16 => {
            MovementHandler.handle(header, payload)
        }
        
        // ===== Combat & Battle =====
        x if x == SP::ObjectAttack as u16
            || x == SP::Struck as u16
            || x == SP::ObjectStruck as u16
            || x == SP::DamageIndicator as u16
            || x == SP::DuraChanged as u16
            || x == SP::HealthChanged as u16
            || x == SP::HeroHealthChanged as u16
            || x == SP::DeleteItem as u16
            || x == SP::Death as u16
            || x == SP::ObjectDied as u16
            || x == SP::GainExperience as u16
            || x == SP::GainHeroExperience as u16
            || x == SP::LevelChanged as u16
            || x == SP::HeroLevelChanged as u16
            || x == SP::ObjectLeveled as u16
            || x == SP::Poisoned as u16
            || x == SP::ObjectPoisoned as u16
            || x == SP::RangeAttack as u16
            || x == SP::ObjectRangeAttack as u16
            || x == SP::Pushed as u16
            || x == SP::ObjectPushed as u16
            || x == SP::UserDashAttack as u16
            || x == SP::ObjectDashAttack as u16
            || x == SP::UserAttackMove as u16
            || x == SP::Revived as u16
            || x == SP::ObjectRevived as u16
            || x == SP::ObjectHealth as u16
            || x == SP::ObjectMana as u16 => {
            CombatHandler.handle(header, payload)
        }
        
        // ===== Chat =====
        x if x == SP::Chat as u16
            || x == SP::ObjectChat as u16 => {
            ChatHandler.handle(header, payload)
        }
        
        // ===== Items & Inventory =====
        x if x == SP::NewItemInfo as u16
            || x == SP::NewHeroInfo as u16
            || x == SP::NewChatItem as u16
            || x == SP::MoveItem as u16
            || x == SP::EquipItem as u16
            || x == SP::MergeItem as u16
            || x == SP::RemoveItem as u16
            || x == SP::RemoveSlotItem as u16
            || x == SP::TakeBackItem as u16
            || x == SP::StoreItem as u16
            || x == SP::SplitItem as u16
            || x == SP::SplitItem1 as u16
            || x == SP::DepositRefineItem as u16
            || x == SP::RetrieveRefineItem as u16
            || x == SP::RefineCancel as u16
            || x == SP::RefineItem as u16
            || x == SP::DepositTradeItem as u16
            || x == SP::RetrieveTradeItem as u16
            || x == SP::UseItem as u16
            || x == SP::DropItem as u16
            || x == SP::TakeBackHeroItem as u16
            || x == SP::TransferHeroItem as u16
            || x == SP::ObjectItem as u16
            || x == SP::ObjectGold as u16
            || x == SP::GainedItem as u16
            || x == SP::GainedGold as u16
            || x == SP::LoseGold as u16
            || x == SP::GainedCredit as u16
            || x == SP::LoseCredit as u16
            || x == SP::RefreshItem as u16
            || x == SP::ObjectHarvest as u16
            || x == SP::ObjectHarvested as u16
            || x == SP::ItemSlotSizeChanged as u16
            || x == SP::ItemSealChanged as u16
            || x == SP::EquipSlotItem as u16
            || x == SP::CombineItem as u16
            || x == SP::ItemUpgraded as u16 => {
            ItemHandler.handle(header, payload)
        }
        
        // ===== NPC & Shop =====
        x if x == SP::NPCResponse as u16
            || x == SP::NPCGoods as u16
            || x == SP::NPCSell as u16
            || x == SP::NPCRepair as u16
            || x == SP::NPCSRepair as u16
            || x == SP::NPCRefine as u16
            || x == SP::NPCCheckRefine as u16
            || x == SP::NPCCollectRefine as u16
            || x == SP::NPCReplaceWedRing as u16
            || x == SP::NPCStorage as u16
            || x == SP::NPCConsign as u16
            || x == SP::NPCMarket as u16
            || x == SP::NPCMarketPage as u16
            || x == SP::ConsignItem as u16
            || x == SP::MarketFail as u16
            || x == SP::MarketSuccess as u16
            || x == SP::SellItem as u16
            || x == SP::CraftItem as u16
            || x == SP::RepairItem as u16
            || x == SP::ItemRepaired as u16
            || x == SP::DefaultNPC as u16
            || x == SP::NPCUpdate as u16
            || x == SP::NPCImageUpdate as u16
            || x == SP::NPCAwakening as u16
            || x == SP::NPCDisassemble as u16
            || x == SP::NPCDowngrade as u16
            || x == SP::NPCReset as u16
            || x == SP::AwakeningNeedMaterials as u16
            || x == SP::AwakeningLockedItem as u16
            || x == SP::Awakening as u16
            || x == SP::NPCPearlGoods as u16
            || x == SP::NPCRequestInput as u16 => {
            NpcHandler.handle(header, payload)
        }
        
        // ===== Group =====
        x if x == SP::SwitchGroup as u16
            || x == SP::DeleteGroup as u16
            || x == SP::DeleteMember as u16
            || x == SP::GroupInvite as u16
            || x == SP::AddMember as u16
            || x == SP::GroupMembersMap as u16
            || x == SP::SendMemberLocation as u16 => {
            GroupHandler.handle(header, payload)
        }
        
        // ===== Guild =====
        x if x == SP::GuildNoticeChange as u16
            || x == SP::GuildMemberChange as u16
            || x == SP::GuildStatus as u16
            || x == SP::GuildInvite as u16
            || x == SP::GuildExpGain as u16
            || x == SP::GuildNameRequest as u16
            || x == SP::GuildStorageGoldChange as u16
            || x == SP::GuildStorageItemChange as u16
            || x == SP::GuildStorageList as u16
            || x == SP::GuildRequestWar as u16
            || x == SP::GuildBuffList as u16
            || x == SP::GuildTerritoryPage as u16
            || x == SP::PurchaseGuildTerritory as u16 => {
            GuildHandler.handle(header, payload)
        }
        
        // ===== Trade =====
        x if x == SP::TradeRequest as u16
            || x == SP::TradeAccept as u16
            || x == SP::TradeGold as u16
            || x == SP::TradeItem as u16
            || x == SP::TradeConfirm as u16
            || x == SP::TradeCancel as u16 => {
            TradeHandler.handle(header, payload)
        }
        
        // ===== Quest =====
        x if x == SP::ChangeQuest as u16
            || x == SP::CompleteQuest as u16
            || x == SP::ShareQuest as u16
            || x == SP::NewQuestInfo as u16
            || x == SP::GainedQuestItem as u16
            || x == SP::DeleteQuestItem as u16 => {
            QuestHandler.handle(header, payload)
        }
        
        // ===== 其他系统功能（暂未分类处理）=====
        x if x == SP::PlayerUpdate as u16
            || x == SP::PlayerInspect as u16
            || x == SP::ChangeAMode as u16
            || x == SP::ChangePMode as u16
            || x == SP::ColourChanged as u16
            || x == SP::ObjectColourChanged as u16
            || x == SP::ObjectGuildNameChanged as u16
            || x == SP::NewMagic as u16
            || x == SP::RemoveMagic as u16
            || x == SP::MagicLeveled as u16
            || x == SP::Magic as u16
            || x == SP::MagicDelay as u16
            || x == SP::MagicCast as u16
            || x == SP::ObjectMagic as u16
            || x == SP::ObjectEffect as u16
            || x == SP::ObjectProjectile as u16
            || x == SP::ObjectName as u16
            || x == SP::UserStorage as u16
            || x == SP::SpellToggle as u16
            || x == SP::MapEffect as u16
            || x == SP::AllowObserve as u16
            || x == SP::AddBuff as u16
            || x == SP::RemoveBuff as u16
            || x == SP::PauseBuff as u16
            || x == SP::ObjectHidden as u16
            || x == SP::ObjectSpell as u16
            || x == SP::InTrapRock as u16
            || x == SP::BaseStatsInfo as u16
            || x == SP::HeroBaseStatsInfo as u16
            || x == SP::UserName as u16
            || x == SP::ChatItemStats as u16
            || x == SP::HeroCreateRequest as u16
            || x == SP::NewHero as u16
            || x == SP::HeroInformation as u16
            || x == SP::UpdateHeroSpawnState as u16
            || x == SP::UnlockHeroAutoPot as u16
            || x == SP::SetAutoPotValue as u16
            || x == SP::SetAutoPotItem as u16
            || x == SP::SetHeroBehaviour as u16
            || x == SP::ManageHeroes as u16
            || x == SP::ChangeHero as u16
            || x == SP::MarriageRequest as u16
            || x == SP::DivorceRequest as u16
            || x == SP::MentorRequest as u16
            || x == SP::MountUpdate as u16
            || x == SP::FishingUpdate as u16
            || x == SP::CancelReincarnation as u16
            || x == SP::RequestReincarnation as u16
            || x == SP::SetConcentration as u16
            || x == SP::SetElemental as u16
            || x == SP::RemoveDelayedExplosion as u16
            || x == SP::ObjectDeco as u16
            || x == SP::ObjectSneaking as u16
            || x == SP::ObjectLevelEffects as u16
            || x == SP::SetBindingShot as u16
            || x == SP::SendOutputMessage as u16
            || x == SP::ReceiveMail as u16
            || x == SP::MailLockedItem as u16
            || x == SP::MailSendRequest as u16
            || x == SP::MailSent as u16
            || x == SP::ParcelCollected as u16
            || x == SP::MailCost as u16
            || x == SP::ResizeInventory as u16
            || x == SP::ResizeStorage as u16
            || x == SP::NewIntelligentCreature as u16
            || x == SP::UpdateIntelligentCreatureList as u16
            || x == SP::IntelligentCreatureEnableRename as u16
            || x == SP::IntelligentCreaturePickup as u16
            || x == SP::TransformUpdate as u16
            || x == SP::FriendUpdate as u16
            || x == SP::LoverUpdate as u16
            || x == SP::MentorUpdate as u16
            || x == SP::GameShopInfo as u16
            || x == SP::GameShopStock as u16
            || x == SP::Rankings as u16
            || x == SP::Opendoor as u16
            || x == SP::GetRentedItems as u16
            || x == SP::ItemRentalRequest as u16
            || x == SP::ItemRentalFee as u16
            || x == SP::ItemRentalPeriod as u16
            || x == SP::DepositRentalItem as u16
            || x == SP::RetrieveRentalItem as u16
            || x == SP::UpdateRentalItem as u16
            || x == SP::CancelItemRental as u16
            || x == SP::ItemRentalLock as u16
            || x == SP::ItemRentalPartnerLock as u16
            || x == SP::CanConfirmItemRental as u16
            || x == SP::ConfirmItemRental as u16
            || x == SP::NewRecipeInfo as u16
            || x == SP::OpenBrowser as u16
            || x == SP::PlaySound as u16
            || x == SP::SetTimer as u16
            || x == SP::ExpireTimer as u16
            || x == SP::UpdateNotice as u16
            || x == SP::Roll as u16
            || x == SP::SetCompass as u16 => {
            // 暂时返回 UnhandledPacket，等待后续实现
            tracing::debug!("📦 未实现的系统功能 packet: 0x{:04X}", opcode);
            vec![GameEvent::UnhandledPacket { opcode: header.opcode }]
        }
        
        // 完全未知的 packet
        _ => {
            tracing::warn!("⚠️ 完全未知的 packet opcode: 0x{:04X}", opcode);
            vec![GameEvent::UnhandledPacket { opcode: header.opcode }]
        }
    }
}

/// 处理出站事件 - 将 GameEvent 转换为网络 packet
///
/// 将游戏层的事件转换为 mir2 协议的 packet 并发送到服务器
fn handle_outbound_event<S: Write>(stream: &mut S, event: GameEvent) -> Result<()> {
    use mir2_shared::packets::{client, serialize_packet};

    match event {
        // ===== 认证相关 =====
        GameEvent::LoginRequest { username, password } => {
            tracing::info!("📤 Sending Login packet: user={}", username);
            let packet = client::Login {
                account_id: username,
                password,
            };
            serialize_packet(stream, &packet)?;
            tracing::info!("✅ Login packet serialized and sent");
        }
        
        GameEvent::NewAccountRequest {
            account_id,
            password,
            birth_date,
            username,
            secret_question,
            secret_answer,
            email,
        } => {
            let packet = client::NewAccount {
                account_id,
                password,
                birth_date_binary: birth_date,
                user_name: username,
                secret_question,
                secret_answer,
                email_address: email,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::ChangePasswordRequest {
            account_id,
            current_password,
            new_password,
        } => {
            let packet = client::ChangePassword {
                account_id,
                current_password,
                new_password,
            };
            serialize_packet(stream, &packet)?;
        }
        
        // ===== 角色管理 =====
        GameEvent::NewCharacterRequest { name, class, gender } => {
            use mir2_shared::enums::{MirClass, MirGender};
            let packet = client::NewCharacter {
                name,
                class: MirClass::try_from(class).unwrap_or(MirClass::Warrior),
                gender: MirGender::try_from(gender).unwrap_or(MirGender::Male),
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::DeleteCharacterRequest { index } => {
            let packet = client::DeleteCharacter {
                character_index: index,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::StartGameRequest { character_index } => {
            let packet = client::StartGame { character_index };
            serialize_packet(stream, &packet)?;
        }
        
        // ===== 移动相关 =====
        GameEvent::WalkRequest { direction } => {
            let packet = client::movement::Walk { direction };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::RunRequest { direction } => {
            let packet = client::movement::Run { direction };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::TurnRequest { direction } => {
            let packet = client::movement::Turn { direction };
            serialize_packet(stream, &packet)?;
        }
        
        // ===== 战斗相关 =====
        GameEvent::AttackRequest { direction, spell } => {
            use mir2_shared::enums::Spell;
            let packet = client::combat::Attack {
                direction,
                spell: Spell::try_from(spell).unwrap_or(Spell::None),
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::MagicRequest {
            spell,
            direction,
            target_id,
            location,
        } => {
            use mir2_shared::{Point, enums::Spell};
            let (x, y) = location.unwrap_or((0, 0));
            let packet = client::combat::Magic {
                spell: Spell::try_from(spell).unwrap_or(Spell::None),
                direction,
                target_id,
                location: Point { x, y },
            };
            serialize_packet(stream, &packet)?;
        }
        
        // ===== 聊天相关 =====
        GameEvent::ChatRequest { message, chat_type: _ } => {
            let packet = client::Chat { message };
            serialize_packet(stream, &packet)?;
            tracing::trace!("💬 Sent chat: {}", packet.message);
        }
        
        // ===== 物品相关 =====
        GameEvent::PickupItemRequest { location: _ } => {
            // PickUp packet 没有参数，客户端自动拾取脚下的物品
            let packet = client::item::PickUp;
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::DropItemRequest { unique_id, count } => {
            let packet = client::item::DropItem {
                unique_id,
                count,
                hero_inventory: false,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::UseItemRequest { unique_id } => {
            let packet = client::item::UseItem { unique_id };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::MoveItemRequest { grid, from, to } => {
            use mir2_shared::enums::MirGridType;
            let packet = client::item::MoveItem {
                grid: MirGridType::try_from(grid).unwrap_or(MirGridType::Inventory),
                from: from as i32,
                to: to as i32,
            };
            serialize_packet(stream, &packet)?;
        }
        
        // ===== 组队相关 =====
        GameEvent::GroupInviteRequest { player_name } => {
            // 在 Mir2 中，组队邀请通过 AddMember 实现
            let packet = client::group::AddMember { name: player_name };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::GroupAcceptRequest => {
            // 接受组队邀请
            let packet = client::group::GroupInvite {
                accept_invite: true,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::GroupDeclineRequest => {
            // 拒绝组队邀请
            let packet = client::group::GroupInvite {
                accept_invite: false,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::GroupLeaveRequest => {
            // 离开组队（通过删除自己实现）
            tracing::warn!("GroupLeaveRequest not implemented - no direct packet");
        }
        
        // ===== 公会相关 =====
        GameEvent::GuildInviteRequest { player_name: _ } => {
            // 公会邀请由服务器处理，客户端无法直接发起
            tracing::warn!("GuildInviteRequest not implemented - server-side only");
        }
        
        GameEvent::GuildAcceptRequest => {
            // 接受公会邀请
            let packet = client::guild::GuildInvite {
                accept_invite: true,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::GuildDeclineRequest => {
            // 拒绝公会邀请
            let packet = client::guild::GuildInvite {
                accept_invite: false,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::GuildLeaveRequest => {
            // 离开公会（通过EditGuildMember实现）
            tracing::warn!("GuildLeaveRequest not implemented - use EditGuildMember");
        }
        
        // ===== 交易相关 =====
        GameEvent::TradeRequest => {
            let packet = client::trade::TradeRequest;
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::TradeReplyRequest { accept } => {
            let packet = client::trade::TradeReply {
                accept_invite: accept,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::TradeGoldRequest { amount } => {
            let packet = client::trade::TradeGold { amount };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::TradeConfirmRequest { locked } => {
            let packet = client::trade::TradeConfirm { locked };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::TradeCancelRequest => {
            let packet = client::trade::TradeCancel;
            serialize_packet(stream, &packet)?;
        }
        
        // ===== NPC 相关 =====
        GameEvent::NPCCallRequest { npc_object_id } => {
            let packet = client::CallNPC {
                object_id: npc_object_id,
                key: String::new(), // 通常为空字符串
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::BuyItemRequest {
            item_index,
            count,
            panel_type,
        } => {
            use mir2_shared::enums::PanelType;
            let packet = client::npc::BuyItem {
                item_index: item_index as u64,
                count: count as u16,
                panel_type: PanelType::try_from(panel_type).unwrap_or(PanelType::Buy),
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::SellItemRequest { unique_id, count } => {
            let packet = client::npc::SellItem {
                unique_id,
                count: count as u16,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::RepairItemRequest { unique_id } => {
            let packet = client::npc::RepairItem { unique_id };
            serialize_packet(stream, &packet)?;
        }
        
        // ===== 任务相关 =====
        GameEvent::AcceptQuestRequest {
            npc_index,
            quest_index,
        } => {
            let packet = client::quest::AcceptQuest {
                npc_index,
                quest_index: quest_index as i32,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::FinishQuestRequest {
            quest_index,
            selected_item,
        } => {
            let packet = client::quest::FinishQuest {
                quest_index: quest_index as i32,
                selected_item_index: selected_item as i32,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::AbandonQuestRequest { quest_index } => {
            let packet = client::quest::AbandonQuest {
                quest_index: quest_index as i32,
            };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::ShareQuestRequest { quest_index } => {
            let packet = client::quest::ShareQuest {
                quest_index: quest_index as i32,
            };
            serialize_packet(stream, &packet)?;
        }
        
        // ===== 连接相关 =====
        GameEvent::KeepAliveSend { time } => {
            let packet = client::KeepAlive { time };
            serialize_packet(stream, &packet)?;
        }
        
        GameEvent::ClientVersionSend { version_hash } => {
            tracing::info!("📤 Sending ClientVersion packet");
            let packet = client::ClientVersion { 
                version_hash: version_hash.clone() 
            };
            serialize_packet(stream, &packet)?;
            tracing::info!("✅ ClientVersion packet sent");
        }
        
        GameEvent::DisconnectRequest => {
            // 断开连接不需要发送packet，直接返回
            tracing::info!("Disconnect requested");
            return Ok(());
        }
        
        // ===== 未实现的事件 =====
        _ => {
            tracing::warn!("⚠️ Unhandled outgoing event: {:?}", event);
            return Ok(());
        }
    }

    stream.flush()?;
    Ok(())
}
