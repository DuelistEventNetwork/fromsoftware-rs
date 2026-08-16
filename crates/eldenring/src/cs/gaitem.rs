use std::fmt::Display;

use bitfield::bitfield;
use pelite::pe64::Pe;
use thiserror::Error;

use crate::{
    cs::{CSRandXorshift, ItemCategory, ItemId, OptionalItemId},
    dlkr::MainHeapAllocator,
    dlut::DLFixedVector,
    rva,
};
use shared::{OwnedPtr, Program, Subclass, Superclass};

#[repr(C)]
#[shared::singleton("CSGaitem")]
pub struct CSGaitemImp {
    vftable: usize,
    pub gaitems: [Option<OwnedPtr<CSGaitemIns, MainHeapAllocator>>; 5120],
    pub gaitem_descriptors: [CSGaitemImpEntry; 5120],
    pub indexes: [u32; 5120],
    pub write_index: u32,
    pub read_index: u32,
    pub rand_xorshift: CSRandXorshift,
    unk23028: [u8; 8],
    /// Becomes true if the CSGaitemImp is being serialized for saving to the save file.
    pub is_being_serialized: bool,
    unk23038: [u8; 7],
}

impl CSGaitemImp {
    pub fn gaitem_ins_by_handle(&self, handle: &GaitemHandle) -> Option<&CSGaitemIns> {
        // Can't do a lookup for a handle that is not supposed to be in here anyway.
        if !handle.is_indexed() {
            return None;
        }

        let index = handle.index() as usize;
        if index > self.gaitems.len() {
            return None;
        }

        Some(self.gaitems[index].as_ref()?.as_ref())
    }

    pub fn gaitem_ins_by_handle_mut(&mut self, handle: &GaitemHandle) -> Option<&mut CSGaitemIns> {
        // Can't do a lookup for a handle that is not supposed to be in here anyway.
        if !handle.is_indexed() {
            return None;
        }

        let index = handle.index() as usize;
        if index > self.gaitems.len() {
            return None;
        }

        Some(self.gaitems[index].as_mut()?.as_mut())
    }

    /// The number of free slots currently available in the gaitem pool, i.e.
    /// how many more indexed [`GaitemHandle`]s (Weapon/Protector/Gem) can be
    /// checked out via [`allocate_indexed_gaitem`](Self::allocate_indexed_gaitem)
    /// before it's exhausted.
    ///
    /// Because the pool's free list is a ring buffer that only distinguishes
    /// "empty" from "not empty" by comparing `read_index`/`write_index`
    /// (see [`checkout_bare_handle`](Self::checkout_bare_handle)), at most
    /// 5119 of the 5120 total slots can ever be free at once — one slot's
    /// worth of the ring is always unrepresentable as free.
    pub fn free_slot_count(&self) -> u32 {
        (self.write_index + 5120 - self.read_index) % 5120
    }

    /// Reserves a free slot in the gaitem pool for `category` and returns a
    /// [`GaitemHandle`] pointing at it, without constructing a backing
    /// [`CSGaitemIns`] yet. Returns `None` if the pool is exhausted.
    ///
    /// Mirrors `CheckoutBareGaitemHandle`.
    fn checkout_bare_handle(&mut self, category: GaitemCategory) -> Option<GaitemHandle> {
        if self.read_index == self.write_index {
            return None;
        }

        let index = self.indexes[self.read_index as usize];
        self.read_index = (self.read_index + 1) % 5120;

        let selector = self.gaitem_descriptors[index as usize].unindexed_gaitem_handle & 0xffffff;
        let handle = GaitemHandle::from_parts(selector, category);
        self.gaitem_descriptors[index as usize].unindexed_gaitem_handle = handle.0;
        Some(handle)
    }

    /// Returns a slot to the free pool and clears its descriptor. Mirrors the
    /// free-list push performed by `RemoveCSGaitemIns`.
    fn free_slot(&mut self, index: u32) {
        self.gaitems[index as usize] = None;
        self.gaitem_descriptors[index as usize] = CSGaitemImpEntry {
            unindexed_gaitem_handle: 0,
            ref_count: 0,
        };
        self.indexes[self.write_index as usize] = index;
        self.write_index = (self.write_index + 1) % 5120;
    }

