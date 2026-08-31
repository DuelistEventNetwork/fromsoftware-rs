use shared::multi_param;

mod generated;

pub use generated::*;

impl EQUIP_PARAM_WEAPON_ST {
    /// The affinity id the gem check runs against, for a weapon given by its
    /// **full** param id (`base + affinity * 100 + reinforce_level`) — mirrors
    /// `GetLockedAffinityIdForGemCheck`, which reads `(paramId % 10000) / 100`
    /// and returns `0` instead whenever the weapon's affinity is locked
    /// (`disable_gem_attr`).
    ///
    /// Takes the full id rather than the `+0` base: `% 10000` is what isolates
    /// the affinity and reinforcement digits, so rounding the level off first
    /// is a different operation that only coincidentally agrees.
    pub fn gem_check_affinity_id(&self, weapon_param_id: u32) -> u32 {
        if self.disable_gem_attr() {
            return 0;
        }
        (weapon_param_id % 10000) / 100
    }

    /// Whether `gem` (an [`EQUIP_PARAM_GEM_ST`] row) can be mounted on this
    /// weapon — mirrors `CanMountGemWithAffinityOnWeapon`, reached from
    /// `CS::EquipGameData::RemoveInvalidAshesOfWarFromEquippedWeapons`. That
    /// path only ever *removes* an already-mounted invalid ash; nothing in the
    /// game keeps a give from mounting one in the first place, so callers
    /// giving a weapon should use this to gate the mount itself instead of
    /// giving a combination the game would immediately have to correct.
    ///
    /// `affinity_id` is this weapon id's own affinity digit (see
    /// [`gem_check_affinity_id`](Self::gem_check_affinity_id)) and
    /// `max_gem_rank` is `ReinforceParamWeapon::enable_gem_rank` for this
    /// weapon's `reinforce_type_id` row — both keyed differently than this
    /// struct itself, so resolving them is left to the caller (e.g.
    /// [`EquipGameData::give_item`](crate::cs::EquipGameData::give_item)).
    ///
    /// Checks, in order:
    /// - This weapon has a gem slot at all (`gem_mount_type` 1 or 2).
    /// - The gem's `can_mount_wep_*` flag for this weapon's `wep_type` is set.
    /// - The affinity is one the gem supports. Infusing *is* mounting an ash,
    ///   so which affinities a weapon can take is a property of the gem, not
    ///   the weapon: each gem carries a 24-bit `configurable_wep_attr`
    ///   whitelist, with `default_wep_attr` naming the one it lands on when
    ///   applied plainly. Lightning Slash, for instance, defaults to
    ///   Lightning but is configurable as Standard/Heavy/Keen/Quality/
    ///   Lightning/Sacred and nothing else. A weapon whose affinity is locked
    ///   (`disable_gem_attr`) instead demands the gem's `default_wep_attr`
    ///   match outright, with Standard (affinity 0) always exempt.
    /// - The gem's `rank` doesn't exceed `max_gem_rank`.
    /// - A special sword art gem isn't blocked by this weapon's
    ///   `restrict_special_sword_art`.
    ///
    /// A weapon carrying an ash innately says nothing about that ash being
    /// mountable: the innate `EquipParamGem` row and the droppable Ash of War
    /// item are separate rows with their own `can_mount_wep_*` flags. Golem's
    /// Halberd has Charge Forth innately (row `105`, which allows its
    /// `AxhammerLarge` type), while the obtainable Charge Forth (row `10500`)
    /// doesn't allow that type at all — so it can never be infused with its
    /// own default ash. That falls out of the `wep_type` check with no
    /// special-casing.
    pub fn can_mount_gem(
        &self,
        gem: &EQUIP_PARAM_GEM_ST,
        affinity_id: u32,
        max_gem_rank: i8,
    ) -> bool {
        if !matches!(self.gem_mount_type(), 1 | 2) {
            return false;
        }
        if !gem.can_mount_wep_type(self.wep_type()) {
            return false;
        }
        if self.disable_gem_attr() {
            if affinity_id != 0 && gem.default_wep_attr() as u32 != affinity_id {
                return false;
            }
        } else if !gem.is_affinity_configurable(affinity_id) {
            return false;
        }
        if gem.rank() > max_gem_rank {
            return false;
        }
        if self.restrict_special_sword_art() & 1 != 0 && gem.is_special_sword_art() & 1 != 0 {
            return false;
        }

        true
    }
}

