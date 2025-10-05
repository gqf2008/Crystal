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
    TeleportToNPC = 37,
    SearchMap = 38,
    Inspect = 39,
    Observe = 40,
    ChangeAMode = 41,
    ChangePMode = 42,
    ChangeTrade = 43,
    Attack = 44,
    RangeAttack = 45,
    Harvest = 46,
    CallNPC = 47,
    BuyItem = 48,
    SellItem = 49,
    CraftItem = 50,
    RepairItem = 51,
    BuyItemBack = 52,
    SRepairItem = 53,
    MagicKey = 54,
    Magic = 55,
    SwitchGroup = 56,
    AddMember = 57,
    DellMember = 58,
    GroupInvite = 59,
    NewHero = 60,
    SetAutoPotValue = 61,
    SetAutoPotItem = 62,
    SetHeroBehaviour = 63,
    ChangeHero = 64,
    TownRevive = 65,
    SpellToggle = 66,
    ConsignItem = 67,
    MarketSearch = 68,
    MarketRefresh = 69,
    MarketPage = 70,
    MarketBuy = 71,
    MarketGetBack = 72,
    MarketSellNow = 73,
    RequestUserName = 74,
    RequestChatItem = 75,
    EditGuildMember = 76,
    EditGuildNotice = 77,
    GuildInvite = 78,
    GuildNameReturn = 79,
    RequestGuildInfo = 80,
    GuildStorageGoldChange = 81,
    GuildStorageItemChange = 82,
    GuildWarReturn = 83,
    MarriageRequest = 84,
    MarriageReply = 85,
    ChangeMarriage = 86,
    DivorceRequest = 87,
    DivorceReply = 88,
    AddMentor = 89,
    MentorReply = 90,
    AllowMentor = 91,
    CancelMentor = 92,
    TradeRequest = 93,
    TradeReply = 94,
    TradeGold = 95,
    TradeConfirm = 96,
    TradeCancel = 97,
    EquipSlotItem = 98,
    FishingCast = 99,
    FishingChangeAutocast = 100,
    AcceptQuest = 101,
    FinishQuest = 102,
    AbandonQuest = 103,
    ShareQuest = 104,
    AcceptReincarnation = 105,
    CancelReincarnation = 106,
    CombineItem = 107,
    AwakeningNeedMaterials = 108,
    AwakeningLockedItem = 109,
    Awakening = 110,
    DisassembleItem = 111,
    DowngradeAwakening = 112,
    ResetAddedItem = 113,
    SendMail = 114,
    ReadMail = 115,
    CollectParcel = 116,
    DeleteMail = 117,
    LockMail = 118,
    MailLockedItem = 119,
    MailCost = 120,
    UpdateIntelligentCreature = 121,
    IntelligentCreaturePickup = 122,
    RequestIntelligentCreatureUpdates = 123,
    AddFriend = 124,
    RemoveFriend = 125,
    RefreshFriends = 126,
    AddMemo = 127,
    GuildBuffUpdate = 128,
    NPCConfirmInput = 129,
    GameshopBuy = 130,
    ReportIssue = 131,
    GetRanking = 132,
    Opendoor = 133,
    GetRentedItems = 134,
    ItemRentalRequest = 135,
    ItemRentalFee = 136,
    ItemRentalPeriod = 137,
    DepositRentalItem = 138,
    RetrieveRentalItem = 139,
    CancelItemRental = 140,
    ItemRentalLockFee = 141,
    ItemRentalLockItem = 142,
    ConfirmItemRental = 143,
    GuildTerritoryPage,
    PurchaseGuildTerritory,
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
    Health = 0,
    Mana = 1,
    Weight = 2,
    Stat = 3,
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
    MinAC = 0,
    MaxAC = 1,
    MinMAC = 2,
    MaxMAC = 3,
    MinDC = 4,
    MaxDC = 5,
    MinMC = 6,
    MaxMC = 7,
    MinSC = 8,
    MaxSC = 9,
    Accuracy = 10,
    Agility = 11,
    HP = 12,
    MP = 13,
    AttackSpeed = 14,
    Luck = 15,
    BagWeight = 16,
    HandWeight = 17,
    WearWeight = 18,
    Reflect = 19,
    Strong = 20,
    Holy = 21,
    Freezing = 22,
    PoisonAttack = 23,
    MagicResist = 30,
    PoisonResist = 31,
    HealthRecovery = 32,
    SpellRecovery = 33,
    PoisonRecovery = 34,
    CriticalRate = 35,
    CriticalDamage = 36,
    MaxACRatePercent = 40,
    MaxMACRatePercent = 41,
    MaxDCRatePercent = 42,
    MaxMCRatePercent = 43,
    MaxSCRatePercent = 44,
    AttackSpeedRatePercent = 45,
    HPRatePercent = 46,
    MPRatePercent = 47,
    HPDrainRatePercent = 48,
    ExpRatePercent = 100,
    ItemDropRatePercent = 101,
    GoldDropRatePercent = 102,
    MineRatePercent = 103,
    GemRatePercent = 104,
    FishRatePercent = 105,
    CraftRatePercent = 106,
    SkillGainMultiplier = 107,
    AttackBonus = 108,
    LoverExpRatePercent = 120,
    MentorDamageRatePercent = 121,
    MentorExpRatePercent = 123,
    DamageReductionPercent = 124,
    EnergyShieldPercent = 125,
    EnergyShieldHPGain = 126,
    ManaPenaltyPercent = 127,
    TeleportManaPenaltyPercent = 128,
    Hero = 129,
    Unknown = 255,
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
    None = 0,
    Default = 1,
    Attack = 2,
    AttackRed = 3,
    NPCTalk = 4,
    TextPrompt = 5,
    Trash = 6,
    Upgrade = 7,
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
    Buy = 0,
    BuySub = 1,
    Craft = 2,
    Sell = 3,
    Repair = 4,
    SpecialRepair = 5,
    Consign = 6,
    Refine = 7,
    CheckRefine = 8,
    Disassemble = 9,
    Downgrade = 10,
    Reset = 11,
    CollectRefine = 12,
    ReplaceWedRing = 13,
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
    Consign = 0,
    Auction = 1,
    GameShop = 2,
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
    Market = 0,
    Consign = 1,
    Auction = 2,
    GameShop = 3,
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
    Normal = 0,
    Light = 1,
    LightInv = 2,
    InvNormal = 3,
    InvLight = 4,
    InvLightInv = 5,
    InvColor = 6,
    InvBackground = 7,
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
    Hit = 0,
    Miss = 1,
    Critical = 2,
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
    None = 0,
    Dc = 1,
    Mc = 2,
    Sc = 3,
    Ac = 4,
    Mac = 5,
    HpMp = 6,
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
    Normal = 0,
    Quest = 1,
    Guild = 2,
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
    None = 0,
    Common = 1,
    Rare = 2,
    Legendary = 3,
    Mythical = 4,
    Heroic = 5,
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
    None = 0,
    Dc = 1,
    Mc = 2,
    Sc = 3,
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
    General = 0,
    Daily = 1,
    Repeatable = 2,
    Story = 3,
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
    None = 0,
    QuestionWhite = 1,
    ExclamationYellow = 2,
    QuestionYellow = 3,
    ExclamationBlue = 5,
    QuestionBlue = 6,
    ExclamationGreen = 52,
    QuestionGreen = 53,
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
    Add = 0,
    Update = 1,
    Remove = 2,
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
    TimeExpired = 0,
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
    Login = 0,
    LevelUp = 1,
    UseItem = 2,
    MapCoord = 3,
    MapEnter = 4,
    Die = 5,
    Trigger = 6,
    CustomCommand = 7,
    OnAcceptQuest = 8,
    OnFinishQuest = 9,
    Daily = 10,
    Client = 11,
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
    None = 99,
    BabyPig = 0,
    Chick = 1,
    Kitten = 2,
    BabySkeleton = 3,
    Baekdon = 4,
    Wimaen = 5,
    BlackKitten = 6,
    BabyDragon = 7,
    OlympicFlame = 8,
    BabySnowMan = 9,
    Frog = 10,
    BabyMonkey = 11,
    AngryBird = 12,
    Foxey = 13,
    MedicalRat = 14,
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
    Guard = 0,
    TaoistGuard = 1,
    Guard2 = 2,
    Hen = 3,
    Deer = 4,
    Scarecrow = 5,
    HookingCat = 6,
    RakingCat = 7,
    Yob = 8,
    Oma = 9,
    CannibalPlant = 10,
    ForestYeti = 11,
    SpittingSpider = 12,
    ChestnutTree = 13,
    EbonyTree = 14,
    LargeMushroom = 15,
    CherryTree = 16,
    OmaFighter = 17,
    OmaWarrior = 18,
    CaveBat = 19,
    CaveMaggot = 20,
    Scorpion = 21,
    Skeleton = 22,
    BoneFighter = 23,
    AxeSkeleton = 24,
    BoneWarrior = 25,
    BoneElite = 26,
    Dung = 27,
    Dark = 28,
    WoomaSoldier = 29,
    WoomaFighter = 30,
    WoomaWarrior = 31,
    FlamingWooma = 32,
    WoomaGuardian = 33,
    WoomaTaurus = 34, // BOSS
    WhimperingBee = 35,
    GiantWorm = 36,
    Centipede = 37,
    BlackMaggot = 38,
    Tongs = 39,
    EvilTongs = 40,
    EvilCentipede = 41,
    BugBat = 42,
    BugBatMaggot = 43,
    WedgeMoth = 44,
    RedBoar = 45,
    BlackBoar = 46,
    SnakeScorpion = 47,
    WhiteBoar = 48,
    EvilSnake = 49,
    BombSpider = 50,
    RootSpider = 51,
    SpiderBat = 52,
    VenomSpider = 53,
    GangSpider = 54,
    GreatSpider = 55,
    LureSpider = 56,
    BigApe = 57,
    EvilApe = 58,
    GrayEvilApe = 59,
    RedEvilApe = 60,
    CrystalSpider = 61,
    RedMoonEvil = 62,
    BigRat = 63,
    ZumaArcher = 64,
    ZumaStatue = 65,
    ZumaGuardian = 66,
    RedThunderZuma = 67,
    ZumaTaurus = 68, // BOSS
    DigOutZombie = 69,
    ClZombie = 70,
    NdZombie = 71,
    CrawlerZombie = 72,
    ShamanZombie = 73,
    Ghoul = 74,
    KingScorpion = 75,
    KingHog = 76,
    DarkDevil = 77,
    BoneFamiliar = 78,
    Shinsu = 79,
    Shinsu1 = 80,
    SpiderFrog = 81,
    HoroBlaster = 82,
    BlueHoroBlaster = 83,
    KekTal = 84,
    VioletKekTal = 85,
    Khazard = 86,
    RoninGhoul = 87,
    ToxicGhoul = 88,
    BoneCaptain = 89,
    BoneSpearman = 90,
    BoneBlademan = 91,
    BoneArcher = 92,
    BoneLord = 93, // BOSS
    Minotaur = 94,
    IceMinotaur = 95,
    ElectricMinotaur = 96,
    WindMinotaur = 97,
    FireMinotaur = 98,
    RightGuard = 99,
    LeftGuard = 100,
    MinotaurKing = 101, // BOSS
    FrostTiger = 102,
    Sheep = 103,
    Wolf = 104,
    ShellNipper = 105,
    Keratoid = 106,
    GiantKeratoid = 107,
    SkyStinger = 108,
    SandWorm = 109,
    VisceralWorm = 110,
    RedSnake = 111,
    TigerSnake = 112,
    Yimoogi = 113,
    GiantWhiteSnake = 114,
    BlueSnake = 115,
    YellowSnake = 116,
    HolyDeva = 117,
    AxeOma = 118,
    SwordOma = 119,
    CrossbowOma = 120,
    WingedOma = 121,
    FlailOma = 122,
    OmaGuard = 123,
    YinDevilNode = 124,
    YangDevilNode = 125,
    OmaKing = 126, // BOSS
    BlackFoxman = 127,
    RedFoxman = 128,
    WhiteFoxman = 129,
    TrapRock = 130,
    GuardianRock = 131,
    ThunderElement = 132,
    CloudElement = 133,
    GreatFoxSpirit = 134, // BOSS
    HedgeKekTal = 135,
    BigHedgeKekTal = 136,
    RedFrogSpider = 137,
    BrownFrogSpider = 138,
    ArcherGuard = 139,
    KatanaGuard = 140,
    ArcherGuard2 = 141,
    Pig = 142,
    Bull = 143,
    Bush = 144,
    ChristmasTree = 145,
    HighAssassin = 146,
    DarkDustPile = 147,
    DarkBrownWolf = 148,
    Football = 149,
    GingerBreadman = 150,
    HalloweenScythe = 151,
    GhastlyLeecher = 152,
    CyanoGhast = 153,
    MutatedManworm = 154,
    CrazyManworm = 155,
    MudPile = 156,
    TailedLion = 157,
    Behemoth = 158, // BOSS
    DarkDevourer = 159,
    PoisonHugger = 160,
    Hugger = 161,
    MutatedHugger = 162,
    DreamDevourer = 163,
    Treasurebox = 164,
    SnowPile = 165,
    Snowman = 166,
    SnowTree = 167,
    GiantEgg = 168,
    RedTurtle = 169,
    GreenTurtle = 170,
    BlueTurtle = 171,
    Catapult1 = 172, // SPECIAL TODO
    Catapult2 = 173, // SPECIAL TODO
    OldSpittingSpider = 174,
    SiegeRepairman = 175, // SPECIAL TODO
    BlueSanta = 176,
    BattleStandard = 177,
    Blank1 = 178,
    RedYimoogi = 179,
    LionRiderMale = 180,   // Not Monster - Skin / Transform
    LionRiderFemale = 181, // Not Monster - Skin / Transform
    Tornado = 182,
    FlameTiger = 183,
    WingedTigerLord = 184, // BOSS
    TowerTurtle = 185,
    FinialTurtle = 186,
    TurtleKing = 187, // BOSS
    DarkTurtle = 188,
    LightTurtle = 189,
    DarkSwordOma = 190,
    DarkAxeOma = 191,
    DarkCrossbowOma = 192,
    DarkWingedOma = 193,
    BoneWhoo = 194,
    DarkSpider = 195, // AI 8
    ViscusWorm = 196,
    ViscusCrawler = 197,
    CrawlerLave = 198,
    DarkYob = 199,
    FlamingMutant = 200,
    StoningStatue = 201, // BOSS
    FlyingStatue = 202,
    ValeBat = 203,
    Weaver = 204,
    VenomWeaver = 205,
    CrackingWeaver = 206,
    ArmingWeaver = 207,
    CrystalWeaver = 208,
    FrozenZumaStatue = 209,
    FrozenZumaGuardian = 210,
    FrozenRedZuma = 211,
    GreaterWeaver = 212,
    SpiderWarrior = 213,
    SpiderBarbarian = 214,
    HellSlasher = 215,
    HellPirate = 216,
    HellCannibal = 217,
    HellKeeper = 218, // BOSS
    HellBolt = 219,
    WitchDoctor = 220,
    ManectricHammer = 221,
    ManectricClub = 222,
    ManectricClaw = 223,
    ManectricStaff = 224,
    NamelessGhost = 225,
    DarkGhost = 226,
    ChaosGhost = 227,
    ManectricBlest = 228,
    ManectricKing = 229,
    Blank2 = 230,
    IcePillar = 231,
    FrostYeti = 232,
    ManectricSlave = 233,
    TrollHammer = 234,
    TrollBomber = 235,
    TrollStoner = 236,
    TrollKing = 237, // BOSS
    FlameSpear = 238,
    FlameMage = 239,
    FlameScythe = 240,
    FlameAssassin = 241,
    FlameQueen = 242, // BOSS
    HellKnight1 = 243,
    HellKnight2 = 244,
    HellKnight3 = 245,
    HellKnight4 = 246,
    HellLord = 247, // BOSS
    WaterGuard = 248,
    IceGuard = 249,
    ElementGuard = 250,
    DemonGuard = 251,
    KingGuard = 252,
    Snake10 = 253,
    Snake11 = 254,
    Snake12 = 255,
    Snake13 = 256,
    Snake14 = 257,
    Snake15 = 258,
    Snake16 = 259,
    Snake17 = 260,
    DeathCrawler = 261,
    BurningZombie = 262,
    MudZombie = 263,
    FrozenZombie = 264,
    UndeadWolf = 265,
    DemonWolf = 266,
    WhiteMammoth = 267,
    DarkBeast = 268,
    LightBeast = 269,  // AI 112
    BloodBaboon = 270, // AI 112
    HardenRhino = 271,
    AncientBringer = 272,
    FightingCat = 273,
    FireCat = 274,  // AI 44
    CatWidow = 275, // AI 112
    StainHammerCat = 276,
    BlackHammerCat = 277,
    StrayCat = 278,
    CatShaman = 279,
    Jar1 = 280,
    Jar2 = 281,
    SeedingsGeneral = 282,
    RestlessJar = 283,
    GeneralMeowMeow = 284, // BOSS
    Bunny = 285,
    Tucson = 286,
    TucsonFighter = 287, // AI 44
    TucsonMage = 288,
    TucsonWarrior = 289,
    Armadillo = 290,
    ArmadilloElder = 291,
    TucsonEgg = 292, // EFFECT 0/1
    PlaguedTucson = 293,
    SandSnail = 294,
    CannibalTentacles = 295,
    TucsonGeneral = 296, // BOSS
    GasToad = 297,
    Mantis = 298,
    SwampWarrior = 299,
    AssassinBird = 300,
    RhinoWarrior = 301,
    RhinoPriest = 302,
    ElephantMan = 303,
    StoneGolem = 304,
    EarthGolem = 305,
    TreeGuardian = 306,
    TreeQueen = 307,
    PeacockSpider = 308,
    DarkBaboon = 309,    // AI 112
    TwinHeadBeast = 310, // AI 112
    OmaCannibal = 311,
    OmaBlest = 312,
    OmaSlasher = 313,
    OmaAssassin = 314,
    OmaMage = 315,
    OmaWitchDoctor = 316,
    LightningBead = 317, // Effect 0, AI 149
    HealingBead = 318,   // Effect 1, AI 149
    PowerUpBead = 319,   // Effect 2, AI 14
    DarkOmaKing = 320,   // BOSS
    CaveStatue = 321,
    Mandrill = 322,
    PlagueCrab = 323,
    CreeperPlant = 324,
    FloatingWraith = 325, // AI 8
    ArmedPlant = 326,
    AvengerPlant = 327,
    Nadz = 328,
    AvengingSpirit = 329,
    AvengingWarrior = 330,
    AxePlant = 331,
    WoodBox = 332,
    ClawBeast = 333,   // AI 8
    DarkCaptain = 334, // BOSS
    SackWarrior = 335,
    WereTiger = 336, // AI 112
    KingHydrax = 337,
    Hydrax = 338,
    HornedMage = 339,
    BlueSoul = 340,
    HornedArcher = 341,
    ColdArcher = 342,
    HornedWarrior = 343,
    FloatingRock = 344,
    ScalyBeast = 345,
    HornedSorceror = 346,
    BoulderSpirit = 347,
    HornedCommander = 348, // BOSS
    MoonStone = 349,
    SunStone = 350,
    LightningStone = 351,
    Turtlegrass = 352,
    ManTree = 353,
    Bear = 354, // Effect 1, AI 112
    Leopard = 355,
    ChieftainArcher = 356,
    ChieftainSword = 357, // BOSS TODO
    StoningSpider = 358,  // Archer Spell mob
    VampireSpider = 359,  // Archer Spell mob
    SpittingToad = 360,   // Archer Spell mob
    SnakeTotem = 361,     // Archer Spell mob
    CharmedSnake = 362,   // Archer Spell mob
    FrozenSoldier = 363,
    FrozenFighter = 364, // AI 44
    FrozenArcher = 365,  // AI 8
    FrozenKnight = 366,
    FrozenGolem = 367,
    IcePhantom = 368, // TODO
    SnowWolf = 369,
    SnowWolfKing = 370, // BOSS
    WaterDragon = 371,
    BlackTortoise = 372,
    Manticore = 373, // TODO
    DragonWarrior = 374,
    DragonArcher = 375,
    Kirin = 376,
    Guard3 = 377,
    ArcherGuard3 = 378,
    Bunny2 = 379,
    FrozenMiner = 380,
    FrozenAxeman = 381,
    FrozenMagician = 382,
    SnowYeti = 383,
    IceCrystalSoldier = 384,
    DarkWraith = 385,
    DarkSpirit = 386,
    CrystalBeast = 387,
    RedOrb = 388,
    BlueOrb = 389,
    YellowOrb = 390,
    GreenOrb = 391,
    WhiteOrb = 392,
    FatalLotus = 393,
    AntCommander = 394,
    CargoBoxwithlogo = 395,
    Doe = 396,
    Reindeer = 397,
    AngryReindeer = 398,
    CargoBox = 399,
    Ram1 = 400,
    Ram2 = 401,
    Kite = 402,
    PurpleFaeFlower = 403,
    Furball = 404,
    GlacierSnail = 405,
    FurbolgWarrior = 406,
    FurbolgArcher = 407,
    FurbolgCommander = 408,
    RedFaeFlower = 409,
    FurbolgGuard = 410,
    GlacierBeast = 411,
    GlacierWarrior = 412,
    ShardGuardian = 413,
    WarriorScroll = 414,  // HoodedSummonerScrolls effect 0
    TaoistScroll = 415,   // HoodedSummonerScrolls effect 1
    WizardScroll = 416,   // HoodedSummonerScrolls effect 2
    AssassinScroll = 417, // HoodedSummonerScrolls effect 3
    HoodedSummoner = 418,
    HoodedIceMage = 419,
    HoodedPriest = 420,
    ShardMaiden = 421,
    KingKong = 422,
    WarBear = 423,
    ReaperPriest = 424,
    ReaperWizard = 425,
    ReaperAssassin = 426,
    LivingVines = 427,
    BlueMonk = 428,
    MutantBeserker = 429,
    MutantGuardian = 430,
    MutantHighPriest = 431,
    MysteriousMage = 432,
    FeatheredWolf = 433,
    MysteriousAssassin = 434,
    MysteriousMonk = 435,
    ManEatingPlant = 436,
    HammerDwarf = 437,
    ArcherDwarf = 438,
    NobleWarrior = 439,
    NobleArcher = 440,
    NoblePriest = 441,
    NobleAssassin = 442,
    Swain = 443,
    RedMutantPlant = 444,
    BlueMutantPlant = 445,
    UndeadHammerDwarf = 446,
    UndeadDwarfArcher = 447,
    AncientStoneGolem = 448,
    Serpentirian = 449,
    Butcher = 450,
    Riklebites = 451,
    FeralTundraFurbolg = 452,
    FeralFlameFurbolg = 453,
    ArcaneTotem = 454,
    SpectralWraith = 455,
    BabyMagmaDragon = 456,
    BloodLord = 457,
    SerpentLord = 458,
    MirEmperor = 459,
    MutantManEatingPlant = 460,
    MutantWarg = 461,
    GrassElemental = 462,
    RockElemental = 463,
    EvilMir = 900,
    EvilMirBody = 901,
    DragonStatue = 902,
    HellBomb1 = 903,
    HellBomb2 = 904,
    HellBomb3 = 905,
    Catapult = 940,
    ChariotBallista = 941,
    Ballista = 942,
    Trebuchet = 943,
    CanonTrebuchet = 944,
    SabukGate = 950,
    PalaceWallLeft = 951,
    PalaceWall1 = 952,
    PalaceWall2 = 953,
    GiGateSouth = 954,
    GiGateEast = 955,
    GiGateWest = 956,
    SSabukWall1 = 957,
    SSabukWall2 = 958,
    SSabukWall3 = 959,
    NammandGate1 = 960,
    NammandGate2 = 961,
    SabukWallSection = 962,
    NammandWallSection = 963,
    FrozenDoor = 964,
    BabyPig = 10000,
    Chick = 10001,
    Kitten = 10002,
    BabySkeleton = 10003,
    Baekdon = 10004,
    Wimaen = 10005,
    BlackKitten = 10006,
    BabyDragon = 10007,
    OlympicFlame = 10008,
    BabySnowMan = 10009,
    Frog = 10010,
    BabyMonkey = 10011,
    AngryBird = 10012,
    Foxey = 10013,
    MedicalRat = 10014,
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
    Standing = 0,
    Walking = 1,
    Running = 2,
    Pushed = 3,
    DashL = 4,
    DashR = 5,
    DashFail = 6,
    Stance = 7,
    Stance2 = 8,
    Attack1 = 9,
    Attack2 = 10,
    Attack3 = 11,
    Attack4 = 12,
    Attack5 = 13,
    AttackRange1 = 14,
    AttackRange2 = 15,
    AttackRange3 = 16,
    Special = 17,
    Struck = 18,
    Harvest = 19,
    Spell = 20,
    Die = 21,
    Dead = 22,
    Skeleton = 23,
    Show = 24,
    Hide = 25,
    Stoned = 26,
    Appear = 27,
    Revive = 28,
    SitDown = 29,
    Mine = 30,
    Sneek = 31,
    DashAttack = 32,
    Lunge = 33,
    WalkingBow = 34,
    RunningBow = 35,
    Jump = 36,
    MountStanding = 37,
    MountWalking = 38,
    MountRunning = 39,
    MountStruck = 40,
    MountAttack = 41,
    FishingCast = 42,
    FishingWait = 43,
    FishingReel = 44,
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
    Walk = 0,
    HighWall = 1,
    LowWall = 2,
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
    Normal = 0,
    Dawn = 1,
    Day = 2,
    Evening = 3,
    Night = 4,
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
    None = 0,
    Player = 1,
    Item = 2,
    Merchant = 3,
    Spell = 4,
    Monster = 5,
    Deco = 6,
    Creature = 7,
    Hero = 8,
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
    Normal = 0,
    Shout = 1,
    System = 2,
    Hint = 3,
    Announcement = 4,
    Group = 5,
    WhisperIn = 6,
    WhisperOut = 7,
    Guild = 8,
    Trainer = 9,
    LevelUp = 10,
    System2 = 11,
    Relationship = 12,
    Mentor = 13,
    Shout2 = 14,
    Shout3 = 15,
    LineMessage = 16,
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
    Nothing = 0,
    Weapon = 1,
    Armour = 2,
    Helmet = 4,
    Necklace = 5,
    Bracelet = 6,
    Ring = 7,
    Amulet = 8,
    Belt = 9,
    Boots = 10,
    Stone = 11,
    Torch = 12,
    Potion = 13,
    Ore = 14,
    Meat = 15,
    CraftingMaterial = 16,
    Scroll = 17,
    Gem = 18,
    Mount = 19,
    Book = 20,
    Script = 21,
    Reins = 22,
    Bells = 23,
    Saddle = 24,
    Ribbon = 25,
    Mask = 26,
    Food = 27,
    Hook = 28,
    Float = 29,
    Bait = 30,
    Finder = 31,
    Reel = 32,
    Fish = 33,
    Quest = 34,
    Awakening = 35,
    Pets = 36,
    Transform = 37,
    Deco = 38,
    Socket = 39,
    MonsterSpawn = 40,
    SiegeAmmo = 41,
    SealedHero = 42,
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
    None = 0,
    Inventory = 1,
    Equipment = 2,
    Trade = 3,
    Storage = 4,
    BuyBack = 5,
    DropPanel = 6,
    Inspect = 7,
    TrustMerchant = 8,
    GuildStorage = 9,
    GuestTrade = 10,
    Mount = 11,
    Fishing = 12,
    QuestInventory = 13,
    AwakenItem = 14,
    Mail = 15,
    Refine = 16,
    Renting = 17,
    GuestRenting = 18,
    Craft = 19,
    Socket = 20,
    HeroEquipment = 21,
    HeroInventory = 22,
    HeroHpItem = 23,
    HeroMpItem = 24,
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
    Weapon = 0,
    Armour = 1,
    Helmet = 2,
    Torch = 3,
    Necklace = 4,
    BraceletL = 5,
    BraceletR = 6,
    RingL = 7,
    RingR = 8,
    Amulet = 9,
    Belt = 10,
    Boots = 11,
    Stone = 12,
    Mount = 13,
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
    Reins = 0,
    Bells = 1,
    Saddle = 2,
    Ribbon = 3,
    Mask = 4,
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
    Hook = 0,
    Float = 1,
    Bait = 2,
    Finder = 3,
    Reel = 4,
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
    Peace = 0,
    Group = 1,
    Guild = 2,
    EnemyGuild = 3,
    RedBrown = 4,
    All = 5,
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
    Both = 0,
    MoveOnly = 1,
    AttackOnly = 2,
    None = 3,
    FocusMasterTarget = 4,
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
    Level = 0,
    MaxAc = 1,
    MaxMac = 2,
    MaxDc = 3,
    MaxMc = 4,
    MaxSc = 5,
    MaxLevel = 6,
    MinAc = 7,
    MinMac = 8,
    MinDc = 9,
    MinMc = 10,
    MinSc = 11,
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
    None = 0,
    Spirit = 1,
    Recall = 2,
    RedOrchid = 3,
    RedFlower = 4,
    Smash = 5,
    HwanDevil = 6,
    Purity = 7,
    FiveString = 8,
    Mundane = 9,
    NokChi = 10,
    TaoProtect = 11,
    Mir = 12,
    Bone = 13,
    Bug = 14,
    WhiteGold = 15,
    WhiteGoldH = 16,
    RedJade = 17,
    RedJadeH = 18,
    Nephrite = 19,
    NephriteH = 20,
    Whisker1 = 21,
    Whisker2 = 22,
    Whisker3 = 23,
    Whisker4 = 24,
    Whisker5 = 25,
    Hyeolryong = 26,
    Monitor = 27,
    Oppressive = 28,
    Paeok = 29,
    Sulgwan = 30,
    BlueFrost = 31,
    DarkGhost = 38,
    BlueFrostH = 39,
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
    None = 0,

    // Warrior
    Fencing = 1,
    Slaying = 2,
    Thrusting = 3,
    HalfMoon = 4,
    ShoulderDash = 5,
    TwinDrakeBlade = 6,
    Entrapment = 7,
    FlamingSword = 8,
    LionRoar = 9,
    CrossHalfMoon = 10,
    BladeAvalanche = 11,
    ProtectionField = 12,
    Rage = 13,
    CounterAttack = 14,
    SlashingBurst = 15,
    Fury = 16,
    ImmortalSkin = 17,

    // Wizard
    FireBall = 31,
    Repulsion = 32,
    ElectricShock = 33,
    GreatFireBall = 34,
    HellFire = 35,
    ThunderBolt = 36,
    Teleport = 37,
    FireBang = 38,
    FireWall = 39,
    Lightning = 40,
    FrostCrunch = 41,
    ThunderStorm = 42,
    MagicShield = 43,
    TurnUndead = 44,
    Vampirism = 45,
    IceStorm = 46,
    FlameDisruptor = 47,
    Mirroring = 48,
    FlameField = 49,
    Blizzard = 50,
    MagicBooster = 51,
    MeteorStrike = 52,
    IceThrust = 53,
    FastMove = 54,
    StormEscape = 55,

    // Taoist
    Healing = 61,
    SpiritSword = 62,
    Poisoning = 63,
    SoulFireBall = 64,
    SummonSkeleton = 65,
    Hiding = 67,
    MassHiding = 68,
    SoulShield = 69,
    Revelation = 70,
    BlessedArmour = 71,
    EnergyRepulsor = 72,
    TrapHexagon = 73,
    Purification = 74,
    MassHealing = 75,
    Hallucination = 76,
    UltimateEnhancer = 77,
    SummonShinsu = 78,
    Reincarnation = 79,
    SummonHolyDeva = 80,
    Curse = 81,
    Plague = 82,
    PoisonCloud = 83,
    EnergyShield = 84,
    PetEnhancer = 85,
    HealingCircle = 86,

    // Assassin
    FatalSword = 91,
    DoubleSlash = 92,
    Haste = 93,
    FlashDash = 94,
    LightBody = 95,
    HeavenlySword = 96,
    FireBurst = 97,
    Trap = 98,
    PoisonSword = 99,
    MoonLight = 100,
    MPEater = 101,
    SwiftFeet = 102,
    DarkBody = 103,
    Hemorrhage = 104,
    CrescentSlash = 105,
    MoonMist = 106,
    CatTongue = 107,

    // Archer
    Focus = 121,
    StraightShot = 122,
    DoubleShot = 123,
    ExplosiveTrap = 124,
    DelayedExplosion = 125,
    Meditation = 126,
    BackStep = 127,
    ElementalShot = 128,
    Concentration = 129,
    Stonetrap = 130,
    ElementalBarrier = 131,
    SummonVampire = 132,
    VampireShot = 133,
    SummonToad = 134,
    PoisonShot = 135,
    CrippleShot = 136,
    SummonSnakes = 137,
    NapalmShot = 138,
    OneWithNature = 139,
    BindingShot = 140,
    MentalState = 141,

    // Custom
    Blink = 151,
    Portal = 152,
    BattleCry = 153,
    FireBounce = 154,
    MeteorShower = 155,

    // Map Events
    DigOutZombie = 200,
    Rubble = 201,
    MapLightning = 202,
    MapLava = 203,
    MapQuake1 = 204,
    MapQuake2 = 205,
    DigOutArmadillo = 206,
    GeneralMeowMeowThunder = 207,
    StoneGolemQuake = 208,
    EarthGolemPile = 209,
    TreeQueenRoot = 210,
    TreeQueenMassRoots = 211,
    TreeQueenGroundRoots = 212,
    TucsonGeneralRock = 213,
    FlyingStatueIceTornado = 214,
    DarkOmaKingNuke = 215,
    HornedSorcererDustTornado = 216,
    HornedCommanderRockFall = 217,
    HornedCommanderRockSpike = 218,
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
    None = 0,
    FatalSword = 1,
    Teleport = 2,
    Healing = 3,
    RedMoonEvil = 4,
    TwinDrakeBlade = 5,
    MagicShieldUp = 6,
    MagicShieldDown = 7,
    GreatFoxSpirit = 8,
    Entrapment = 9,
    Reflect = 10,
    Critical = 11,
    Mine = 12,
    ElementalBarrierUp = 13,
    ElementalBarrierDown = 14,
    DelayedExplosion = 15,
    MPEater = 16,
    Hemorrhage = 17,
    Bleeding = 18,
    AwakeningSuccess = 19,
    AwakeningFail = 20,
    AwakeningMiss = 21,
    AwakeningHit = 22,
    StormEscape = 23,
    TurtleKing = 24,
    Behemoth = 25,
    Stunned = 26,
    IcePillar = 27,
    KingGuard = 28,
    KingGuard2 = 29,
    DeathCrawlerBreath = 30,
    FlamingMutantWeb = 31,
    FurbolgWarriorCritical = 32,
    Tester = 33,
    MoonMist = 34,
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
    None = 0,

    // Magics
    TemporalFlux = 1,
    Hiding = 2,
    Haste = 3,
    SwiftFeet = 4,
    Fury = 5,
    SoulShield = 6,
    BlessedArmour = 7,
    LightBody = 8,
    UltimateEnhancer = 9,
    ProtectionField = 10,
    Rage = 11,
    Curse = 12,
    MoonLight = 13,
    DarkBody = 14,
    Concentration = 15,
    VampireShot = 16,
    PoisonShot = 17,
    CounterAttack = 18,
    MentalState = 19,
    EnergyShield = 20,
    MagicBooster = 21,
    PetEnhancer = 22,
    ImmortalSkin = 23,
    MagicShield = 24,
    ElementalBarrier = 25,

    // Monster
    HornedArcherBuff = 50,
    ColdArcherBuff = 51,
    GeneralMeowMeowShield = 52,
    RhinoPriestDebuff = 53,
    PowerBeadBuff = 54,
    HornedWarriorShield = 55,
    HornedCommanderShield = 56,
    Blindness = 57,

    // Special
    GameMaster = 100,
    General = 101,
    Exp = 102,
    Drop = 103,
    Gold = 104,
    BagWeight = 105,
    Transform = 106,
    Lover = 107,
    Mentee = 108,
    Mentor = 109,
    Guild = 110,
    Prison = 111,
    Rested = 112,
    Skill = 113,
    ClearRing = 114,
    Newbie = 115,

    // Stats
    Impact = 200,
    Magic = 201,
    Taoist = 202,
    Storm = 203,
    HealthAid = 204,
    ManaAid = 205,
    Defence = 206,
    MagicDefence = 207,
    WonderDrug = 208,
    Knapsack = 209,
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
    None = 0,
    ResetDuration = 1,
    StackDuration = 2,
    StackStat = 3,
    StackStatAndDuration = 4,
    Infinite = 5,
    ResetStat = 6,
    ResetStatAndDuration = 7,
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
    AcAgility = 0,
    Ac = 1,
    MacAgility = 2,
    Mac = 3,
    Agility = 4,
    Repulsion = 5,
    None = 6,
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
    Request = 0,
    Auto = 1,
    Forced = 2,
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
    CapturePalace = 0,
    KingOfHill = 1,
    Random = 2,
    Classic = 3,
    ControlPoints = 4,
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
    Closed = 0,
    Opening = 1,
    Open = 2,
    Closing = 3,
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
    Automatic = 0,
    SemiAutomatic = 1,
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
    None = 0,
    Unsummoned = 1,
    Summoned = 2,
    Dead = 3,
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
    False = 0,
    True = 1,
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
    Any = 0,
    Sold = 1,
    Expired = 2,
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
    NewHeroInfo = 33,
    NewChatItem = 34,
    MoveItem = 35,
    EquipItem = 36,
    MergeItem = 37,
    RemoveItem = 38,
    RemoveSlotItem = 39,
    TakeBackItem = 40,
    StoreItem = 41,
    SplitItem = 42,
    SplitItem1 = 43,
    DepositRefineItem = 44,
    RetrieveRefineItem = 45,
    RefineCancel = 46,
    RefineItem = 47,
    DepositTradeItem = 48,
    RetrieveTradeItem = 49,
    UseItem = 50,
    DropItem = 51,
    TakeBackHeroItem = 52,
    TransferHeroItem = 53,
    PlayerUpdate = 54,
    PlayerInspect = 55,
    LogOutSuccess = 56,
    LogOutFailed = 57,
    ReturnToLogin = 58,
    TimeOfDay = 59,
    ChangeAMode = 60,
    ChangePMode = 61,
    ObjectItem = 62,
    ObjectGold = 63,
    GainedItem = 64,
    GainedGold = 65,
    LoseGold = 66,
    GainedCredit = 67,
    LoseCredit = 68,
    ObjectMonster = 69,
    ObjectAttack = 70,
    Struck = 71,
    ObjectStruck = 72,
    DamageIndicator = 73,
    DuraChanged = 74,
    HealthChanged = 75,
    HeroHealthChanged = 76,
    DeleteItem = 77,
    Death = 78,
    ObjectDied = 79,
    ColourChanged = 80,
    ObjectColourChanged = 81,
    ObjectGuildNameChanged = 82,
    GainExperience = 83,
    GainHeroExperience = 84,
    LevelChanged = 85,
    HeroLevelChanged = 86,
    ObjectLeveled = 87,
    ObjectHarvest = 88,
    ObjectHarvested = 89,
    ObjectNpc = 90,
    NPCResponse = 91,
    ObjectHide = 92,
    ObjectShow = 93,
    Poisoned = 94,
    ObjectPoisoned = 95,
    MapChanged = 96,
    ObjectTeleportOut = 97,
    ObjectTeleportIn = 98,
    TeleportIn = 99,
    NPCGoods = 100,
    NPCSell = 101,
    NPCRepair = 102,
    NPCSRepair = 103,
    NPCRefine = 104,
    NPCCheckRefine = 105,
    NPCCollectRefine = 106,
    NPCReplaceWedRing = 107,
    NPCStorage = 108,
    SellItem = 109,
    CraftItem = 110,
    RepairItem = 111,
    ItemRepaired = 112,
    ItemSlotSizeChanged = 113,
    ItemSealChanged = 114,
    NewMagic = 115,
    RemoveMagic = 116,
    MagicLeveled = 117,
    Magic = 118,
    MagicDelay = 119,
    MagicCast = 120,
    ObjectMagic = 121,
    ObjectEffect = 122,
    ObjectProjectile = 123,
    RangeAttack = 124,
    Pushed = 125,
    ObjectPushed = 126,
    ObjectName = 127,
    UserStorage = 128,
    SwitchGroup = 129,
    DeleteGroup = 130,
    DeleteMember = 131,
    GroupInvite = 132,
    AddMember = 133,
    Revived = 134,
    ObjectRevived = 135,
    SpellToggle = 136,
    ObjectHealth = 137,
    ObjectMana = 138,
    MapEffect = 139,
    AllowObserve = 140,
    ObjectRangeAttack = 141,
    AddBuff = 142,
    RemoveBuff = 143,
    PauseBuff = 144,
    ObjectHidden = 145,
    RefreshItem = 146,
    ObjectSpell = 147,
    UserDash = 148,
    ObjectDash = 149,
    UserDashFail = 150,
    ObjectDashFail = 151,
    NPCConsign = 152,
    NPCMarket = 153,
    NPCMarketPage = 154,
    ConsignItem = 155,
    MarketFail = 156,
    MarketSuccess = 157,
    ObjectSitDown = 158,
    InTrapRock = 159,
    BaseStatsInfo = 160,
    HeroBaseStatsInfo = 161,
    UserName = 162,
    ChatItemStats = 163,
    GuildNoticeChange = 164,
    GuildMemberChange = 165,
    GuildStatus = 166,
    GuildInvite = 167,
    GuildExpGain = 168,
    GuildNameRequest = 169,
    GuildStorageGoldChange = 170,
    GuildStorageItemChange = 171,
    GuildStorageList = 172,
    GuildRequestWar = 173,
    HeroCreateRequest = 174,
    NewHero = 175,
    HeroInformation = 176,
    UpdateHeroSpawnState = 177,
    UnlockHeroAutoPot = 178,
    SetAutoPotValue = 179,
    SetAutoPotItem = 180,
    SetHeroBehaviour = 181,
    ManageHeroes = 182,
    ChangeHero = 183,
    DefaultNPC = 184,
    NPCUpdate = 185,
    NPCImageUpdate = 186,
    MarriageRequest = 187,
    DivorceRequest = 188,
    MentorRequest = 189,
    TradeRequest = 190,
    TradeAccept = 191,
    TradeGold = 192,
    TradeItem = 193,
    TradeConfirm = 194,
    TradeCancel = 195,
    MountUpdate = 196,
    EquipSlotItem = 197,
    FishingUpdate = 198,
    ChangeQuest = 199,
    CompleteQuest = 200,
    ShareQuest = 201,
    NewQuestInfo = 202,
    GainedQuestItem = 203,
    DeleteQuestItem = 204,
    CancelReincarnation = 205,
    RequestReincarnation = 206,
    UserBackStep = 207,
    ObjectBackStep = 208,
    UserDashAttack = 209,
    ObjectDashAttack = 210,
    UserAttackMove = 211,
    CombineItem = 212,
    ItemUpgraded = 213,
    SetConcentration = 214,
    SetElemental = 215,
    RemoveDelayedExplosion = 216,
    ObjectDeco = 217,
    ObjectSneaking = 218,
    ObjectLevelEffects = 219,
    SetBindingShot = 220,
    SendOutputMessage = 221,
    NPCAwakening = 222,
    NPCDisassemble = 223,
    NPCDowngrade = 224,
    NPCReset = 225,
    AwakeningNeedMaterials = 226,
    AwakeningLockedItem = 227,
    Awakening = 228,
    ReceiveMail = 229,
    MailLockedItem = 230,
    MailSendRequest = 231,
    MailSent = 232,
    ParcelCollected = 233,
    MailCost = 234,
    ResizeInventory = 235,
    ResizeStorage = 236,
    NewIntelligentCreature = 237,
    UpdateIntelligentCreatureList = 238,
    IntelligentCreatureEnableRename = 239,
    IntelligentCreaturePickup = 240,
    NPCPearlGoods = 241,
    TransformUpdate = 242,
    FriendUpdate = 243,
    LoverUpdate = 244,
    MentorUpdate = 245,
    GuildBuffList = 246,
    NPCRequestInput = 247,
    GameShopInfo = 248,
    GameShopStock = 249,
    Rankings = 250,
    Opendoor = 251,
    GetRentedItems = 252,
    ItemRentalRequest = 253,
    ItemRentalFee = 254,
    ItemRentalPeriod = 255,
    DepositRentalItem = 256,
    RetrieveRentalItem = 257,
    UpdateRentalItem = 258,
    CancelItemRental = 259,
    ItemRentalLock = 260,
    ItemRentalPartnerLock = 261,
    CanConfirmItemRental = 262,
    ConfirmItemRental = 263,
    NewRecipeInfo = 264,
    OpenBrowser = 265,
    PlaySound = 266,
    SetTimer = 267,
    ExpireTimer = 268,
    UpdateNotice = 269,
    Roll = 270,
    SetCompass = 271,
    GroupMembersMap = 272,
    SendMemberLocation = 273,
    GuildTerritoryPage = 274,
    PurchaseGuildTerritory = 275,
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
        let value = Spell::try_from(155).expect("spell enum");
        assert_eq!(value, Spell::MeteorShower);
    }

    #[test]
    fn buff_type_offsets() {
        assert_eq!(BuffType::MagicShield as u8, 24);
        assert_eq!(BuffType::HornedArcherBuff as u8, 50);
        assert_eq!(BuffType::Impact as u8, 200);
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
        let raw = Monster::OmaKing as u16;
        let value = Monster::try_from(raw).expect("monster enum");
        assert_eq!(value, Monster::OmaKing);
        assert_eq!(Monster::FlameQueen as u16, 242);
    }

    #[test]
    fn monster_pet_ids() {
        let pet_raw = 10008u16;
        let pet = Monster::try_from(pet_raw).expect("pet monster enum");
        assert_eq!(pet, Monster::OlympicFlame);
    }

    #[test]
    fn mir_action_roundtrip() {
        let raw = MirAction::MountAttack as u8;
        let action = MirAction::try_from(raw).expect("mir action");
        assert_eq!(action, MirAction::MountAttack);
        assert_eq!(MirAction::FishingReel as u8, 44);
    }
}

// ==================== Phase 1.2 Additional Types ====================

/// Color represented as ARGB (Alpha, Red, Green, Blue)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
            argb: ((alpha as i32) << 24) | ((red as i32) << 16) | ((green as i32) << 8) | (blue as i32),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color { argb: 0 }
    }
}