    /// Allocates and registers a [`CSGaitemIns`]-family instance of the
    /// correct concrete type for `category`, on the game's main heap,
    /// returning a [`GaitemHandle`] with a starting ref count of 1.
    ///
    /// Only valid for [`GaitemCategory::Weapon`], [`GaitemCategory::Protector`],
    /// and [`GaitemCategory::Gem`] — the categories backed by a real instance.
    /// Mirrors `GetGaItemHandleWeapon`/`GetGaItemHandleProtector`/`GetGaItemHandleGem`.
    pub fn allocate_indexed_gaitem(&mut self, item_id: ItemId) -> Option<GaitemHandle> {
        let category = match item_id.category() {
            ItemCategory::Weapon => GaitemCategory::Weapon,
            ItemCategory::Protector => GaitemCategory::Protector,
            ItemCategory::Gem => GaitemCategory::Gem,
            _ => return None,
        };

        let handle = self.checkout_bare_handle(category)?;
        let index = handle.index() as usize;

        let base = CSGaitemIns {
            vftable: 0,
            gaitem_handle: handle,
            item_id: OptionalItemId::from(item_id.into_inner()),
        };

        // Each subclass's vftable is set up front, before allocating, since
        // `OwnedPtr::new_subclass` takes the value by move and only hands
        // back an `OwnedPtr<CSGaitemIns, _>` afterward, which can no longer
        // see the concrete type to fix up its vftable field.
        let stored = match category {
            GaitemCategory::Weapon => OwnedPtr::new_subclass(CSWepGaitemIns {
                gaitem_ins: CSGaitemIns {
                    vftable: CSWepGaitemIns::vmt_va() as usize,
                    ..base
                },
                durability: 0,
                reinforcement_param_id: 0,
                gem_slot_table: CSGemSlotTable {
                    vtable: csgem_slot_table_vmt(),
                    gem_slots: [CSGemSlot {
                        vtable: csgem_slot_vmt(),
                        gaitem_handle: GaitemHandle(0),
                    }],
                },
            }),
            GaitemCategory::Protector => OwnedPtr::new_subclass(CSProGaitemIns {
                gaitem_ins: CSGaitemIns {
                    vftable: CSProGaitemIns::vmt_va() as usize,
                    ..base
                },
                durability: 0,
                reinforcement: 0,
            }),
            GaitemCategory::Gem => OwnedPtr::new_subclass(CSGemGaitemIns {
                gaitem_ins: CSGaitemIns {
                    vftable: CSGemGaitemIns::vmt_va() as usize,
                    ..base
                },
                weapon_handle: GaitemHandle(0),
            }),
            _ => unreachable!(),
        };
        self.gaitems[index] = Some(stored);

        self.gaitem_descriptors[index].ref_count = 1;
        Some(handle)
    }

    /// Builds a [`GaitemHandle`] for a non-indexed category
    /// ([`GaitemCategory::Goods`] or [`GaitemCategory::Accessory`]), which
    /// isn't backed by a [`CSGaitemIns`] and requires no heap allocation.
    ///
    /// Mirrors `GetGaItemHandleGoods`/`GetGaItemHandleAccessory` via
    /// `MakeBareGaitemHandle`.
    pub fn allocate_partial_gaitem(&self, category: GaitemCategory, param_id: u32) -> GaitemHandle {
        GaitemHandle::from_parts(param_id, category)
    }

    /// Increments the ref count of the [`CSGaitemIns`] backing `handle`. A
    /// no-op for non-indexed handles or the null handle.
    ///
    /// Mirrors `IncreaseGaitemHandleRefCount`.
    pub fn increase_ref_count(&mut self, handle: GaitemHandle) {
        if !handle.is_indexed() || handle.0 == 0 {
            return;
        }

        let index = handle.index() as usize;
        if index >= self.gaitem_descriptors.len() {
            return;
        }

        if self.gaitem_descriptors[index].unindexed_gaitem_handle == handle.0 {
            self.gaitem_descriptors[index].ref_count += 1;
        }
    }

