use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const KEYBINDS_FILENAME: &str = "KeyBinds.ini";
const GUIDE_LINES: &[&str] = &[
    "RequireAlt,RequireShift,RequireTilde,RequireCtrl",
    "have 3 options: 0/1/2",
    "0 < you cannot have this key pressed to use the function",
    "1 < you have to have this key pressed to use this function",
    "2 < it doesnt matter if you press this key to use this function",
    "by default just use 2, unless you have 2 functions on the same key",
    "example: change attack mode (ctrl+h) and help (h)",
    "if you set either of those to requireshift 2, then they wil both work at the same time or not work",
    "",
    "To get the value for RequireKey look at:",
    "https://msdn.microsoft.com/en-us/library/system.windows.forms.keys(v=vs.110).aspx",
];

macro_rules! keybind_options {
    ($($variant:ident),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum KeybindOption {
            $( $variant ),+
        }

        impl KeybindOption {
            pub fn iter() -> impl Iterator<Item = KeybindOption> {
                [ $( KeybindOption::$variant ),+ ].into_iter()
            }

            pub fn as_str(&self) -> &'static str {
                match self {
                    $( KeybindOption::$variant => stringify!($variant), )+
                }
            }
        }

        impl fmt::Display for KeybindOption {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl KeybindOption {
            fn from_str(value: &str) -> Option<Self> {
                match value {
                    $( stringify!($variant) => Some(KeybindOption::$variant), )+
                    _ => None,
                }
            }
        }
    };
}

keybind_options![
    Bar1Skill1,
    Bar1Skill2,
    Bar1Skill3,
    Bar1Skill4,
    Bar1Skill5,
    Bar1Skill6,
    Bar1Skill7,
    Bar1Skill8,
    Bar2Skill1,
    Bar2Skill2,
    Bar2Skill3,
    Bar2Skill4,
    Bar2Skill5,
    Bar2Skill6,
    Bar2Skill7,
    Bar2Skill8,
    Inventory,
    Inventory2,
    Equipment,
    Equipment2,
    Skills,
    Skills2,
    Creature,
    MountWindow,
    Mount,
    Fishing,
    Skillbar,
    Mentor,
    Relationship,
    Friends,
    Guilds,
    GameShop,
    Quests,
    Closeall,
    Options,
    Options2,
    Group,
    Belt,
    BeltFlip,
    Pickup,
    Belt1,
    Belt1Alt,
    Belt2,
    Belt2Alt,
    Belt3,
    Belt3Alt,
    Belt4,
    Belt4Alt,
    Belt5,
    Belt5Alt,
    Belt6,
    Belt6Alt,
    Belt7,
    Belt7Alt,
    Belt8,
    Belt8Alt,
    Logout,
    Exit,
    CreaturePickup,
    CreatureAutoPickup,
    Minimap,
    Bigmap,
    Trade,
    Rental,
    ChangeAttackmode,
    AttackmodePeace,
    AttackmodeGroup,
    AttackmodeGuild,
    AttackmodeEnemyguild,
    AttackmodeRedbrown,
    AttackmodeAll,
    ChangePetmode,
    PetmodeBoth,
    PetmodeMoveonly,
    PetmodeAttackonly,
    PetmodeNone,
    Help,
    Keybind,
    Autorun,
    Cameramode,
    Screenshot,
    DropView,
    TargetDead,
    Ranking,
    AddGroupMember,
    HeroSkill1,
    HeroSkill2,
    HeroSkill3,
    HeroSkill4,
    HeroSkill5,
    HeroSkill6,
    HeroSkill7,
    HeroSkill8,
    HeroInventory,
    HeroEquipment,
    HeroSkills,
    TargetSpellLockOn,
    PetmodeFocusMasterTarget,
];

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierRequirement {
    NotPressed = 0,
    MustPress = 1,
    Either = 2,
}

