use std::ptr::NonNull;

use crate::cs::ChrIns;

/// Index into [`CSChrResistModule::resistances`].
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusIndex {
    Poison = 0,
    ScarletRot = 1,
    Blood = 2,
    Curse = 3,
    Freeze = 4,
    Sleep = 5,
    Madness = 6,
}

#[repr(C)]
/// Holds the character's status-effect resistances
///
/// Source of name: RTTI
pub struct CSChrResistModule {
    vftable: usize,
    pub owner: NonNull<ChrIns>,
    unk10: [u8; 0x1c],
    pub resistances: [i32; 8],
    unk40: [u8; 0x6c],
    pub status_clear_flags: u32,
    /// Set to `true` to have the character's resistances recalculated.
    pub recalculation_requested: bool,
    unkbd: [u8; 3],
}

impl CSChrResistModule {
    pub fn resistance(&self, status: StatusIndex) -> i32 {
        self.resistances[status as usize]
    }
}
