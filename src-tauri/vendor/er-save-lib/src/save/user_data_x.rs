use deku::ctx::Endian;
use deku::prelude::*;
use deku::{DekuRead, DekuWrite};

use std::io::Cursor;

use super::util::{FloatVector3, FloatVector4, MapId, Util};

#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian, end: usize, is_ps: bool")]
pub struct UserDataX {
    // Checksum (PC only)
    #[deku(skip, cond = "is_ps", count = "0x10")]
    pub checksum: Vec<u8>,

    // File version
    pub version: u32,

    // Current Map Id
    pub map_id: [u8; 4],

    // Could be just random data
    unk0x8: [u8; 0x8],
    // Definetly seemed like random data. Game is using random function to generate.
    unk0x10: [u8; 0x10],

    // Gaitem Map
    #[deku(count = "if *version <= 81 {0x13FE} else {0x1400}")]
    pub gaitem_map: Vec<Gaitem>,

    // Player data
    pub player_game_data: PlayerGameData,

    // SPEffects
    #[deku(count = "0xD")]
    pub sp_effects: Vec<SPEffect>,

    // Equipment data and Inventory data
    pub equipped_items_equip_index: EquippedItemsEquipIndex,
    pub active_weapon_slots_and_arm_style: ActiveWeaponSlotsAndArmStyle,
    pub equipped_items_item_id: EquippedItemsItemIds,
    pub equipped_items_gaitem_handle: EquppedItemsGaitemHandles,
    #[deku(ctx = "0xa80, 0x180")]
    pub inventory_held: Invenotry,
    pub equipped_spells: EquippedSpells,
    pub equipped_items: EquippedItems,
    pub equipped_gestures: EquippedGestures,
    pub acquired_projectiles: AcquiredProjectiles,
    pub equipped_armaments_and_items: EquippedArmamentsAndItems,
    pub equipped_physics: EquippedPhysics,

    // Face data
    #[deku(ctx = "false")]
    pub face_data: FaceData,

    // Inventory Data (Storage Box)
    #[deku(ctx = "0x780, 0x80")]
    pub inventory_storage_box: Invenotry,

    // Gestures
    pub gestures: Gestures,

    // Unlocked regions
    pub unlocked_regions: Regions,

    // Horse Data
    pub horse: RideGameData,

    #[deku(assert = "*control_byte_maybe == 1 || *control_byte_maybe == 0")]
    control_byte_maybe: u8,

    // Blood Stain
    pub blood_stain: BloodStain,

    unk_gamedataman_0x120_or_gamedataman_0x130: u32,
    unk_gamedataman_0x88: u32,

    // Menu Profile Save Load
    pub menu_profile_save_load: MenuSaveLoad,

    // Trophy Equip Data
    pub trophy_equip_data: TrophyEquipData,

    // Gaitem Game Data
    pub gaitem_game_data: GaitemGameData,

    // Tutorial Game Data
    pub tutorial_data: TutorialData,

    gameman_0x8c: u8,
    gameman_0x8d: u8,
    gameman_0x8e: u8,

    // Death Count
    pub total_deaths_count: u32,

    // Character Type {
    //     None = -1,
    //     Phantom = 1,
    //     Invader = 2,
    //     Ghost = 3,
    //     DeadGhost = 10,
    //     NakedGhost = 11,
    //     Unkown = 13,
    //     NakedGhost2 = 14,
    //     Invader2 = 15,
    //     Invader3 = 16,
    //     Blue = 17,
    //     Invader4 = 18
    // }
    pub character_type: i32,

    // Deactivates warp, resting at sites of grace, etc..
    pub in_online_session_flag: u8,

    // For phantom or invader character types this value is 0. Otherwise it's 8.
    #[deku(assert = "*character_type_online == 8 || *character_type_online == 0")]
    pub character_type_online: u32,

    // Last Grace EntityId + 1000
    pub last_rested_grace: u32,

    // Seems to indicate wether a player is alone or has someone in their world
    #[deku(assert = "*not_alone_flag == 1 || *not_alone_flag == 0")]
    pub not_alone_flag: u8,

    // 1 = 10 second
    pub in_game_countdown_timer: u32,

    // Is set conditionally. Can either be gameman with offset 0x124 or 0x134
    unk_gamedataman_0x124_or_gamedataman_0x134: u32,

    // Event Flags
    #[deku(bytes_read = "0x1BF99F")]
    pub event_flags: Vec<u8>,
    #[deku(assert_eq = "0")]
    event_flags_terminator: u8,

    // Field Area
    pub field_area: FieldArea,

    // World Area
    pub world_area: WorldArea,

    // World Geom Man (GEOM)
    pub world_geom_man: WorldGeomMan,

    // World Geom Man (GEOF)
    pub world_geom_man2: WorldGeomMan,

    // RendMan
    pub rend_man: RendMan,

    // Player Coordinates
    pub player_coordinates: PlayerCoordinates,

    // GameMan 0x5BE
    game_man_0x5be: u8,

    // GameMan 0x5BF
    game_man_0x5bf: u8,

    // Spawn Point Entity Id
    pub spawn_point_entity_id: u32,

    // GameMan 0x5BF
    game_man_0xb64: u32,

    // Temp spawn point entity id, only for post 65  save version
    #[deku(skip, cond = "*version < 65")]
    pub temp_spawn_point_entity_id: u32,

