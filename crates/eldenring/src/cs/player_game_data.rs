use std::ops::{Index, IndexMut};
use std::ptr::NonNull;

use bitfield::bitfield;
use thiserror::Error;

use crate::dlkr::MainHeapAllocator;
use crate::{
    ArrayWithHeader, DLList, DLVector,
    cs::{
        ChrType, EquipParamGem, EquipParamGoods, EquipParamWeapon, MultiplayRole,
        QuickmatchDesiredTeam,
    },
    from_net::FNVector,
};
use shared::{
    FromStatic, IsEmpty, MaybeEmpty, NonEmptyIteratorExt, NonEmptyIteratorMutExt, OwnedPtr,
};

use crate::cs::{
    CSGaitemImp, FaceData, FieldInsHandle, GaitemHandle, ItemCategory, ItemId, OptionalItemId,
    SoloParamRepository,
};

#[repr(C)]
/// Source of name: RTTI
pub struct PlayerGameData {
    vftable: usize,
    /// Event id of this game data owner
    pub character_event_id: i32,
    pub player_id: u32,
    pub current_hp: u32,
    pub current_max_hp: u32,
    pub base_max_hp: u32,
    pub current_fp: u32,
    pub current_max_fp: u32,
    pub base_max_fp: u32,
    unk28: f32,
    pub current_stamina: u32,
    pub current_max_stamina: u32,
    pub base_max_stamina: u32,
    unk38: f32,
    pub vigor: u32,
    pub mind: u32,
    pub endurance: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub faith: u32,
    pub arcane: u32,
    pub base_hero_point: f32,
    pub base_hero_point_2: f32,
    pub base_durability: f32,
    pub level: u32,
    pub rune_count: u32,
    pub rune_memory: u32,
    unk74: u32,
    pub poison_resist: u32,
    pub rot_resist: u32,
    pub bleed_resist: u32,
    pub death_resist: u32,
    pub frost_resist: u32,
    pub sleep_resist: u32,
    pub madness_resist: u32,
    pub pending_block_clear_bonus: f32,
    pub chr_type: ChrType,
    pub character_name: [u16; 17],
    pub gender: u8,
    pub archetype: u8,
    pub vow_type: u8,
    unkc1: u8,
    pub voice_type: u8,
    pub starting_gift: u8,
    unkc4: u8,
    pub unlocked_magic_slots: u8,
    pub unlocked_talisman_slots: u8,
    pub matchmaking_spirit_ashes_level: u8,
    pub total_summon_count: u32,
    pub coop_success_count: u32,
    /// Index into [crate::cs::GameDataMan]'s player game data array
    pub game_data_man_index: u32,
    unkd4: [u8; 0xb],
    pub furlcalling_finger_remedy_active: bool,
    unke0: u8,
    unke1: u8,
    pub matching_weapon_level: u8,
    pub white_ring_active: u8,
    pub blue_ring_active: u8,
    /// [MultiplayRole] of the player this game data belongs to
    pub multiplay_role: MultiplayRole,
    unke6: u8,
    /// True if the player is in their own world.
    pub is_my_world: bool,
    unke8: [u8; 0x3],
    unke9: bool,
    pub character_id: u32,
    pub invasions_success_count: u32,
    pub solo_breakin_point: u32,
    pub invaders_killed: u32,
    pub scadutree_blessing: u8,
    pub reversed_spirit_ash: u8,
    pub resist_curse_item_count: u8,
    pub rune_arc_active: bool,
    unk100: bool,
    pub max_hp_flask: u8,
    pub max_fp_flask: u8,
    unk103: [u8; 0x4],
    pub sell_region: SellRegion,
    unk108: u8,
    pub reached_max_rune_memory: u8,
    unk10a: [u8; 0xE],
    pub password: [u16; 0x8],
    unk128: u16,
    group_password_1: [u16; 0x8],
    unk13a: u16,
    group_password_2: [u16; 0x8],
    unk14c: u16,
    group_password_3: [u16; 0x8],
    unk15e: u16,
    group_password_4: [u16; 0x8],
    unk170: u16,
    group_password_5: [u16; 0x8],
    unk182: [u8; 0x36],
    pub sp_effects: [PlayerGameDataSpEffect; 0xD],
    /// Level after any buffs and corrections
    pub effective_vigor: u32,
    /// Level after any buffs and corrections
    pub effective_mind: u32,
    /// Level after any buffs and corrections
    pub effective_endurance: u32,
    /// Level after any buffs and corrections
    pub effective_vitality: u32,
    /// Level after any buffs and corrections
    pub effective_strength: u32,
    /// Level after any buffs and corrections
    pub effective_dexterity: u32,
    /// Level after any buffs and corrections
    pub effective_intelligence: u32,
    /// Level after any buffs and corrections
    pub effective_faith: u32,
    /// Level after any buffs and corrections
    pub effective_arcane: u32,
    unk2ac: u32,
    pub equipment: EquipGameData,
    pub face_data: FaceData,
    /// Describes the storage box contents.
    pub storage: Option<OwnedPtr<EquipInventoryData, MainHeapAllocator>>,
    gesture_game_data: usize,
    ride_game_data: usize,
    unk8e8: usize,
    /// True when this game data belongs to the main (local) player.
    pub is_main_player: bool,
    /// Did this player agreed to voice chat?
    pub is_voice_chat_enabled: bool,
    unk8f2: [u8; 6],
    unk8f8: usize,
    unk900: [u8; 36],
    pub hp_estus_rate: f32,
    pub hp_estus_additional: u8,
    _pad929: [u8; 3],
    pub fp_estus_rate: f32,
    pub fp_estus_additional: u8,
    _pad931: [u8; 3],
    unk934: u32,
    /// Vector of all visited play area IDs
    pub visited_areas: FNVector<u32>,
    pub mount_handle: FieldInsHandle,
    unk958: FieldInsHandle,
    unk560: u8,
    pub damage_negation_physical: i32,
    pub attack_rating: PlayerDataAttackRating,
    pub damage_negation_magic: i32,
    unk984: f32,
    unk988: f32,
    pub max_equip_load: f32,
    unk990: u32,
    pub damage_negation_strike: i32,
    pub damage_negation_slash: i32,
    pub damage_negation_pierce: i32,
    pub damage_negation_fire: i32,
    pub damage_negation_lightning: i32,
    pub damage_negation_holy: i32,
    unused_defence_status: [f32; 8],
    pub resistance_gauges: [u32; 7],
    pub resistance_gauge_max: [u32; 7],
    unused_gauge_list: [f32; 7],
    pub proc_status_timers: [f32; 7],
    pub proc_status_timer_max: [f32; 7],
    unka58: u32,
    pub frontend_flags: PlayerGameDataFrontendFlags,
    pub sa_toughness_total: u32,
    unka64: u32,
    pub berserker_kills: u8,
    pub berserker_kills_target: u8,
    pub berserker_target_reached: bool,
    pub quickmatch_kill_count: u8,
    unka6c: [u8; 0x4],
    pub poise: f32,
    pub discovery: u32,
    pub effective_unlocked_magic_slots: u32,
    menu_ref_special_effect_1: usize,
    menu_ref_special_effect_2: usize,
    menu_ref_special_effect_3: usize,
    pub is_using_festering_bloody_finger: bool,
    pub used_invasion_item_type: PlayerDataInvasionItemType,
    unka9a: [u8; 2],
    pub packed_time_stamp: u32,
    /// team type for quickmatch, WhiteSign (2) for allies.
    pub quick_match_team: u8,
    /// [0,1) range used to sample spawn locations in quickmatch.
    pub quick_match_spawn_slot_fraction: f32,
    pub quick_match_duel_points: u16,
    pub quick_match_united_combat_points: u16,
    pub quick_match_spirit_ashes_points: u16,
    pub quickmatch_duel_rank: u8,
    pub quickmatch_united_combat_rank: u8,
    pub quickmatch_spirit_ashes_rank: u8,
    /// Whether this player is the lead player in quickmatch (player with the most points).
    pub is_quick_match_lead: bool,
    /// Whether this player is in [`HostInGame`] or [`GuestInGame`] state in quickmatch.
    ///
    /// [`HostInGame`]: crate::cs::CSQuickMatchingCtrlState::HostInGame
    /// [`GuestInGame`]: crate::cs::CSQuickMatchingCtrlState::GuestInGame
    pub is_quick_match_in_game: bool,
    pub is_quick_match_host: bool,
    pub quick_match_map_load_ready: bool,
    pub quick_match_desired_team: QuickmatchDesiredTeam,
    unkab6: u8,
    /// Should sign cooldown be enabled?
    /// Each time your coop player dies and you have someone in your world
    /// you will get a cooldown depending on [crate::param::WHITE_SIGN_COOL_TIME_PARAM_ST] and level from [crate::cs::SosSignMan::white_sign_cool_time_param_id]
    pub sign_cooldown_enabled: bool,
    unkab8: [u8; 0x2],
    pub has_preorder_gesture: bool,
    pub has_preorder_sote_gesture: bool,
    pub host_scadutree_blessing: u8,
    pub host_scaling_applied: bool,
    unkabe: [u8; 0x32],
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SellRegion {
    None = 0,
    Japan = 1,
    NorthAmerica = 2,
    Europe = 3,
    Asia = 4,
    Global = 5,
}

bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq, Hash)]
    pub struct PlayerGameDataFrontendFlags(u8);
    impl Debug;

    bool;
    pub disable_status_effect_bars, set_disable_status_effect_bars: 0;
    pub rune_arc_active, set_rune_arc_active: 1;
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerDataInvasionItemType {
    BloodyFinger = 0,
    FesteringBloodyFinger = 1,
    RecusantFinger = 2,
}

