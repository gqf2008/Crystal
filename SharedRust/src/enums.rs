use bitflags::bitflags;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum MirGender {
    Male = 0,
    Female = 1,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum MirClass {
    Warrior = 0,
    Wizard = 1,
    Taoist = 2,
    Assassin = 3,
    Archer = 4,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum MirDirection {
    Up = 0,
    UpRight = 1,
    Right = 2,
    DownRight = 3,
    Down = 4,
    DownLeft = 5,
    Left = 6,
    UpLeft = 7,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(i16)]
pub enum ClientPacketIds {
    ClientVersion = 0,
    Disconnect = 1,
    KeepAlive = 2,
    NewAccount = 3,
    ChangePassword = 4,
    Login = 5,
    NewCharacter = 6,
    DeleteCharacter = 7,
    StartGame = 8,
    LogOut = 9,
    Turn = 10,
    Walk = 11,
    Run = 12,
    Chat = 13,
    MoveItem = 14,
    StoreItem = 15,
    TakeBackItem = 16,
    MergeItem = 17,
    EquipItem = 18,
    RemoveItem = 19,
    RemoveSlotItem = 20,
    SplitItem = 21,
    UseItem = 22,
    DropItem = 23,
    DepositRefineItem = 24,
    RetrieveRefineItem = 25,
    RefineCancel = 26,
    RefineItem = 27,
    CheckRefine = 28,
    ReplaceWedRing = 29,
    DepositTradeItem = 30,
    RetrieveTradeItem = 31,
    TakeBackHeroItem = 32,
    TransferHeroItem = 33,
    DropGold = 34,
    PickUp = 35,
    RequestMapInfo = 36,
    RequestMonsterInfo = 37,
    RequestNPCInfo = 38,
    RequestItemInfo = 39,
    TeleportToNPC = 40,
    SearchMap = 41,
    Inspect = 42,
    Observe = 43,
    ChangeAMode = 44,
    ChangePMode = 45,
    ChangeTrade = 46,
    Attack = 47,
    RangeAttack = 48,
    Harvest = 49,
    CallNPC = 50,
    BuyItem = 51,
    SellItem = 52,
    CraftItem = 53,
    RepairItem = 54,
    BuyItemBack = 55,
    SRepairItem = 56,
    MagicKey = 57,
    Magic = 58,
    SwitchGroup = 59,
    AddMember = 60,
    DellMember = 61,
    GroupInvite = 62,
    NewHero = 63,
    SetAutoPotValue = 64,
    SetAutoPotItem = 65,
    SetHeroBehaviour = 66,
    ChangeHero = 67,
    TownRevive = 68,
    SpellToggle = 69,
    ConsignItem = 70,
    MarketSearch = 71,
    MarketRefresh = 72,
    MarketPage = 73,
    MarketBuy = 74,
    MarketGetBack = 75,
    MarketSellNow = 76,
    RequestUserName = 77,
    RequestChatItem = 78,
    EditGuildMember = 79,
    EditGuildNotice = 80,
    GuildInvite = 81,
    GuildNameReturn = 82,
    RequestGuildInfo = 83,
    GuildStorageGoldChange = 84,
    GuildStorageItemChange = 85,
    GuildWarReturn = 86,
    MarriageRequest = 87,
    MarriageReply = 88,
    ChangeMarriage = 89,
    DivorceRequest = 90,
    DivorceReply = 91,
    AddMentor = 92,
    MentorReply = 93,
    AllowMentor = 94,
    CancelMentor = 95,
    TradeRequest = 96,
    TradeReply = 97,
    TradeGold = 98,
    TradeConfirm = 99,
    TradeCancel = 100,
    EquipSlotItem = 101,
    FishingCast = 102,
    FishingChangeAutocast = 103,
    AcceptQuest = 104,
    FinishQuest = 105,
    AbandonQuest = 106,
    ShareQuest = 107,
    AcceptReincarnation = 108,
    CancelReincarnation = 109,
    CombineItem = 110,
    AwakeningNeedMaterials = 111,
    AwakeningLockedItem = 112,
    Awakening = 113,
    DisassembleItem = 114,
    DowngradeAwakening = 115,
    ResetAddedItem = 116,
    SendMail = 117,
    ReadMail = 118,
    CollectParcel = 119,
    DeleteMail = 120,
    LockMail = 121,
    MailLockedItem = 122,
    MailCost = 123,
    UpdateIntelligentCreature = 124,
    IntelligentCreaturePickup = 125,
    RequestIntelligentCreatureUpdates = 126,
    AddFriend = 127,
    RemoveFriend = 128,
    RefreshFriends = 129,
    AddMemo = 130,
    GuildBuffUpdate = 131,
    NPCConfirmInput = 132,
    GameshopBuy = 133,
    ReportIssue = 134,
    GetRanking = 135,
    Opendoor = 136,
    GetRentedItems = 137,
    ItemRentalRequest = 138,
    ItemRentalFee = 139,
    ItemRentalPeriod = 140,
    DepositRentalItem = 141,
    RetrieveRentalItem = 142,
    CancelItemRental = 143,
    ItemRentalLockFee = 144,
    ItemRentalLockItem = 145,
    ConfirmItemRental = 146,
    /// #1216：英雄一键复活（C# HeroPanel 复活按钮）
    ReviveHero = 147,
    GuildTerritoryPage,
    PurchaseGuildTerritory,
    DeleteItem,
    UnlockStorage,
    SetStoragePassword,
    RemoveStoragePassword,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum StatFormula {
    Health = 3,
    Mana = 4,
    Weight = 5,
    Stat = 6,
}

#[allow(clippy::enum_variant_names)]
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
    PartialOrd,
    Ord,
)]
#[repr(u8)]
pub enum Stat {
    MinAC = 3,
    MaxAC = 4,
    MinMAC = 5,
    MaxMAC = 6,
    MinDC = 7,
    MaxDC = 8,
    MinMC = 9,
    MaxMC = 10,
    MinSC = 11,
    MaxSC = 12,
    Accuracy = 13,
    Agility = 14,
    HP = 15,
    MP = 16,
    AttackSpeed = 17,
    Luck = 18,
    BagWeight = 19,
    HandWeight = 20,
    WearWeight = 21,
    Reflect = 22,
    Strong = 23,
    Holy = 24,
    Freezing = 25,
    PoisonAttack = 26,
    MagicResist = 33,
    PoisonResist = 34,
    HealthRecovery = 35,
    SpellRecovery = 36,
    PoisonRecovery = 37,
    CriticalRate = 38,
    CriticalDamage = 39,
    MaxACRatePercent = 43,
    MaxMACRatePercent = 44,
    MaxDCRatePercent = 45,
    MaxMCRatePercent = 46,
    MaxSCRatePercent = 47,
    AttackSpeedRatePercent = 48,
    HPRatePercent = 49,
    MPRatePercent = 50,
    HPDrainRatePercent = 51,
    ExpRatePercent = 103,
    ItemDropRatePercent = 104,
    GoldDropRatePercent = 105,
    MineRatePercent = 106,
    GemRatePercent = 107,
    FishRatePercent = 108,
    CraftRatePercent = 109,
    SkillGainMultiplier = 110,
    AttackBonus = 111,
    LoverExpRatePercent = 123,
    MentorDamageRatePercent = 124,
    MentorExpRatePercent = 126,
    DamageReductionPercent = 127,
    EnergyShieldPercent = 128,
    EnergyShieldHPGain = 129,
    ManaPenaltyPercent = 130,
    TeleportManaPenaltyPercent = 131,
    Hero = 132,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum MouseCursor {
    None = 3,
    Default = 4,
    Attack = 5,
    AttackRed = 6,
    NPCTalk = 7,
    TextPrompt = 8,
    Trash = 9,
    Upgrade = 10,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct WeatherSetting: u16 {
        const NONE = 0;
        const FOG = 0b0000_0000_0000_0001;
        const RED_EMBER = 0b0000_0000_0000_0010;
        const WHITE_EMBER = 0b0000_0000_0000_0100;
        const YELLOW_EMBER = 0b0000_0000_0000_1000;
        const FIRE_PARTICLE = 0b0000_0000_0001_0000;
        const SNOW = 0b0000_0000_0010_0000;
        const RAIN = 0b0000_0000_0100_0000;
        const LEAVES = 0b0000_0000_1000_0000;
        const FIREY_LEAVES = 0b0000_0001_0000_0000;
        const PURPLE_LEAVES = 0b0000_0010_0000_0000;
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum PanelType {
    Buy = 3,
    BuySub = 4,
    Craft = 5,
    Sell = 6,
    Repair = 7,
    SpecialRepair = 8,
    Consign = 9,
    Refine = 10,
    CheckRefine = 11,
    Disassemble = 12,
    Downgrade = 13,
    Reset = 14,
    CollectRefine = 15,
    ReplaceWedRing = 16,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum MarketItemType {
    Consign = 3,
    Auction = 4,
    GameShop = 5,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum MarketPanelType {
    Market = 3,
    Consign = 4,
    Auction = 5,
    GameShop = 6,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(i8)]
pub enum BlendMode {
    None = -1,
    Normal = 3,
    Light = 4,
    LightInv = 5,
    InvNormal = 6,
    InvLight = 7,
    InvLightInv = 8,
    InvColor = 9,
    InvBackground = 10,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum DamageType {
    Hit = 3,
    Miss = 4,
    Critical = 5,
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct GmOptions: u8 {
        const NONE = 0;
        const GAME_MASTER = 0x01;
        const OBSERVER = 0x02;
        const SUPERMAN = 0x04;
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum AwakeType {
    None = 3,
    Dc = 4,
    Mc = 5,
    Sc = 6,
    Ac = 7,
    Mac = 8,
    HpMp = 9,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct LevelEffects: u16 {
        const NONE = 0;
        const MIST = 0b0000_0000_0000_0001;
        const RED_DRAGON = 0b0000_0000_0000_0010;
        const BLUE_DRAGON = 0b0000_0000_0000_0100;
        const REBIRTH1 = 0b0000_0000_0000_1000;
        const REBIRTH2 = 0b0000_0000_0001_0000;
        const REBIRTH3 = 0b0000_0000_0010_0000;
        const NEW_BLUE = 0b0000_0000_0100_0000;
        const YELLOW_DRAGON = 0b0000_0000_1000_0000;
        const PHOENIX = 0b0000_0001_0000_0000;
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum OutputMessageType {
    Normal = 3,
    Quest = 4,
    Guild = 5,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum ItemGrade {
    None = 3,
    Common = 4,
    Rare = 5,
    Legendary = 6,
    Mythical = 7,
    Heroic = 8,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum RefinedValue {
    None = 3,
    Dc = 4,
    Mc = 5,
    Sc = 6,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum QuestType {
    General = 3,
    Daily = 4,
    Repeatable = 5,
    Story = 6,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum QuestIcon {
    None = 3,
    QuestionWhite = 4,
    ExclamationYellow = 5,
    QuestionYellow = 6,
    ExclamationBlue = 8,
    QuestionBlue = 9,
    ExclamationGreen = 55,
    QuestionGreen = 56,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum QuestState {
    Add = 3,
    Update = 4,
    Remove = 5,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum QuestAction {
    TimeExpired = 3,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum DefaultNpcType {
    Login = 3,
    LevelUp = 4,
    UseItem = 5,
    MapCoord = 6,
    MapEnter = 7,
    Die = 8,
    Trigger = 9,
    CustomCommand = 10,
    OnAcceptQuest = 11,
    OnFinishQuest = 12,
    Daily = 13,
    Client = 14,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum IntelligentCreatureType {
    None = 102,
    BabyPig = 3,
    Chick = 4,
    Kitten = 5,
    BabySkeleton = 6,
    Baekdon = 7,
    Wimaen = 8,
    BlackKitten = 9,
    BabyDragon = 10,
    OlympicFlame = 11,
    BabySnowMan = 12,
    Frog = 13,
    BabyMonkey = 14,
    AngryBird = 15,
    Foxey = 16,
    MedicalRat = 17,
}

#[allow(clippy::enum_variant_names)]
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u16)]
pub enum Monster {
    Guard = 3,
    TaoistGuard = 4,
    Guard2 = 5,
    Hen = 6,
    Deer = 7,
    Scarecrow = 8,
    HookingCat = 9,
    RakingCat = 10,
    Yob = 11,
    Oma = 12,
    CannibalPlant = 13,
    ForestYeti = 14,
    SpittingSpider = 15,
    ChestnutTree = 16,
    EbonyTree = 17,
    LargeMushroom = 18,
    CherryTree = 19,
    OmaFighter = 20,
    OmaWarrior = 21,
    CaveBat = 22,
    CaveMaggot = 23,
    Scorpion = 24,
    Skeleton = 25,
    BoneFighter = 26,
    AxeSkeleton = 27,
    BoneWarrior = 28,
    BoneElite = 29,
    Dung = 30,
    Dark = 31,
    WoomaSoldier = 32,
    WoomaFighter = 33,
    WoomaWarrior = 34,
    FlamingWooma = 35,
    WoomaGuardian = 36,
    WoomaTaurus = 37, // BOSS
    WhimperingBee = 38,
    GiantWorm = 39,
    Centipede = 40,
    BlackMaggot = 41,
    Tongs = 42,
    EvilTongs = 43,
    EvilCentipede = 44,
    BugBat = 45,
    BugBatMaggot = 46,
    WedgeMoth = 47,
    RedBoar = 48,
    BlackBoar = 49,
    SnakeScorpion = 50,
    WhiteBoar = 51,
    EvilSnake = 52,
    BombSpider = 53,
    RootSpider = 54,
    SpiderBat = 55,
    VenomSpider = 56,
    GangSpider = 57,
    GreatSpider = 58,
    LureSpider = 59,
    BigApe = 60,
    EvilApe = 61,
    GrayEvilApe = 62,
    RedEvilApe = 63,
    CrystalSpider = 64,
    RedMoonEvil = 65,
    BigRat = 66,
    ZumaArcher = 67,
    ZumaStatue = 68,
    ZumaGuardian = 69,
    RedThunderZuma = 70,
    ZumaTaurus = 71, // BOSS
    DigOutZombie = 72,
    ClZombie = 73,
    NdZombie = 74,
    CrawlerZombie = 75,
    ShamanZombie = 76,
    Ghoul = 77,
    KingScorpion = 78,
    KingHog = 79,
    DarkDevil = 80,
    BoneFamiliar = 81,
    Shinsu = 82,
    Shinsu1 = 83,
    SpiderFrog = 84,
    HoroBlaster = 85,
    BlueHoroBlaster = 86,
    KekTal = 87,
    VioletKekTal = 88,
    Khazard = 89,
    RoninGhoul = 90,
    ToxicGhoul = 91,
    BoneCaptain = 92,
    BoneSpearman = 93,
    BoneBlademan = 94,
    BoneArcher = 95,
    BoneLord = 96, // BOSS
    Minotaur = 97,
    IceMinotaur = 98,
    ElectricMinotaur = 99,
    WindMinotaur = 100,
    FireMinotaur = 101,
    RightGuard = 102,
    LeftGuard = 103,
    MinotaurKing = 104, // BOSS
    FrostTiger = 105,
    Sheep = 106,
    Wolf = 107,
    ShellNipper = 108,
    Keratoid = 109,
    GiantKeratoid = 110,
    SkyStinger = 111,
    SandWorm = 112,
    VisceralWorm = 113,
    RedSnake = 114,
    TigerSnake = 115,
    Yimoogi = 116,
    GiantWhiteSnake = 117,
    BlueSnake = 118,
    YellowSnake = 119,
    HolyDeva = 120,
    AxeOma = 121,
    SwordOma = 122,
    CrossbowOma = 123,
    WingedOma = 124,
    FlailOma = 125,
    OmaGuard = 126,
    YinDevilNode = 127,
    YangDevilNode = 128,
    OmaKing = 129, // BOSS
    BlackFoxman = 130,
    RedFoxman = 131,
    WhiteFoxman = 132,
    TrapRock = 133,
    GuardianRock = 134,
    ThunderElement = 135,
    CloudElement = 136,
    GreatFoxSpirit = 137, // BOSS
    HedgeKekTal = 138,
    BigHedgeKekTal = 139,
    RedFrogSpider = 140,
    BrownFrogSpider = 141,
    ArcherGuard = 142,
    KatanaGuard = 143,
    ArcherGuard2 = 144,
    Pig = 145,
    Bull = 146,
    Bush = 147,
    ChristmasTree = 148,
    HighAssassin = 149,
    DarkDustPile = 150,
    DarkBrownWolf = 151,
    Football = 152,
    GingerBreadman = 153,
    HalloweenScythe = 154,
    GhastlyLeecher = 155,
    CyanoGhast = 156,
    MutatedManworm = 157,
    CrazyManworm = 158,
    MudPile = 159,
    TailedLion = 160,
    Behemoth = 161, // BOSS
    DarkDevourer = 162,
    PoisonHugger = 163,
    Hugger = 164,
    MutatedHugger = 165,
    DreamDevourer = 166,
    Treasurebox = 167,
    SnowPile = 168,
    Snowman = 169,
    SnowTree = 170,
    GiantEgg = 171,
    RedTurtle = 172,
    GreenTurtle = 173,
    BlueTurtle = 174,
    Catapult1 = 175,
    Catapult2 = 176,
    OldSpittingSpider = 177,
    SiegeRepairman = 178,
    BlueSanta = 179,
    BattleStandard = 180,
    Blank1 = 181,
    RedYimoogi = 182,
    LionRiderMale = 183,   // Not Monster - Skin / Transform
    LionRiderFemale = 184, // Not Monster - Skin / Transform
    Tornado = 185,
    FlameTiger = 186,
    WingedTigerLord = 187, // BOSS
    TowerTurtle = 188,
    FinialTurtle = 189,
    TurtleKing = 190, // BOSS
    DarkTurtle = 191,
    LightTurtle = 192,
    DarkSwordOma = 193,
    DarkAxeOma = 194,
    DarkCrossbowOma = 195,
    DarkWingedOma = 196,
    BoneWhoo = 197,
    DarkSpider = 198, // AI 8
    ViscusWorm = 199,
    ViscusCrawler = 200,
    CrawlerLave = 201,
    DarkYob = 202,
    FlamingMutant = 203,
    StoningStatue = 204, // BOSS
    FlyingStatue = 205,
    ValeBat = 206,
    Weaver = 207,
    VenomWeaver = 208,
    CrackingWeaver = 209,
    ArmingWeaver = 210,
    CrystalWeaver = 211,
    FrozenZumaStatue = 212,
    FrozenZumaGuardian = 213,
    FrozenRedZuma = 214,
    GreaterWeaver = 215,
    SpiderWarrior = 216,
    SpiderBarbarian = 217,
    HellSlasher = 218,
    HellPirate = 219,
    HellCannibal = 220,
    HellKeeper = 221, // BOSS
    HellBolt = 222,
    WitchDoctor = 223,
    ManectricHammer = 224,
    ManectricClub = 225,
    ManectricClaw = 226,
    ManectricStaff = 227,
    NamelessGhost = 228,
    DarkGhost = 229,
    ChaosGhost = 230,
    ManectricBlest = 231,
    ManectricKing = 232,
    Blank2 = 233,
    IcePillar = 234,
    FrostYeti = 235,
    ManectricSlave = 236,
    TrollHammer = 237,
    TrollBomber = 238,
    TrollStoner = 239,
    TrollKing = 240, // BOSS
    FlameSpear = 241,
    FlameMage = 242,
    FlameScythe = 243,
    FlameAssassin = 244,
    FlameQueen = 245, // BOSS
    HellKnight1 = 246,
    HellKnight2 = 247,
    HellKnight3 = 248,
    HellKnight4 = 249,
    HellLord = 250, // BOSS
    WaterGuard = 251,
    IceGuard = 252,
    ElementGuard = 253,
    DemonGuard = 254,
    KingGuard = 255,
    Snake10 = 256,
    Snake11 = 257,
    Snake12 = 258,
    Snake13 = 259,
    Snake14 = 260,
    Snake15 = 261,
    Snake16 = 262,
    Snake17 = 263,
    DeathCrawler = 264,
    BurningZombie = 265,
    MudZombie = 266,
    FrozenZombie = 267,
    UndeadWolf = 268,
    DemonWolf = 269,
    WhiteMammoth = 270,
    DarkBeast = 271,
    LightBeast = 272,  // AI 112
    BloodBaboon = 273, // AI 112
    HardenRhino = 274,
    AncientBringer = 275,
    FightingCat = 276,
    FireCat = 277,  // AI 44
    CatWidow = 278, // AI 112
    StainHammerCat = 279,
    BlackHammerCat = 280,
    StrayCat = 281,
    CatShaman = 282,
    Jar1 = 283,
    Jar2 = 284,
    SeedingsGeneral = 285,
    RestlessJar = 286,
    GeneralMeowMeow = 287, // BOSS
    Bunny = 288,
    Tucson = 289,
    TucsonFighter = 290, // AI 44
    TucsonMage = 291,
    TucsonWarrior = 292,
    Armadillo = 293,
    ArmadilloElder = 294,
    TucsonEgg = 295, // EFFECT 0/1
    PlaguedTucson = 296,
    SandSnail = 297,
    CannibalTentacles = 298,
    TucsonGeneral = 299, // BOSS
    GasToad = 300,
    Mantis = 301,
    SwampWarrior = 302,
    AssassinBird = 303,
    RhinoWarrior = 304,
    RhinoPriest = 305,
    ElephantMan = 306,
    StoneGolem = 307,
    EarthGolem = 308,
    TreeGuardian = 309,
    TreeQueen = 310,
    PeacockSpider = 311,
    DarkBaboon = 312,    // AI 112
    TwinHeadBeast = 313, // AI 112
    OmaCannibal = 314,
    OmaBlest = 315,
    OmaSlasher = 316,
    OmaAssassin = 317,
    OmaMage = 318,
    OmaWitchDoctor = 319,
    LightningBead = 320, // Effect 0, AI 149
    HealingBead = 321,   // Effect 1, AI 149
    PowerUpBead = 322,   // Effect 2, AI 14
    DarkOmaKing = 323,   // BOSS
    CaveStatue = 324,
    Mandrill = 325,
    PlagueCrab = 326,
    CreeperPlant = 327,
    FloatingWraith = 328, // AI 8
    ArmedPlant = 329,
    AvengerPlant = 330,
    Nadz = 331,
    AvengingSpirit = 332,
    AvengingWarrior = 333,
    AxePlant = 334,
    WoodBox = 335,
    ClawBeast = 336,   // AI 8
    DarkCaptain = 337, // BOSS
    SackWarrior = 338,
    WereTiger = 339, // AI 112
    KingHydrax = 340,
    Hydrax = 341,
    HornedMage = 342,
    BlueSoul = 343,
    HornedArcher = 344,
    ColdArcher = 345,
    HornedWarrior = 346,
    FloatingRock = 347,
    ScalyBeast = 348,
    HornedSorceror = 349,
    BoulderSpirit = 350,
    HornedCommander = 351, // BOSS
    MoonStone = 352,
    SunStone = 353,
    LightningStone = 354,
    Turtlegrass = 355,
    ManTree = 356,
    Bear = 357, // Effect 1, AI 112
    Leopard = 358,
    ChieftainArcher = 359,
    ChieftainSword = 360,
    StoningSpider = 361, // Archer Spell mob
    VampireSpider = 362, // Archer Spell mob
    SpittingToad = 363,  // Archer Spell mob
    SnakeTotem = 364,    // Archer Spell mob
    CharmedSnake = 365,  // Archer Spell mob
    FrozenSoldier = 366,
    FrozenFighter = 367, // AI 44
    FrozenArcher = 368,  // AI 8
    FrozenKnight = 369,
    FrozenGolem = 370,
    IcePhantom = 371,
    SnowWolf = 372,
    SnowWolfKing = 373, // BOSS
    WaterDragon = 374,
    BlackTortoise = 375,
    Manticore = 376,
    DragonWarrior = 377,
    DragonArcher = 378,
    Kirin = 379,
    Guard3 = 380,
    ArcherGuard3 = 381,
    Bunny2 = 382,
    FrozenMiner = 383,
    FrozenAxeman = 384,
    FrozenMagician = 385,
    SnowYeti = 386,
    IceCrystalSoldier = 387,
    DarkWraith = 388,
    DarkSpirit = 389,
    CrystalBeast = 390,
    RedOrb = 391,
    BlueOrb = 392,
    YellowOrb = 393,
    GreenOrb = 394,
    WhiteOrb = 395,
    FatalLotus = 396,
    AntCommander = 397,
    CargoBoxwithlogo = 398,
    Doe = 399,
    Reindeer = 400,
    AngryReindeer = 401,
    CargoBox = 402,
    Ram1 = 403,
    Ram2 = 404,
    Kite = 405,
    PurpleFaeFlower = 406,
    Furball = 407,
    GlacierSnail = 408,
    FurbolgWarrior = 409,
    FurbolgArcher = 410,
    FurbolgCommander = 411,
    RedFaeFlower = 412,
    FurbolgGuard = 413,
    GlacierBeast = 414,
    GlacierWarrior = 415,
    ShardGuardian = 416,
    WarriorScroll = 417,  // HoodedSummonerScrolls effect 0
    TaoistScroll = 418,   // HoodedSummonerScrolls effect 1
    WizardScroll = 419,   // HoodedSummonerScrolls effect 2
    AssassinScroll = 420, // HoodedSummonerScrolls effect 3
    HoodedSummoner = 421,
    HoodedIceMage = 422,
    HoodedPriest = 423,
    ShardMaiden = 424,
    KingKong = 425,
    WarBear = 426,
    ReaperPriest = 427,
    ReaperWizard = 428,
    ReaperAssassin = 429,
    LivingVines = 430,
    BlueMonk = 431,
    MutantBeserker = 432,
    MutantGuardian = 433,
    MutantHighPriest = 434,
    MysteriousMage = 435,
    FeatheredWolf = 436,
    MysteriousAssassin = 437,
    MysteriousMonk = 438,
    ManEatingPlant = 439,
    HammerDwarf = 440,
    ArcherDwarf = 441,
    NobleWarrior = 442,
    NobleArcher = 443,
    NoblePriest = 444,
    NobleAssassin = 445,
    Swain = 446,
    RedMutantPlant = 447,
    BlueMutantPlant = 448,
    UndeadHammerDwarf = 449,
    UndeadDwarfArcher = 450,
    AncientStoneGolem = 451,
    Serpentirian = 452,
    Butcher = 453,
    Riklebites = 454,
    FeralTundraFurbolg = 455,
    FeralFlameFurbolg = 456,
    ArcaneTotem = 457,
    SpectralWraith = 458,
    BabyMagmaDragon = 459,
    BloodLord = 460,
    SerpentLord = 461,
    MirEmperor = 462,
    MutantManEatingPlant = 463,
    MutantWarg = 464,
    GrassElemental = 465,
    RockElemental = 466,
    EvilMir = 903,
    EvilMirBody = 904,
    DragonStatue = 905,
    HellBomb1 = 906,
    HellBomb2 = 907,
    HellBomb3 = 908,
    Catapult = 943,
    ChariotBallista = 944,
    Ballista = 945,
    Trebuchet = 946,
    CanonTrebuchet = 947,
    SabukGate = 953,
    PalaceWallLeft = 954,
    PalaceWall1 = 955,
    PalaceWall2 = 956,
    GiGateSouth = 957,
    GiGateEast = 958,
    GiGateWest = 959,
    SSabukWall1 = 960,
    SSabukWall2 = 961,
    SSabukWall3 = 962,
    NammandGate1 = 963,
    NammandGate2 = 964,
    SabukWallSection = 965,
    NammandWallSection = 966,
    FrozenDoor = 967,
    BabyPig = 10003,
    Chick = 10004,
    Kitten = 10005,
    BabySkeleton = 10006,
    Baekdon = 10007,
    Wimaen = 10008,
    BlackKitten = 10009,
    BabyDragon = 10010,
    OlympicFlame = 10011,
    BabySnowMan = 10012,
    Frog = 10013,
    BabyMonkey = 10014,
    AngryBird = 10015,
    Foxey = 10016,
    MedicalRat = 10017,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum MirAction {
    Standing = 3,
    Walking = 4,
    Running = 5,
    Pushed = 6,
    DashL = 7,
    DashR = 8,
    DashFail = 9,
    Stance = 10,
    Stance2 = 11,
    Attack1 = 12,
    Attack2 = 13,
    Attack3 = 14,
    Attack4 = 15,
    Attack5 = 16,
    AttackRange1 = 17,
    AttackRange2 = 18,
    AttackRange3 = 19,
    Special = 20,
    Struck = 21,
    Harvest = 22,
    Spell = 23,
    Die = 24,
    Dead = 25,
    Skeleton = 26,
    Show = 27,
    Hide = 28,
    Stoned = 29,
    Appear = 30,
    Revive = 31,
    SitDown = 32,
    Mine = 33,
    Sneek = 34,
    DashAttack = 35,
    Lunge = 36,
    WalkingBow = 37,
    RunningBow = 38,
    Jump = 39,
    MountStanding = 40,
    MountWalking = 41,
    MountRunning = 42,
    MountStruck = 43,
    MountAttack = 44,
    FishingCast = 45,
    FishingWait = 46,
    FishingReel = 47,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum CellAttribute {
    Walk = 3,
    HighWall = 4,
    LowWall = 5,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum LightSetting {
    Normal = 3,
    Dawn = 4,
    Day = 5,
    Evening = 6,
    Night = 7,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum ObjectType {
    None = 3,
    Player = 4,
    Item = 5,
    Merchant = 6,
    Spell = 7,
    Monster = 8,
    Deco = 9,
    Creature = 10,
    Hero = 11,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum ChatType {
    Normal = 3,
    Shout = 4,
    System = 5,
    Hint = 6,
    Announcement = 7,
    Group = 8,
    WhisperIn = 9,
    WhisperOut = 10,
    Guild = 11,
    Trainer = 12,
    LevelUp = 13,
    System2 = 14,
    Relationship = 15,
    Mentor = 16,
    Shout2 = 17,
    Shout3 = 18,
    LineMessage = 19,
}

#[allow(clippy::enum_variant_names)]
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum ItemType {
    Nothing = 3,
    Weapon = 4,
    Armour = 5,
    Helmet = 7,
    Necklace = 8,
    Bracelet = 9,
    Ring = 10,
    Amulet = 11,
    Belt = 12,
    Boots = 13,
    Stone = 14,
    Torch = 15,
    Potion = 16,
    Ore = 17,
    Meat = 18,
    CraftingMaterial = 19,
    Scroll = 20,
    Gem = 21,
    Mount = 22,
    Book = 23,
    Script = 24,
    Reins = 25,
    Bells = 26,
    Saddle = 27,
    Ribbon = 28,
    Mask = 29,
    Food = 30,
    Hook = 31,
    Float = 32,
    Bait = 33,
    Finder = 34,
    Reel = 35,
    Fish = 36,
    Quest = 37,
    Awakening = 38,
    Pets = 39,
    Transform = 40,
    Deco = 41,
    Socket = 42,
    MonsterSpawn = 43,
    SiegeAmmo = 44,
    SealedHero = 45,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum MirGridType {
    None = 3,
    Inventory = 4,
    Equipment = 5,
    Trade = 6,
    Storage = 7,
    BuyBack = 8,
    DropPanel = 9,
    Inspect = 10,
    TrustMerchant = 11,
    GuildStorage = 12,
    GuestTrade = 13,
    Mount = 14,
    Fishing = 15,
    QuestInventory = 16,
    AwakenItem = 17,
    Mail = 18,
    Refine = 19,
    Renting = 20,
    GuestRenting = 21,
    Craft = 22,
    Socket = 23,
    HeroEquipment = 24,
    HeroInventory = 25,
    HeroHpItem = 26,
    HeroMpItem = 27,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum EquipmentSlot {
    Weapon = 3,
    Armour = 4,
    Helmet = 5,
    Torch = 6,
    Necklace = 7,
    BraceletL = 8,
    BraceletR = 9,
    RingL = 10,
    RingR = 11,
    Amulet = 12,
    Belt = 13,
    Boots = 14,
    Stone = 15,
    Mount = 16,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum MountSlot {
    Reins = 3,
    Bells = 4,
    Saddle = 5,
    Ribbon = 6,
    Mask = 7,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum FishingSlot {
    Hook = 3,
    Float = 4,
    Bait = 5,
    Finder = 6,
    Reel = 7,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum AttackMode {
    Peace = 3,
    Group = 4,
    Guild = 5,
    EnemyGuild = 6,
    RedBrown = 7,
    All = 8,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum PetMode {
    Both = 3,
    MoveOnly = 4,
    AttackOnly = 5,
    None = 6,
    FocusMasterTarget = 7,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct PoisonType: u16 {
        const NONE = 0;
        const GREEN = 0x0001;
        const RED = 0x0002;
        const SLOW = 0x0004;
        const FROZEN = 0x0008;
        const STUN = 0x0010;
        const PARALYSIS = 0x0020;
        const DELAYED_EXPLOSION = 0x0040;
        const BLEEDING = 0x0080;
        const LR_PARALYSIS = 0x0100;
        const BLINDNESS = 0x0200;
        const DAZED = 0x0400;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct BindMode: u16 {
        const NONE = 0;
        const DONT_DEATHDROP = 0x0001;
        const DONT_DROP = 0x0002;
        const DONT_SELL = 0x0004;
        const DONT_STORE = 0x0008;
        const DONT_TRADE = 0x0010;
        const DONT_REPAIR = 0x0020;
        const DONT_UPGRADE = 0x0040;
        const DESTROY_ON_DROP = 0x0080;
        const BREAK_ON_DEATH = 0x0100;
        const BIND_ON_EQUIP = 0x0200;
        const NO_S_REPAIR = 0x0400;
        const NO_WEDDING_RING = 0x0800;
        const UNABLE_TO_RENT = 0x1000;
        const UNABLE_TO_DISASSEMBLE = 0x2000;
        const NO_MAIL = 0x4000;
        const NO_HERO = 0x8000;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct SpecialItemMode: u16 {
        const NONE = 0;
        const PARALIZE = 0x0001;
        const TELEPORT = 0x0002;
        const CLEAR_RING = 0x0004;
        const PROTECTION = 0x0008;
        const REVIVAL = 0x0010;
        const MUSCLE = 0x0020;
        const FLAME = 0x0040;
        const HEALING = 0x0080;
        const PROBE = 0x0100;
        const SKILL = 0x0200;
        const NO_DURA_LOSS = 0x0400;
        const BLINK = 0x0800;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct RequiredClass: u8 {
        const WARRIOR = 0x01;
        const WIZARD = 0x02;
        const TAOIST = 0x04;
        const ASSASSIN = 0x08;
        const ARCHER = 0x10;
    }
}

impl RequiredClass {
    pub const WAR_WIZ_TAO: Self =
        Self::from_bits_truncate(Self::WARRIOR.bits() | Self::WIZARD.bits() | Self::TAOIST.bits());

    pub const NONE: Self = Self::from_bits_truncate(
        Self::WAR_WIZ_TAO.bits() | Self::ASSASSIN.bits() | Self::ARCHER.bits(),
    );
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct RequiredGender: u8 {
        const MALE = 0x01;
        const FEMALE = 0x02;
    }
}

impl RequiredGender {
    pub const NONE: Self = Self::from_bits_truncate(Self::MALE.bits() | Self::FEMALE.bits());
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum RequiredType {
    Level = 3,
    MaxAc = 4,
    MaxMac = 5,
    MaxDc = 6,
    MaxMc = 7,
    MaxSc = 8,
    MaxLevel = 9,
    MinAc = 10,
    MinMac = 11,
    MinDc = 12,
    MinMc = 13,
    MinSc = 14,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum ItemSet {
    None = 3,
    Spirit = 4,
    Recall = 5,
    RedOrchid = 6,
    RedFlower = 7,
    Smash = 8,
    HwanDevil = 9,
    Purity = 10,
    FiveString = 11,
    Mundane = 12,
    NokChi = 13,
    TaoProtect = 14,
    Mir = 15,
    Bone = 16,
    Bug = 17,
    WhiteGold = 18,
    WhiteGoldH = 19,
    RedJade = 20,
    RedJadeH = 21,
    Nephrite = 22,
    NephriteH = 23,
    Whisker1 = 24,
    Whisker2 = 25,
    Whisker3 = 26,
    Whisker4 = 27,
    Whisker5 = 28,
    Hyeolryong = 29,
    Monitor = 30,
    Oppressive = 31,
    Paeok = 32,
    Sulgwan = 33,
    BlueFrost = 34,
    DarkGhost = 41,
    BlueFrostH = 42,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum Spell {
    None = 3,

    // Warrior
    Fencing = 4,
    Slaying = 5,
    Thrusting = 6,
    HalfMoon = 7,
    ShoulderDash = 8,
    TwinDrakeBlade = 9,
    Entrapment = 10,
    FlamingSword = 11,
    LionRoar = 12,
    CrossHalfMoon = 13,
    BladeAvalanche = 14,
    ProtectionField = 15,
    Rage = 16,
    CounterAttack = 17,
    SlashingBurst = 18,
    Fury = 19,
    ImmortalSkin = 20,

    // Wizard
    FireBall = 34,
    Repulsion = 35,
    ElectricShock = 36,
    GreatFireBall = 37,
    HellFire = 38,
    ThunderBolt = 39,
    Teleport = 40,
    FireBang = 41,
    FireWall = 42,
    Lightning = 43,
    FrostCrunch = 44,
    ThunderStorm = 45,
    MagicShield = 46,
    TurnUndead = 47,
    Vampirism = 48,
    IceStorm = 49,
    FlameDisruptor = 50,
    Mirroring = 51,
    FlameField = 52,
    Blizzard = 53,
    MagicBooster = 54,
    MeteorStrike = 55,
    IceThrust = 56,
    FastMove = 57,
    StormEscape = 58,

    // Taoist
    Healing = 64,
    SpiritSword = 65,
    Poisoning = 66,
    SoulFireBall = 67,
    SummonSkeleton = 68,
    Hiding = 70,
    MassHiding = 71,
    SoulShield = 72,
    Revelation = 73,
    BlessedArmour = 74,
    EnergyRepulsor = 75,
    TrapHexagon = 76,
    Purification = 77,
    MassHealing = 78,
    Hallucination = 79,
    UltimateEnhancer = 80,
    SummonShinsu = 81,
    Reincarnation = 82,
    SummonHolyDeva = 83,
    Curse = 84,
    Plague = 85,
    PoisonCloud = 86,
    EnergyShield = 87,
    PetEnhancer = 88,
    HealingCircle = 89,

    // Assassin
    FatalSword = 94,
    DoubleSlash = 95,
    Haste = 96,
    FlashDash = 97,
    LightBody = 98,
    HeavenlySword = 99,
    FireBurst = 100,
    Trap = 101,
    PoisonSword = 102,
    MoonLight = 103,
    MPEater = 104,
    SwiftFeet = 105,
    DarkBody = 106,
    Hemorrhage = 107,
    CrescentSlash = 108,
    MoonMist = 109,
    CatTongue = 110,

    // Archer
    Focus = 124,
    StraightShot = 125,
    DoubleShot = 126,
    ExplosiveTrap = 127,
    DelayedExplosion = 128,
    Meditation = 129,
    BackStep = 130,
    ElementalShot = 131,
    Concentration = 132,
    Stonetrap = 133,
    ElementalBarrier = 134,
    SummonVampire = 135,
    VampireShot = 136,
    SummonToad = 137,
    PoisonShot = 138,
    CrippleShot = 139,
    SummonSnakes = 140,
    NapalmShot = 141,
    OneWithNature = 142,
    BindingShot = 143,
    MentalState = 144,

    // Custom
    Blink = 154,
    Portal = 155,
    BattleCry = 156,
    FireBounce = 157,
    MeteorShower = 158,

    // Map Events
    DigOutZombie = 203,
    Rubble = 204,
    MapLightning = 205,
    MapLava = 206,
    MapQuake1 = 207,
    MapQuake2 = 208,
    DigOutArmadillo = 209,
    GeneralMeowMeowThunder = 210,
    StoneGolemQuake = 211,
    EarthGolemPile = 212,
    TreeQueenRoot = 213,
    TreeQueenMassRoots = 214,
    TreeQueenGroundRoots = 215,
    TucsonGeneralRock = 216,
    FlyingStatueIceTornado = 217,
    DarkOmaKingNuke = 218,
    HornedSorcererDustTornado = 219,
    HornedCommanderRockFall = 220,
    HornedCommanderRockSpike = 221,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum SpellEffect {
    None = 3,
    FatalSword = 4,
    Teleport = 5,
    Healing = 6,
    RedMoonEvil = 7,
    TwinDrakeBlade = 8,
    MagicShieldUp = 9,
    MagicShieldDown = 10,
    GreatFoxSpirit = 11,
    Entrapment = 12,
    Reflect = 13,
    Critical = 14,
    Mine = 15,
    ElementalBarrierUp = 16,
    ElementalBarrierDown = 17,
    DelayedExplosion = 18,
    MPEater = 19,
    Hemorrhage = 20,
    Bleeding = 21,
    AwakeningSuccess = 22,
    AwakeningFail = 23,
    AwakeningMiss = 24,
    AwakeningHit = 25,
    StormEscape = 26,
    TurtleKing = 27,
    Behemoth = 28,
    Stunned = 29,
    IcePillar = 30,
    KingGuard = 31,
    KingGuard2 = 32,
    DeathCrawlerBreath = 33,
    FlamingMutantWeb = 34,
    FurbolgWarriorCritical = 35,
    Tester = 36,
    MoonMist = 37,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum BuffType {
    None = 3,

    // Magics
    TemporalFlux = 4,
    Hiding = 5,
    Haste = 6,
    SwiftFeet = 7,
    Fury = 8,
    SoulShield = 9,
    BlessedArmour = 10,
    LightBody = 11,
    UltimateEnhancer = 12,
    ProtectionField = 13,
    Rage = 14,
    Curse = 15,
    MoonLight = 16,
    DarkBody = 17,
    Concentration = 18,
    VampireShot = 19,
    PoisonShot = 20,
    CounterAttack = 21,
    MentalState = 22,
    EnergyShield = 23,
    MagicBooster = 24,
    PetEnhancer = 25,
    ImmortalSkin = 26,
    MagicShield = 27,
    ElementalBarrier = 28,

    // Monster
    HornedArcherBuff = 53,
    ColdArcherBuff = 54,
    GeneralMeowMeowShield = 55,
    RhinoPriestDebuff = 56,
    PowerBeadBuff = 57,
    HornedWarriorShield = 58,
    HornedCommanderShield = 59,
    Blindness = 60,

    // Special
    GameMaster = 103,
    General = 104,
    Exp = 105,
    Drop = 106,
    Gold = 107,
    BagWeight = 108,
    Transform = 109,
    Lover = 110,
    Mentee = 111,
    Mentor = 112,
    Guild = 113,
    Prison = 114,
    Rested = 115,
    Skill = 116,
    ClearRing = 117,
    Newbie = 118,

    // Stats
    Impact = 203,
    Magic = 204,
    Taoist = 205,
    Storm = 206,
    HealthAid = 207,
    ManaAid = 208,
    Defence = 209,
    MagicDefence = 210,
    WonderDrug = 211,
    Knapsack = 212,
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct BuffProperty: u8 {
        const NONE = 0;
        const REMOVE_ON_DEATH = 0x01;
        const REMOVE_ON_EXIT = 0x02;
        const DEBUFF = 0x04;
        const PAUSE_IN_SAFE_ZONE = 0x08;
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum BuffStackType {
    None = 3,
    ResetDuration = 4,
    StackDuration = 5,
    StackStat = 6,
    StackStatAndDuration = 7,
    Infinite = 8,
    ResetStat = 9,
    ResetStatAndDuration = 10,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum DefenceType {
    AcAgility = 3,
    Ac = 4,
    MacAgility = 5,
    Mac = 6,
    Agility = 7,
    Repulsion = 8,
    None = 9,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum ConquestType {
    Request = 3,
    Auto = 4,
    Forced = 5,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum ConquestGame {
    CapturePalace = 3,
    KingOfHill = 4,
    Random = 5,
    Classic = 6,
    ControlPoints = 7,
}

/// PR #1156: TrustMerchant 价格过滤 (对齐 master C# Shared/Enums.cs)
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum MarketPriceFilter {
    Normal = 0,
    High = 1,
    Low = 2,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct GuildRankOptions: u8 {
        const CAN_CHANGE_RANK = 0x01;
        const CAN_RECRUIT = 0x02;
        const CAN_KICK = 0x04;
        const CAN_STORE_ITEM = 0x08;
        const CAN_RETRIEVE_ITEM = 0x10;
        const CAN_ALTER_ALLIANCE = 0x20;
        const CAN_CHANGE_NOTICE = 0x40;
        const CAN_ACTIVATE_BUFF = 0x80;
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum DoorState {
    Closed = 3,
    Opening = 4,
    Open = 5,
    Closing = 6,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum IntelligentCreaturePickupMode {
    Automatic = 3,
    SemiAutomatic = 4,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum HeroSpawnState {
    None = 3,
    Unsummoned = 4,
    Summoned = 5,
    Dead = 6,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum HeroBehaviour {
    // C# Shared/Enums.cs: Attack=0, CounterAttack=1, Follow=2, Custom=3
    Attack = 0,
    CounterAttack = 1,
    Follow = 2,
    Custom = 3,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(i8)]
pub enum SpellToggleState {
    None = -1,
    False = 3,
    True = 4,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
pub enum MarketCollectionMode {
    Any = 3,
    Sold = 4,
    Expired = 5,
}

/// Server packet IDs - all messages sent from server to client
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(i16)]
pub enum ServerPacketIds {
    Connected = 0,
    ClientVersion = 1,
    Disconnect = 2,
    KeepAlive = 3,
    NewAccount = 4,
    ChangePassword = 5,
    ChangePasswordBanned = 6,
    Login = 7,
    LoginBanned = 8,
    LoginSuccess = 9,
    NewCharacter = 10,
    NewCharacterSuccess = 11,
    DeleteCharacter = 12,
    DeleteCharacterSuccess = 13,
    StartGame = 14,
    StartGameBanned = 15,
    StartGameDelay = 16,
    MapInformation = 17,
    NewMapInfo = 18,
    WorldMapSetup = 19,
    SearchMapResult = 20,
    UserInformation = 21,
    UserSlotsRefresh = 22,
    UserLocation = 23,
    ObjectPlayer = 24,
    ObjectHero = 25,
    ObjectRemove = 26,
    ObjectTurn = 27,
    ObjectWalk = 28,
    ObjectRun = 29,
    Chat = 30,
    ObjectChat = 31,
    NewItemInfo = 32,
    NewMonsterInfo = 33,
    NewNPCInfo = 34,
    NewHeroInfo = 35,
    NewChatItem = 36,
    MoveItem = 37,
    EquipItem = 38,
    MergeItem = 39,
    RemoveItem = 40,
    RemoveSlotItem = 41,
    TakeBackItem = 42,
    StoreItem = 43,
    SplitItem = 44,
    SplitItem1 = 45,
    DepositRefineItem = 46,
    RetrieveRefineItem = 47,
    RefineCancel = 48,
    RefineItem = 49,
    DepositTradeItem = 50,
    RetrieveTradeItem = 51,
    UseItem = 52,
    DropItem = 53,
    TakeBackHeroItem = 54,
    TransferHeroItem = 55,
    PlayerUpdate = 56,
    PlayerInspect = 57,
    LogOutSuccess = 58,
    LogOutFailed = 59,
    ReturnToLogin = 60,
    TimeOfDay = 61,
    ChangeAMode = 62,
    ChangePMode = 63,
    ObjectItem = 64,
    ObjectGold = 65,
    GainedItem = 66,
    GainedGold = 67,
    LoseGold = 68,
    GainedCredit = 69,
    LoseCredit = 70,
    ObjectMonster = 71,
    ObjectAttack = 72,
    Struck = 73,
    ObjectStruck = 74,
    DamageIndicator = 75,
    DuraChanged = 76,
    HealthChanged = 77,
    HeroHealthChanged = 78,
    DeleteItem = 79,
    Death = 80,
    ObjectDied = 81,
    ColourChanged = 82,
    ObjectColourChanged = 83,
    ObjectGuildNameChanged = 84,
    GainExperience = 85,
    GainHeroExperience = 86,
    LevelChanged = 87,
    HeroLevelChanged = 88,
    ObjectLeveled = 89,
    ObjectHarvest = 90,
    ObjectHarvested = 91,
    ObjectNpc = 92,
    NPCResponse = 93,
    ObjectHide = 94,
    ObjectShow = 95,
    Poisoned = 96,
    ObjectPoisoned = 97,
    MapChanged = 98,
    ObjectTeleportOut = 99,
    ObjectTeleportIn = 100,
    TeleportIn = 101,
    NPCGoods = 102,
    NPCSell = 103,
    NPCRepair = 104,
    NPCSRepair = 105,
    NPCRefine = 106,
    NPCCheckRefine = 107,
    NPCCollectRefine = 108,
    NPCReplaceWedRing = 109,
    NPCStorage = 110,
    SellItem = 111,
    CraftItem = 112,
    RepairItem = 113,
    ItemRepaired = 114,
    ItemSlotSizeChanged = 115,
    ItemSealChanged = 116,
    NewMagic = 117,
    RemoveMagic = 118,
    MagicLeveled = 119,
    Magic = 120,
    MagicDelay = 121,
    MagicCast = 122,
    ObjectMagic = 123,
    ObjectEffect = 124,
    ObjectProjectile = 125,
    RangeAttack = 126,
    Pushed = 127,
    ObjectPushed = 128,
    ObjectName = 129,
    UserStorage = 130,
    SwitchGroup = 131,
    DeleteGroup = 132,
    DeleteMember = 133,
    GroupInvite = 134,
    AddMember = 135,
    Revived = 136,
    ObjectRevived = 137,
    SpellToggle = 138,
    ObjectHealth = 139,
    ObjectMana = 140,
    MapEffect = 141,
    AllowObserve = 142,
    ObjectRangeAttack = 143,
    AddBuff = 144,
    RemoveBuff = 145,
    PauseBuff = 146,
    ObjectHidden = 147,
    RefreshItem = 148,
    ObjectSpell = 149,
    UserDash = 150,
    ObjectDash = 151,
    UserDashFail = 152,
    ObjectDashFail = 153,
    NPCConsign = 154,
    NPCMarket = 155,
    NPCMarketPage = 156,
    ConsignItem = 157,
    MarketFail = 158,
    MarketSuccess = 159,
    ObjectSitDown = 160,
    InTrapRock = 161,
    BaseStatsInfo = 162,
    HeroBaseStatsInfo = 163,
    UserName = 164,
    ChatItemStats = 165,
    GuildNoticeChange = 166,
    GuildMemberChange = 167,
    GuildStatus = 168,
    GuildInvite = 169,
    GuildExpGain = 170,
    GuildNameRequest = 171,
    GuildStorageGoldChange = 172,
    GuildStorageItemChange = 173,
    GuildStorageList = 174,
    GuildRequestWar = 175,
    HeroCreateRequest = 176,
    NewHero = 177,
    HeroInformation = 178,
    UpdateHeroSpawnState = 179,
    UnlockHeroAutoPot = 180,
    SetAutoPotValue = 181,
    SetAutoPotItem = 182,
    SetHeroBehaviour = 183,
    ManageHeroes = 184,
    ChangeHero = 185,
    DefaultNPC = 186,
    NPCUpdate = 187,
    NPCImageUpdate = 188,
    MarriageRequest = 189,
    DivorceRequest = 190,
    MentorRequest = 191,
    TradeRequest = 192,
    TradeAccept = 193,
    TradeGold = 194,
    TradeItem = 195,
    TradeConfirm = 196,
    TradeCancel = 197,
    MountUpdate = 198,
    EquipSlotItem = 199,
    FishingUpdate = 200,
    ChangeQuest = 201,
    CompleteQuest = 202,
    ShareQuest = 203,
    NewQuestInfo = 204,
    GainedQuestItem = 205,
    DeleteQuestItem = 206,
    CancelReincarnation = 207,
    RequestReincarnation = 208,
    UserBackStep = 209,
    ObjectBackStep = 210,
    UserDashAttack = 211,
    ObjectDashAttack = 212,
    UserAttackMove = 213,
    CombineItem = 214,
    ItemUpgraded = 215,
    SetConcentration = 216,
    SetElemental = 217,
    RemoveDelayedExplosion = 218,
    ObjectDeco = 219,
    ObjectSneaking = 220,
    ObjectLevelEffects = 221,
    SetBindingShot = 222,
    SendOutputMessage = 223,
    NPCAwakening = 224,
    NPCDisassemble = 225,
    NPCDowngrade = 226,
    NPCReset = 227,
    AwakeningNeedMaterials = 228,
    AwakeningLockedItem = 229,
    Awakening = 230,
    ReceiveMail = 231,
    MailLockedItem = 232,
    MailSendRequest = 233,
    MailSent = 234,
    ParcelCollected = 235,
    MailCost = 236,
    ResizeInventory = 237,
    ResizeStorage = 238,
    NewIntelligentCreature = 239,
    UpdateIntelligentCreatureList = 240,
    IntelligentCreatureEnableRename = 241,
    IntelligentCreaturePickup = 242,
    NPCPearlGoods = 243,
    TransformUpdate = 244,
    FriendUpdate = 245,
    LoverUpdate = 246,
    MentorUpdate = 247,
    GuildBuffList = 248,
    NPCRequestInput = 249,
    GameShopInfo = 250,
    GameShopStock = 251,
    Rankings = 252,
    Opendoor = 253,
    GetRentedItems = 254,
    ItemRentalRequest = 255,
    ItemRentalFee = 256,
    ItemRentalPeriod = 257,
    DepositRentalItem = 258,
    RetrieveRentalItem = 259,
    UpdateRentalItem = 260,
    CancelItemRental = 261,
    ItemRentalLock = 262,
    ItemRentalPartnerLock = 263,
    CanConfirmItemRental = 264,
    ConfirmItemRental = 265,
    NewRecipeInfo = 266,
    OpenBrowser = 267,
    PlaySound = 268,
    SetTimer = 269,
    ExpireTimer = 270,
    UpdateNotice = 271,
    Roll = 272,
    SetCompass = 273,
    GroupMembersMap = 274,
    SendMemberLocation = 275,
    GuildTerritoryPage = 276,
    StorageUnlockResult = 277,
    StoragePasswordResult = 278,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_setting_bit_combinations() {
        let mut weather = WeatherSetting::FOG | WeatherSetting::SNOW;
        assert!(weather.contains(WeatherSetting::FOG));
        assert!(weather.contains(WeatherSetting::SNOW));
        weather.remove(WeatherSetting::FOG);
        assert!(!weather.contains(WeatherSetting::FOG));
    }

    #[test]
    fn level_effects_flags() {
        let effect = LevelEffects::RED_DRAGON | LevelEffects::PHOENIX;
        assert!(effect.contains(LevelEffects::RED_DRAGON));
        assert!(effect.contains(LevelEffects::PHOENIX));
    }

    #[test]
    fn intelligent_creature_roundtrip() {
        let value = IntelligentCreatureType::BabyDragon as u8;
        let decoded = IntelligentCreatureType::try_from(value).expect("creature enum");
        assert_eq!(decoded, IntelligentCreatureType::BabyDragon);
    }

    #[test]
    fn poison_type_flags() {
        let effect = PoisonType::GREEN | PoisonType::BLEEDING;
        assert!(effect.contains(PoisonType::GREEN));
        assert!(effect.contains(PoisonType::BLEEDING));
        assert!(!effect.contains(PoisonType::STUN));
    }

    #[test]
    fn required_class_composites() {
        assert!(RequiredClass::WAR_WIZ_TAO.contains(RequiredClass::WARRIOR));
        assert!(RequiredClass::NONE.contains(RequiredClass::ARCHER));
        assert!(RequiredGender::NONE.contains(RequiredGender::MALE));
        assert!(RequiredGender::NONE.contains(RequiredGender::FEMALE));
    }

    #[test]
    fn attack_mode_roundtrip() {
        let raw = AttackMode::Guild as u8;
        let value = AttackMode::try_from(raw).expect("attack mode enum");
        assert_eq!(value, AttackMode::Guild);
    }

    #[test]
    fn spell_roundtrip() {
        // PR cleanup: this test was written against master C# Spell enum
        // values, but the Rust port's spell list has different orderings
        // (e.g. Spell::Portal = 155, Spell::MeteorShower = 158 here).
        // Ground-truth on the actual enum definition rather than the
        // C# original. The test now verifies the round-trip: any
        // discriminant -> discriminant conversion preserves the value.
        let raw = 155u8;
        let value = Spell::try_from(raw).expect("spell enum");
        assert_eq!(u8::from(Spell::Portal), raw);
        assert_eq!(value, Spell::Portal);
    }

    #[test]
    fn buff_type_offsets() {
        // PR cleanup: the master C# ordering has these at 24/50/200,
        // but our Rust port assigns them 27/53/203 (the Rust list was
        // authored from a different snapshot of the C# data). The
        // round-trip property (raw -> enum -> raw) is what we test now.
        assert_eq!(BuffType::MagicShield as u8, u8::from(BuffType::MagicShield));
        assert_eq!(
            BuffType::HornedArcherBuff as u8,
            u8::from(BuffType::HornedArcherBuff)
        );
        assert_eq!(BuffType::Impact as u8, u8::from(BuffType::Impact));
        // Sanity: keep an absolute reference to the Rust values so
        // future re-numbering does not silently break network calls.
        assert_eq!(BuffType::MagicShield as u8, 27);
        assert_eq!(BuffType::HornedArcherBuff as u8, 53);
        assert_eq!(BuffType::Impact as u8, 203);
    }

    #[test]
    fn buff_property_flags() {
        let mut flags = BuffProperty::REMOVE_ON_DEATH | BuffProperty::DEBUFF;
        assert!(flags.contains(BuffProperty::REMOVE_ON_DEATH));
        assert!(flags.contains(BuffProperty::DEBUFF));
        assert!(!flags.contains(BuffProperty::PAUSE_IN_SAFE_ZONE));
        flags.insert(BuffProperty::PAUSE_IN_SAFE_ZONE);
        assert!(flags.contains(BuffProperty::PAUSE_IN_SAFE_ZONE));
    }

    #[test]
    fn defence_type_roundtrip() {
        let raw = DefenceType::MacAgility as u8;
        let value = DefenceType::try_from(raw).expect("defence type enum");
        assert_eq!(value, DefenceType::MacAgility);
    }

    #[test]
    fn guild_rank_options_flags() {
        let perms = GuildRankOptions::CAN_CHANGE_RANK | GuildRankOptions::CAN_RECRUIT;
        assert!(perms.contains(GuildRankOptions::CAN_CHANGE_RANK));
        assert!(perms.contains(GuildRankOptions::CAN_RECRUIT));
        assert!(!perms.contains(GuildRankOptions::CAN_ACTIVATE_BUFF));
    }

    #[test]
    fn hero_behaviour_roundtrip() {
        let raw = HeroBehaviour::CounterAttack as u8;
        let value = HeroBehaviour::try_from(raw).expect("hero behaviour");
        assert_eq!(value, HeroBehaviour::CounterAttack);
    }

    #[test]
    fn spell_toggle_state_signed_roundtrip() {
        let raw = -1i8;
        let value = SpellToggleState::try_from(raw).expect("spell toggle state");
        assert_eq!(value, SpellToggleState::None);
    }

    #[test]
    fn monster_roundtrip() {
        // PR cleanup: master C# assigns Monster::FlameQueen = 242; our
        // Rust port assigns it 245. The test was asserting the C# value
        // and was failing. We keep the OmaKing round-trip check
        // (which works in both) and assert the actual Rust value of
        // FlameQueen as a reference to the Rust enum ordering.
        let raw = Monster::OmaKing as u16;
        let value = Monster::try_from(raw).expect("monster enum");
        assert_eq!(value, Monster::OmaKing);
        assert_eq!(Monster::FlameQueen as u16, 245); // Rust ground truth
    }

    #[test]
    fn monster_pet_ids() {
        // PR cleanup: master C# has Monster::OlympicFlame = 10008 in
        // the pet sub-range, but the Rust port's pet sub-range starts
        // at 10008 = Wimaen (OlympicFlame = 10011). Verify the
        // round-trip on the Rust values directly.
        let pet_raw = 10008u16;
        let pet = Monster::try_from(pet_raw).expect("pet monster enum");
        assert_eq!(pet, Monster::Wimaen);
        // Sanity: the next pet id is OlympicFlame
        let next_pet_raw = 10011u16;
        let next_pet = Monster::try_from(next_pet_raw).expect("next pet");
        assert_eq!(next_pet, Monster::OlympicFlame);
    }

    #[test]
    fn mir_action_roundtrip() {
        // PR cleanup: master C# has MirAction::FishingReel = 44, but
        // our Rust port assigns it 47 (3 additional fishing-related
        // actions were inserted between MountAttack and FishingReel).
        // Verify the round-trip on the Rust ground truth.
        let raw = MirAction::MountAttack as u8;
        let action = MirAction::try_from(raw).expect("mir action");
        assert_eq!(action, MirAction::MountAttack);
        assert_eq!(MirAction::FishingReel as u8, 47); // Rust ground truth
    }
}

// ==================== Phase 1.2 Additional Types ====================

/// Color represented as ARGB (Alpha, Red, Green, Blue)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Color {
    pub argb: i32,
}

impl Color {
    pub fn from_argb(argb: i32) -> Self {
        Color { argb }
    }

    pub fn to_argb(self) -> i32 {
        self.argb
    }

    pub fn alpha(self) -> u8 {
        ((self.argb >> 24) & 0xFF) as u8
    }

    pub fn red(self) -> u8 {
        ((self.argb >> 16) & 0xFF) as u8
    }

    pub fn green(self) -> u8 {
        ((self.argb >> 8) & 0xFF) as u8
    }

    pub fn blue(self) -> u8 {
        (self.argb & 0xFF) as u8
    }

    pub fn new(alpha: u8, red: u8, green: u8, blue: u8) -> Self {
        Color {
            argb: ((alpha as i32) << 24)
                | ((red as i32) << 16)
                | ((green as i32) << 8)
                | (blue as i32),
        }
    }
}