impl EQUIP_PARAM_GEM_ST {
    /// Whether this gem can be applied at `affinity_id`, per its 24-bit
    /// `configurable_wep_attr` whitelist — the affinities the ash offers when
    /// infusing, as opposed to [`default_wep_attr`](Self::default_wep_attr),
    /// the single one it lands on by default.
    ///
    /// Mirrors `EquipParamGem::IsAffinityConfigurable`, which reads the same
    /// 24 flags into a stack array and indexes it, returning `false` for
    /// anything outside `0..24`.
    pub fn is_affinity_configurable(&self, affinity_id: u32) -> bool {
        match affinity_id {
            0 => self.configurable_wep_attr00(),
            1 => self.configurable_wep_attr01(),
            2 => self.configurable_wep_attr02(),
            3 => self.configurable_wep_attr03(),
            4 => self.configurable_wep_attr04(),
            5 => self.configurable_wep_attr05(),
            6 => self.configurable_wep_attr06(),
            7 => self.configurable_wep_attr07(),
            8 => self.configurable_wep_attr08(),
            9 => self.configurable_wep_attr09(),
            10 => self.configurable_wep_attr10(),
            11 => self.configurable_wep_attr11(),
            12 => self.configurable_wep_attr12(),
            13 => self.configurable_wep_attr13(),
            14 => self.configurable_wep_attr14(),
            15 => self.configurable_wep_attr15(),
            16 => self.configurable_wep_attr16(),
            17 => self.configurable_wep_attr17(),
            18 => self.configurable_wep_attr18(),
            19 => self.configurable_wep_attr19(),
            20 => self.configurable_wep_attr20(),
            21 => self.configurable_wep_attr21(),
            22 => self.configurable_wep_attr22(),
            23 => self.configurable_wep_attr23(),
            _ => false,
        }
    }

    /// `can_mount_wep_*` selected by `EQUIP_PARAM_WEAPON_ST::wep_type`,
    /// matching `CheckIfWepTypeCanEquipGem`'s switch.
    fn can_mount_wep_type(&self, wep_type: u16) -> bool {
        match wep_type {
            1 => self.can_mount_wep_dagger(),
            3 => self.can_mount_wep_sword_normal(),
            5 => self.can_mount_wep_sword_large(),
            7 => self.can_mount_wep_sword_gigantic(),
            9 => self.can_mount_wep_saber_normal(),
            11 => self.can_mount_wep_saber_large(),
            13 => self.can_mount_wep_katana(),
            14 => self.can_mount_wep_sword_double_edge(),
            15 => self.can_mount_wep_sword_pierce(),
            16 => self.can_mount_wep_rapier_heavy(),
            17 => self.can_mount_wep_axe_normal(),
            19 => self.can_mount_wep_axe_large(),
            21 => self.can_mount_wep_hammer_normal(),
            23 => self.can_mount_wep_hammer_large(),
            24 => self.can_mount_wep_flail(),
            25 => self.can_mount_wep_spear_normal(),
            27 => self.can_mount_wep_spear_large(),
            28 => self.can_mount_wep_spear_heavy(),
            29 => self.can_mount_wep_spear_axe(),
            31 => self.can_mount_wep_sickle(),
            35 => self.can_mount_wep_knuckle(),
            37 => self.can_mount_wep_claw(),
            39 => self.can_mount_wep_whip(),
            41 => self.can_mount_wep_axhammer_large(),
            50 => self.can_mount_wep_bow_small(),
            51 => self.can_mount_wep_bow_normal(),
            53 => self.can_mount_wep_bow_large(),
            55 => self.can_mount_wep_closs_bow(),
            56 => self.can_mount_wep_ballista(),
            57 => self.can_mount_wep_staff(),
            59 => self.can_mount_wep_sorcery(),
            61 => self.can_mount_wep_talisman(),
            65 => self.can_mount_wep_shield_small(),
            67 => self.can_mount_wep_shield_normal(),
            69 => self.can_mount_wep_shield_large(),
            87 => self.can_mount_wep_torch(),
            88 => self.can_mount_wep_hand_to_hand(),
            89 => self.can_mount_wep_perfume_bottle(),
            90 => self.can_mount_wep_thrusting_shield(),
            91 => self.can_mount_wep_throwing_weapon(),
            92 => self.can_mount_wep_reverse_hand_sword(),
            93 => self.can_mount_wep_light_greatsword(),
            94 => self.can_mount_wep_great_katana(),
            95 => self.can_mount_wep_beast_claw(),
            _ => false,
        }
    }
}