#[repr(C)]
pub struct PlayerDataAttackRating {
    pub left_armament_primary: i32,
    pub right_armament_primary: i32,
    pub left_armament_secondary: i32,
    pub right_armament_secondary: i32,
    pub left_armament_tertiary: i32,
    pub right_armament_tertiary: i32,
}

#[repr(C)]
pub struct PlayerGameDataSpEffect {
    pub sp_effect_id: u32,
    pub duration: f32,
    unk8: u32,
    unkc: u32,
}

#[repr(C)]
pub struct ItemReplenishStateEntry {
    pub item_id: OptionalItemId,
    pub auto_replenish: bool,
}

#[repr(C)]
pub struct ItemReplenishStateEntryUnk {
    pub item_id: OptionalItemId,
    pub auto_replenish: bool,
}

#[repr(C)]
/// Tracks the state of item replenishment from the chest when you sit at a Site of Grace
pub struct ItemReplenishStateTracker {
    entries: [ItemReplenishStateEntry; 2048],
    unk4000: u32,
    unk4004: u32,
    pub count: u64,
    unk4010: [ItemReplenishStateEntryUnk; 256],
}

impl ItemReplenishStateTracker {
    pub fn entries(&self) -> &[ItemReplenishStateEntry] {
        &self.entries[..self.count as usize]
    }

    pub fn entries_mut(&mut self) -> &mut [ItemReplenishStateEntry] {
        &mut self.entries[..self.count as usize]
    }
}

#[repr(C)]
pub struct QMItemBackupVectorItem {
    pub item_id: OptionalItemId,
    pub quantity: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ChrAsmEquipEntries {
    pub weapon_primary_left: ItemId,
    pub weapon_primary_right: ItemId,
    pub weapon_secondary_left: ItemId,
    pub weapon_secondary_right: ItemId,
    pub weapon_tertiary_left: ItemId,
    pub weapon_tertiary_right: ItemId,
    pub arrow_primary: OptionalItemId,
    pub bolt_primary: OptionalItemId,
    pub arrow_secondary: OptionalItemId,
    pub bolt_secondary: OptionalItemId,
    pub arrow_tertiary: OptionalItemId,
    pub bolt_tertiary: OptionalItemId,
    pub protector_head: ItemId,
    pub protector_chest: ItemId,
    pub protector_hands: ItemId,
    pub protector_legs: ItemId,
    pub unused40: OptionalItemId,
    pub accessories: [OptionalItemId; 4],
    pub covenant: OptionalItemId,
    pub quick_tems: [OptionalItemId; 10],
    pub pouch: [OptionalItemId; 6],
    unk98: u32,
}

impl Index<ChrAsmSlot> for ChrAsmEquipEntries {
    type Output = OptionalItemId;
    fn index(&self, index: ChrAsmSlot) -> &OptionalItemId {
        match index {
            ChrAsmSlot::WeaponLeft1 => self.weapon_primary_left.as_optional(),
            ChrAsmSlot::WeaponRight1 => self.weapon_primary_right.as_optional(),
            ChrAsmSlot::WeaponLeft2 => self.weapon_secondary_left.as_optional(),
            ChrAsmSlot::WeaponRight2 => self.weapon_secondary_right.as_optional(),
            ChrAsmSlot::WeaponLeft3 => self.weapon_tertiary_left.as_optional(),
            ChrAsmSlot::WeaponRight3 => self.weapon_tertiary_right.as_optional(),
            ChrAsmSlot::Arrow1 => &self.arrow_primary,
            ChrAsmSlot::Bolt1 => &self.bolt_primary,
            ChrAsmSlot::Arrow2 => &self.arrow_secondary,
            ChrAsmSlot::Bolt2 => &self.bolt_secondary,
            ChrAsmSlot::Arrow3 => &self.arrow_tertiary,
            ChrAsmSlot::Bolt3 => &self.bolt_tertiary,
            ChrAsmSlot::ProtectorHead => self.protector_head.as_optional(),
            ChrAsmSlot::ProtectorChest => self.protector_chest.as_optional(),
            ChrAsmSlot::ProtectorHands => self.protector_hands.as_optional(),
            ChrAsmSlot::ProtectorLegs => self.protector_legs.as_optional(),
            ChrAsmSlot::Unused16 => &self.unused40,
            ChrAsmSlot::Accessory1 => &self.accessories[0],
            ChrAsmSlot::Accessory2 => &self.accessories[1],
            ChrAsmSlot::Accessory3 => &self.accessories[2],
            ChrAsmSlot::Accessory4 => &self.accessories[3],
            ChrAsmSlot::AccessoryCovenant => &self.covenant,
        }
    }
}

impl IndexMut<ChrAsmSlot> for ChrAsmEquipEntries {
    fn index_mut(&mut self, index: ChrAsmSlot) -> &mut OptionalItemId {
        match index {
            ChrAsmSlot::WeaponLeft1 => self.weapon_primary_left.as_optional_mut(),
            ChrAsmSlot::WeaponRight1 => self.weapon_primary_right.as_optional_mut(),
            ChrAsmSlot::WeaponLeft2 => self.weapon_secondary_left.as_optional_mut(),
            ChrAsmSlot::WeaponRight2 => self.weapon_secondary_right.as_optional_mut(),
            ChrAsmSlot::WeaponLeft3 => self.weapon_tertiary_left.as_optional_mut(),
            ChrAsmSlot::WeaponRight3 => self.weapon_tertiary_right.as_optional_mut(),
            ChrAsmSlot::Arrow1 => &mut self.arrow_primary,
            ChrAsmSlot::Bolt1 => &mut self.bolt_primary,
            ChrAsmSlot::Arrow2 => &mut self.arrow_secondary,
            ChrAsmSlot::Bolt2 => &mut self.bolt_secondary,
            ChrAsmSlot::Arrow3 => &mut self.arrow_tertiary,
            ChrAsmSlot::Bolt3 => &mut self.bolt_tertiary,
            ChrAsmSlot::ProtectorHead => self.protector_head.as_optional_mut(),
            ChrAsmSlot::ProtectorChest => self.protector_chest.as_optional_mut(),
            ChrAsmSlot::ProtectorHands => self.protector_hands.as_optional_mut(),
            ChrAsmSlot::ProtectorLegs => self.protector_legs.as_optional_mut(),
            ChrAsmSlot::Unused16 => &mut self.unused40,
            ChrAsmSlot::Accessory1 => &mut self.accessories[0],
            ChrAsmSlot::Accessory2 => &mut self.accessories[1],
            ChrAsmSlot::Accessory3 => &mut self.accessories[2],
            ChrAsmSlot::Accessory4 => &mut self.accessories[3],
            ChrAsmSlot::AccessoryCovenant => &mut self.covenant,
        }
    }
}

#[repr(C)]
pub struct EquipGameData {
    vftable: usize,
    pub equipment_item_idx_list: [u32; 22],
    unk60: usize,
    unk68: u32,
    pub chr_asm: ChrAsm,
    pub equip_inventory_data: EquipInventoryData,
    pub equip_magic_data: OwnedPtr<EquipMagicData, MainHeapAllocator>,
    pub equip_item_data: EquipItemData,
    equip_gesture_data: usize,
    /// Tracker for the item replenishing from the chest
    pub item_replenish_state_tracker:
        Option<OwnedPtr<ItemReplenishStateTracker, MainHeapAllocator>>,
    pub qm_item_backup_vector:
        Option<OwnedPtr<DLVector<QMItemBackupVectorItem>, MainHeapAllocator>>,
    pub equipment_entries: ChrAsmEquipEntries,
    pub physick_tears: [OptionalItemId; 2],
    pub extra_physick_tear: OptionalItemId,
    pub player_game_data: NonNull<PlayerGameData>,
    /// Whether this equipment data belongs to the main (local) player.
    pub is_main_player: bool,
    /// Result of the last attempt to add an item to the inventory
    pub last_add_item_result: LastAddItemResult,
    /// Bitfield tracking which equipment slots have fully broken equipment
    /// Used to sync visuals of broken equipment in multiplayer
    /// (DS3 leftover)
    pub broken_equipment_slots: BrokenEquipmentSlots,
    unk404: [u8; 0xac],
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LastAddItemResult {
    Success = 0,
    UniqueItemDuplicate = 2,
    InventoryFull = 4,
}

bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq, Hash)]
    /// Flags indicating that certain equipment slots are fully broken
    /// (DS3 leftover)
    pub struct BrokenEquipmentSlots(u32);
    impl Debug;

    bool;
    pub weapon_left1, set_weapon_left1: 0;
    pub weapon_right1, set_weapon_right1: 1;
    pub weapon_left2, set_weapon_left2: 2;
    pub weapon_right2, set_weapon_right2: 3;
    pub weapon_left3, set_weapon_left3: 4;
    pub weapon_right3, set_weapon_right3: 5;
    pub arrow1, set_arrow1: 6;
    pub bolt1, set_bolt1: 7;
    pub arrow2, set_arrow2: 8;
    pub bolt2, set_bolt2: 9;
    pub arrow3, set_arrow3: 10;
    pub bolt3, set_bolt3: 11;
    pub protector_head, set_protector_head: 12;
    pub protector_chest, set_protector_chest: 13;
    pub protector_hands, set_protector_hands: 14;
    pub protector_legs, set_protector_legs: 15;
    pub unused16, set_unused16: 16;
    pub accessory1, set_accessory1: 17;
    pub accessory2, set_accessory2: 18;
    pub accessory3, set_accessory3: 19;
    pub accessory4, set_accessory4: 20;
    pub accessory_covenant, set_accessory_covenant: 21;
}

