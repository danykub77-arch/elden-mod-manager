use std::io::Cursor;

use deku::ctx::Endian;
use deku::prelude::*;
use deku::{DekuRead, DekuWrite};

use super::user_data_x::{
    ActiveWeaponSlotsAndArmStyle, EquippedItemsItemIds, EquppedItemsGaitemHandles, FaceData,
};
use super::util::{MapId, Util};

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: Endian, start: usize, size: usize, is_ps: bool"
)]
pub struct UserData10 {
    // Checksum (PC only)
    #[deku(skip, cond = "is_ps", count = "0x10")]
    checksum: Vec<u8>,

    // File version
    pub version: u32,

    // SteamId
    pub steam_id: u64,

    // Settings
    pub settings: Settings,

    // #[deku(skip, cond = "true", default = "deku::byte_offset")]
    // pub byte_offset: usize,

    // Menu System Save Load
    pub menu_system_save_load: MenuSystemSaveLoad,

    // Profile Summary
    pub profile_summary: ProfileSummary,

    gamedataman0xd0: u32,
    gamedataman0x75: u8,

    // PCOptionData (PC ONLY)
    #[deku(skip, cond = "is_ps")]
    pub pc_option_data: PCOptionData,

    // Key Config Save Load
    pub key_config_save_load: KeyConfigSaveLoad,

    game_man_0x118: u64,

    // Empty calories
    #[deku(count = "size - (deku::byte_offset - start)")]
    pub rest: Vec<u8>,
}

// Settings
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct Settings {
    pub camera_speed: u8,
    pub controller_vibration: u8,
    pub brightness: u8,
    pub unk0x3: u8,
    pub music_volume: u8,
    pub sound_effects_volume: u8,
    pub voice_volume: u8,
    pub display_blood: u8,
    pub subtitles: u8,
    pub hud: u8,
    pub camera_x_axis: u8,
    pub camera_y_axis: u8,
    pub toggle_auto_lockon: u8,
    pub camera_auto_wall_recovery: u8,
    pub unk0xe: u8,
    pub unk0xf: u8,
    pub reset_camera_y_axis: u8,
    pub cinematic_effects: u8,
    pub unk0x12: u8,
    pub perform_matchmaking: u8,
    pub unk0x14: u8,
    pub unk0x15: u8,
    pub manual_attack_aim: u8,
    pub autotarget: u8,
    pub launchsettings: u8,
    pub send_summon_sign: u8,
    pub unk0x1a: u8,
    pub hdr: u8,
    pub hdr_adjust_brightness: u8,
    pub hdr_maximum_brightness: u8,
    pub hdr_adjust_saturation: u8,
    pub unk0x1f: u8,
    pub master_volume: u8,
    pub is_raytracing_on: u8,
    pub mark_new_items: u8,
    pub show_recent_tabs: u8,
    #[deku(assert_eq = "0")]
    unk0x24: u64,
    #[deku(assert_eq = "0")]
    unk0x2c: u16,
    pub show_tutorials: u8,
    pub camera_auto_rotation: u8,
    #[deku(count = "0x110")]
    pub justzero: Vec<u8>,
}

impl UserData10 {
    pub fn read<R: std::io::Read>(
        reader: &mut deku::reader::Reader<R>,
        endian: Endian,
        start: usize,
        size: usize,
        is_ps: bool,
    ) -> Result<Self, DekuError> {
        let user_data_10 = Self::from_reader_with_ctx(reader, (endian, start, size, is_ps))?;
        Ok(user_data_10)
    }

    pub fn write<W: std::io::Write>(
        writer: &mut deku::writer::Writer<W>,
        endian: Endian,
        start: usize,
        size: usize,
        is_ps: bool,
        user_data_10: &Self,
    ) -> Result<(), DekuError> {
        if is_ps {
            user_data_10.to_writer(writer, (endian, start, size, is_ps))?;
            return Ok(());
        }

        let mut buffer = Vec::new();
        {
            let mut temp_writer = Writer::new(Cursor::new(&mut buffer));
            user_data_10.to_writer(&mut temp_writer, (endian, start, size, is_ps))?;
        }

        Util::update_checksum(&mut buffer);

        writer.write_bytes(&buffer)?;
        Ok(())
    }
}

// Menu System Save Load
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct MenuSystemSaveLoad {
    unk0x0: u16,
    unk0x2: u16,
    pub size: u32,
    #[deku(count = "size")]
    pub data: Vec<u8>,
}

// Profile Summary
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct ProfileSummary {
    pub active_profiles: [bool; 10],
    #[deku(count = "10")]
    pub profiles: Vec<Profile>,
}
// Profile
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct Profile {
    #[deku(
        reader = "Util::read_wstring(deku::reader, 32)",
        writer = "Util::write_wstring(deku::writer, &character_name, 32)"
    )]
    pub character_name: String,
    character_name_terminator: u16,
    pub level: u32,
    pub seconds_played: u32,
    pub runes_memory: u32,
    pub map_id: MapId,
    pub unk0x34: u32,
    #[deku(ctx = "true")]
    pub face_data: FaceData,
    // #[deku(skip, cond = "true", default = "deku::byte_offset")]
    // pub byte_offset: usize,
    pub equipment: ProfileEquipment,
    pub gender: u8,
    pub archetype: u8,
    pub starting_gift: u8,
    profile_summary_character_0x293: u8,
    profile_summary_character_0x294: u8,
    profile_summary_character_0x295: u8,
    profile_summary_character_0x298: u32,
}

// Profile Equipment
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Clone)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct ProfileEquipment {
    unk0x0: u64,
    pub active_weapon_slots_and_arm_style: ActiveWeaponSlotsAndArmStyle,
    pub equipped_items_gaitem_handle: EquppedItemsGaitemHandles,
    pub equipped_items_item_id: EquippedItemsItemIds,
    unk0xd4: i32,
    unk0xd8: u32,
    unk0xdc: [u32; 3],
}

// PCOptionData
#[derive(PartialEq, Debug, DekuRead, DekuWrite, Default)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct PCOptionData {
    unk0x0: u32,
    unk0xc: u8,
    unk0xd: u8,
    unk0xe: u8,
    unk0xf: u8,
    unk0x10: u64,
    unk0x18: u16,
    #[deku(count = "0xa0/2")]
    unk0x12: Vec<u16>,
}

// KeyConfigSaveLoad
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: Endian")]
pub struct KeyConfigSaveLoad {
    unk0x0: u16,
    unk0x2: u16,
    pub size: u32,
    #[deku(count = "*size")]
    pub data: Vec<u8>,
}
