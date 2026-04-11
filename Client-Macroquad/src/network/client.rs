// 网络客户端 - 简单双线程实现
//
// 设计：满足 Read + Write trait 即可，内部两个线程处理读写
// - read_thread: 持续读packet → 解析为 NetworkEvent → 发送到游戏线程
// - write_thread: 持续recv NetworkEvent → 转换为packet → 发送到服务器
//
// 使用：
//   let (tx, rx) = Network::new((write_stream, read_stream));
//   tx.send(NetworkEvent::LoginRequest {...});  // 游戏 → 网络
//   let events = rx.try_iter().collect();    // 网络 → 游戏

use crate::network::handlers::NetworkEvent;
use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::io::{Read, Write};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::Duration;

static OUT_WALK_COUNT: AtomicU64 = AtomicU64::new(0);
static OUT_RUN_COUNT: AtomicU64 = AtomicU64::new(0);
static OUT_ATTACK_COUNT: AtomicU64 = AtomicU64::new(0);

// server protection: avoid flooding Attack packets (some servers disconnect on spam)
static LAST_ATTACK_SENT_MS: AtomicI64 = AtomicI64::new(0);
static OUT_ATTACK_DROPPED: AtomicU64 = AtomicU64::new(0);

// server protection: avoid flooding movement packets (some servers disconnect on "Large amount of Packets")
static LAST_MOVE_SENT_MS: AtomicI64 = AtomicI64::new(0);
static OUT_MOVE_DROPPED: AtomicU64 = AtomicU64::new(0);

/// 网络客户端 - 零大小类型
///
/// 此结构体本身不存储任何数据，只提供静态的 `new()` 方法来创建网络连接。
/// 实际的网络 IO 由内部启动的两个线程处理，通过 channels 与游戏线程通信。
pub struct Network;


impl Network {
    /// 创建并启动网络客户端
    ///
    /// 返回：(发送channel, 接收channel)
    /// - 发送channel: 游戏 → 网络 (NetworkEvent)
    /// - 接收channel: 网络 → 游戏 (NetworkEvent)
    pub fn new<W, R>(
        (w, r): (W, R),
        client_version_hash: [u8; 16],
    ) -> (Sender<NetworkEvent>, Receiver<NetworkEvent>)
    where
        W: Write + Send + 'static,
        R: Read + Send + 'static,
    {
        let (game_to_net_tx, game_to_net_rx) = bounded(1024);
        let (net_to_game_tx, net_to_game_rx) = bounded(1024);

        // 用于跨线程的“断线/关闭”信号，避免 read 线程退出后 write 线程仍在发 KeepAlive。
        let shutdown = Arc::new(AtomicBool::new(false));

        // 读线程：packet → NetworkEvent
        {
            let tx = net_to_game_tx.clone();
            let to_write = game_to_net_tx.clone(); // 用于自动发送ClientVersion等
            let shutdown_flag = shutdown.clone();
            std::thread::Builder::new()
                .name("net-read".into())
                .spawn(move || {
                    read_loop(r, tx, to_write, client_version_hash, shutdown_flag);
                })
                .expect("Failed to spawn read thread");
        }

        // 写线程：NetworkEvent → packet
        {
            let rx = game_to_net_rx;
            let shutdown_flag = shutdown.clone();
            std::thread::Builder::new()
                .name("net-write".into())
                .spawn(move || {
                    write_loop(w, rx, shutdown_flag);
                })
                .expect("Failed to spawn write thread");
        }

        (game_to_net_tx, net_to_game_rx)
    }
}