#[repr(C)]
pub struct InventoryItemListAccessor {
    pub head: NonNull<MaybeEmpty<EquipInventoryDataListEntry>>,
    pub length: NonNull<u32>,
}

#[repr(C)]
pub struct InventoryItemsData {
    /// How many items can one hold in total?
    pub global_capacity: u32,

    /// The maximum capacity of the normal items inventory.
    pub normal_items_capacity: u32,

    /// A pointer to the head of the normal items inventory.
    pub normal_items_head: OwnedPtr<MaybeEmpty<EquipInventoryDataListEntry>, MainHeapAllocator>,

    /// How many normal-item entries are occupied.
    ///
    /// A count of live entries, not an extent: slots are sparse, and this
    /// tracks occupancy across the gaps rather than the highest slot in use
    /// (that's [`EquipInventoryData::highest_item_slot`]). `InsertNormalItem`
    /// increments it whenever it fills an empty slot — including one *below*
    /// the current value — and `RemoveItemEntryBySlot` decrements it for any
    /// slot it clears.
    ///
    /// Nothing in the game iterates by this. Reads index the array directly
    /// by global slot and null-check the entry
    /// (`GetInventoryItemEntryByIndex`), so it's bookkeeping to keep
    /// consistent rather than a bound to respect.
    pub normal_items_len: u32,

    /// The maximum capacity of the key items inventory.
    pub key_items_capacity: u32,

    /// A pointer to the head of the key items inventory.
    pub key_items_head: OwnedPtr<MaybeEmpty<EquipInventoryDataListEntry>, MainHeapAllocator>,

    /// How many key-item entries are occupied. Same semantics as
    /// [`normal_items_len`](Self::normal_items_len) — a count of live
    /// entries across sparse slots, not an extent, and not used as an
    /// iteration bound by the game.
    pub key_items_len: u32,

    /// The maximum capacity of the multiplayer key items inventory.
    pub multiplay_key_items_capacity: u32,

    /// Holds key items that are available in multiplayer.
    ///
    /// Populated by `SwapKeyItemsAccessor` when entering multiplayer: it
    /// walks `key_items` and copies across every `Goods` entry whose
    /// `goodsType` is `GREAT_RUNE`, `REGENERATIVE_MATERIAL` or
    /// `WONDROUS_PHYSICK_TEAR` (great runes, pots and wondrous physick
    /// tears), preserving each one's quantity and `sort_id`.
    pub multiplay_key_items_head:
        OwnedPtr<MaybeEmpty<EquipInventoryDataListEntry>, MainHeapAllocator>,

    /// How many multiplayer key-item entries are occupied. Same semantics as
    /// [`normal_items_len`](Self::normal_items_len).
    pub multiplay_key_items_len: u32,

    /// Pointers to the active normal item list and its length. All inventory
    /// reads and writes in the game go through this.
    ///
    /// Unlike `key_items_accessor`, this is always the same as `normal_items`.
    pub normal_items_accessor: InventoryItemListAccessor,

    /// Pointers to the active key item list and its length. All inventory reads
    /// and writes in the game go through this.
    ///
    /// In single-player, this typically points to `key_items`. In multiplayer,
    /// it switches to `multiplay_key_items`.
    pub key_items_accessor: InventoryItemListAccessor,

    /// Contains the indices into the item ID mapping list.
    pub item_id_mapping_indices: OwnedPtr<[i16; 2017], MainHeapAllocator>,
    pub item_id_mapping_pool_len: u32,
    /// Contains table of item IDs and their corresponding location in the equip inventory data
    /// lists.
    pub item_id_mapping: OwnedPtr<ArrayWithHeader<ItemIdMapping>, MainHeapAllocator>,
    /// Index of the latest `item_id_mapping` free head, or -1 if none
    pub item_id_mapping_free_head: i16,
}

impl InventoryItemsData {
    /// Returns an iterator over all the non-empty entries in the player's
    /// inventory.
    ///
    /// This iterates over key items first, followed by normal items.
    pub fn items(&self) -> impl Iterator<Item = &EquipInventoryDataListEntry> {
        self.current_key_entries()
            .iter()
            .chain(self.normal_entries().iter())
            .non_empty()
    }