/// A trait that contains the fields shared across all four equipment
/// parameters.
#[multi_param(
    EQUIP_PARAM_ACCESSORY_ST,
    EQUIP_PARAM_GEM_ST,
    EQUIP_PARAM_GOODS_ST,
    EQUIP_PARAM_PROTECTOR_ST,
    EQUIP_PARAM_WEAPON_ST
)]
pub trait EquipParam {
    fields! {
        sell_value: i32,
        sort_id: i32,
        sort_group_id: u8,
        rarity: u8,
        sale_value: i32,
        is_deposit: bool,
        is_discard: bool,
        is_drop: bool,
    }

    /// Returns this as an [EQUIP_PARAM_ACCESSORY_ST], if it is one.
    fn as_accessory(&self) -> Option<&EQUIP_PARAM_ACCESSORY_ST> {
        if let EquipParamStruct::EQUIP_PARAM_ACCESSORY_ST(s) = self.as_enum() {
            Some(s)
        } else {
            None
        }
    }

    /// Returns this as a mutable [EQUIP_PARAM_ACCESSORY_ST], if it is one.
    fn as_accessory_mut(&mut self) -> Option<&mut EQUIP_PARAM_ACCESSORY_ST> {
        if let EquipParamStructMut::EQUIP_PARAM_ACCESSORY_ST(s) = self.as_enum_mut() {
            Some(s)
        } else {
            None
        }
    }

    /// Returns this as an [EQUIP_PARAM_GOODS_ST], if it is one.
    fn as_goods(&self) -> Option<&EQUIP_PARAM_GOODS_ST> {
        if let EquipParamStruct::EQUIP_PARAM_GOODS_ST(s) = self.as_enum() {
            Some(s)
        } else {
            None
        }
    }

    /// Returns this as a mutable [EQUIP_PARAM_GOODS_ST], if it is one.
    fn as_goods_mut(&mut self) -> Option<&mut EQUIP_PARAM_GOODS_ST> {
        if let EquipParamStructMut::EQUIP_PARAM_GOODS_ST(s) = self.as_enum_mut() {
            Some(s)
        } else {
            None
        }
    }

    /// Returns this as an [EQUIP_PARAM_PROTECTOR_ST], if it is one.
    fn as_protector(&self) -> Option<&EQUIP_PARAM_PROTECTOR_ST> {
        if let EquipParamStruct::EQUIP_PARAM_PROTECTOR_ST(s) = self.as_enum() {
            Some(s)
        } else {
            None
        }
    }

    /// Returns this as a mutable [EQUIP_PARAM_PROTECTOR_ST], if it is one.
    fn as_protector_mut(&mut self) -> Option<&mut EQUIP_PARAM_PROTECTOR_ST> {
        if let EquipParamStructMut::EQUIP_PARAM_PROTECTOR_ST(s) = self.as_enum_mut() {
            Some(s)
        } else {
            None
        }
    }