    // Only for post 66 save version
    #[deku(skip, cond = "*version < 66")]
    game_man_0xcb3: u8,

    // NetMan
    pub net_man: NetMan,

    // World Area Weather
    pub world_area_weather: WorldAreaWeather,

    // World Area Time
    pub world_area_time: WorldAreaTime,

    // Base Version
    pub base_version: BaseVersion,

    // SteamId
    pub steam_id: u64,

    // PS5Activity
    pub ps5_activity: PS5Activity,

    // DLC
    pub dlc: DLC,

    // Player Game Data Hash
    #[deku(writer = "PlayerGameDataHash::write(
            deku::writer, 
            endian, 
            player_game_data, 
            equipped_items_item_id, 
            equipped_armaments_and_items, 
            equipped_spells
        )")]
    pub player_data_hash: PlayerGameDataHash,

    #[deku(count = "end - deku::byte_offset")]
    pub rest: Vec<u8>,
}

impl UserDataX {
    pub fn read<R: std::io::Read>(
        reader: &mut deku::reader::Reader<R>,
        endian: Endian,
        start: usize,
        size: usize,
        count: usize,
        is_ps: bool,
    ) -> Result<Vec<Self>, DekuError> {
        let mut user_data_x_vec: Vec<Self> = Vec::with_capacity(count);
        for i in 0..count {
            let end = (start + size * i) + size;
            let user_data_x = Self::from_reader_with_ctx(reader, (endian, end, is_ps))?;
            user_data_x_vec.push(user_data_x)
        }
        Ok(user_data_x_vec)
    }

    pub fn write<W: std::io::Write>(
        writer: &mut deku::writer::Writer<W>,
        endian: Endian,
        start: usize,
        size: usize,
        is_ps: bool,
        user_data_x_vec: &Vec<Self>,
    ) -> Result<(), DekuError> {
        for (i, user_data_x) in user_data_x_vec.iter().enumerate() {
            let end = (start + size * i) + size;

            if is_ps {
                user_data_x.to_writer(writer, (endian, end, is_ps))?;
                continue;
            }

            let mut buffer = Vec::new();
            {
                let mut temp_writer = Writer::new(Cursor::new(&mut buffer));
                user_data_x.to_writer(&mut temp_writer, (endian, start + size * i, is_ps))?;
            }

            Util::update_checksum(&mut buffer);

            writer.write_bytes(&buffer)?;
        }
        Ok(())
    }
}

// Gaitem Map
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct Gaitem {
    #[deku(assert = "
            (*gaitem_handle & 0xf0000000) == 0 ||
            (*gaitem_handle & 0xf0000000) == 0x80000000 ||
            (*gaitem_handle & 0xf0000000) == 0x90000000 ||
            (*gaitem_handle & 0xf0000000) == 0xc0000000")]
    pub gaitem_handle: u32,
    #[deku(assert = "
            (*item_id == 0xFFFFFFFF) ||
            (*item_id & 0xf0000000) == 0 ||
            (*item_id & 0xf0000000) == 0x10000000 ||
            (*item_id & 0xf0000000) == 0x80000000
        ")]
    pub item_id: u32,
    #[deku(
        skip,
        cond = "*gaitem_handle == 0 || 
        *gaitem_handle & 0xf0000000 == 0xc0000000"
    )]
    pub unk0x10: Option<i32>,
    #[deku(
        skip,
        cond = "*gaitem_handle == 0 || 
        *gaitem_handle & 0xf0000000 == 0xC0000000"
    )]
    pub unk0x14: Option<i32>,
    #[deku(
        skip,
        cond = "*gaitem_handle == 0 || 
        *gaitem_handle & 0xf0000000 != 0x80000000"
    )]
    pub gem_gaitem_handle: Option<i32>,
    #[deku(
        skip,
        cond = "*gaitem_handle == 0 || 
        *gaitem_handle & 0xf0000000 != 0x80000000",
        assert = "(
            (
                (
                    *gaitem_handle == 0 || 
                    *gaitem_handle & 0xf0000000 != 0x80000000
                ) && unk0x1c.is_none()
            )
            ||
            (
                (
                    *gaitem_handle != 0 || 
                    *gaitem_handle & 0xf0000000 == 0x80000000
                ) && *unk0x1c == Some(0)
            )
        )"
    )]
    pub unk0x1c: Option<u8>,
}