    /// Returns an iterator over all the mutable non-empty entries in the
    /// player's inventory.
    ///
    /// This iterates over key items first, followed by normal items.
    pub fn items_mut(&self) -> impl Iterator<Item = &mut EquipInventoryDataListEntry> {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.key_items_accessor.head.as_ptr(),
                *self.key_items_accessor.length.as_ref() as usize,
            )
        }
        .iter_mut()
        .chain(
            unsafe {
                std::slice::from_raw_parts_mut(
                    self.normal_items_head.as_ptr(),
                    self.normal_items_capacity as usize,
                )
            }
            .iter_mut(),
        )
        .non_empty()
    }

    /// A slice over all the normal item [EquipInventoryDataListEntry] allocated
    /// for this [InventoryItemsData], whether or not they're empty or in range
    /// of [normal_items_len](Self::normal_items_len).
    pub fn normal_entries(&self) -> &[MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts(
                self.normal_items_head.as_ptr(),
                self.normal_items_capacity as usize,
            )
        }
    }

    /// A mutable slice over all the normal item [EquipInventoryDataListEntry]
    /// allocated for this [InventoryItemsData], whether or not they're empty or
    /// in range of [normal_items_len](Self::normal_items_len).
    pub fn normal_entries_mut(&mut self) -> &mut [MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.normal_items_head.as_ptr(),
                self.normal_items_capacity as usize,
            )
        }
    }

    /// Whether there's no more room left in the normal items inventory and
    /// picking up a new item will fail.
    pub fn is_normal_items_full(&self) -> bool {
        self.normal_items_len >= self.normal_items_capacity
            && self.normal_entries().iter().all(|e| !e.is_empty())
    }

    /// A slice over all the key item [EquipInventoryDataListEntry] allocated
    /// for this [InventoryItemsData], whether or not they're empty or in range
    /// of [key_items_len](Self::key_items_len).
    pub fn key_entries(&self) -> &[MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts(self.key_items_head.as_ptr(), self.key_items_len as usize)
        }
    }

    /// A mutable slice over all the key item [EquipInventoryDataListEntry]
    /// allocated for this [InventoryItemsData], whether or not they're empty or
    /// in range of [key_items_len](Self::key_items_len).
    pub fn key_entries_mut(&mut self) -> &mut [MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.key_items_head.as_ptr(),
                self.key_items_len as usize,
            )
        }
    }

    /// Whether there's no more room left in the key items inventory and picking
    /// up a new item will fail.
    pub fn is_key_items_full(&self) -> bool {
        self.key_items_len >= self.key_items_capacity
            && self.key_entries().iter().all(|e| !e.is_empty())
    }

    /// A slice over all the multiplayer key item [EquipInventoryDataListEntry]
    /// allocated for this [InventoryItemsData], whether or not they're empty or
    /// in range of [multiplay_key_items_len](Self::multiplay_key_items_len).
    pub fn multiplay_key_entries(&self) -> &[MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts(
                self.multiplay_key_items_head.as_ptr(),
                self.multiplay_key_items_len as usize,
            )
        }
    }

    /// A mutable slice over all the multiplayer key item
    /// [EquipInventoryDataListEntry] allocated for this [InventoryItemsData],
    /// whether or not they're empty or in range of
    /// [multiplay_key_items_len](Self::multiplay_key_items_len).
    pub fn multiplay_key_entries_mut(&mut self) -> &mut [MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.multiplay_key_items_head.as_ptr(),
                self.multiplay_key_items_len as usize,
            )
        }
    }

    /// Whether there's no more room left in the multiplayer items inventory and
    /// picking up a new item will fail.
    pub fn is_multiplay_key_items_full(&self) -> bool {
        self.multiplay_key_items_len >= self.multiplay_key_items_capacity
            && self.multiplay_key_entries().iter().all(|e| !e.is_empty())
    }

    /// A slice over all the key item [EquipInventoryDataListEntry] allocated
    /// for this [InventoryItemsData], whether or not they're empty or in range
    /// of the associated length field.
    ///
    /// This is equivalent to either [key_entries](Self::key_entries) and
    /// [multiplay_key_entries](Self::multiplay_key_entries), depending on
    /// whether the player is currently in a multiplayer session.
    pub fn current_key_entries(&self) -> &[MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts(
                self.key_items_accessor.head.as_ptr(),
                self.key_items_capacity as usize,
            )
        }
    }

    /// A mutable slice over all the key item [EquipInventoryDataListEntry]
    /// allocated for this [InventoryItemsData], whether or not they're empty or
    /// in range of the associated length field.
    ///
    /// This is equivalent to either [key_entries_mut](Self::key_entries_mut)
    /// and [multiplay_key_entries_mut](Self::multiplay_key_entries_mut),
    /// depending on whether the player is currently in a multiplayer session.
    pub fn current_key_entries_mut(&mut self) -> &mut [MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.key_items_accessor.head.as_ptr(),
                self.key_items_capacity as usize,
            )
        }
    }

    pub fn entry_at_slot(&self, slot: u32) -> Option<&MaybeEmpty<EquipInventoryDataListEntry>> {
        let key_cap = self.key_items_capacity;
        if slot < key_cap {
            Some(self.current_key_entries().get(slot as usize)?)
        } else {
            let offset = (slot - key_cap) as usize;
            Some(self.normal_entries().get(offset)?)
        }
    }

    pub fn entry_at_slot_mut(
        &mut self,
        slot: u32,
    ) -> Option<&mut MaybeEmpty<EquipInventoryDataListEntry>> {
        let key_cap = self.key_items_capacity;
        if slot < key_cap {
            Some(self.current_key_entries_mut().get_mut(slot as usize)?)
        } else {
            let offset = (slot - key_cap) as usize;
            Some(self.normal_entries_mut().get_mut(offset)?)
        }
    }

    pub fn get_entry(&self, item_id: ItemId) -> Option<&EquipInventoryDataListEntry> {
        let slot = self.find_item_idx(item_id)?;
        self.entry_at_slot(slot)?.as_option()
    }

    pub fn get_entry_mut(&mut self, item_id: ItemId) -> Option<&mut EquipInventoryDataListEntry> {
        let slot = self.find_item_idx(item_id)?;
        self.entry_at_slot_mut(slot)?.as_option_mut()
    }

    fn bucket_for(item_id: ItemId) -> usize {
        item_id.into_inner() as usize % 2017
    }

    fn mapping_entry_mut(&mut self, index: i16) -> Option<&mut ItemIdMapping> {
        unsafe { self.item_id_mapping.as_mut_slice().get_mut(index as usize) }
    }
    fn mapping_entry(&self, index: i16) -> Option<&ItemIdMapping> {
        unsafe { self.item_id_mapping.as_slice().get(index as usize) }
    }

    /// Pop one entry from the free list. Returns `None` if pool is full
    ///
    /// The popped entry's chain field holds the *next* free entry, 1-based
    /// with `0` meaning "no next" — so the new head is
    /// `next_chain_idx().unwrap_or(-1)`, matching
    /// `InsertItemIntoLookupMap`'s `((mapping >> 0xc) & 0xfff) - 1`, which
    /// lands on `-1` for the final entry. Propagating that `None` outward
    /// instead would abandon the pop on the last free entry and, since
    /// `update_item_id_mapping` treats a failed pop as "give up", leave the
    /// item absent from the lookup map while its inventory entry still
    /// exists — invisible to `find_item_idx` and to the game's own
    /// `GetItemInventoryIdx`.
    fn pop_free_entry(&mut self) -> Option<i16> {
        let head = self.item_id_mapping_free_head;
        if head < 0 {
            return None;
        }
        let next = unsafe { self.item_id_mapping.as_slice() }
            .get(head as usize)?
            .next_chain_idx()
            .unwrap_or(-1);
        self.item_id_mapping_free_head = next;
        Some(head)
    }

    /// Walk the collision chain for `item_id`'s bucket, returning
    /// `(prev_idx, cur_idx)` where `cur_idx` is the matching entry.
    /// Returns `None` if the item is not in the table
    fn find_chain_entry(&self, item_id: ItemId) -> Option<(Option<i16>, i16)> {
        let bucket = Self::bucket_for(item_id);
        let mut prev_idx: Option<i16> = None;
        let mut cur_idx = self.item_id_mapping_indices[bucket];

        while cur_idx >= 0 {
            let e = self.mapping_entry(cur_idx)?;
            if e.item_id.as_valid() == Some(item_id) {
                return Some((prev_idx, cur_idx));
            }
            prev_idx = Some(cur_idx);
            cur_idx = e.next_chain_idx()?;
        }

        None
    }

    /// Upserts `(item_id -> inventory slot index)` into the hash table, keeping the
    /// **minimum** slot index for each item_id
    ///
    /// Call this after inserting or updating any inventory entry
    pub fn update_item_id_mapping(&mut self, item_id: ItemId, inventory_slot: i16) {
        // `item_slot` is a 12-bit field, and the game masks on both the write
        // (`mapping | param_3 & 0xfff`) and the "keep the minimum" compare
        // (`param_3 < (mapping & 0xfff)`). Masking here keeps the comparison
        // below apples-to-apples: comparing a full-width value against the
        // truncated one that's actually stored would pick the wrong winner
        // for any slot at or above 4096.
        let slot_bits = (inventory_slot as u16) & 0xfff;

        match self.find_chain_entry(item_id) {
            Some((_, cur_idx)) => {
                // Entry exists, so keep minimum slot
                if let Some(e) = self.mapping_entry_mut(cur_idx)
                    && slot_bits < e.mapping.item_slot()
                {
                    e.mapping.set_item_slot(slot_bits);
                }
            }
            None => {
                // Not present, so allocate and either point the bucket or
                // append to the tail of the existing chain
                let new_idx = match self.pop_free_entry() {
                    Some(i) => i,
                    None => return,
                };

                let bucket = Self::bucket_for(item_id);
                let first_raw = self.item_id_mapping_indices[bucket];

                if first_raw < 0 {
                    // Empty bucket
                    self.item_id_mapping_indices[bucket] = new_idx;
                } else {
                    // Walk to tail and append
                    let mut cur_idx = first_raw;
                    loop {
                        let next = self.mapping_entry(cur_idx).and_then(|e| e.next_chain_idx());
                        match next {
                            Some(n) => cur_idx = n,
                            None => {
                                if let Some(e) = self.mapping_entry_mut(cur_idx) {
                                    e.set_next_chain_idx(Some(new_idx as u16));
                                }
                                break;
                            }
                        }
                    }
                }

                if let Some(e) = self.mapping_entry_mut(new_idx) {
                    e.item_id = OptionalItemId::from(item_id.into_inner());
                    e.mapping.set_item_slot(slot_bits);
                    e.set_next_chain_idx(None);
                    e.mark_in_use();
                }
            }
        }
    }

    /// Drops `item_id`'s lookup-map entry — but only once no slot holds it
    /// any more.
    ///
    /// Inventory slots are sparse: removing one copy of an item doesn't shift
    /// the others, and the map deliberately tracks the *lowest* slot holding
    /// a given id (see [`update_item_id_mapping`](Self::update_item_id_mapping),
    /// mirroring `InsertItemIntoLookupMap`'s `if (param_3 < current)` guard).
    /// So when the copy being removed is the one the map points at, the entry
    /// has to be repointed at the next-lowest remaining copy rather than
    /// unlinked — otherwise every surviving copy becomes invisible to
    /// [`find_item_idx`](Self::find_item_idx).
    pub fn remove_item_id_mapping(&mut self, item_id: ItemId) {
        let (prev_idx, cur_idx) = match self.find_chain_entry(item_id) {
            Some(pair) => pair,
            None => return,
        };

        // Still owned elsewhere? Repoint at the lowest remaining slot and
        // keep the entry linked.
        if let Some(lowest) = self.lowest_slot_holding(item_id) {
            if let Some(e) = self.mapping_entry_mut(cur_idx) {
                e.mapping.set_item_slot(lowest as u16);
            }
            return;
        }

        let next = self.mapping_entry(cur_idx).and_then(|e| e.next_chain_idx());
        let free_head = self.item_id_mapping_free_head;

        // Unlink
        match prev_idx {
            None => {
                let bucket = Self::bucket_for(item_id);
                self.item_id_mapping_indices[bucket] = next.unwrap_or(-1);
            }
            Some(prev) => {
                if let Some(e) = self.mapping_entry_mut(prev) {
                    e.set_next_chain_idx(next.map(|n| n as u16));
                }
            }
        }

        // Return to free list
        if let Some(e) = self.mapping_entry_mut(cur_idx) {
            e.item_id = OptionalItemId::NONE;
            e.set_next_chain_idx(if free_head >= 0 {
                Some(free_head as u16)
            } else {
                None
            });
            e.mark_free();
        }
        self.item_id_mapping_free_head = cur_idx;
    }

    /// The lowest global slot index still holding `item_id`, scanning the
    /// entry arrays directly rather than the lookup map.
    ///
    /// Used by [`remove_item_id_mapping`](Self::remove_item_id_mapping) to
    /// decide whether an id is genuinely gone or merely lost the copy the map
    /// happened to point at.
    fn lowest_slot_holding(&self, item_id: ItemId) -> Option<u32> {
        let key_cap = self.key_items_capacity;

        let key = self
            .current_key_entries()
            .iter()
            .position(|e| {
                e.as_option()
                    .is_some_and(|e| e.item_id.as_valid() == Some(item_id))
            })
            .map(|i| i as u32);

        key.or_else(|| {
            self.normal_entries()
                .iter()
                .position(|e| {
                    e.as_option()
                        .is_some_and(|e| e.item_id.as_valid() == Some(item_id))
                })
                .map(|i| key_cap + i as u32)
        })
    }

    /// O(1) lookup of an item's first inventory slot via the hash table.
    /// Returns `None` if the item is not present
    pub fn find_item_idx(&self, item_id: ItemId) -> Option<u32> {
        let (_, cur_idx) = self.find_chain_entry(item_id)?;
        Some(self.mapping_entry(cur_idx)?.mapping.item_slot() as u32)
    }

    /// Finds the first empty slot in the key items array, scanning its full
    /// capacity — slots are sparse, so the first free one is often below the
    /// highest in use. Returns the slot's **offset within the key items
    /// array** (not a global inventory slot index), or `None` if there's no
    /// empty slot.
    ///
    /// Mirrors the scan in `InsertKeyItem`.
    fn find_empty_key_offset(&self) -> Option<u32> {
        let entries = unsafe {
            std::slice::from_raw_parts(
                self.key_items_head.as_ptr(),
                self.key_items_capacity as usize,
            )
        };
        entries.iter().position(|e| e.is_empty()).map(|i| i as u32)
    }

    /// Finds the first empty slot in the normal items array, scanning its
    /// full capacity — slots are sparse, so the first free one is often
    /// below the highest in use. Returns the slot's **offset within the
    /// normal items array** (not a global inventory slot index), or `None`
    /// if there's no empty slot.
    ///
    /// Mirrors the scan in `InsertNormalItem`.
    fn find_empty_normal_offset(&self) -> Option<u32> {
        self.normal_entries()
            .iter()
            .position(|e| e.is_empty())
            .map(|i| i as u32)
    }

    /// The number of empty slots in the key items array, scanning its full
    /// capacity. This is how many more distinct key-item entries can be
    /// inserted before it's full.
    pub fn empty_key_slot_count(&self) -> u32 {
        let entries = unsafe {
            std::slice::from_raw_parts(
                self.key_items_head.as_ptr(),
                self.key_items_capacity as usize,
            )
        };
        entries.iter().filter(|e| e.is_empty()).count() as u32
    }

    /// The number of empty slots in the normal items array, scanning its
    /// full capacity. This is how many more distinct normal-item entries can
    /// be inserted before it's full.
    pub fn empty_normal_slot_count(&self) -> u32 {
        self.normal_entries()
            .iter()
            .filter(|e| e.is_empty())
            .count() as u32
    }

    /// Writes `entry` into the first empty key-item or normal-item slot
    /// (per `is_key_item`), and registers it in the item ID lookup table.
    /// Returns the global inventory slot index the entry was written to, or
    /// `None` if there's no free slot.
    ///
    /// `key_items_len`/`normal_items_len` are incremented by one on a
    /// successful insert, mirroring `InsertKeyItem`/`InsertNormalItem`'s
    /// unconditional `count += 1` — they're occupancy counts, so this holds
    /// even when the slot filled sits below other occupied ones.
    ///
    /// Takes a gaitem reference for `entry.gaitem_handle` on success, since
    /// the new entry holds one of its own for as long as it lives — mirroring
    /// the real `InventoryItemEntry::InventoryItemEntry`, which installs the
    /// handle through `swapInventoryItemGaItemHandles_` (a ref-counting swap)
    /// rather than a plain assignment. It balances the release
    /// [`remove_entry`](Self::remove_entry) hands back; without it the gaitem
    /// is under-referenced and gets freed out from under the still-live
    /// entry, leaving a dangling pool slot that crashes
    /// `CSGaitemImp::Serialize`/`Deserialize` on the next menu transition.
    /// A no-op for the non-indexed (`Goods`/`Accessory`) handles, which
    /// aren't refcounted.
    ///
    /// Mirrors `InsertKeyItem`/`InsertNormalItem`.
    pub fn insert_entry(
        &mut self,
        entry: EquipInventoryDataListEntry,
        is_key_item: bool,
    ) -> Option<u32> {
        // An entry with no item is the empty state, not something to insert.
        let item_id = entry.item_id.as_valid()?;
        let gaitem_handle = entry.gaitem_handle;

        let global_slot = if is_key_item {
            self.find_empty_key_offset()?
        } else {
            self.key_items_capacity + self.find_empty_normal_offset()?
        };

        // Write the entry before touching any counter, so a failed write
        // can't leave the length claiming a slot that was never filled — the
        // free-slot scans run over the full capacity, and a length that ran
        // ahead of them would hand out an index nothing lives at.
        let slot = self.entry_at_slot_mut(global_slot)?;
        *slot = MaybeEmpty::new(entry);

        if is_key_item {
            self.key_items_len += 1;
            unsafe {
                *self.key_items_accessor.length.as_mut() = self.key_items_len;
            }
        } else {
            self.normal_items_len += 1;
        }

        self.update_item_id_mapping(item_id, global_slot as i16);

        if let Ok(gaitem) = unsafe { CSGaitemImp::instance_mut() } {
            gaitem.increase_ref_count(gaitem_handle);
        }

        Some(global_slot)
    }

    /// Clears the entry at `slot`, removing it from the item ID lookup table
    /// and decrementing the relevant occupancy count by one, mirroring
    /// `RemoveItemEntryBySlot`'s unconditional `count -= 1`. Returns the
    /// [`GaitemHandle`] the cleared entry held, if any, so the caller can
    /// release it via
    /// [`CSGaitemImp::release_handle`](crate::cs::CSGaitemImp::release_handle).
    ///
    /// Mirrors `RemoveItemEntryBySlot`.
    pub fn remove_entry(&mut self, slot: u32) -> Option<GaitemHandle> {
        let entry = self.entry_at_slot_mut(slot)?;
        // `as_option` already established the entry is occupied, so its
        // `item_id` is necessarily a valid one.
        let (item_id, gaitem_handle) = entry
            .as_option()
            .and_then(|e| Some((e.item_id.as_valid()?, e.gaitem_handle)))?;

        entry.clear();

        if slot < self.key_items_capacity {
            self.key_items_len -= 1;
            unsafe {
                *self.key_items_accessor.length.as_mut() = self.key_items_len;
            }
        } else {
            self.normal_items_len -= 1;
        }

        self.remove_item_id_mapping(item_id);
        Some(gaitem_handle)
    }
}