/// 读线程：持续读取 packet 并转换为 NetworkEvent
fn read_loop<S: Read + Send>(
    mut stream: S,
    tx: Sender<NetworkEvent>,
    to_write: Sender<NetworkEvent>,
    client_version_hash: [u8; 16],
    shutdown: Arc<AtomicBool>,
) {
    use mir2_shared::packets::PacketHeader;
    use mir2_shared::data::stats::SharedError;

    loop {
        let header = {
            match PacketHeader::read_from(&mut stream) {
                Ok(h) => h,
                Err(e) => {
                    shutdown.store(true, Ordering::Relaxed);

                    // Windows 下常见：服务端在 accept 后立刻 close，会表现为 read_u16 UnexpectedEof。
                    // 这通常意味着：IPBlock/MaxIP 限制、端口不对、或服务端尚未进入 Running 状态。
                    let mut reason = e.to_string();
                    if let SharedError::Io(ioe) = &e {
                        if ioe.kind() == std::io::ErrorKind::UnexpectedEof {
                            reason = "Server closed connection immediately (EOF while reading header). \
Possible causes: server IP blocked (often 24h ban after Invalid packet / MaxPacket), MaxIP limit, wrong port, or server not ready.".to_string();
                            tracing::warn!(
                                "Read header EOF: server closed immediately. \
Check server console for 'Too many connections' / 'Invalid packet' / 'Large amount of Packets'. \
If IP was blocked, restart server or use GM command CLEARIPBLOCKS; also verify ServerAddr & server Setup.ini settings."
                            );
                        } else {
                            tracing::error!("Read header IO error: {}", ioe);
                        }
                    } else {
                        tracing::error!("Read header error: {}", e);
                    }
                    let _ = tx.send(NetworkEvent::Disconnected {
                        reason,
                    });
                    break;
                }
            }
        };

        // 读取 payload
        let payload_len = (header.length as usize).saturating_sub(PacketHeader::HEADER_SIZE);
       
        const MAX_PAYLOAD: usize = 1024 * 1024;
        if payload_len > MAX_PAYLOAD {
            tracing::error!("FATAL: payload_len {} > MAX {}", payload_len, MAX_PAYLOAD);
            break;
        }
        
        let mut payload = vec![0u8; payload_len];
        {
            if let Err(e) = stream.read_exact(&mut payload) {
                shutdown.store(true, Ordering::Relaxed);
                tracing::error!("Read payload error: {}", e);
                let _ = tx.send(NetworkEvent::Disconnected {
                    reason: e.to_string(),
                });
                break;
            }
        }

        // 转换为 NetworkEvent（使用现有的 handlers）
        let events = decode_packet(&header, &payload);
        for event in events {
            // 特殊处理：Connected事件后自动发送ClientVersion
            if matches!(event, NetworkEvent::Connected) {
                // 立即发送ClientVersion到write线程
                if let Err(e) = to_write.send(NetworkEvent::ClientVersionSend {
                    version_hash: client_version_hash.to_vec(),
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

/// 写线程：持续接收 NetworkEvent 并转换为 packet
///
/// 自动心跳机制: 如果超过 5 秒没有发送任何包,自动发送 KeepAlive 防止服务器超时断开
fn write_loop<S: Write + Send>(mut stream: S, rx: Receiver<NetworkEvent>, shutdown: Arc<AtomicBool>) {
    let heartbeat_interval = Duration::from_secs(5); // 5秒发送一次心跳 (服务器超时是10秒)

    loop {
        if shutdown.load(Ordering::Relaxed) {
            tracing::info!("Write thread stopping (shutdown signaled)");
            return;
        }

        // 使用 heartbeat_interval 作为超时时间,这样每个心跳周期都会检查一次
        match rx.recv_timeout(heartbeat_interval) {
            Ok(event) => {
                // 收到游戏层的事件,立即发送
                if let Err(e) = handle_outbound_event(&mut stream, event) {
                    shutdown.store(true, Ordering::Relaxed);
                    tracing::error!("Send packet error: {}", e);
                    return;
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                if shutdown.load(Ordering::Relaxed) {
                    tracing::info!("Write thread stopping (shutdown signaled)");
                    return;
                }
                // 超时 = 游戏层在 heartbeat_interval 时间内没有发送任何事件
                // 发送心跳包
                let keep_alive = NetworkEvent::KeepAliveSend {
                    time: chrono::Utc::now().timestamp_millis(),
                };
                if let Err(e) = handle_outbound_event(&mut stream, keep_alive) {
                    shutdown.store(true, Ordering::Relaxed);
                    tracing::error!("❌ Send KeepAlive error: {}", e);
                    return;
                }
                tracing::debug!("💓 Auto-sent KeepAlive (preventing timeout)");
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                tracing::info!("Game thread closed, stopping write thread");
                return;
            }
        }
    }
}

/// 分发 packet 到对应的 handler
///
/// 根据 ServerPacketIds 枚举将不同类型的 packet 路由到专门的 handler 处理
fn decode_packet(header: &mir2_shared::packets::PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
    use crate::network::handlers::*;
    use mir2_shared::enums::ServerPacketIds as SP;

    let opcode = header.opcode as u16;

    // 根据 opcode 分发到对应的 handler
    match opcode {
        // ===== Connection & Authentication =====
        x if x == SP::Connected as u16
            || x == SP::ClientVersion as u16
            || x == SP::Disconnect as u16
            || x == SP::KeepAlive as u16 =>
        {
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
            || x == SP::ReturnToLogin as u16 =>
        {
            CharacterHandler.handle(header, payload)
        }

        // ===== Map & World =====
        x if x == SP::MapInformation as u16
            || x == SP::NewMapInfo as u16
            || x == SP::WorldMapSetup as u16
            || x == SP::SearchMapResult as u16
            || x == SP::MapChanged as u16
            || x == SP::TimeOfDay as u16 =>
        {
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
            || x == SP::ObjectSitDown as u16 =>
        {
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
            || x == SP::ObjectMana as u16
            // Magic/Spell packets
            || x == SP::NewMagic as u16
            || x == SP::RemoveMagic as u16
            || x == SP::MagicLeveled as u16
            || x == SP::Magic as u16
            || x == SP::MagicDelay as u16
            || x == SP::MagicCast as u16
            || x == SP::ObjectMagic as u16
            || x == SP::ObjectEffect as u16
            || x == SP::ObjectProjectile as u16
            || x == SP::SpellToggle as u16
            // Buff packets
            || x == SP::AddBuff as u16
            || x == SP::RemoveBuff as u16
            || x == SP::PauseBuff as u16 =>
        {
            CombatHandler.handle(header, payload)
        }

        // ===== Chat =====
        x if x == SP::Chat as u16 || x == SP::ObjectChat as u16 => {
            ChatHandler.handle(header, payload)
        }

        // ===== Player =====
        x if x == SP::PlayerInspect as u16 => {
            PlayerHandler.handle(header, payload)
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
            || x == SP::ItemUpgraded as u16 =>
        {
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
            || x == SP::NPCRequestInput as u16 =>
        {
            NpcHandler.handle(header, payload)
        }

        // ===== Market/Consign =====
        x if x == SP::NPCConsign as u16
            || x == SP::NPCMarket as u16
            || x == SP::NPCMarketPage as u16
            || x == SP::ConsignItem as u16
            || x == SP::MarketFail as u16
            || x == SP::MarketSuccess as u16 =>
        {
            MarketHandler.handle(header, payload)
        }

        // ===== Group =====
        x if x == SP::SwitchGroup as u16
            || x == SP::DeleteGroup as u16
            || x == SP::DeleteMember as u16
            || x == SP::GroupInvite as u16
            || x == SP::AddMember as u16
            || x == SP::GroupMembersMap as u16
            || x == SP::SendMemberLocation as u16 =>
        {
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
            || x == SP::PurchaseGuildTerritory as u16 =>
        {
            GuildHandler.handle(header, payload)
        }

        // ===== Trade =====
        x if x == SP::TradeRequest as u16
            || x == SP::TradeAccept as u16
            || x == SP::TradeGold as u16
            || x == SP::TradeItem as u16
            || x == SP::TradeConfirm as u16
            || x == SP::TradeCancel as u16 =>
        {
            TradeHandler.handle(header, payload)
        }

        // ===== Quest =====
        x if x == SP::ChangeQuest as u16
            || x == SP::CompleteQuest as u16
            || x == SP::ShareQuest as u16
            || x == SP::NewQuestInfo as u16
            || x == SP::GainedQuestItem as u16
            || x == SP::DeleteQuestItem as u16 =>
        {
            QuestHandler.handle(header, payload)
        }

        // ===== Player State =====
        x if x == SP::PlayerUpdate as u16
            || x == SP::ChangeAMode as u16
            || x == SP::ChangePMode as u16
            || x == SP::ColourChanged as u16
            || x == SP::ObjectColourChanged as u16
            || x == SP::ObjectGuildNameChanged as u16
            || x == SP::ObjectName as u16
            || x == SP::UserName as u16
            || x == SP::ChatItemStats as u16 =>
        {
            CharacterHandler.handle(header, payload)
        }

        // ===== Hero =====
        x if x == SP::HeroCreateRequest as u16
            || x == SP::NewHero as u16
            || x == SP::HeroInformation as u16
            || x == SP::UpdateHeroSpawnState as u16
            || x == SP::UnlockHeroAutoPot as u16
            || x == SP::SetAutoPotValue as u16
            || x == SP::SetAutoPotItem as u16
            || x == SP::SetHeroBehaviour as u16
            || x == SP::ManageHeroes as u16
            || x == SP::ChangeHero as u16
            || x == SP::HeroBaseStatsInfo as u16 =>
        {
            HeroHandler.handle(header, payload)
        }

        // ===== Mail =====
        x if x == SP::ReceiveMail as u16
            || x == SP::MailLockedItem as u16
            || x == SP::MailSendRequest as u16
            || x == SP::MailSent as u16
            || x == SP::ParcelCollected as u16
            || x == SP::MailCost as u16 =>
        {
            MailHandler.handle(header, payload)
        }

        // ===== Intelligent Creature =====
        x if x == SP::NewIntelligentCreature as u16
            || x == SP::UpdateIntelligentCreatureList as u16
            || x == SP::IntelligentCreatureEnableRename as u16
            || x == SP::IntelligentCreaturePickup as u16 =>
        {
            CreatureHandler.handle(header, payload)
        }

        // ===== Social (Marriage/Mentor/Lover) =====
        x if x == SP::MarriageRequest as u16
            || x == SP::DivorceRequest as u16
            || x == SP::MentorRequest as u16
            || x == SP::LoverUpdate as u16
            || x == SP::MentorUpdate as u16 =>
        {
            SocialHandler.handle(header, payload)
        }

        // ===== Fishing =====
        x if x == SP::FishingUpdate as u16 => {
            UiEventsHandler.handle(header, payload)
        }

        // ===== Reincarnation =====
        x if x == SP::CancelReincarnation as u16
            || x == SP::RequestReincarnation as u16 =>
        {
            CharacterHandler.handle(header, payload)
        }

        // ===== Item Rental =====
        x if x == SP::GetRentedItems as u16
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
            || x == SP::ConfirmItemRental as u16 =>
        {
            ItemHandler.handle(header, payload)
        }

        // ===== Misc / Combat & Status =====
        x if x == SP::SetConcentration as u16
            || x == SP::SetElemental as u16
            || x == SP::RemoveDelayedExplosion as u16
            || x == SP::ObjectDeco as u16
            || x == SP::ObjectSneaking as u16
            || x == SP::ObjectLevelEffects as u16
            || x == SP::SetBindingShot as u16
            || x == SP::SendOutputMessage as u16
            || x == SP::InTrapRock as u16
            || x == SP::BaseStatsInfo as u16
            || x == SP::ObjectHidden as u16
            || x == SP::ObjectSpell as u16
            || x == SP::MapEffect as u16
            || x == SP::AllowObserve as u16
            || x == SP::UserStorage as u16 =>
        {
            CombatHandler.handle(header, payload)
        }

        // ===== Item Resize / Transform / Door / Rental =====
        x if x == SP::ResizeInventory as u16
            || x == SP::ResizeStorage as u16
            || x == SP::TransformUpdate as u16
            || x == SP::NewRecipeInfo as u16
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
            || x == SP::ConfirmItemRental as u16 =>
        {
            ItemHandler.handle(header, payload)
        }

        // ===== UI / Timer / Notice / Misc =====
        x if x == SP::SetTimer as u16
            || x == SP::ExpireTimer as u16
            || x == SP::UpdateNotice as u16
            || x == SP::Roll as u16
            || x == SP::SetCompass as u16
            || x == SP::OpenBrowser as u16
            || x == SP::FishingUpdate as u16
            || x == SP::Rankings as u16
            || x == SP::GameShopInfo as u16
            || x == SP::GameShopStock as u16 =>
        {
            UiEventsHandler.handle(header, payload)
        }

        // ===== UI / 表现层事件 =====
        x if x == SP::PlaySound as u16 || x == SP::MountUpdate as u16 => {
            UiEventsHandler.handle(header, payload)
        }

        // 完全未知的 packet
        _ => {
            tracing::warn!("⚠️ 完全未知的 packet opcode: 0x{:04X}", opcode);
            vec![NetworkEvent::UnhandledPacket {
                opcode: header.opcode,
            }]
        }
    }
}

/// 处理出站事件 - 将 NetworkEvent 转换为网络 packet
///
/// 将游戏层的事件转换为 mir2 协议的 packet 并发送到服务器
fn handle_outbound_event<S: Write>(stream: &mut S, event: NetworkEvent) -> Result<()> {
    use mir2_shared::packets::{client, serialize_packet};

    match event {
        // ===== 认证相关 =====
        NetworkEvent::LoginRequest { username, password } => {
            tracing::info!("📤 Sending Login packet: user={}", username);
            let packet = client::Login {
                account_id: username,
                password,
            };
            serialize_packet(stream, &packet)?;
            tracing::info!("✅ Login packet serialized and sent");
        }

        NetworkEvent::NewAccountRequest {
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

        NetworkEvent::ChangePasswordRequest {
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
        NetworkEvent::NewCharacterRequest {
            name,
            class,
            gender,
        } => {
            use mir2_shared::enums::{MirClass, MirGender};
            let packet = client::NewCharacter {
                name,
                class: MirClass::try_from(class).unwrap_or(MirClass::Warrior),
                gender: MirGender::try_from(gender).unwrap_or(MirGender::Male),
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::DeleteCharacterRequest { index } => {
            let packet = client::DeleteCharacter {
                character_index: index,
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::StartGameRequest { character_index } => {
            let packet = client::StartGame { character_index };
            serialize_packet(stream, &packet)?;
        }

        // ===== 移动相关 =====
        NetworkEvent::WalkRequest { direction } => {
            // 节流：服务端默认 5 秒窗口最多 50 次 receive（MaxPacket=50），
            // Walk/Run 往往每个包都变成一次 receive 回调，走路/跑步必须限制发送频率。
            // 注意：这里是兜底节流（输入层还会限速）。
            // 原版客户端移动节拍约 100ms，并且 Run 一次推进 2 格，因此实际移动包频率会低于逐格发送。
            const MIN_MOVE_INTERVAL_MS: i64 = 100;
            let now_ms = chrono::Utc::now().timestamp_millis();
            let last_ms = LAST_MOVE_SENT_MS.load(Ordering::Relaxed);
            let dt = now_ms.saturating_sub(last_ms);
            if last_ms != 0 && dt < MIN_MOVE_INTERVAL_MS {
                let dropped = OUT_MOVE_DROPPED.fetch_add(1, Ordering::Relaxed) + 1;
                if dropped == 1 || dropped % 100 == 0 {
                    tracing::debug!(
                        "🧯 Throttled WalkRequest (dropped x{}, dt={}ms < {}ms)",
                        dropped,
                        dt,
                        MIN_MOVE_INTERVAL_MS
                    );
                }
                return Ok(());
            }
            LAST_MOVE_SENT_MS.store(now_ms, Ordering::Relaxed);

            let packet = client::movement::Walk { direction };
            serialize_packet(stream, &packet)?;

            let n = OUT_WALK_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            if n == 1 || n % 30 == 0 {
                tracing::info!("📤 Sent Walk x{} dir={:?}", n, packet.direction);
            }
        }

        NetworkEvent::RunRequest { direction } => {
            const MIN_MOVE_INTERVAL_MS: i64 = 100;
            let now_ms = chrono::Utc::now().timestamp_millis();
            let last_ms = LAST_MOVE_SENT_MS.load(Ordering::Relaxed);
            let dt = now_ms.saturating_sub(last_ms);
            if last_ms != 0 && dt < MIN_MOVE_INTERVAL_MS {
                let dropped = OUT_MOVE_DROPPED.fetch_add(1, Ordering::Relaxed) + 1;
                if dropped == 1 || dropped % 100 == 0 {
                    tracing::debug!(
                        "🧯 Throttled RunRequest (dropped x{}, dt={}ms < {}ms)",
                        dropped,
                        dt,
                        MIN_MOVE_INTERVAL_MS
                    );
                }
                return Ok(());
            }
            LAST_MOVE_SENT_MS.store(now_ms, Ordering::Relaxed);

            let packet = client::movement::Run { direction };
            serialize_packet(stream, &packet)?;

            let n = OUT_RUN_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            if n == 1 || n % 30 == 0 {
                tracing::info!("📤 Sent Run x{} dir={:?}", n, packet.direction);
            }
        }

        NetworkEvent::TurnRequest { direction } => {
            let packet = client::movement::Turn { direction };
            serialize_packet(stream, &packet)?;
        }

        // ===== 战斗相关 =====
        NetworkEvent::AttackRequest { direction, spell } => {
            use mir2_shared::enums::Spell;

            // 节流：避免攻击包过于频繁导致服务端判定异常（"Large amount of Packets. LastPackets: Attack"）
            // 注：这里用系统时间毫秒做简单门限；即使上层逻辑误触发每帧攻击，也不会把连接打爆。
            const MIN_ATTACK_INTERVAL_MS: i64 = 250;
            let now_ms = chrono::Utc::now().timestamp_millis();
            let last_ms = LAST_ATTACK_SENT_MS.load(Ordering::Relaxed);
            let dt = now_ms.saturating_sub(last_ms);
            if last_ms != 0 && dt < MIN_ATTACK_INTERVAL_MS {
                let dropped = OUT_ATTACK_DROPPED.fetch_add(1, Ordering::Relaxed) + 1;
                // 采样提示，避免刷屏
                if dropped == 1 || dropped % 50 == 0 {
                    tracing::debug!(
                        "🧯 Throttled AttackRequest (dropped x{}, dt={}ms < {}ms)",
                        dropped,
                        dt,
                        MIN_ATTACK_INTERVAL_MS
                    );
                }
                return Ok(());
            }
            LAST_ATTACK_SENT_MS.store(now_ms, Ordering::Relaxed);

            let packet = client::combat::Attack {
                direction,
                spell: Spell::try_from(spell).unwrap_or(Spell::None),
            };
            serialize_packet(stream, &packet)?;

            let n = OUT_ATTACK_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            if n == 1 || n % 10 == 0 {
                tracing::info!("📤 Sent Attack x{} dir={:?} spell={:?}", n, packet.direction, packet.spell);
            }
        }

        NetworkEvent::MagicRequest {
            spell,
            direction,
            target_id,
            location,
        } => {
            use mir2_shared::{enums::Spell, Point};
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
        NetworkEvent::ChatRequest {
            message,
            linked_items,
        } => {
            let packet = client::Chat {
                message,
                linked_items,
            };
            serialize_packet(stream, &packet)?;
            tracing::trace!("💬 Sent chat: {}", packet.message);
        }

        NetworkEvent::InspectRequest { object_id } => {
            let packet = client::Inspect { object_id };
            serialize_packet(stream, &packet)?;
        }

        // ===== 物品相关 =====
        NetworkEvent::PickupItemRequest { location: _ } => {
            // PickUp packet 没有参数，客户端自动拾取脚下的物品
            let packet = client::item::PickUp;
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::DropItemRequest { unique_id, count } => {
            let packet = client::item::DropItem {
                unique_id,
                count,
                hero_inventory: false,
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::UseItemRequest { unique_id } => {
            let packet = client::item::UseItem { unique_id };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::MoveItemRequest { grid, from, to } => {
            use mir2_shared::enums::MirGridType;
            let packet = client::item::MoveItem {
                grid: MirGridType::try_from(grid).unwrap_or(MirGridType::Inventory),
                from: from as i32,
                to: to as i32,
            };
            serialize_packet(stream, &packet)?;
        }

        // ===== 组队相关 =====
        NetworkEvent::GroupInviteRequest { player_name } => {
            // 在 Mir2 中，组队邀请通过 AddMember 实现
            let packet = client::group::AddMember { name: player_name };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::GroupAcceptRequest => {
            // 接受组队邀请
            let packet = client::group::GroupInvite {
                accept_invite: true,
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::GroupDeclineRequest => {
            // 拒绝组队邀请
            let packet = client::group::GroupInvite {
                accept_invite: false,
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::GroupLeaveRequest => {
            // 离开组队（通过删除自己实现）
            tracing::warn!("GroupLeaveRequest not implemented - no direct packet");
        }

        // ===== 公会相关 =====
        NetworkEvent::GuildInviteRequest { player_name: _ } => {
            // 公会邀请由服务器处理，客户端无法直接发起
            tracing::warn!("GuildInviteRequest not implemented - server-side only");
        }

        NetworkEvent::GuildAcceptRequest => {
            // 接受公会邀请
            let packet = client::guild::GuildInvite {
                accept_invite: true,
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::GuildDeclineRequest => {
            // 拒绝公会邀请
            let packet = client::guild::GuildInvite {
                accept_invite: false,
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::GuildLeaveRequest => {
            // 离开公会（通过EditGuildMember实现）
            tracing::warn!("GuildLeaveRequest not implemented - use EditGuildMember");
        }

        // ===== 交易相关 =====
        NetworkEvent::TradeRequest => {
            let packet = client::trade::TradeRequest;
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::TradeReplyRequest { accept } => {
            let packet = client::trade::TradeReply {
                accept_invite: accept,
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::TradeGoldRequest { amount } => {
            let packet = client::trade::TradeGold { amount };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::TradeConfirmRequest { locked } => {
            let packet = client::trade::TradeConfirm { locked };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::TradeCancelRequest => {
            let packet = client::trade::TradeCancel;
            serialize_packet(stream, &packet)?;
        }

        // ===== NPC 相关 =====
        NetworkEvent::NPCCallRequest { npc_object_id, key } => {
            let packet = client::CallNPC {
                object_id: npc_object_id,
                key: key.clone(),
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::BuyItemRequest {
            item_index,
            count,
            panel_type,
        } => {
            use mir2_shared::enums::PanelType;
            let packet = client::npc::BuyItem {
                item_index,
                count: count as u16,
                panel_type: PanelType::try_from(panel_type).unwrap_or(PanelType::Buy),
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::SellItemRequest { unique_id, count } => {
            let packet = client::npc::SellItem {
                unique_id,
                count: count as u16,
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::RepairItemRequest { unique_id } => {
            let packet = client::npc::RepairItem { unique_id };
            serialize_packet(stream, &packet)?;
        }

        // ===== 任务相关 =====
        NetworkEvent::AcceptQuestRequest {
            npc_index,
            quest_index,
        } => {
            let packet = client::quest::AcceptQuest {
                npc_index,
                quest_index: quest_index as i32,
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::FinishQuestRequest {
            quest_index,
            selected_item,
        } => {
            let packet = client::quest::FinishQuest {
                quest_index: quest_index as i32,
                selected_item_index: selected_item as i32,
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::AbandonQuestRequest { quest_index } => {
            let packet = client::quest::AbandonQuest {
                quest_index: quest_index as i32,
            };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::ShareQuestRequest { quest_index } => {
            let packet = client::quest::ShareQuest {
                quest_index: quest_index as i32,
            };
            serialize_packet(stream, &packet)?;
        }

        // ===== 连接相关 =====
        NetworkEvent::KeepAliveSend { time } => {
            let packet = client::KeepAlive { time };
            serialize_packet(stream, &packet)?;
        }

        NetworkEvent::ClientVersionSend { version_hash } => {
            tracing::info!("📤 Sending ClientVersion packet");
            let packet = client::ClientVersion {
                version_hash: version_hash.clone(),
            };
            serialize_packet(stream, &packet)?;
            tracing::info!("✅ ClientVersion packet sent");
        }

        NetworkEvent::DisconnectRequest => {
            // 断开连接不需要发送packet，直接返回
            tracing::info!("Disconnect requested");
            return Ok(());
        }

        // ===== 物品操作扩展 =====

        NetworkEvent::EquipItemRequest { unique_id } => {
            use mir2_shared::enums::MirGridType;
            let packet = client::item::EquipItem {
                grid: MirGridType::Inventory,
                unique_id,
                to: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 EquipItem: unique_id={}", unique_id);
        }

        NetworkEvent::RemoveItemRequest { unique_id } => {
            use mir2_shared::enums::MirGridType;
            let packet = client::item::RemoveItem {
                grid: MirGridType::Equipment,
                unique_id,
                to: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 RemoveItem: unique_id={}", unique_id);
        }

        NetworkEvent::RemoveSlotItemRequest { slot } => {
            use mir2_shared::enums::MirGridType;
            let packet = client::item::RemoveSlotItem {
                grid: MirGridType::Equipment,
                unique_id: 0,
                to: 0,
                from_slot: slot as i32,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 RemoveSlotItem: slot={}", slot);
        }

        NetworkEvent::SplitItemRequest { unique_id, count } => {
            use mir2_shared::enums::MirGridType;
            let packet = client::item::SplitItem {
                grid: MirGridType::Inventory,
                unique_id,
                count,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 SplitItem: unique_id={}, count={}", unique_id, count);
        }

        NetworkEvent::MergeItemRequest { from, to } => {
            use mir2_shared::enums::MirGridType;
            let packet = client::item::MergeItem {
                grid_from: MirGridType::Inventory,
                grid_to: MirGridType::Inventory,
                id_from: from,
                id_to: to,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 MergeItem: from={}, to={}", from, to);
        }

        NetworkEvent::StoreItemRequest { unique_id } => {
            let packet = client::item::StoreItem {
                from: 0,
                to: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 StoreItem: unique_id={} (slot mapping not available)", unique_id);
        }

        NetworkEvent::TakeBackItemRequest { unique_id } => {
            let packet = client::item::TakeBackItem {
                from: 0,
                to: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 TakeBackItem: unique_id={} (slot mapping not available)", unique_id);
        }

        NetworkEvent::DropGoldRequest { amount } => {
            let packet = client::item::DropGold { amount };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 DropGold: amount={}", amount);
        }

        NetworkEvent::EquipSlotItemRequest { slot, unique_id } => {
            use mir2_shared::enums::MirGridType;
            let packet = client::EquipSlotItem {
                grid: MirGridType::Inventory,
                unique_id,
                to_slot: slot as i32,
                grid_to: MirGridType::Equipment,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 EquipSlotItem: slot={}, unique_id={}", slot, unique_id);
        }

        NetworkEvent::CombineItemRequest { from, to } => {
            use mir2_shared::enums::MirGridType;
            let packet = client::CombineItem {
                grid: MirGridType::Inventory,
                id_from: from,
                id_to: to,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 CombineItem: from={}, to={}", from, to);
        }

        NetworkEvent::DropItemStackRequest { unique_id, count } => {
            let packet = client::item::DropItem {
                unique_id,
                count,
                hero_inventory: false,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 DropItemStack: unique_id={}, count={}", unique_id, count);
        }

        // ===== 魔法/技能 =====

        NetworkEvent::MagicKeySet => {
            use mir2_shared::enums::Spell;
            let packet = client::combat::MagicKey {
                spell: Spell::None,
                key: 0,
                old_key: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 MagicKeySet (no spell/key info in event)");
        }

        // ===== 好友 =====

        NetworkEvent::AddFriendRequest { name } => {
            let packet = client::friend::AddFriend {
                name: name.clone(),
                blocked: false,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 AddFriend: name={}", name);
        }

        NetworkEvent::RemoveFriendRequest { object_id } => {
            let packet = client::friend::RemoveFriend {
                character_index: object_id as i32,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 RemoveFriend: object_id={}", object_id);
        }

        NetworkEvent::RefreshFriendsRequest => {
            let packet = client::friend::RefreshFriends;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 RefreshFriends");
        }

        NetworkEvent::AddMemoRequest { object_id, memo } => {
            let packet = client::friend::AddMemo {
                character_index: object_id as i32,
                memo: memo.clone(),
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 AddMemo: object_id={} memo={}", object_id, memo);
        }

        // ===== 公会扩展 =====

        NetworkEvent::EditGuildMember { member_name, rank } => {
            let packet = client::guild::EditGuildMember {
                change_type: 0,
                rank_index: rank,
                name: member_name.clone(),
                rank_name: String::new(),
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 EditGuildMember: member_name={}, rank={}", member_name, rank);
        }

        NetworkEvent::EditGuildNotice { notice } => {
            let packet = client::guild::EditGuildNotice {
                notice_lines: vec![notice],
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 EditGuildNotice");
        }

        NetworkEvent::GuildNameReturn => {
            let packet = client::guild::GuildNameReturn {
                name: String::new(),
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 GuildNameReturn");
        }

        NetworkEvent::RequestGuildInfo => {
            let packet = client::guild::RequestGuildInfo {
                info_type: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 RequestGuildInfo");
        }

        NetworkEvent::GuildStorageGoldChange { amount } => {
            let packet = client::guild::GuildStorageGoldChange {
                change_type: if amount >= 0 { 0 } else { 1 },
                amount: amount.unsigned_abs() as u32,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 GuildStorageGoldChange: amount={}", amount);
        }

        NetworkEvent::GuildStorageItemChangeRequest => {
            let packet = client::guild::GuildStorageItemChange {
                change_type: 0,
                from_slot: 0,
                to_slot: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 GuildStorageItemChange (slot info not available)");
        }

        NetworkEvent::GuildWarReturn => {
            let packet = client::guild::GuildWarReturn {
                guild_name: String::new(),
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 GuildWarReturn");
        }

        NetworkEvent::GuildBuffUpdate { buff_id, action } => {
            let packet = client::guild::GuildBuffUpdate {
                action,
                buff_id: buff_id as i32,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 GuildBuffUpdate: buff_id={}, action={}", buff_id, action);
        }

        // ===== NPC 扩展 =====

        NetworkEvent::LogOutRequest => {
            let packet = client::LogOut;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 LogOut");
        }

        NetworkEvent::HarvestRequest => {
            use mir2_shared::enums::MirDirection;
            let packet = client::combat::Harvest {
                direction: MirDirection::Up,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 Harvest");
        }

        NetworkEvent::BuyItemBackRequest => {
            let packet = client::npc::BuyItemBack {
                unique_id: 0,
                count: 1,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 BuyItemBack (unique_id not available)");
        }

        NetworkEvent::SRepairItemRequest { unique_id } => {
            let packet = client::npc::SRepairItem { unique_id };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 SRepairItem: unique_id={}", unique_id);
        }

        NetworkEvent::CheckRefineRequest => {
            let packet = client::CheckRefine {
                unique_id: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 CheckRefine (unique_id not available)");
        }

        NetworkEvent::ReplaceWedRingRequest => {
            let packet = client::ReplaceWedRing {
                unique_id: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 ReplaceWedRing (unique_id not available)");
        }

        NetworkEvent::NPCConfirmInput { npc_id, input } => {
            let packet = client::npc::NPCConfirmInput {
                npc_id,
                page_name: String::new(),
                value: input,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 NPCConfirmInput: npc_id={}", npc_id);
        }

        // ===== 英雄 =====

        NetworkEvent::CreateHeroRequest { name } => {
            use mir2_shared::enums::{MirClass, MirGender};
            let packet = client::hero::NewHero {
                name,
                gender: MirGender::Male,
                class: MirClass::Warrior,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 NewHero");
        }

        NetworkEvent::SetHeroAutoPotValue { pot_type, value } => {
            let packet = client::hero::SetAutoPotValue {
                stat: pot_type,
                value: (value as u8).min(100),
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 SetAutoPotValue: stat={}, value={}", pot_type, value);
        }

        NetworkEvent::SetHeroAutoPotItem { item_id } => {
            let packet = client::hero::SetAutoPotItem {
                grid: 0,
                item_index: item_id as u64,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 SetAutoPotItem: item_id={}", item_id);
        }

        NetworkEvent::SetHeroBehaviourRequest { behaviour } => {
            use mir2_shared::enums::HeroBehaviour;
            let packet = client::hero::SetHeroBehaviour {
                behaviour: HeroBehaviour::try_from(behaviour).unwrap_or(HeroBehaviour::Attack),
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 SetHero Behaviour: behaviour={}", behaviour);
        }

        NetworkEvent::ChangeHeroRequest { hero_index } => {
            let packet = client::hero::ChangeHero {
                list_index: hero_index,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 ChangeHero: hero_index={}", hero_index);
        }

        // ===== 邮件 =====

        NetworkEvent::SendMailRequest { to, subject, body } => {
            let packet = client::mail::SendMail {
                name: to.clone(),
                message: format!("{}\n{}", subject, body),
                gold: 0,
                items_idx: [0u64; 5],
                stamped: false,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 SendMail: to={}", to);
        }

        NetworkEvent::ReadMailRequest { mail_id } => {
            let packet = client::mail::ReadMail { mail_id };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 ReadMail: mail_id={}", mail_id);
        }

        NetworkEvent::CollectParcelRequest { mail_id } => {
            let packet = client::mail::CollectParcel { mail_id };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 CollectParcel: mail_id={}", mail_id);
        }

        NetworkEvent::DeleteMailRequest { mail_id } => {
            let packet = client::mail::DeleteMail { mail_id };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 DeleteMail: mail_id={}", mail_id);
        }

        NetworkEvent::LockMailRequest { mail_id } => {
            let packet = client::mail::LockMail {
                mail_id,
                lock: true,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 LockMail: mail_id={}", mail_id);
        }

        // ===== 市场/寄售 =====

        NetworkEvent::ConsignItemRequest { item_id, price } => {
            use mir2_shared::enums::MarketPanelType;
            let packet = client::market::ConsignItem {
                unique_id: item_id,
                price: price as u32,
                panel_type: MarketPanelType::Consign,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 ConsignItem: item_id={}, price={}", item_id, price);
        }

        NetworkEvent::MarketSearchRequest { query } => {
            use mir2_shared::enums::{MarketPanelType, ItemType};
            let packet = client::market::MarketSearch {
                match_text: query,
                item_type: ItemType::Nothing,
                user_mode: false,
                min_shape: 0,
                max_shape: 0,
                market_type: MarketPanelType::Consign,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 MarketSearch");
        }

        NetworkEvent::MarketRefreshRequest => {
            let packet = client::market::MarketRefresh;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 MarketRefresh");
        }

        NetworkEvent::MarketPageRequest { page } => {
            let packet = client::market::MarketPage { page: page as i32 };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 MarketPage: page={}", page);
        }

        NetworkEvent::MarketBuyRequest { listing_id } => {
            let packet = client::market::MarketBuy {
                auction_id: listing_id,
                bid_price: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 MarketBuy: listing_id={}", listing_id);
        }

        NetworkEvent::MarketGetBackRequest { listing_id } => {
            let packet = client::market::MarketGetBack {
                mode: 0,
                auction_id: listing_id,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 MarketGetBack: listing_id={}", listing_id);
        }

        NetworkEvent::MarketSellNowRequest { item_id } => {
            let packet = client::market::MarketSellNow {
                auction_id: item_id,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 MarketSellNow: item_id={}", item_id);
        }

        // ===== 智能宠物 =====

        NetworkEvent::UpdateIntelligentCreatureRequest => {
            let packet = client::UpdateIntelligentCreature {
                summon_me: false,
                unsummon_me: false,
                release_me: false,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 UpdateIntelligentCreature");
        }

        NetworkEvent::IntelligentCreaturePickupRequest => {
            use mir2_shared::Point;
            let packet = client::IntelligentCreaturePickup {
                mouse_mode: false,
                location: Point { x: 0, y: 0 },
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 IntelligentCreaturePickup");
        }

        NetworkEvent::RequestIntelligentCreatureUpdates => {
            let packet = client::RequestIntelligentCreatureUpdates {
                update: true,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 RequestIntelligentCreatureUpdates");
        }

        // ===== 社交（婚姻/师徒）=====

        NetworkEvent::MarriageRequestSend { target: _ } => {
            let packet = client::MarriageRequest;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 MarriageRequest");
        }

        NetworkEvent::MarriageReply { accept } => {
            let packet = client::MarriageReply {
                accept_invite: accept,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 MarriageReply: accept={}", accept);
        }

        NetworkEvent::ChangeMarriageRequest => {
            let packet = client::ChangeMarriage;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 ChangeMarriage");
        }

        NetworkEvent::DivorceRequestSend => {
            let packet = client::DivorceRequest;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 DivorceRequest");
        }

        NetworkEvent::DivorceReply { accept } => {
            let packet = client::DivorceReply {
                accept_invite: accept,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 DivorceReply: accept={}", accept);
        }

        NetworkEvent::AddMentorRequest { name } => {
            let packet = client::AddMentor { name: name.clone() };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 AddMentor: name={}", name);
        }

        NetworkEvent::MentorReply { accept } => {
            let packet = client::MentorReply {
                accept_invite: accept,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 MentorReply: accept={}", accept);
        }

        NetworkEvent::AllowMentorRequest { enabled } => {
            if enabled {
                let packet = client::AllowMentor;
                serialize_packet(stream, &packet)?;
            }
            tracing::debug!("📤 AllowMentor: enabled={}", enabled);
        }

        NetworkEvent::CancelMentorRequest => {
            let packet = client::CancelMentor;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 CancelMentor");
        }

        // ===== 租赁 =====

        NetworkEvent::GetRentedItemsRequest => {
            let packet = client::item::GetRentedItems;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 GetRentedItems");
        }

        NetworkEvent::RentalItemDepositRequest { item_id: _ } => {
            // DepositRentalItem uses from/to slots, not item_id
            tracing::debug!("📤 DepositRentalItem: slot mapping not available from item_id");
        }

        NetworkEvent::RentalItemRetrieveRequest { item_id: _ } => {
            // RetrieveRentalItem uses from/to slots, not item_id
            tracing::debug!("📤 RetrieveRentalItem: slot mapping not available from item_id");
        }

        NetworkEvent::ItemRentalConfirm => {
            let packet = client::ConfirmItemRental;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 ConfirmItemRental");
        }

        NetworkEvent::ItemRentalCancel => {
            let packet = client::CancelItemRental;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 CancelItemRental");
        }

        // ===== 钓鱼 =====

        NetworkEvent::FishingCastRequest => {
            let packet = client::FishingCast {
                cast_out: true,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 FishingCast");
        }

        NetworkEvent::FishingAutocastToggle { enabled } => {
            let packet = client::FishingChangeAutocast {
                auto_cast: enabled,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 FishingChangeAutocast: enabled={}", enabled);
        }

        // ===== 坐骑操作 =====
        // 注意：当前协议无独立坐骑发包，通常通过 NPC 对话触发
        NetworkEvent::MountRideRequest { mount_type } => {
            tracing::debug!("🐴 MountRideRequest (type={}) - 需通过 NPC 触发，暂未实现发包", mount_type);
        }
        NetworkEvent::MountDismountRequest => {
            tracing::debug!("🐴 MountDismountRequest - 需通过 NPC 触发，暂未实现发包");
        }

        // ===== 转生 =====

        NetworkEvent::AcceptReincarnationRequest => {
            let packet = client::AcceptReincarnation;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 AcceptReincarnation");
        }

        NetworkEvent::CancelReincarnationRequest => {
            let packet = client::CancelReincarnation;
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 CancelReincarnation");
        }

        // ===== 游戏商店/排名/报告 =====

        NetworkEvent::GameShopBuyRequest { item_id, count } => {
            let packet = client::GameshopBuy {
                g_index: item_id as i32,
                quantity: (count as u8).min(255),
                p_type: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 GameshopBuy: item_id={}, count={}", item_id, count);
        }

        NetworkEvent::ReportIssueRequest { issue } => {
            let packet = client::ReportIssue {
                message: issue,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 ReportIssue");
        }

        NetworkEvent::GetRankingRequest { ranking_type } => {
            let packet = client::GetRanking {
                rank_index: ranking_type,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 GetRanking: ranking_type={}", ranking_type);
        }

        // ===== 门/地图 =====

        NetworkEvent::OpenDoorRequest { door_id } => {
            let packet = client::Opendoor {
                door_index: (door_id as u8).min(255),
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 Opendoor: door_id={}", door_id);
        }

        NetworkEvent::RequestMapInfoRequest => {
            let packet = client::npc::RequestMapInfo {
                map_index: 0,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 RequestMapInfo (map_index not available)");
        }

        NetworkEvent::TeleportToNPCRequest { npc_name } => {
            let packet = client::npc::TeleportToNPC {
                object_id: 0, // npc_name needs resolution to object_id
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 TeleportToNPC: npc_name={} (object_id not resolved)", npc_name);
        }

        NetworkEvent::SearchMapRequest { query } => {
            let packet = client::npc::SearchMap {
                text: query,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 SearchMap");
        }

        NetworkEvent::ObserveRequest { target } => {
            let packet = client::Observe {
                name: target,
            };
            serialize_packet(stream, &packet)?;
            tracing::debug!("📤 Observe");
        }

        // ===== 未实现的事件 =====
        // 注意：大部分 NetworkEvent 是 server→client 的入站事件，
        // 不需要出站发送。这个分支是安全兜底。
        _ => {}
    }

    stream.flush()?;
    Ok(())
}