    /// Releases a reference to the [`CSGaitemIns`] backing `handle`,
    /// deallocating it and freeing its slot back to the pool if this was the
    /// last reference. A no-op for non-indexed handles or the null handle.
    ///
    /// Mirrors `CS::GaItemImp::RemoveHandle`.
    pub fn release_handle(&mut self, handle: GaitemHandle) {
        if !handle.is_indexed() || handle.0 == 0 {
            return;
        }

        let index = handle.index() as usize;
        if index >= self.gaitem_descriptors.len() {
            return;
        }

        if self.gaitem_descriptors[index].unindexed_gaitem_handle != handle.0 {
            return;
        }

        if self.gaitem_descriptors[index].ref_count < 2 {
            self.gaitem_descriptors[index].ref_count = 0;
            self.free_slot(index as u32);
        } else {
            self.gaitem_descriptors[index].ref_count -= 1;
        }
    }

    /// Overwrites `*slot` with `new_handle`, correctly adjusting ref counts
    /// on both the old and new handle (incrementing the new one first, then
    /// releasing the old one). A no-op if `*slot == new_handle`.
    ///
    /// **Any write to a [`GaitemHandle`] field that might already hold a
    /// live handle (an inventory entry, a `ChrAsm` equip slot, ...) must go
    /// through this method rather than a bare assignment**, or it will leak
    /// the old handle's `CSGaitemIns` allocation and ref count.
    ///
    /// Mirrors `swapInventoryItemGaItemHandles_`.
    pub fn swap_handle(&mut self, slot: &mut GaitemHandle, new_handle: GaitemHandle) {
        if *slot == new_handle {
            return;
        }

        self.increase_ref_count(new_handle);
        self.release_handle(*slot);
        *slot = new_handle;
    }
}

/// The VA of `CS::CSGemSlotTable`'s vtable.
///
/// `CSGemSlotTable` doesn't derive [`Subclass`], since it isn't a subclass of
/// [`CSGaitemIns`], so its vtable is looked up directly by RVA instead of
/// through [`Subclass::vmt_va`].
fn csgem_slot_table_vmt() -> usize {
    Program::current()
        .rva_to_va(rva::get().csgem_slot_table_vmt)
        .expect("csgem_slot_table_vmt RVA not found in executable") as usize
}

/// The VA of `CS::CSGemSlot`'s vtable. See [`csgem_slot_table_vmt`] for why
/// this is looked up directly rather than through [`Subclass::vmt_va`].
fn csgem_slot_vmt() -> usize {
    Program::current()
        .rva_to_va(rva::get().csgem_slot_vmt)
        .expect("csgem_slot_vmt RVA not found in executable") as usize
}

#[repr(C)]
#[derive(Superclass)]
#[superclass(children(CSWepGaitemIns, CSGemGaitemIns, CSProGaitemIns))]
pub struct CSGaitemIns {
    vftable: usize,
    pub gaitem_handle: GaitemHandle,
    pub item_id: OptionalItemId,
}

#[repr(C)]
pub struct CSGaitemImpEntry {
    pub unindexed_gaitem_handle: u32,
    pub ref_count: u32,
}

bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq, Hash)]
    pub struct GaitemHandle(u32);
    impl Debug;

    /// The index of the GaitemIns inside the CSGaitemImp.
    pub index, _: 15, 0;
    _, set_index: 15, 0;

    pub selector, _: 23, 0;
    _, set_selector: 23, 0;

    /// Indicates if the gaitem handle refers to a GaitemIns available in CSGaitemImp.
    /// Will be true for Protectors, Weapons and Gems.
    pub is_indexed, _: 23;
    _, set_is_indexed: 23;

    u8;
    /// The category of the GaitemHandle.
    pub category_raw, _: 30, 28;
    _, set_category_raw: 30, 28;

    /// A flag that is always set along with the category.
    /// Separated into it's own bitfield to avoid bitshifts on the category.
    category_flag, set_category_flag: 31;
}