#[repr(C)]
pub struct EquipInventoryData {
    vftable: usize,
    pub items_data: InventoryItemsData,
    /// The highest global slot index in use, **not** a count of items —
    /// inventory slots are sparse, and this ignores the gaps between them.
    ///
    /// `InsertItem` raises it to cover a newly filled slot
    /// (`if (highestItemSlot < idx) highestItemSlot = idx;`), and
    /// `EquipInventoryData::RemoveItem` lowers it by one — but *only* when
    /// the slot being removed is exactly this one
    /// (`if (idx == highestItemSlot) highestItemSlot -= 1;`), so it isn't a
    /// true maximum after removals in the middle.
    ///
    /// Several of the game's own scans are bounded by it and walk
    /// `0..=highest_item_slot`, null-checking each entry as they go
    /// (`GetQuantityByItemId`, `AdjustQuantityBy`,
    /// `GetInventoryItemEntryByIndex`); an entry past the mark is present in
    /// memory but invisible to them.
    pub highest_item_slot: u32,
    /// Next sort ID to assign to newly added items.
    /// Used to sort items by acquisition order.
    pub next_sort_id: u32,
    /// Count of all pot items by their pot group
    pub pot_items_count: [u32; 16],
    /// Capacity of all pot items by their pot group
    pub pot_items_capacity: [u32; 16],
    /// List of item indices to show in the "Recent Items" inventory tab.
    /// Capped at 64 and shown in the UI back to front.
    pub recent_item_indices: DLList<u32>,
    /// True will allow consumables stack up to 600 like in storage box.
    pub unlimited_consumables: bool,
    /// Should pots be limited to amount of pot capacity by their group?
    pub limited_pots: bool,
    unk122: u8,
    unk123: u8,
    unk124: u32,
}

#[derive(Debug, Default, Clone)]
pub struct AddedItem {
    /// Falls short of the requested quantity when the stack, the inventory or
    /// the gaitem pool filled up.
    pub quantity: u32,
    /// Slots now holding the item: one for a stacking item, one per copy
    /// otherwise.
    pub slots: Vec<u32>,
    /// `slots.len()` when the entries are new, `0` when an existing stack was
    /// topped up.
    pub created_entries: u32,
}

/// Weapon category value for arrows, the only weapon-category items that
/// stack. Source: `EQUIP_PARAM_WEAPON_ST::weapon_category`.
const WEAPON_CATEGORY_ARROW: u8 = 0xd;
/// Weapon category value for bolts, the only other weapon-category items
/// that stack. Source: `EQUIP_PARAM_WEAPON_ST::weapon_category`.
const WEAPON_CATEGORY_BOLT: u8 = 0xe;

impl EquipInventoryData {
    /// Inserts `entry` via [`InventoryItemsData::insert_entry`], then raises
    /// [`highest_item_slot`](Self::highest_item_slot) to cover the slot it
    /// landed in.
    ///
    /// That bookkeeping is why this exists rather than callers reaching for
    /// `items_data.insert_entry` directly: the real `InsertItem` does
    /// `if (itemEntriesCount < fromInventoryIdx) itemEntriesCount = fromInventoryIdx;`
    /// on every insert, and several of the game's own scans are bounded by
    /// it (`GetQuantityByItemId` and `AdjustQuantityBy` both iterate
    /// `idx < itemEntriesCount + 1`). Leave it stale and an entry written
    /// past the mark is physically present but invisible to those lookups.
    ///
    /// Takes ownership of the entry's [`GaitemHandle`], so
    /// [`add_item`](Self::add_item) is the way in from outside the crate.
    pub(crate) fn insert_entry(
        &mut self,
        entry: EquipInventoryDataListEntry,
        is_key_item: bool,
    ) -> Option<u32> {
        let slot = self.items_data.insert_entry(entry, is_key_item)?;

        if self.highest_item_slot < slot {
            self.highest_item_slot = slot;
        }

        Some(slot)
    }