    /// Returns this as an [EQUIP_PARAM_WEAPON_ST], if it is one.
    fn as_weapon(&self) -> Option<&EQUIP_PARAM_WEAPON_ST> {
        if let EquipParamStruct::EQUIP_PARAM_WEAPON_ST(s) = self.as_enum() {
            Some(s)
        } else {
            None
        }
    }

    /// Returns this as a mutable [EQUIP_PARAM_WEAPON_ST], if it is one.
    fn as_weapon_mut(&mut self) -> Option<&mut EQUIP_PARAM_WEAPON_ST> {
        if let EquipParamStructMut::EQUIP_PARAM_WEAPON_ST(s) = self.as_enum_mut() {
            Some(s)
        } else {
            None
        }
    }
}

/// A trait that contains the fields shared across all equipment parameters that
/// aren't armor.
#[multi_param(
    EQUIP_PARAM_ACCESSORY_ST,
    EQUIP_PARAM_GEM_ST,
    EQUIP_PARAM_GOODS_ST,
    EQUIP_PARAM_WEAPON_ST
)]
pub trait EquipParamNonProtector: EquipParam {
    fields! {
        icon_id: u16,
        trophy_seq_id: i16,
    }
}

/// A trait that contains the fields shared across the four equipment parameters
/// that typically represent physical objects (everything but ashes of war).
#[multi_param(
    EQUIP_PARAM_ACCESSORY_ST,
    EQUIP_PARAM_GOODS_ST,
    EQUIP_PARAM_PROTECTOR_ST,
    EQUIP_PARAM_WEAPON_ST
)]
pub trait EquipParamPhysical: EquipParam {
    fields! {
        weight: f32,
    }
}

/// A trait that contains the fields shared across the equipment parameters that
/// the player can wear as equipment (talismans, armor, and weapons).
#[multi_param(
    EQUIP_PARAM_ACCESSORY_ST,
    EQUIP_PARAM_PROTECTOR_ST,
    EQUIP_PARAM_WEAPON_ST
)]
pub trait EquipParamWearable: EquipParamPhysical {
    fields! {
        equip_model_id: u16,
        trophy_s_grade_id: i16,
        equip_model_category: u8,
        equip_model_gender: u8,
        #[multi_param(
            rename(param = EQUIP_PARAM_PROTECTOR_ST, name = "resident_sp_effect_id"),
            rename(param = EQUIP_PARAM_WEAPON_ST, name = "resident_sp_effect_id"),
        )]
        resident_sp_effect_id1: i32,
        #[multi_param(rename(param = EQUIP_PARAM_WEAPON_ST, name = "resident_sp_effect_id1"))]
        resident_sp_effect_id2: i32,
        #[multi_param(rename(param = EQUIP_PARAM_WEAPON_ST, name = "resident_sp_effect_id2"))]
        resident_sp_effect_id3: i32,
    }
}

/// A trait that contains the fields shared across the equipment parameters that
/// don't involve weapons (talismans, armor, and goods).
#[multi_param(
    EQUIP_PARAM_ACCESSORY_ST,
    EQUIP_PARAM_GOODS_ST,
    EQUIP_PARAM_PROTECTOR_ST
)]
pub trait EquipParamNonWeapon: EquipParamPhysical {
    fields! {
        basic_price: i32,
        shop_lv: i16,
    }
}

/// A trait that contains the fields shared across the equipment parameters that
/// commonly provide passive effects (goods and talismans).
#[multi_param(EQUIP_PARAM_ACCESSORY_ST, EQUIP_PARAM_GOODS_ST)]
pub trait EquipParamPassive: EquipParamPhysical {
    fields! {
        sfx_variation_id: i32,
        behavior_id: i32,
        basic_price: i32,
        ref_category: u8,
        sp_effect_category: u8,
        vagrant_item_lot_id: i32,
        vagrant_bonus_ene_drop_item_lot_id: i32,
        vagrant_item_ene_drop_item_lot_id: i32,
    }
}