// Player
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct PlayerGameData {
    unk0x0: u32,
    unk0x4: u32,
    pub hp: u32,
    pub max_hp: u32,
    pub base_max_hp: u32,
    pub fp: u32,
    pub max_fp: u32,
    pub base_max_fp: u32,
    #[deku(assert_eq = "0")]
    unk0x20: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub base_max_sp: u32,
    #[deku(assert_eq = "0")]
    unk0x30: u32,
    pub vigor: u32,
    pub mind: u32,
    pub endurance: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub faith: u32,
    pub arcane: u32,
    #[deku(assert_eq = "0")]
    pub unk0x54: u32,
    #[deku(assert_eq = "0")]
    unk0x58: u32,
    #[deku(assert_eq = "0")]
    unk0x5c: u32,
    pub level: u32,
    pub runes: u32,
    pub runes_memory: u32,
    unk0x6c: u32,
    pub poison_buildup: u32,
    pub rot_buildup: u32,
    pub bleed_buildup: u32,
    pub death_buildup: u32,
    pub frost_buildup: u32,
    pub sleep_buildup: u32,
    pub madness_buildup: u32,
    unk0x8c: u32,
    unk0x90: u32,
    #[deku(
        reader = "Util::read_wstring(deku::reader, 32)",
        writer = "Util::write_wstring(deku::writer, &character_name, 32)"
    )]
    pub character_name: String,
    #[deku(assert_eq = "0")]
    pub terminator: u16,
    pub gender: u8,
    pub archetype: u8,
    unk0xb8: u8,
    unk0xb9: u8,
    pub voice_type: u8,
    pub gift: u8,
    unk0xbc: u8,
    unk0xbd: u8,
    pub additional_talisman_slot_count: u8,
    pub summon_spirit_level: u8,
    unk0xc0: [u8; 0x18],
    pub furl_calling_finger_on: bool,
    unk0xd9: u8,
    pub matchmaking_weapon_level: u8,
    pub white_chipher_ring_on: bool,
    pub blue_cipher_ring_on: bool,
    unk0xdd: [u8; 0x1a],
    pub great_rune_on: bool,
    unk0xf8: u8,
    pub max_crimson_flask_count: u8,
    pub max_cerulean_flask_count: u8,
    unk0xfb: [u8; 0x15],
    #[deku(
        reader = "Util::read_wstring(deku::reader, 16)",
        writer = "Util::write_wstring(deku::writer, &password, 16)"
    )]
    pub password: String,
    #[deku(assert_eq = "0")]
    password_terminator: u16,
    #[deku(
        reader = "Util::read_wstring(deku::reader, 16)",
        writer = "Util::write_wstring(deku::writer, &group_password1, 16)"
    )]
    pub group_password1: String,
    #[deku(assert_eq = "0")]
    group_password1d_terminator: u16,
    #[deku(
        reader = "Util::read_wstring(deku::reader, 16)",
        writer = "Util::write_wstring(deku::writer, &group_password2, 16)"
    )]
    pub group_password2: String,
    #[deku(assert_eq = "0")]
    group_password2d_terminator: u16,
    #[deku(
        reader = "Util::read_wstring(deku::reader, 16)",
        writer = "Util::write_wstring(deku::writer, &group_password3, 16)"
    )]
    pub group_password3: String,
    #[deku(assert_eq = "0")]
    group_password3d_terminator: u16,
    #[deku(
        reader = "Util::read_wstring(deku::reader, 16)",
        writer = "Util::write_wstring(deku::writer, &group_password4, 16)"
    )]
    pub group_password4: String,
    #[deku(assert_eq = "0")]
    group_password4d_terminator: u16,
    #[deku(
        reader = "Util::read_wstring(deku::reader, 16)",
        writer = "Util::write_wstring(deku::writer, &group_password5, 16)"
    )]
    pub group_password5: String,
    #[deku(assert_eq = "0")]
    group_password5d_terminator: u16,
    unk0x17c: [u8; 0x34],
}

// SPeffects
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct SPEffect {
    sp_effect_id: i32,
    remaining_time: f32,
    unk0x8: u32,
    unk0x10: u32,
}

// Equipped Items Equip Indexes
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct EquippedItemsEquipIndex {
    pub left_hand_armament1: u32,
    pub right_hand_armament1: u32,
    pub left_hand_armament2: u32,
    pub right_hand_armament2: u32,
    pub left_hand_armament3: u32,
    pub right_hand_armament3: u32,
    pub arrows1: u32,
    pub bolts1: u32,
    pub arrows2: u32,
    pub bolts2: u32,
    unk0x28: u32,
    unk0x2c: u32,
    pub head: u32,
    pub chest: u32,
    pub arms: u32,
    pub legs: u32,
    unk0x40: u32,
    pub talisman1: u32,
    pub talisman2: u32,
    pub talisman3: u32,
    pub talisman4: u32,
    unk0x54: u32,
}

// Active weapon slot, arrow and bolt
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct ActiveWeaponSlotsAndArmStyle {
    pub arm_style: u32,
    #[deku(assert = "*left_hand_weapon_active_slot < 3")]
    pub left_hand_weapon_active_slot: u32,
    #[deku(assert = "*right_hand_weapon_active_slot < 3")]
    pub right_hand_weapon_active_slot: u32,
    #[deku(assert = "*left_arrow_active_slot < 2")]
    pub left_arrow_active_slot: u32,
    #[deku(assert = "*right_arrow_active_slot < 2")]
    pub right_arrow_active_slot: u32,
    #[deku(assert = "*left_bolt_active_slot < 2")]
    pub left_bolt_active_slot: u32,
    #[deku(assert = "*right_bolt_active_slot < 2")]
    pub right_bolt_active_slot: u32,
}

// Equipped Items Param Ids
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct EquippedItemsItemIds {
    pub left_hand_armament1: u32,
    pub right_hand_armament1: u32,
    pub left_hand_armament2: u32,
    pub right_hand_armament2: u32,
    pub left_hand_armament3: u32,
    pub right_hand_armament3: u32,
    pub arrows1: u32,
    pub bolts1: u32,
    pub arrows2: u32,
    pub bolts2: u32,
    unk0x28: u32,
    unk2c: u32,
    pub head: u32,
    pub chest: u32,
    pub arms: u32,
    pub legs: u32,
    unk40: u32,
    pub talisman1: u32,
    pub talisman2: u32,
    pub talisman3: u32,
    pub talisman4: u32,
    pub unk0x54: u32,
}