#[derive(Debug, Error)]
pub enum GaitemHandleError {
    #[error("Not a valid Gaitem handle category {0}")]
    InvalidCategory(u8),
}

impl GaitemHandle {
    pub fn from_parts(selector: u32, category: GaitemCategory) -> Self {
        let mut handle = GaitemHandle(0);
        handle.set_selector(selector);
        handle.set_category_raw(category as u8);
        handle.set_category_flag(true);
        handle
    }

    pub fn category(self) -> Result<GaitemCategory, GaitemHandleError> {
        GaitemCategory::try_from(self.category_raw())
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GaitemCategory {
    Weapon = 0,
    Protector = 1,
    Accessory = 2,
    Goods = 3,
    Gem = 4,
}

impl TryFrom<u8> for GaitemCategory {
    type Error = GaitemHandleError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(GaitemCategory::Weapon),
            1 => Ok(GaitemCategory::Protector),
            2 => Ok(GaitemCategory::Accessory),
            3 => Ok(GaitemCategory::Goods),
            4 => Ok(GaitemCategory::Gem),
            _ => Err(GaitemHandleError::InvalidCategory(value)),
        }
    }
}

impl Display for GaitemHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.category() {
            Ok(category) => match self.is_indexed() {
                true => write!(
                    f,
                    "GaitemHandle({},0x{:x},{:?})",
                    self.index(),
                    self.selector(),
                    category
                ),
                false => write!(f, "GaitemHandle(-1,{},{:?})", self.selector(), category),
            },
            Err(err) => write!(f, "GaitemHandle(0x{:x},{:?})", self.0, err),
        }
    }
}

#[repr(C)]
#[derive(Subclass)]
pub struct CSWepGaitemIns {
    pub gaitem_ins: CSGaitemIns,
    /// Item durability mechanic. Unused in ER.
    pub durability: u32,
    /// DS3 leftover from when weapons reinforcement was tracked here.
    pub reinforcement_param_id: u32,
    /// Gem slots, used for ashes of war in ER.
    pub gem_slot_table: CSGemSlotTable,
}

#[repr(C)]
pub struct CSGemSlotTable {
    vtable: usize,
    pub gem_slots: [CSGemSlot; 1],
}

#[repr(C)]
pub struct CSGemSlot {
    vtable: usize,
    /// Refers to the actual gem entry in the CSGaitemImp.
    pub gaitem_handle: GaitemHandle,
}

#[repr(C)]
#[derive(Subclass)]
pub struct CSGemGaitemIns {
    pub gaitem_ins: CSGaitemIns,
    /// Handle of the weapon this gem is attached to
    pub weapon_handle: GaitemHandle,
}

#[repr(C)]
#[derive(Subclass)]
pub struct CSProGaitemIns {
    pub gaitem_ins: CSGaitemIns,
    /// Item durability mechanic. Unused in ER.
    pub durability: u32,
    /// Reinforcement mechanic. Unused in ER.
    pub reinforcement: u32,
}

#[repr(C)]
pub struct CSGaitemGameDataEntry {
    pub item_id: OptionalItemId,
    pub already_acquired: bool,
}

#[repr(C)]
pub struct CSGaitemGameData {
    pub igame_data_elem_vftable: usize,
    pub gaitem_entries: DLFixedVector<CSGaitemGameDataEntry, 14000>,
}

#[cfg(test)]
mod test {
    use crate::cs::{
        CSGaitemImp, CSGaitemIns, CSGemGaitemIns, CSGemSlot, CSGemSlotTable, CSWepGaitemIns,
    };

    #[test]
    fn proper_sizes() {
        assert_eq!(0x19038, size_of::<CSGaitemImp>());
        assert_eq!(0x10, size_of::<CSGaitemIns>());
        assert_eq!(0x30, size_of::<CSWepGaitemIns>());
        assert_eq!(0x18, size_of::<CSGemSlotTable>());
        assert_eq!(0x10, size_of::<CSGemSlot>());
        assert_eq!(0x18, size_of::<CSGemGaitemIns>());
    }
}