    /// Removes the entry at `slot` via
    /// [`InventoryItemsData::remove_entry`], then lowers
    /// [`highest_item_slot`](Self::highest_item_slot) if `slot` was the one
    /// it pointed at.
    ///
    /// Mirrors `EquipInventoryData::RemoveItem`'s
    /// `if (idx == highestItemSlot) highestItemSlot -= 1;` — note it steps
    /// down by exactly one rather than rescanning for the next occupied
    /// slot, so after removals the mark is an upper bound rather than a
    /// tight maximum. Every scan it bounds null-checks each entry anyway.
    ///
    /// Hands back the entry's [`GaitemHandle`] for the caller to release, so
    /// [`remove_item`](Self::remove_item) is the way in from outside the crate.
    pub(crate) fn remove_entry(&mut self, slot: u32) -> Option<GaitemHandle> {
        let handle = self.items_data.remove_entry(slot)?;

        if slot == self.highest_item_slot {
            self.highest_item_slot = self.highest_item_slot.saturating_sub(1);
        }

        Some(handle)
    }

    /// Whether `item_id` stacks (has a quantity greater than 1 in a single
    /// inventory entry) rather than occupying one entry per copy. True for
    /// all Goods, and for Weapons in the arrow/bolt categories.
    ///
    /// Mirrors `IsStackable`.
    pub fn is_stackable(item_id: ItemId) -> bool {
        match item_id.category() {
            ItemCategory::Goods => true,
            ItemCategory::Weapon => {
                let Ok(repo) = (unsafe { SoloParamRepository::instance() }) else {
                    return false;
                };
                let Some(weapon) = repo.get::<EquipParamWeapon>(item_id.base_weapon_param_id())
                else {
                    return false;
                };
                matches!(
                    weapon.weapon_category(),
                    WEAPON_CATEGORY_ARROW | WEAPON_CATEGORY_BOLT
                )
            }
            _ => false,
        }
    }

    /// Whether `item_id` is a unique key item: a Goods item the player can
    /// only ever hold one copy of.
    ///
    /// Mirrors `IsKeyItem`.
    pub fn is_key_item(item_id: ItemId) -> bool {
        if item_id.category() != ItemCategory::Goods {
            return false;
        }
        let Ok(repo) = (unsafe { SoloParamRepository::instance() }) else {
            return false;
        };
        const KEY_ITEM: u8 = 0x1;
        const WONDROUS_PHYSICK_TEAR: u8 = 0xa;
        const REGENERATIVE_MATERIAL: u8 = 0xb;
        const GREAT_RUNE: u8 = 0xf;

        repo.get::<EquipParamGoods>(item_id.param_id())
            .map(|goods| {
                matches!(
                    goods.goods_type(),
                    KEY_ITEM | WONDROUS_PHYSICK_TEAR | REGENERATIVE_MATERIAL | GREAT_RUNE
                )
            })
            .unwrap_or(false)
    }

    /// The maximum quantity a single inventory entry of `item_id` can hold.
    ///
    /// For Goods, this is the pot-group remaining capacity if the item is
    /// part of a limited pot group and [limited_pots](Self::limited_pots) is
    /// set, otherwise `EQUIP_PARAM_GOODS_ST::max_num` (or 99 if unset),
    /// overridden to a large amount if
    /// [unlimited_consumables](Self::unlimited_consumables) is set. For
    /// arrow/bolt Weapons, this is `EQUIP_PARAM_WEAPON_ST::max_arrow_quantity`.
    /// Returns 0 for anything else.
    ///
    /// Mirrors `GetMaxAmountForItem`/`GetMaxQuantityForItemEntry`.
    pub fn max_stack_for(&self, item_id: ItemId) -> u32 {
        let Ok(repo) = (unsafe { SoloParamRepository::instance() }) else {
            return 0;
        };

        // `GetMaxAmountForItem` tests this *before* dispatching on category,
        // so the override covers every item, not just Goods.
        if self.unlimited_consumables {
            return self.unlimited_consumables_max_stack(item_id);
        }

        match item_id.category() {
            ItemCategory::Goods => {
                let Some(goods) = repo.get::<EquipParamGoods>(item_id.param_id()) else {
                    return 0;
                };

                let pot_group = goods.pot_group_id();
                if pot_group >= 0 {
                    if self.limited_pots {
                        return self.pot_items_capacity[pot_group as usize]
                            .saturating_sub(self.pot_items_count[pot_group as usize]);
                    }
                    return 99;
                }

                if goods.max_num() > 0 {
                    goods.max_num() as u32
                } else {
                    99
                }
            }
            ItemCategory::Weapon => {
                let Some(weapon) = repo.get::<EquipParamWeapon>((item_id.param_id() / 100) * 100)
                else {
                    return 0;
                };
                if matches!(
                    weapon.weapon_category(),
                    WEAPON_CATEGORY_ARROW | WEAPON_CATEGORY_BOLT
                ) {
                    weapon.max_arrow_quantity() as u32
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// The stack ceiling under
    /// [`unlimited_consumables`](Self::unlimited_consumables), which is what
    /// the storage box sets.
    ///
    /// Mirrors `GetMaxItemCountForUnlimitedConsumables`.
    /// Despite the name it isn't a blanket "unlimited": each category answers
    /// differently, and only ammo and Goods stack meaningfully.
    fn unlimited_consumables_max_stack(&self, item_id: ItemId) -> u32 {
        let Ok(repo) = (unsafe { SoloParamRepository::instance() }) else {
            return 0;
        };

        match item_id.category() {
            // Ammo stacks to 600 here; any other weapon is one per entry.
            ItemCategory::Weapon => repo
                .get::<EquipParamWeapon>((item_id.param_id() / 100) * 100)
                .filter(|weapon| {
                    matches!(
                        weapon.weapon_category(),
                        WEAPON_CATEGORY_ARROW | WEAPON_CATEGORY_BOLT
                    )
                })
                .map_or(1, |_| 600),
            // `maxRepositoryNum`, the item's own storage-box ceiling, not a
            // flat 600 — that value just happens to be what most rows carry.
            ItemCategory::Goods => repo
                .get::<EquipParamGoods>(item_id.param_id())
                .map_or(99, |goods| goods.max_repository_num() as u32),
            ItemCategory::Gem => repo
                .get::<EquipParamGem>(item_id.param_id())
                .map_or(0, |_| 1),
            ItemCategory::Protector | ItemCategory::Accessory => 1,
        }
    }

    /// How many more of `item_id` can actually be added, accounting for what's
    /// already held.
    ///
    /// [`max_stack_for`](Self::max_stack_for) answers two different questions
    /// depending on the item, matching `GetMaxAmountForItem`:
    /// for a pot-group item under `limited_pots` it already returns the
    /// group's *remaining headroom*, while for everything else it returns a
    /// per-entry *stack ceiling*. Subtracting the entry's own quantity is
    /// therefore right for the latter and wrong for the former — a pot group's
    /// budget is shared across all its items, so its count has already been
    /// deducted.
    ///
    /// `slot` is the entry the quantity would be added to, or `None` when the
    /// item isn't held yet.
    pub fn headroom_for(&self, item_id: ItemId, slot: Option<u32>) -> u32 {
        let max = self.max_stack_for(item_id);

        if self.limited_pots && Self::pot_group_for(item_id) >= 0 {
            // Already headroom — do not deduct the entry's quantity again.
            return max;
        }

        let held = slot
            .and_then(|slot| self.items_data.entry_at_slot(slot))
            .and_then(|entry| entry.as_option())
            .map(|entry| entry.quantity)
            .unwrap_or(0);

        max.saturating_sub(held)
    }

    /// Adds up to `quantity` of `item_id`, clamped to
    /// [`headroom_for`](Self::headroom_for) and the free slots, so
    /// [`u32::MAX`] fills the stack. [`is_stackable`](Self::is_stackable) items
    /// merge into an existing entry; everything else takes one entry per copy.
    ///
    /// Mirrors `EquipInventoryData::InsertItem`, resolving gaitem handles the
    /// way `AddInventoryEquipByItemId` does.
    /// Carries none of the player-side bookkeeping
    /// [`EquipGameData::give_item`](crate::cs::EquipGameData::give_item) adds,
    /// which is what lets it also serve the storage box — where the caps come
    /// from `maxRepositoryNum` instead of `maxNum`.
    pub fn add_item(&mut self, item_id: ItemId, quantity: u32) -> AddedItem {
        if quantity == 0 {
            return AddedItem::default();
        }

        if Self::is_stackable(item_id)
            && let Some(slot) = self.items_data.find_item_idx(item_id)
        {
            let added = self.headroom_for(item_id, Some(slot)).min(quantity);
            let Some(entry) = self
                .items_data
                .entry_at_slot_mut(slot)
                .and_then(|e| e.as_option_mut())
            else {
                return AddedItem::default();
            };

            entry.quantity += added;
            let pot_group = entry.pot_group;
            if added > 0 && pot_group >= 0 {
                self.pot_items_count[pot_group as usize] += added;
            }
            return AddedItem {
                quantity: added,
                slots: vec![slot],
                created_entries: 0,
            };
        }

        if Self::is_stackable(item_id) {
            let wanted = self.headroom_for(item_id, None).min(quantity);
            return match self.insert_new_entry(item_id, wanted) {
                Some((_, slot)) => AddedItem {
                    quantity: wanted,
                    slots: vec![slot],
                    created_entries: 1,
                },
                None => AddedItem::default(),
            };
        }

        // One entry per copy, each with its own handle, until the inventory
        // runs out of slots.
        let mut slots = Vec::new();
        while (slots.len() as u32) < quantity
            && let Some((_, slot)) = self.insert_new_entry(item_id, 1)
        {
            slots.push(slot);
        }

        AddedItem {
            quantity: slots.len() as u32,
            created_entries: slots.len() as u32,
            slots,
        }
    }

    /// Creates one entry holding `quantity` of `item_id`, allocating the gaitem
    /// handle it takes ownership of. Returns the entry's handle and slot, or
    /// `None` if the inventory or gaitem pool was full.
    pub(crate) fn insert_new_entry(
        &mut self,
        item_id: ItemId,
        quantity: u32,
    ) -> Option<(GaitemHandle, u32)> {
        if quantity == 0 {
            return None;
        }

        let Ok(gaitem) = (unsafe { crate::cs::CSGaitemImp::instance_mut() }) else {
            return None;
        };
        let gaitem_handle = gaitem.allocate_for_item(item_id)?;

        let pot_group = Self::pot_group_for(item_id);
        let sort_id = self.next_sort_id;
        self.next_sort_id += 1;

        let is_key_item = Self::is_key_item(item_id);
        let Some(slot) = self.insert_entry(
            EquipInventoryDataListEntry {
                gaitem_handle,
                item_id: item_id.into(),
                quantity,
                sort_id,
                is_new: true,
                pot_group,
            },
            is_key_item,
        ) else {
            gaitem.release_handle(gaitem_handle);
            return None;
        };

        if pot_group >= 0 {
            self.pot_items_count[pot_group as usize] += quantity;
        }

        Some((gaitem_handle, slot))
    }

    /// Removes up to `quantity` of `item_id`, clearing its entry and releasing
    /// the [`GaitemHandle`] if the stack reaches zero. Returns how many were
    /// removed.
    ///
    /// Mirrors the negative-quantity branch of `GetAddOrRemoveAmount` and
    /// `AdjustItemCountByIndex`.
    pub fn remove_item(&mut self, item_id: ItemId, quantity: u32) -> u32 {
        let Some(slot) = self.items_data.find_item_idx(item_id) else {
            return 0;
        };
        let Some(entry) = self
            .items_data
            .entry_at_slot_mut(slot)
            .and_then(|e| e.as_option_mut())
        else {
            return 0;
        };

        let removed = quantity.min(entry.quantity);
        let pot_group = entry.pot_group;
        entry.quantity -= removed;

        if pot_group >= 0 && removed > 0 {
            self.pot_items_count[pot_group as usize] =
                self.pot_items_count[pot_group as usize].saturating_sub(removed);
        }

        if entry.quantity == 0
            && let Some(handle) = self.remove_entry(slot)
            && let Ok(gaitem) = unsafe { crate::cs::CSGaitemImp::instance_mut() }
        {
            gaitem.release_handle(handle);
        }

        removed
    }

    /// The pot group of `item_id`, or -1 if it's not part of one.
    fn pot_group_for(item_id: ItemId) -> i32 {
        if item_id.category() != ItemCategory::Goods {
            return -1;
        }
        let Ok(repo) = (unsafe { SoloParamRepository::instance() }) else {
            return -1;
        };
        repo.get::<EquipParamGoods>(item_id.param_id())
            .map(|goods| goods.pot_group_id() as i32)
            .unwrap_or(-1)
    }
}

bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq, Hash)]
    struct ItemIdMappingBits(u32);
    impl Debug;

    bool;
    pub is_free, set_is_free: 24;

    u16;
    mapping_index, set_mapping_index: 23, 12;
    item_slot, set_item_slot: 11, 0;
}

#[repr(C)]
pub struct ItemIdMapping {
    pub item_id: OptionalItemId,
    mapping: ItemIdMappingBits,
}

impl ItemIdMapping {
    /// Returns the offset of the next item ID mapping with the same modulo result.
    pub fn next_mapping_item(&self) -> i16 {
        self.mapping.mapping_index() as i16 - 1
    }