impl ModifierRequirement {
    fn parse(value: &str, fallback: Self) -> Self {
        match value.trim() {
            "0" => ModifierRequirement::NotPressed,
            "1" => ModifierRequirement::MustPress,
            "2" => ModifierRequirement::Either,
            _ => fallback,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            ModifierRequirement::NotPressed => "0",
            ModifierRequirement::MustPress => "1",
            ModifierRequirement::Either => "2",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModifierState {
    pub alt: ModifierRequirement,
    pub shift: ModifierRequirement,
    pub tilde: ModifierRequirement,
    pub ctrl: ModifierRequirement,
}

impl ModifierState {
    pub const fn new(
        alt: ModifierRequirement,
        shift: ModifierRequirement,
        tilde: ModifierRequirement,
        ctrl: ModifierRequirement,
    ) -> Self {
        Self {
            alt,
            shift,
            tilde,
            ctrl,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBind {
    pub option: KeybindOption,
    pub group: String,
    pub description: String,
    pub key: String,
    pub modifiers: ModifierState,
}

impl KeyBind {
    pub fn new(
        option: KeybindOption,
        group: impl Into<String>,
        description: impl Into<String>,
        key: impl Into<String>,
        alt: ModifierRequirement,
        shift: ModifierRequirement,
        tilde: ModifierRequirement,
        ctrl: ModifierRequirement,
    ) -> Self {
        Self {
            option,
            group: group.into(),
            description: description.into(),
            key: key.into(),
            modifiers: ModifierState::new(alt, shift, tilde, ctrl),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyBindSettings {
    path: PathBuf,
    binds: BTreeMap<KeybindOption, KeyBind>,
    #[allow(dead_code)]
    defaults: BTreeMap<KeybindOption, KeyBind>,
}

impl KeyBindSettings {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(KEYBINDS_FILENAME);
        let defaults = default_bindings_map();
        let mut binds = defaults.clone();

        if path.exists() {
            read_ini_file(&path, &mut binds)
                .with_context(|| format!("failed to load keybinds from `{}`", path.display()))?;
        } else {
            write_ini_file(&path, &binds)?;
        }

        Ok(Self {
            path,
            binds,
            defaults,
        })
    }

    pub fn save(&self) -> Result<()> {
        write_ini_file(&self.path, &self.binds)
    }

    #[allow(dead_code)]
    pub fn reset_to_defaults(&mut self) {
        self.binds = self.defaults.clone();
    }

    pub fn len(&self) -> usize {
        self.binds.len()
    }

    pub fn binding(&self, option: KeybindOption) -> &KeyBind {
        self.binds
            .get(&option)
            .expect("keybind option missing from bindings")
    }

    pub fn binding_mut(&mut self, option: KeybindOption) -> &mut KeyBind {
        self.binds
            .get_mut(&option)
            .expect("keybind option missing from bindings")
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = &KeyBind> {
        self.binds.values()
    }
}

fn write_ini_file(path: &Path, binds: &BTreeMap<KeybindOption, KeyBind>) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory `{}`", parent.display()))?;
        }
    }

    let file = File::create(path)
        .with_context(|| format!("failed to write keybinds to `{}`", path.display()))?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "[Guide]")?;
    for (index, line) in GUIDE_LINES.iter().enumerate() {
        writeln!(writer, "{:02}={}", index + 1, line)?;
    }
    writeln!(writer)?;

    for bind in binds.values() {
        writeln!(writer, "[{}]", bind.option)?;
        writeln!(writer, "RequireAlt={}", bind.modifiers.alt.as_str())?;
        writeln!(writer, "RequireShift={}", bind.modifiers.shift.as_str())?;
        writeln!(writer, "RequireTilde={}", bind.modifiers.tilde.as_str())?;
        writeln!(writer, "RequireCtrl={}", bind.modifiers.ctrl.as_str())?;
        writeln!(writer, "RequireKey={}", bind.key)?;
        writeln!(writer)?;
    }

    writer.flush()?;
    Ok(())
}

fn read_ini_file(path: &Path, binds: &mut BTreeMap<KeybindOption, KeyBind>) -> Result<()> {
    let file = File::open(path)
        .with_context(|| format!("failed to read keybinds from `{}`", path.display()))?;
    let reader = BufReader::new(file);
    let mut current: Option<KeybindOption> = None;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section_name = &trimmed[1..trimmed.len() - 1];
            if section_name.eq_ignore_ascii_case("Guide") {
                current = None;
            } else {
                current = KeybindOption::from_str(section_name);
            }
            continue;
        }

        let (key, value) = match trimmed.split_once('=') {
            Some(pair) => pair,
            None => continue,
        };

        let option = match current {
            Some(option) => option,
            None => continue,
        };

        if let Some(bind) = binds.get_mut(&option) {
            let value = value.trim();
            match key.trim() {
                "RequireAlt" => {
                    bind.modifiers.alt = ModifierRequirement::parse(value, bind.modifiers.alt);
                }
                "RequireShift" => {
                    bind.modifiers.shift = ModifierRequirement::parse(value, bind.modifiers.shift);
                }
                "RequireTilde" => {
                    bind.modifiers.tilde = ModifierRequirement::parse(value, bind.modifiers.tilde);
                }
                "RequireCtrl" => {
                    bind.modifiers.ctrl = ModifierRequirement::parse(value, bind.modifiers.ctrl);
                }
                "RequireKey" => bind.key = value.to_string(),
                _ => {}
            }
        }
    }

    Ok(())
}

fn default_bindings_map() -> BTreeMap<KeybindOption, KeyBind> {
    default_bindings_list()
        .into_iter()
        .map(|bind| (bind.option, bind))
        .collect()
}

fn default_bindings_list() -> Vec<KeyBind> {
    use ModifierRequirement::{Either, MustPress, NotPressed};

    let mut list = Vec::new();

    macro_rules! push {
        ($group:literal, $desc:literal, $option:ident, $alt:expr, $shift:expr, $tilde:expr, $ctrl:expr, $key:literal) => {{
            list.push(KeyBind::new(
                KeybindOption::$option,
                $group,
                $desc,
                $key,
                $alt,
                $shift,
                $tilde,
                $ctrl,
            ));
        }};
    }

    push!(
        "Dialogs",
        "Inventory Open/Close",
        Inventory,
        Either,
        Either,
        Either,
        Either,
        "F9"
    );
    push!(
        "Dialogs",
        "Inventory Open/Close Alt",
        Inventory2,
        Either,
        Either,
        Either,
        NotPressed,
        "I"
    );
    push!(
        "Dialogs",
        "Equipment Open/Close",
        Equipment,
        Either,
        Either,
        Either,
        Either,
        "F10"
    );
    push!(
        "Dialogs",
        "Equipment Open/Close Alt",
        Equipment2,
        Either,
        Either,
        Either,
        NotPressed,
        "C"
    );
    push!(
        "Dialogs",
        "Skills Open/Close",
        Skills,
        Either,
        Either,
        Either,
        Either,
        "F11"
    );
    push!(
        "Dialogs",
        "Skills Open/Close Alt",
        Skills2,
        Either,
        Either,
        Either,
        NotPressed,
        "S"
    );
    push!(
        "Dialogs",
        "Hero Inventory Open/Close",
        HeroInventory,
        Either,
        Either,
        Either,
        MustPress,
        "I"
    );
    push!(
        "Dialogs",
        "Hero Equipment Open/Close",
        HeroEquipment,
        Either,
        Either,
        Either,
        MustPress,
        "C"
    );
    push!(
        "Dialogs",
        "Hero Skills Open/Close",
        HeroSkills,
        Either,
        Either,
        Either,
        MustPress,
        "S"
    );
    push!(
        "Dialogs",
        "Creatures Open/Close",
        Creature,
        Either,
        Either,
        Either,
        Either,
        "E"
    );
    push!(
        "Dialogs",
        "Mount Open/Close",
        MountWindow,
        Either,
        Either,
        Either,
        Either,
        "J"
    );
    push!(
        "Dialogs",
        "Fishing Open/Close",
        Fishing,
        Either,
        Either,
        Either,
        Either,
        "N"
    );
    push!(
        "Dialogs",
        "Skillbar Open/Close",
        Skillbar,
        Either,
        Either,
        Either,
        Either,
        "R"
    );
    push!(
        "Dialogs",
        "Mentor Open/Close",
        Mentor,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Dialogs",
        "Relationship Open/Close",
        Relationship,
        Either,
        Either,
        Either,
        Either,
        "L"
    );
    push!(
        "Dialogs",
        "Friends Open/Close",
        Friends,
        Either,
        Either,
        Either,
        Either,
        "F"
    );
    push!(
        "Dialogs",
        "Guild Open/Close",
        Guilds,
        Either,
        Either,
        Either,
        NotPressed,
        "G"
    );
    push!(
        "Dialogs",
        "Gameshop Open/Close",
        GameShop,
        Either,
        Either,
        Either,
        Either,
        "Y"
    );
    push!(
        "Dialogs",
        "Quest Diary Open/Close",
        Quests,
        NotPressed,
        Either,
        Either,
        Either,
        "Q"
    );
    push!(
        "Dialogs",
        "Rental Open/Close",
        Rental,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Dialogs",
        "Options Open/Close",
        Options,
        Either,
        Either,
        Either,
        Either,
        "F12"
    );
    push!(
        "Dialogs",
        "Options Open/Close Alt",
        Options2,
        Either,
        Either,
        Either,
        Either,
        "O"
    );
    push!(
        "Dialogs",
        "Group Open/Close",
        Group,
        Either,
        Either,
        Either,
        Either,
        "P"
    );
    push!(
        "Dialogs",
        "Belt Open/Close",
        Belt,
        Either,
        Either,
        Either,
        NotPressed,
        "Z"
    );
    push!(
        "Dialogs",
        "Minimap Open/Close",
        Minimap,
        Either,
        Either,
        Either,
        Either,
        "V"
    );
    push!(
        "Dialogs",
        "Bigmap Open/Close",
        Bigmap,
        Either,
        Either,
        Either,
        Either,
        "B"
    );
    push!(
        "Dialogs",
        "Ranking Open/Close",
        Ranking,
        Either,
        Either,
        Either,
        Either,
        "K"
    );
    push!(
        "Dialogs",
        "Help Open/Close",
        Help,
        Either,
        NotPressed,
        Either,
        NotPressed,
        "H"
    );
    push!(
        "Dialogs",
        "Keybinds Open/Close",
        Keybind,
        Either,
        Either,
        Either,
        Either,
        "U"
    );
    push!(
        "Dialogs",
        "Close All Windows",
        Closeall,
        Either,
        Either,
        Either,
        Either,
        "Escape"
    );

    push!(
        "Skillbar",
        "Skillbar Slot 1",
        Bar1Skill1,
        Either,
        NotPressed,
        NotPressed,
        NotPressed,
        "F1"
    );
    push!(
        "Skillbar",
        "Skillbar Slot 2",
        Bar1Skill2,
        Either,
        NotPressed,
        NotPressed,
        NotPressed,
        "F2"
    );
    push!(
        "Skillbar",
        "Skillbar Slot 3",
        Bar1Skill3,
        Either,
        NotPressed,
        NotPressed,
        NotPressed,
        "F3"
    );
    push!(
        "Skillbar",
        "Skillbar Slot 4",
        Bar1Skill4,
        Either,
        NotPressed,
        NotPressed,
        NotPressed,
        "F4"
    );
    push!(
        "Skillbar",
        "Skillbar Slot 5",
        Bar1Skill5,
        Either,
        NotPressed,
        NotPressed,
        NotPressed,
        "F5"
    );
    push!(
        "Skillbar",
        "Skillbar Slot 6",
        Bar1Skill6,
        Either,
        NotPressed,
        NotPressed,
        NotPressed,
        "F6"
    );
    push!(
        "Skillbar",
        "Skillbar Slot 7",
        Bar1Skill7,
        Either,
        NotPressed,
        NotPressed,
        NotPressed,
        "F7"
    );
    push!(
        "Skillbar",
        "Skillbar Slot 8",
        Bar1Skill8,
        Either,
        NotPressed,
        NotPressed,
        NotPressed,
        "F8"
    );

    push!(
        "Skillbar",
        "Skillbar Alt Slot 1",
        Bar2Skill1,
        Either,
        NotPressed,
        NotPressed,
        MustPress,
        "F1"
    );
    push!(
        "Skillbar",
        "Skillbar Alt Slot 2",
        Bar2Skill2,
        Either,
        NotPressed,
        NotPressed,
        MustPress,
        "F2"
    );
    push!(
        "Skillbar",
        "Skillbar Alt Slot 3",
        Bar2Skill3,
        Either,
        NotPressed,
        NotPressed,
        MustPress,
        "F3"
    );
    push!(
        "Skillbar",
        "Skillbar Alt Slot 4",
        Bar2Skill4,
        Either,
        NotPressed,
        NotPressed,
        MustPress,
        "F4"
    );
    push!(
        "Skillbar",
        "Skillbar Alt Slot 5",
        Bar2Skill5,
        Either,
        NotPressed,
        NotPressed,
        MustPress,
        "F5"
    );
    push!(
        "Skillbar",
        "Skillbar Alt Slot 6",
        Bar2Skill6,
        Either,
        NotPressed,
        NotPressed,
        MustPress,
        "F6"
    );
    push!(
        "Skillbar",
        "Skillbar Alt Slot 7",
        Bar2Skill7,
        Either,
        NotPressed,
        NotPressed,
        MustPress,
        "F7"
    );
    push!(
        "Skillbar",
        "Skillbar Alt Slot 8",
        Bar2Skill8,
        Either,
        NotPressed,
        NotPressed,
        MustPress,
        "F8"
    );

    push!(
        "Skillbar",
        "Hero Skillbar Slot 1",
        HeroSkill1,
        Either,
        MustPress,
        NotPressed,
        NotPressed,
        "F1"
    );
    push!(
        "Skillbar",
        "Hero Skillbar Slot 2",
        HeroSkill2,
        Either,
        MustPress,
        NotPressed,
        NotPressed,
        "F2"
    );
    push!(
        "Skillbar",
        "Hero Skillbar Slot 3",
        HeroSkill3,
        Either,
        MustPress,
        NotPressed,
        NotPressed,
        "F3"
    );
    push!(
        "Skillbar",
        "Hero Skillbar Slot 4",
        HeroSkill4,
        Either,
        MustPress,
        NotPressed,
        NotPressed,
        "F4"
    );
    push!(
        "Skillbar",
        "Hero Skillbar Slot 5",
        HeroSkill5,
        Either,
        MustPress,
        NotPressed,
        NotPressed,
        "F5"
    );
    push!(
        "Skillbar",
        "Hero Skillbar Slot 6",
        HeroSkill6,
        Either,
        MustPress,
        NotPressed,
        NotPressed,
        "F6"
    );
    push!(
        "Skillbar",
        "Hero Skillbar Slot 7",
        HeroSkill7,
        Either,
        MustPress,
        NotPressed,
        NotPressed,
        "F7"
    );
    push!(
        "Skillbar",
        "Hero Skillbar Slot 8",
        HeroSkill8,
        Either,
        MustPress,
        NotPressed,
        NotPressed,
        "F8"
    );

    push!(
        "Belt",
        "Rotate Belt",
        BeltFlip,
        Either,
        Either,
        Either,
        MustPress,
        "Z"
    );
    push!(
        "Belt",
        "Belt Slot 1",
        Belt1,
        Either,
        Either,
        Either,
        Either,
        "D1"
    );
    push!(
        "Belt",
        "Belt Slot 1 Alt",
        Belt1Alt,
        Either,
        Either,
        Either,
        Either,
        "NumPad1"
    );
    push!(
        "Belt",
        "Belt Slot 2",
        Belt2,
        Either,
        Either,
        Either,
        Either,
        "D2"
    );
    push!(
        "Belt",
        "Belt Slot 2 Alt",
        Belt2Alt,
        Either,
        Either,
        Either,
        Either,
        "NumPad2"
    );
    push!(
        "Belt",
        "Belt Slot 3",
        Belt3,
        Either,
        Either,
        Either,
        Either,
        "D3"
    );
    push!(
        "Belt",
        "Belt Slot 3 Alt",
        Belt3Alt,
        Either,
        Either,
        Either,
        Either,
        "NumPad3"
    );
    push!(
        "Belt",
        "Belt Slot 4",
        Belt4,
        Either,
        Either,
        Either,
        Either,
        "D4"
    );
    push!(
        "Belt",
        "Belt Slot 4 Alt",
        Belt4Alt,
        Either,
        Either,
        Either,
        Either,
        "NumPad4"
    );
    push!(
        "Belt",
        "Belt Slot 5",
        Belt5,
        Either,
        Either,
        Either,
        Either,
        "D5"
    );
    push!(
        "Belt",
        "Belt Slot 5 Alt",
        Belt5Alt,
        Either,
        Either,
        Either,
        Either,
        "NumPad5"
    );
    push!(
        "Belt",
        "Belt Slot 6",
        Belt6,
        Either,
        Either,
        Either,
        Either,
        "D6"
    );
    push!(
        "Belt",
        "Belt Slot 6 Alt",
        Belt6Alt,
        Either,
        Either,
        Either,
        Either,
        "NumPad6"
    );
    push!(
        "Belt",
        "Belt Slot 7",
        Belt7,
        Either,
        Either,
        Either,
        Either,
        "D7"
    );
    push!(
        "Belt",
        "Belt Slot 7 Alt",
        Belt7Alt,
        Either,
        Either,
        Either,
        Either,
        "NumPad7"
    );
    push!(
        "Belt",
        "Belt Slot 8",
        Belt8,
        Either,
        Either,
        Either,
        Either,
        "D8"
    );
    push!(
        "Belt",
        "Belt Slot 8 Alt",
        Belt8Alt,
        Either,
        Either,
        Either,
        Either,
        "NumPad8"
    );

    push!("General", "Logout", Logout, MustPress, Either, Either, Either, "X");
    push!("General", "Exit", Exit, MustPress, Either, Either, Either, "Q");
    push!(
        "General",
        "Mount/Dismount",
        Mount,
        Either,
        Either,
        Either,
        Either,
        "M"
    );
    push!(
        "General",
        "Pickup Floor Item",
        Pickup,
        Either,
        Either,
        Either,
        Either,
        "Tab"
    );
    push!(
        "General",
        "Creature Item Pickup",
        CreaturePickup,
        NotPressed,
        Either,
        Either,
        Either,
        "X"
    );
    push!(
        "General",
        "Creature Auto Pickup",
        CreatureAutoPickup,
        MustPress,
        Either,
        Either,
        NotPressed,
        "A"
    );
    push!(
        "General",
        "Request Trade",
        Trade,
        Either,
        Either,
        Either,
        Either,
        "T"
    );
    push!(
        "General",
        "Recruit Group Member",
        AddGroupMember,
        Either,
        Either,
        Either,
        MustPress,
        "G"
    );

    push!(
        "Toggle",
        "Toggle Attack Mode",
        ChangeAttackmode,
        Either,
        NotPressed,
        Either,
        MustPress,
        "H"
    );
    push!(
        "Toggle",
        "Set Attack Mode : Peace",
        AttackmodePeace,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Toggle",
        "Set Attack Mode : Group",
        AttackmodeGroup,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Toggle",
        "Set Attack Mode : Guild",
        AttackmodeGuild,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Toggle",
        "Set Attack Mode : Enemy Guild",
        AttackmodeEnemyguild,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Toggle",
        "Set Attack Mode : Red/Brown",
        AttackmodeRedbrown,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Toggle",
        "Set Attack Mode : All",
        AttackmodeAll,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Toggle",
        "Toggle Pet Mode",
        ChangePetmode,
        NotPressed,
        Either,
        Either,
        MustPress,
        "A"
    );
    push!(
        "Toggle",
        "Set Pet Mode : Both",
        PetmodeBoth,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Toggle",
        "Set Pet Mode : Move Only",
        PetmodeMoveonly,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Toggle",
        "Set Pet Mode : Attack Only",
        PetmodeAttackonly,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Toggle",
        "Set Pet Mode : None",
        PetmodeNone,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Toggle",
        "Set Pet Mode : Focus Master Target",
        PetmodeFocusMasterTarget,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Toggle",
        "Toggle Autorun",
        Autorun,
        Either,
        Either,
        Either,
        Either,
        "D"
    );
    push!(
        "Toggle",
        "Toggle Camera Mode",
        Cameramode,
        Either,
        Either,
        Either,
        Either,
        "Insert"
    );
    push!(
        "Toggle",
        "Take Screenshot",
        Screenshot,
        Either,
        Either,
        Either,
        Either,
        "PrintScreen"
    );
    push!(
        "Toggle",
        "Toggle Dropview",
        DropView,
        Either,
        Either,
        Either,
        Either,
        "Tab"
    );
    push!(
        "General",
        "Toggle Target Dead",
        TargetDead,
        Either,
        Either,
        Either,
        Either,
        "None"
    );
    push!(
        "Combat",
        "Hold to enable target spell lock-on",
        TargetSpellLockOn,
        Either,
        Either,
        Either,
        Either,
        "None"
    );

    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use tempfile::tempdir;

    #[test]
    fn defaults_cover_all_options() {
        let defaults = default_bindings_map();
        let count = KeybindOption::iter().count();
        assert_eq!(defaults.len(), count);
    }

    #[test]
    fn load_creates_file_if_missing() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path();
        let path = root.join(KEYBINDS_FILENAME);
        assert!(!path.exists());

        let bindings = KeyBindSettings::load(root)?;
        assert!(path.exists());
        assert_eq!(bindings.len(), KeybindOption::iter().count());
        Ok(())
    }

    #[test]
    fn roundtrip_persists_changes() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path();
        let mut bindings = KeyBindSettings::load(root)?;

        {
            let entry = bindings.binding_mut(KeybindOption::Inventory);
            entry.key = "F1".to_string();
            entry.modifiers.ctrl = ModifierRequirement::MustPress;
        }

        bindings.save()?;
        drop(bindings);

        let bindings = KeyBindSettings::load(root)?;
        let entry = bindings.binding(KeybindOption::Inventory);
        assert_eq!(entry.key, "F1");
        assert_eq!(entry.modifiers.ctrl, ModifierRequirement::MustPress);
        Ok(())
    }
}