// Equipped Items GaitemHandles
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct EquppedItemsGaitemHandles {
    pub left_hand_armament1: u32,
    pub right_hand_armament1: u32,
    pub left_hand_armament2: u32,
    pub right_hand_armament2: u32,
    pub left_hand_armament3: u32,
    pub right_hand_armament3: u32,
    pub arrows1: u32,
    pub bolts1: u32,
    pub arrows2: u32,
    pub bolts2: u32,
    unk0x44: u32,
    unk48: u32,
    pub head: u32,
    pub chest: u32,
    pub arms: u32,
    pub legs: u32,
    unk5c: u32,
    pub talisman1: u32,
    pub talisman2: u32,
    pub talisman3: u32,
    pub talisman4: u32,
    unk0x54: u32,
}

// Inventory (Held and Storage Box)
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(
    endian = "endian",
    ctx = "endian: Endian, common_items_capacity: u32, key_items_capacity: u32"
)]
pub struct Invenotry {
    #[deku(assert = "*common_item_count <= common_items_capacity")]
    pub common_item_count: u32,
    #[deku(count = "common_items_capacity")]
    pub common_items: Vec<InvenotryItem>,
    #[deku(assert = "*key_item_count <= key_items_capacity")]
    pub key_item_count: u32,
    #[deku(count = "key_items_capacity")]
    pub key_items: Vec<InvenotryItem>,
    pub equip_index_counter: u32,
    pub aquistion_index_counter: u32,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct InvenotryItem {
    pub gaitem_handle: u32,
    #[deku(assert = "*quantity <= 999")]
    pub quantity: u32,
    pub aqcuistion_index: u32,
}

// Equipped Spells
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct EquippedSpells {
    #[deku(count = "14")]
    pub spellslot: Vec<Spell>,
    #[deku(assert = "*active_index < 0xc || *active_index == 0xffffffff")]
    pub active_index: u32,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct Spell {
    pub spell_id: u32,
    unk0x4: u32,
}

// Equipped Items
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct EquippedItems {
    #[deku(count = "0xa")]
    pub quick_items: Vec<EquippedItem>,
    #[deku(assert = "*active_quick_item_index < 10 || *active_quick_item_index == 0xffffffff")]
    pub active_quick_item_index: u32,
    #[deku(count = "0x6")]
    pub pouch_items: Vec<EquippedItem>,
    unk0x84: u32,
    unk0x88: u32,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct EquippedItem {
    pub gaitem_handle: u32,
    pub equip_index: u32,
}

// Equipped Gestures
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct EquippedGestures {
    #[deku(count = "0x6")]
    equipped_gesture: Vec<u32>,
}

// Aquired Projectiles
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct AcquiredProjectiles {
    pub count: u32,
    #[deku(count = "*count")]
    projectiles: Vec<Projectile>,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct Projectile {
    pub id: u32,
    unk0x4: u32,
}

// Equipped Weapons, Amor, Talisman and Items
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct EquippedArmamentsAndItems {
    pub left_hand_armament1: u32,
    pub right_hand_armament1: u32,
    pub left_hand_armament2: u32,
    pub right_hand_armament2: u32,
    pub left_hand_armament3: u32,
    pub right_hand_armament3: u32,
    pub arrows1: u32,
    pub bolts1: u32,
    pub arrows2: u32,
    pub bolts2: u32,
    unk0x28: u32,
    unk0x2c: u32,
    pub head: u32,
    pub chest: u32,
    pub arms: u32,
    pub legs: u32,
    unk0x40: u32,
    pub talisman1: u32,
    pub talisman2: u32,
    pub talisman3: u32,
    pub talisman4: u32,
    unk0x54: u32,
    pub quickitem1: u32,
    pub quickitem2: u32,
    pub quickitem3: u32,
    pub quickitem4: u32,
    pub quickitem5: u32,
    pub quickitem6: u32,
    pub quickitem7: u32,
    pub quickitem8: u32,
    pub quickitem9: u32,
    pub quickitem10: u32,
    pub pouch1: u32,
    pub pouch2: u32,
    pub pouch3: u32,
    pub pouch4: u32,
    pub pouch5: u32,
    pub pouch6: u32,
    unk0x98: u32,
}

// Equipped Physics
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct EquippedPhysics {
    pub slot1: u32,
    pub slot2: u32,
    unk0x8: u32,
}

// Face Data
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian, in_profile_summary: bool")]
pub struct FaceData {
    #[deku(assert = "*facedata0x150 == 0 || *facedata0x150 == -1")]
    facedata0x150: i32,
    #[deku(assert = "magic == &[0x46, 0x41, 0x43, 0x45] || magic == &[0, 0, 0, 0]")]
    magic: [u8; 4],
    alignment: u32,
    size: u32,
    #[deku(pad_bytes_after = "3")]
    facemodel: u8,
    #[deku(pad_bytes_after = "3")]
    hairmodel: u8,
    #[deku(pad_bytes_after = "3")]
    unk0x14: u8,
    #[deku(pad_bytes_after = "3")]
    eye_brow_model: u8,
    #[deku(pad_bytes_after = "3")]
    beardmodel: u8,
    #[deku(pad_bytes_after = "3")]
    eyepatchmodel: u8,
    #[deku(pad_bytes_after = "3")]
    unk0x24: u8,
    #[deku(pad_bytes_after = "3")]
    unk0x28: u8,
    apparent_age: u8,
    facial_aesthetic: u8,
    form_emphasis: u8,
    unk0xf: u8,
    _brow_ridge_height: u8,
    inner_brow_ridge: u8,
    outer_brow_ridge: u8,
    cheek_bone_height: u8,
    cheek_bone_depth: u8,
    cheek_bone_width: u8,
    cheek_bone_prostrution: u8,
    cheeks: u8,
    chin_tip_position: u8,
    chin_length: u8,
    chin_prostrusion: u8,
    chin_depth: u8,
    chin_size: u8,
    chin_height: u8,
    chin_width: u8,
    eye_position: u8,
    eye_size: u8,
    eye_slant: u8,
    eye_spacing: u8,
    nose_size: u8,
    nose_forehead_ratio: u8,
    unk0x45: u8,
    face_protrusion: u8,
    vertical_face_ratio: u8,
    facial_features_lant: u8,
    horizontal_face_ratio: u8,
    unk0x4a: u8,
    forehead_depth: u8,
    forehead_protrusion: u8,
    unk0x4d: u8,
    jaw_prostrusion: u8,
    jaw_width: u8,
    lower_jaw: u8,
    jaw_contour: u8,
    lip_shape: u8,
    lip_size: u8,
    lip_fullness: u8,
    mouth_expression: u8,
    lip_prostrusion: u8,
    lip_thickness: u8,
    mouth_prostrusion: u8,
    mout_hslant: u8,
    occulsion: u8,
    mouth_position: u8,
    mouth_width: u8,
    mouth_chin_distance: u8,
    nose_ridge_depth: u8,
    nose_ridge_length: u8,
    nose_position: u8,
    nose_tip_height: u8,
    nostril_slant: u8,
    nostril_size: u8,
    nostril_width: u8,
    nose_prostrution: u8,
    nose_bridge_height: u8,
    bridge_protrusion1: u8,
    bridge_protrusion2: u8,
    nose_bridge_width: u8,
    nose_height: u8,
    nose_slant: u8,
    unk0x6c: [u8; 64],
    head_size: u8,
    chest_size: u8,
    abdomen_size: u8,
    arms_size: u8,
    legs_size: u8,
    unk0xb1: [u8; 2],
    skin_color_r: u8,
    skin_color_g: u8,
    skin_color_b: u8,
    skin_luster: u8,
    pores: u8,
    stubble: u8,
    dark_circles: u8,
    dark_circle_color_r: u8,
    dark_circle_color_g: u8,
    dark_circle_color_b: u8,
    cheeks_color_intensity: u8,
    cheek_color_r: u8,
    cheek_color_g: u8,
    cheek_color_b: u8,
    eyeliner: u8,
    eyeliner_color_r: u8,
    eyeliner_color_g: u8,
    eyeliner_color_b: u8,
    eye_shadow_lower: u8,
    eye_shadow_lower_color_r: u8,
    eye_shadow_lower_color_g: u8,
    eye_shadow_lower_color_b: u8,
    eye_shadow_upper: u8,
    eye_shadow_upper_color_r: u8,
    eye_shadow_upper_color_g: u8,
    eye_shadow_upper_color_b: u8,
    lipstick: u8,
    lipstick_color_r: u8,
    lipstick_color_g: u8,
    lipstick_color_b: u8,
    tatto_markposition_horizontal: u8,
    tatto_markposition_vertical: u8,
    tatto_markangle: u8,
    tatto_markexpansion: u8,
    tatto_mark_color_r: u8,
    tatto_mark_color_g: u8,
    tatto_mark_color_b: u8,
    unk0xd8: u8,
    tatto_mark_flip: u8,
    bodyhair: u8,
    body_hair_color_r: u8,
    body_hair_color_g: u8,
    body_hair_color_b: u8,
    right_iris_color_r: u8,
    right_iris_color_g: u8,
    right_iris_color_b: u8,
    right_iris_size: u8,
    right_eye_clouding: u8,
    right_eye_clouding_color_r: u8,
    right_eye_clouding_color_g: u8,
    right_eye_clouding_color_b: u8,
    right_eye_white_color_r: u8,
    right_eye_white_color_g: u8,
    right_eye_white_color_b: u8,
    right_eye_position: u8,
    left_iris_color_r: u8,
    left_iris_color_g: u8,
    left_iris_color_b: u8,
    left_iris_size: u8,
    left_eye_clouding: u8,
    left_eye_clouding_color_r: u8,
    left_eye_clouding_color_g: u8,
    left_eye_clouding_color_b: u8,
    left_eye_white_color_r: u8,
    left_eye_white_color_g: u8,
    left_eye_white_color_b: u8,
    left_eye_position: u8,
    hair_color_r: u8,
    hair_color_g: u8,
    hari_color_b: u8,
    luster: u8,
    hair_root_darkness: u8,
    white_hairs: u8,
    beard_color_r: u8,
    beard_color_g: u8,
    beard_color_b: u8,
    beard_luster: u8,
    hair_root_darkness2: u8,
    beard_white_hairs: u8,
    brow_color_r: u8,
    brow_color_g: u8,
    brow_color_b: u8,
    brow_luster: u8,
    brow_rootdarkness: u8,
    brow_whitehairs: u8,
    eyeleash_color_r: u8,
    eyeleash_color_g: u8,
    eyeleash_color_b: u8,
    eyepatch_color_r: u8,
    eyepatch_color_g: u8,
    eyepatch_color_b: u8,
    unk0x10e: [u8; 0x12],

    #[deku(skip, cond = "in_profile_summary")]
    unk0x124: bool,
    #[deku(skip, cond = "in_profile_summary")]
    unk0x125: u16,
    #[deku(skip, cond = "in_profile_summary")]
    unk0x127: u64,
}

// Gestures
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct Gestures {
    #[deku(count = "0x40")]
    ids: Vec<u32>,
}

// Regions
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct Regions {
    pub count: u32,
    #[deku(count = "*count")]
    pub ids: Vec<u32>,
}

// Ride Game Data
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct RideGameData {
    pub coordinates: FloatVector3,
    pub map_id: MapId,
    pub angle: FloatVector4,
    pub hp: i32,
    pub state: u32,
}

// BloodStain
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct BloodStain {
    pub coordinates: FloatVector3,
    pub angle: FloatVector4,
    unk0x1c: u32,
    unk0x20: u32,
    unk0x24: u32,
    unk0x28: u32,
    unk0x2c: u32,
    unk0x30: i32,
    pub runes: i32,
    pub map_id: MapId,
    unk0x3c: u32,
    unk0x38: u32,
}

// Menu Save Load
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct MenuSaveLoad {
    unk0x0: u16,
    unk0x2: u16,
    pub size: u32,
    #[deku(count = "*size")]
    pub data: Vec<u8>,
}

// Trophy Equip Data
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct TrophyEquipData {
    unk0x0: u32,
    unk0x4: [u8; 0x10],
    unk0x14: [u8; 0x10],
    unk0x24: [u8; 0x10],
}

// Gaitem Data
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct GaitemGameDataEntry {
    pub id: u32,
    #[deku(pad_bytes_after = "3")]
    unk0x4: u8,
    pub next_item_id: u32,
    #[deku(pad_bytes_after = "3")]
    unk0xc: u8,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct GaitemGameData {
    pub count: i64,
    #[deku(count = "7000")]
    pub entries: Vec<GaitemGameDataEntry>,
}

// Tutorial Data
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian, total_count: u32")]
pub struct TutorialDataChunk {
    pub count: u32,
    #[deku(skip, cond = "*count == 0", count = "(total_count-0x4)/4")]
    pub ids: Vec<u32>,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct TutorialData {
    unk0x0: u16,
    unk0x2: u16,
    pub size: u32,
    #[deku(ctx = "*size")]
    pub data: TutorialDataChunk,
}

// Field Area
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct FieldArea {
    pub size: i32,
    #[deku(bytes_read = "size")]
    pub data: Option<Vec<u32>>,
}

// World Area
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct WorldBlockChrData {
    magic: [u8; 4],
    pub map_id: MapId,
    pub size: i32,
    unk0xc: u32,
    #[deku(skip, cond = "*size < 1", count = "*size - 0x10")]
    pub data: Vec<u8>,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct WorldAreaChrData {
    magic: [u8; 4],
    #[deku(assert = "*unk_0x21042700 == 0x21042700 || *unk_0x21042700 == 0")]
    unk_0x21042700: u32,
    unk0x8: u32,
    unk0xc: u32,
    #[deku(until = "|d: &WorldBlockChrData| d.size < 1")]
    pub data: Vec<WorldBlockChrData>,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct WorldArea {
    pub size: i32,
    pub data: WorldAreaChrData,
}

// World Geom Man
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct WorldGeomDataChunk {
    map_id: MapId,
    pub size: i32,
    unk_0x8: u64,
    #[deku(skip, cond = "*size < 1", count = "*size-0x10")]
    pub data: Vec<u8>,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct WorldGeomData {
    magic: [u8; 4],
    unk_0x4: u32,
    #[deku(until = "|d: &WorldGeomDataChunk| d.size < 1")]
    pub data: Vec<WorldGeomDataChunk>,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct WorldGeomMan {
    pub size: i32,
    pub data: WorldGeomData,
}

// RendMan
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian, size: i32")]
pub struct StageManEntry {
    #[deku(skip, cond = "size < 1", count = "size")]
    pub data: Vec<u8>,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian, size: i32")]
pub struct StageMan {
    count: i32,
    #[deku(skip, cond = "*count < 1", count = "*count", ctx = "(size-4)/(*count)")]
    pub data: Vec<StageManEntry>,
}
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct RendMan {
    pub size: i32,
    #[deku(ctx = "*size")]
    pub data: StageMan,
}

// Player Coordinates
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct PlayerCoordinates {
    pub coordinates: FloatVector3,
    pub map_id: MapId,
    pub angle: FloatVector4,
    #[deku(assert = "*game_man_0xbf0 == 0 || *game_man_0xbf0 == 1")]
    game_man_0xbf0: u8,
    pub unk_coordinates: FloatVector3,
    pub unk_angle: FloatVector4,
}

// NetMan
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct NetMan {
    #[deku(assert = "*unk0x0 == 2 || *unk0x0 == 0")]
    unk0x0: u32,
    #[deku(count = "0x20000")]
    pub data: Vec<u8>,
}

// World Area Weather
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct WorldAreaWeather {
    pub area_id: u16,
    pub weather_type: u16,
    pub timer: u32,
    padding: u32,
}

// World Area Time
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct WorldAreaTime {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

// Base Version
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct BaseVersion {
    pub base_version_copy: u32,
    pub base_version: u32,
    #[deku(assert = "*is_latest_version == 0 || *is_latest_version == 1")]
    pub is_latest_version: u32,
    unk0xc: u32,
}

// PS5Activity
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct PS5Activity {
    data: [u8; 0x20],
}

// DLC
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct DLC {
    data: [u8; 0x32],
}

// Player Data Hash
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct PlayerGameDataHash {
    pub level: u32,
    pub stats: u32,
    pub archetype: u32,
    pub playergame_data_0xc0: u32,
    pub padding: u32,
    pub runes: u32,
    pub runes_memory: u32,
    pub equipped_weapons: u32,
    pub equipped_armors_and_talismans: u32,
    pub equipped_items: u32,
    pub equipped_spells: u32,

    #[deku(count = "0x54")]
    rest: Vec<u8>,
}

impl PlayerGameDataHash {
    pub fn write<W: std::io::Write>(
        writer: &mut deku::writer::Writer<W>,
        endian: Endian,
        player_game_data: &PlayerGameData,
        equipped_items_item_id: &EquippedItemsItemIds,
        equipped_armaments_and_items: &EquippedArmamentsAndItems,
        equipped_spells: &EquippedSpells,
    ) -> Result<(), DekuError> {
        let calculated_player_game_data_hash = Self::calculate_hash(
            player_game_data,
            equipped_items_item_id,
            equipped_armaments_and_items,
            equipped_spells,
        );
        calculated_player_game_data_hash.to_writer(writer, endian)?;
        Ok(())
    }

    fn calculate_hash(
        player_game_data: &PlayerGameData,
        equipped_items_item_id: &EquippedItemsItemIds,
        equipped_armaments_and_items: &EquippedArmamentsAndItems,
        equipped_spells: &EquippedSpells,
    ) -> PlayerGameDataHash {
        // Level hash
        let level = Self::byte_hash(&player_game_data.level.to_le_bytes());

        // Stats hash
        let mut stats = Vec::new();
        stats.extend(player_game_data.vigor.to_le_bytes());
        stats.extend(player_game_data.mind.to_le_bytes());
        stats.extend(player_game_data.endurance.to_le_bytes());
        stats.extend(player_game_data.strength.to_le_bytes());
        stats.extend(player_game_data.dexterity.to_le_bytes());
        stats.extend(player_game_data.intelligence.to_le_bytes());
        stats.extend(player_game_data.faith.to_le_bytes());
        stats.extend(player_game_data.arcane.to_le_bytes());
        stats.extend(player_game_data.unk0x54.to_le_bytes());
        let stats = Self::byte_hash(&stats);

        // Archetype hash
        let archetype = Self::byte_hash(&(player_game_data.archetype as u32).to_le_bytes());

        // player_game_dataGameData0xC0 hash
        let playergame_data_0xc0 = Self::byte_hash(&player_game_data.unk0xb8.to_le_bytes()[..1]);

        // Runes hash
        let runes = Self::byte_hash(&player_game_data.runes.to_le_bytes());

        // Runes memory hash
        let runes_memory = Self::byte_hash(&player_game_data.runes_memory.to_le_bytes());

        // Equipped weapons
        let mut equipped_weapon_ids = Vec::new();
        equipped_weapon_ids.extend((equipped_items_item_id.left_hand_armament1).to_le_bytes());
        equipped_weapon_ids.extend((equipped_items_item_id.left_hand_armament2).to_le_bytes());
        equipped_weapon_ids.extend((equipped_items_item_id.left_hand_armament3).to_le_bytes());
        equipped_weapon_ids.extend((equipped_items_item_id.right_hand_armament1).to_le_bytes());
        equipped_weapon_ids.extend((equipped_items_item_id.right_hand_armament2).to_le_bytes());
        equipped_weapon_ids.extend((equipped_items_item_id.right_hand_armament3).to_le_bytes());
        equipped_weapon_ids.extend((equipped_items_item_id.arrows1).to_le_bytes());
        equipped_weapon_ids.extend((equipped_items_item_id.arrows2).to_le_bytes());
        equipped_weapon_ids.extend((equipped_items_item_id.bolts1).to_le_bytes());
        equipped_weapon_ids.extend((equipped_items_item_id.bolts2).to_le_bytes());
        let equipped_weapons = Self::byte_hash(&equipped_weapon_ids);

        // Equipped armors and talismans
        let mut equipped_armors_and_talismans_ids = Vec::new();
        equipped_armors_and_talismans_ids.extend((equipped_items_item_id.head).to_le_bytes());
        equipped_armors_and_talismans_ids.extend((equipped_items_item_id.chest).to_le_bytes());
        equipped_armors_and_talismans_ids.extend((equipped_items_item_id.arms).to_le_bytes());
        equipped_armors_and_talismans_ids.extend((equipped_items_item_id.legs).to_le_bytes());
        equipped_armors_and_talismans_ids.extend((equipped_items_item_id.talisman1).to_le_bytes());
        equipped_armors_and_talismans_ids.extend((equipped_items_item_id.talisman2).to_le_bytes());
        equipped_armors_and_talismans_ids.extend((equipped_items_item_id.talisman3).to_le_bytes());
        equipped_armors_and_talismans_ids.extend((equipped_items_item_id.talisman4).to_le_bytes());
        equipped_armors_and_talismans_ids.extend((equipped_items_item_id.unk0x54).to_le_bytes());
        let equipped_armors_and_talismans = Self::byte_hash(&equipped_armors_and_talismans_ids);

        // Equipped Items
        let mut equipped_items_ids = Vec::new();
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.quickitem1 != 0xffffffff {
                equipped_armaments_and_items.quickitem1 & 0x0fffffff
            } else {
                equipped_armaments_and_items.quickitem1
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.quickitem2 != 0xffffffff {
                equipped_armaments_and_items.quickitem2 & 0x0fffffff
            } else {
                equipped_armaments_and_items.quickitem2
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.quickitem3 != 0xffffffff {
                equipped_armaments_and_items.quickitem3 & 0x0fffffff
            } else {
                equipped_armaments_and_items.quickitem3
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.quickitem4 != 0xffffffff {
                equipped_armaments_and_items.quickitem4 & 0x0fffffff
            } else {
                equipped_armaments_and_items.quickitem4
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.quickitem5 != 0xffffffff {
                equipped_armaments_and_items.quickitem5 & 0x0fffffff
            } else {
                equipped_armaments_and_items.quickitem5
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.quickitem6 != 0xffffffff {
                equipped_armaments_and_items.quickitem6 & 0x0fffffff
            } else {
                equipped_armaments_and_items.quickitem6
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.quickitem7 != 0xffffffff {
                equipped_armaments_and_items.quickitem7 & 0x0fffffff
            } else {
                equipped_armaments_and_items.quickitem7
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.quickitem8 != 0xffffffff {
                equipped_armaments_and_items.quickitem8 & 0x0fffffff
            } else {
                equipped_armaments_and_items.quickitem8
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.quickitem9 != 0xffffffff {
                equipped_armaments_and_items.quickitem9 & 0x0fffffff
            } else {
                equipped_armaments_and_items.quickitem9
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.quickitem10 != 0xffffffff {
                equipped_armaments_and_items.quickitem10 & 0x0fffffff
            } else {
                equipped_armaments_and_items.quickitem10
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.pouch1 != 0xffffffff {
                equipped_armaments_and_items.pouch1 & 0x0fffffff
            } else {
                equipped_armaments_and_items.pouch1
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.pouch2 != 0xffffffff {
                equipped_armaments_and_items.pouch2 & 0x0fffffff
            } else {
                equipped_armaments_and_items.pouch2
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.pouch3 != 0xffffffff {
                equipped_armaments_and_items.pouch3 & 0x0fffffff
            } else {
                equipped_armaments_and_items.pouch3
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.pouch4 != 0xffffffff {
                equipped_armaments_and_items.pouch4 & 0x0fffffff
            } else {
                equipped_armaments_and_items.pouch4
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.pouch5 != 0xffffffff {
                equipped_armaments_and_items.pouch5 & 0x0fffffff
            } else {
                equipped_armaments_and_items.pouch5
            })
            .to_le_bytes(),
        );
        equipped_items_ids.extend(
            (if equipped_armaments_and_items.pouch6 != 0xffffffff {
                equipped_armaments_and_items.pouch6 & 0x0fffffff
            } else {
                equipped_armaments_and_items.pouch6
            })
            .to_le_bytes(),
        );
        let equipped_items = Self::byte_hash(&equipped_items_ids);

        let mut equipped_spell_ids = Vec::new();
        for i in 0..14 {
            equipped_spell_ids.extend(equipped_spells.spellslot[i].spell_id.to_le_bytes());
        }
        let equipped_spells = Self::byte_hash(&equipped_spell_ids);

        PlayerGameDataHash {
            level,
            stats,
            archetype,
            playergame_data_0xc0,
            padding: 0,
            runes,
            runes_memory,
            equipped_weapons,
            equipped_armors_and_talismans,
            equipped_items,
            equipped_spells,
            rest: vec![0; 0x54],
        }
    }

    fn byte_hash(bytes: &[u8]) -> u32 {
        let mut lo: u32 = 1;
        let mut hi: u32 = 0;

        for byte in bytes {
            lo = lo + *byte as u32;
            hi = hi + lo;
        }

        let lo_hashed = Self::compute_hashed_value(lo);
        let hi_hashed = Self::compute_hashed_value(hi);

        (lo_hashed | (hi_hashed << 0x10)).wrapping_mul(2)
    }

    fn compute_hashed_value(input: u32) -> u32 {
        const MULTIPLIER: u32 = 0x80078071;
        const MOD_CONSTANT: i32 = -0xfff1;
        const SHIFT_AMOUNT: u32 = 15;
        let product = (MULTIPLIER as u64).wrapping_mul(input as u64);
        let upper_bits = (product >> 32) as u32;
        let shifted_upper_bits = upper_bits >> SHIFT_AMOUNT;
        let mod_product = (shifted_upper_bits as i32).wrapping_mul(MOD_CONSTANT) as u32;
        input.wrapping_add(mod_product)
    }
}