    /// Returns the index of the item slot. This index is first checked against the key items
    /// capacity to see if it's contained in that. If not you will need to subtract the key items
    /// capacity to get the index for the normal items list.
    pub fn item_slot(&self) -> i16 {
        self.mapping.item_slot() as i16
    }

    pub fn is_free(&self) -> bool {
        self.mapping.is_free()
    }

    /// Next entry in the collision chain, 0-based. `None` = end of chain.
    /// The stored value is 1-based (0 means no next entry)
    pub fn next_chain_idx(&self) -> Option<i16> {
        let one_based = self.mapping.mapping_index() as i16;
        if one_based == 0 {
            None
        } else {
            Some(one_based - 1)
        }
    }

    fn set_next_chain_idx(&mut self, idx: Option<u16>) {
        let one_based = idx.map_or(0, |i| i + 1);
        self.mapping.set_mapping_index(one_based);
    }

    fn mark_in_use(&mut self) {
        self.mapping.set_is_free(false);
    }

    fn mark_free(&mut self) {
        self.mapping.set_is_free(true);
    }
}

#[repr(C)]
pub struct EquipInventoryDataListEntry {
    /// Handle to the gaitem instance which describes additional properties to the inventory item,
    /// like durability and gems in the case of weapons.
    pub gaitem_handle: GaitemHandle,
    /// The item this entry holds, or [`OptionalItemId::NONE`] when the entry
    /// is empty — the state [`MaybeEmpty`] keys on, and the reason this is an
    /// [`OptionalItemId`] rather than an [`ItemId`].
    pub item_id: OptionalItemId,
    /// Quantity of the item we have.
    pub quantity: u32,
    /// Sort ID used to sort items by acquisition order.
    pub sort_id: u32,
    /// Whether the item is newly acquired and should be highlighted in the UI if
    /// "Mark New Items" option is enabled.
    pub is_new: bool,
    /// [pot group] of the item, or -1 if not a pot item.
    ///
    /// [pot group]: crate::param::EQUIP_PARAM_GOODS_ST::pot_group_id
    pub pot_group: i32,
}

unsafe impl IsEmpty for EquipInventoryDataListEntry {
    fn is_empty(value: &MaybeEmpty<EquipInventoryDataListEntry>) -> bool {
        // Safety: `item_id` is `OptionalItemId`, which is valid for every bit
        // pattern, so it's readable whether or not the entry is occupied.
        !unsafe { value.as_non_null().as_ref() }.item_id.is_valid()
    }
}

impl EquipInventoryDataListEntry {
    /// This entry's item, or `None` if the entry is empty.
    ///
    /// Entries reached through [`items`](InventoryItemsData::items) or
    /// [`MaybeEmpty::as_option`] are always occupied, so this returns `Some`
    /// for them.
    pub fn item(&self) -> Option<ItemId> {
        self.item_id.as_valid()
    }
}

impl Default for EquipInventoryDataListEntry {
    /// An empty entry: a null `gaitem_handle` and a `NONE` `item_id`.
    ///
    /// Both fields matter, because the game tests emptiness two different
    /// ways depending on the code path: `RebuildLookupMapping` skips entries
    /// whose `itemId` is `-1`, while `InsertKeyItem`/`InsertNormalItem`
    /// search for a free slot using `InventoryItemEntry::IsGaItemHandleNull`
    /// (`gaItemHandle == 0`). Clearing only one leaves a slot that's
    /// invisible to the lookup map but permanently unavailable for
    /// insertion, still carrying a dangling gaitem handle — which the game
    /// then picks up when it rebuilds state from these arrays on a menu
    /// transition.
    fn default() -> Self {
        Self {
            gaitem_handle: GaitemHandle(0),
            item_id: OptionalItemId::NONE,
            quantity: 0,
            sort_id: 0,
            is_new: false,
            pot_group: -1,
        }
    }
}

#[repr(C)]
pub struct EquipMagicData {
    vftable: usize,
    pub equip_game_data: NonNull<EquipGameData>,
    pub entries: [EquipMagicItem; 14],
    pub selected_slot: i32,
    unk84: u32,
}

#[repr(C)]
pub struct EquipMagicItem {
    pub param_id: i32,
    pub charges: i32,
}

#[repr(C)]
pub struct EquipItemData {
    vftable: usize,
    pub quick_slots: [EquipDataItem; 10],
    pub pouch_slots: [EquipDataItem; 6],
    pub great_rune: EquipDataItem,
    pub equip_entries: NonNull<ChrAsmEquipEntries>,
    pub inventory: NonNull<EquipInventoryData>,
    pub selected_quick_slot: i32,
    unka4: u32,
}

#[repr(C)]
pub struct EquipDataItem {
    pub gaitem_handle: GaitemHandle,
    pub index: i32,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChrAsmSlot {
    WeaponLeft1 = 0,
    WeaponRight1 = 1,
    WeaponLeft2 = 2,
    WeaponRight2 = 3,
    WeaponLeft3 = 4,
    WeaponRight3 = 5,
    Arrow1 = 6,
    Bolt1 = 7,
    Arrow2 = 8,
    Bolt2 = 9,
    Arrow3 = 10,
    Bolt3 = 11,
    ProtectorHead = 12,
    ProtectorChest = 13,
    ProtectorHands = 14,
    ProtectorLegs = 15,
    Unused16 = 16,
    Accessory1 = 17,
    Accessory2 = 18,
    Accessory3 = 19,
    Accessory4 = 20,
    AccessoryCovenant = 21,
}

impl<T> Index<ChrAsmSlot> for [T; 22] {
    type Output = T;

    fn index(&self, index: ChrAsmSlot) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<ChrAsmSlot> for [T; 22] {
    fn index_mut(&mut self, index: ChrAsmSlot) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

#[derive(Debug, Error)]
pub enum ChrAsmSlotError {
    #[error("Invalid ChrAsmSlot index: {0}")]
    InvalidIndex(u32),
}

impl ChrAsmSlot {
    /// Whether an item of `category` can be equipped into this slot.
    ///
    /// Weapon slots take `Weapon` (arrow/bolt slots included — ammo is
    /// `Weapon`-category, distinguished by `weaponCategory` rather than by
    /// item category), protector slots take `Protector`, and accessory slots
    /// take `Accessory`. `Goods` and `Gem` have no `ChrAsmSlot` at all: goods
    /// live in quick/pouch slots on `EquipItemData`, and gems are mounted
    /// into a weapon's gem slot.
    pub fn accepts(self, category: ItemCategory) -> bool {
        match self {
            ChrAsmSlot::WeaponLeft1
            | ChrAsmSlot::WeaponRight1
            | ChrAsmSlot::WeaponLeft2
            | ChrAsmSlot::WeaponRight2
            | ChrAsmSlot::WeaponLeft3
            | ChrAsmSlot::WeaponRight3
            | ChrAsmSlot::Arrow1
            | ChrAsmSlot::Bolt1
            | ChrAsmSlot::Arrow2
            | ChrAsmSlot::Bolt2
            | ChrAsmSlot::Arrow3
            | ChrAsmSlot::Bolt3 => category == ItemCategory::Weapon,

            ChrAsmSlot::ProtectorHead
            | ChrAsmSlot::ProtectorChest
            | ChrAsmSlot::ProtectorHands
            | ChrAsmSlot::ProtectorLegs => category == ItemCategory::Protector,

            ChrAsmSlot::Accessory1
            | ChrAsmSlot::Accessory2
            | ChrAsmSlot::Accessory3
            | ChrAsmSlot::Accessory4
            | ChrAsmSlot::AccessoryCovenant => category == ItemCategory::Accessory,

            ChrAsmSlot::Unused16 => false,
        }
    }

    pub fn from_index(index: u32) -> Result<Self, ChrAsmSlotError> {
        match index {
            0 => Ok(ChrAsmSlot::WeaponLeft1),
            1 => Ok(ChrAsmSlot::WeaponRight1),
            2 => Ok(ChrAsmSlot::WeaponLeft2),
            3 => Ok(ChrAsmSlot::WeaponRight2),
            4 => Ok(ChrAsmSlot::WeaponLeft3),
            5 => Ok(ChrAsmSlot::WeaponRight3),
            6 => Ok(ChrAsmSlot::Arrow1),
            7 => Ok(ChrAsmSlot::Bolt1),
            8 => Ok(ChrAsmSlot::Arrow2),
            9 => Ok(ChrAsmSlot::Bolt2),
            10 => Ok(ChrAsmSlot::Arrow3),
            11 => Ok(ChrAsmSlot::Bolt3),
            12 => Ok(ChrAsmSlot::ProtectorHead),
            13 => Ok(ChrAsmSlot::ProtectorChest),
            14 => Ok(ChrAsmSlot::ProtectorHands),
            15 => Ok(ChrAsmSlot::ProtectorLegs),
            16 => Ok(ChrAsmSlot::Unused16),
            17 => Ok(ChrAsmSlot::Accessory1),
            18 => Ok(ChrAsmSlot::Accessory2),
            19 => Ok(ChrAsmSlot::Accessory3),
            20 => Ok(ChrAsmSlot::Accessory4),
            21 => Ok(ChrAsmSlot::AccessoryCovenant),
            _ => Err(ChrAsmSlotError::InvalidIndex(index)),
        }
    }
}

#[repr(C)]
pub struct ChrAsmEquipmentSlots {
    /// Points to the slot in the equipment list used for rendering the left-hand weapon.
    /// 0 for primary, 1 for secondary, 2 for tertiary.
    pub left_weapon_slot: u32,
    /// Points to the slot in the equipment list used for rendering the right-hand weapon.
    /// 0 for primary, 1 for secondary, 2 for tertiary.
    pub right_weapon_slot: u32,
    /// Points to the slot in the equipment list used for rendering the left-hand arrow.
    /// 0 for primary, 1 for secondary.
    pub left_arrow_slot: u32,
    /// Points to the slot in the equipment list used for rendering the right-hand arrow.
    /// 0 for primary, 1 for secondary.
    pub right_arrow_slot: u32,
    /// Points to the slot in the equipment list used for rendering the left-hand bolt.
    /// 0 for primary, 1 for secondary.
    pub left_bolt_slot: u32,
    /// Points to the slot in the equipment list used for rendering the right-hand bolt.
    /// 0 for primary, 1 for secondary.
    pub right_bolt_slot: u32,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChrAsmArmStyle {
    EmptyHanded = 0,
    OneHanded = 1,
    LeftBothHands = 2,
    RightBothHands = 3,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChrAsmHand {
    Left = 0,
    Right = 1,
}

#[repr(C)]
pub struct ChrAsmEquipment {
    /// Determines how you're holding your weapon.
    pub arm_style: ChrAsmArmStyle,
    pub selected_slots: ChrAsmEquipmentSlots,
}

impl ChrAsmEquipment {
    pub fn active_weapon_slot(&self, hand: ChrAsmHand) -> ChrAsmSlot {
        match (self.arm_style, hand) {
            (ChrAsmArmStyle::LeftBothHands, _) => self.active_left_weapon_slot(),
            (ChrAsmArmStyle::RightBothHands, _) => self.active_right_weapon_slot(),
            (_, ChrAsmHand::Left) => self.active_left_weapon_slot(),
            (_, ChrAsmHand::Right) => self.active_right_weapon_slot(),
        }
    }

    pub fn is_two_handing(&self) -> bool {
        matches!(
            self.arm_style,
            ChrAsmArmStyle::LeftBothHands | ChrAsmArmStyle::RightBothHands
        )
    }

    pub fn active_left_weapon_slot(&self) -> ChrAsmSlot {
        match self.selected_slots.left_weapon_slot {
            0 => ChrAsmSlot::WeaponLeft1,
            1 => ChrAsmSlot::WeaponLeft2,
            2 => ChrAsmSlot::WeaponLeft3,
            _ => ChrAsmSlot::WeaponLeft1,
        }
    }

    pub fn active_right_weapon_slot(&self) -> ChrAsmSlot {
        match self.selected_slots.right_weapon_slot {
            0 => ChrAsmSlot::WeaponRight1,
            1 => ChrAsmSlot::WeaponRight2,
            2 => ChrAsmSlot::WeaponRight3,
            _ => ChrAsmSlot::WeaponRight1,
        }
    }
}

#[repr(C)]
/// Describes how the character should be rendered in terms of selecting the
/// appropriate parts to be rendered.
///
/// Source of name: RTTI in earlier games (vmt has been removed from ER after some patch?)
pub struct ChrAsm {
    unk0: i32,
    unk4: i32,
    pub equipment: ChrAsmEquipment,
    /// Holds references to the inventory slots for each equipment piece.
    pub gaitem_handles: [GaitemHandle; 22],
    /// Holds the param IDs for each equipment piece.
    pub equipment_param_ids: [i32; 22],
    unkd4: u32,
    unkd8: u32,
    /// List of all 12 "weapon" slots (from [`WeaponLeft1`] to [`Bolt3`])
    /// and whether or not they are in "loaded" state.
    /// E.g. using crossbow once in [`WeaponLeft1`] slot will set this slot state to `true`,
    /// making next use play shoot animation instead of reload.
    ///
    /// [`WeaponLeft1`]: ChrAsmSlot::WeaponLeft1
    /// [`Bolt3`]: ChrAsmSlot::Bolt3
    pub bolt_loaded_states: [bool; 12],
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EquipmentDurabilityStatus {
    Ok = 0,
    AtRisk = 1,
    Broken = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_id_mapping() {
        let mapping = ItemIdMapping {
            item_id: OptionalItemId::from(0x40002760),
            mapping: ItemIdMappingBits(0x003B8000),
        };
        assert_eq!(mapping.item_id, OptionalItemId::from(0x40002760));
        assert_eq!(
            mapping.next_mapping_item(),
            ((mapping.mapping.0 >> 12) & 0xFFF) as i16 - 1
        );
        assert_eq!(mapping.item_slot(), (mapping.mapping.0 & 0xFFF) as i16);
    }

    /// The last entry on the free list stores `0` in its chain field ("no
    /// next"), which has to pop as a valid entry that leaves the head at
    /// `-1` — not fail the pop. Failing it makes `update_item_id_mapping`
    /// bail, so the item keeps its inventory entry but never enters the
    /// lookup map, and `find_item_idx`/`GetItemInventoryIdx` can't see it.
    #[test]
    fn last_free_entry_pops_and_empties_the_list() {
        // Two entries: 0 -> 1 -> end. Chain field is 1-based, 0 = no next.
        let mut entries = [
            ItemIdMapping {
                item_id: OptionalItemId::NONE,
                mapping: ItemIdMappingBits(0),
            },
            ItemIdMapping {
                item_id: OptionalItemId::NONE,
                mapping: ItemIdMappingBits(0),
            },
        ];
        entries[0].set_next_chain_idx(Some(1));
        entries[1].set_next_chain_idx(None);

        assert_eq!(entries[0].next_chain_idx(), Some(1));
        assert_eq!(entries[1].next_chain_idx(), None);

        // Popping entry 0 leaves head at 1; popping entry 1 — the last one —
        // must still succeed and leave the head at -1.
        let head_after_first = entries[0].next_chain_idx().unwrap_or(-1);
        assert_eq!(head_after_first, 1);

        let head_after_last = entries[1].next_chain_idx().unwrap_or(-1);
        assert_eq!(
            head_after_last, -1,
            "last free entry must empty the list, not abort the pop"
        );
    }
}
