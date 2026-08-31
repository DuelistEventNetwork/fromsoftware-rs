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
    /// Head of the free-slot ring — the position `checkout_bare_handle` pops
    /// from. `freeTableIdxQueueHeadId` at offset `0x19008`; it precedes the
    /// tail in memory, so the two must not be declared the other way round:
    /// swapping them makes allocation pop from the tail and frees push onto
    /// the head, handing out indices whose gaitems are still live and
    /// overwriting them in place (no generation bump, since `free_slot` never
    /// ran on them).
    pub read_index: u32,
    /// Tail of the free-slot ring — the position `free_slot` pushes onto.
    /// `freeTableIdxQueueEndId` at offset `0x1900c`.
    pub write_index: u32,
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

        // Mirrors `CheckoutBareGaitemHandle`: the descriptor's stored value is
        // reused verbatim as the low 24 bits, *including* bit 23
        // (`is_indexed`), which is pre-seeded per pool slot and preserved
        // across checkout/free cycles rather than being re-derived here. Only
        // the category nibble is rewritten.
        let stored = GaitemHandle(self.gaitem_descriptors[index as usize].unindexed_gaitem_handle);
        let mut handle = GaitemHandle(0);
        handle.set_uncategorized(stored.uncategorized());
        handle.set_category_raw(category as u8);
        handle.set_category_flag(true);
        debug_assert_eq!(
            handle.index(),
            index,
            "gaitem descriptor {index} stores a handle for slot {}",
            handle.index(),
        );
        self.gaitem_descriptors[index as usize].unindexed_gaitem_handle = handle.0;
        Some(handle)
    }

    /// Returns a slot to the free pool. Mirrors the free-list push performed
    /// by `RemoveCSGaitemIns`.
    ///
    /// The descriptor's stored handle is **not** cleared: its index is kept
    /// and its generation counter bumped, exactly as the real function does.
    /// Every slot's descriptor must always decode back to its own slot
    /// number — `checkout_bare_handle` reuses the stored value verbatim as
    /// the next handle's payload, and `increase_ref_count`/`release_handle`
    /// find a descriptor by `handle.index()`. Zeroing it here would detach
    /// descriptors from their slots on the next reuse, handing out handles
    /// whose index points at an unrelated pool row.
    fn free_slot(&mut self, index: u32) {
        self.gaitems[index as usize] = None;

        let descriptor = &mut self.gaitem_descriptors[index as usize];
        let stored = GaitemHandle(descriptor.unindexed_gaitem_handle);

        let mut handle = GaitemHandle(0);
        handle.set_generation(stored.generation().wrapping_add(1));
        handle.set_index(index);
        handle.set_is_indexed(true);

        descriptor.unindexed_gaitem_handle = handle.uncategorized();
        descriptor.ref_count = 0;

        // The tail is advanced *before* the write, matching `RemoveCSGaitemIns`:
        //
        //     uVar2 = (freeTableIdxQueueEndId + 1) % 0x1400;
        //     freeTableIdxQueueEndId = uVar2;
        //     freeTableIdxQueue[uVar2] = index;
        //
        // so the slot lands at the *new* tail, not the old one. Writing at the
        // old tail instead puts the freed index on the position the next
        // `checkout_bare_handle` is about to read while the tail moves past it,
        // which desynchronises the ring: `read_index` overruns `write_index`,
        // the `read_index == write_index` empty test stops firing, and
        // checkouts start returning the queue's untouched initial contents
        // (seeded `freeTableIdxQueue[i] = i`) — handing out indices whose
        // gaitems are still allocated and referenced.
        self.write_index = (self.write_index + 1) % 5120;
        self.indexes[self.write_index as usize] = index;
    }

    /// Allocates whichever handle kind `item_id`'s category calls for: a bare
    /// one for Goods and Accessories, an indexed one for the rest.
    ///
    /// Mirrors `GetGaitemHandleByItemId`.
    pub fn allocate_for_item(&mut self, item_id: ItemId) -> Option<GaitemHandle> {
        match item_id.category() {
            ItemCategory::Goods => {
                Some(self.allocate_partial_gaitem(GaitemCategory::Goods, item_id.param_id()))
            }
            ItemCategory::Accessory => {
                Some(self.allocate_partial_gaitem(GaitemCategory::Accessory, item_id.param_id()))
            }
            ItemCategory::Weapon | ItemCategory::Protector | ItemCategory::Gem => {
                self.allocate_indexed_gaitem(item_id)
            }
        }
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

        let stored = match category {
            GaitemCategory::Weapon => OwnedPtr::new_subclass(CSWepGaitemIns::new(handle, item_id)),
            GaitemCategory::Protector => {
                OwnedPtr::new_subclass(CSProGaitemIns::new(handle, item_id))
            }
            GaitemCategory::Gem => OwnedPtr::new_subclass(CSGemGaitemIns::new(handle, item_id)),
            _ => unreachable!(),
        };
        self.gaitems[index] = Some(stored);

        // `IncreaseGaitemHandleRefCount`, exactly as the real
        // `GetGaItemHandleWeapon`/`GetGaItemHandleProtector`/
        // `GetGaItemHandleGem` do after `CheckoutBareGaitemHandle` — an
        // *increment*, never an assignment. A freshly-freed slot sits at 0, so
        // this normally lands on 1 either way; the difference matters when the
        // slot is handed out while a stale handle still references it, where
        // assigning would silently reset a live count and leave that handle
        // pointing at a slot now holding a different item.
        self.increase_ref_count(handle);
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

    /// Mounts the ash of war backed by `ash_handle` onto the weapon backed
    /// by `weapon_handle`, replacing whatever ash the weapon currently has
    /// mounted (if any). A no-op if either handle doesn't resolve to a real
    /// [`CSWepGaitemIns`]/[`CSGemGaitemIns`] instance.
    ///
    /// Follows the game's own equip-ash-of-war path: the weapon's single gem
    /// slot (`gem_slot_table.gem_slots[0]`) holds the ash's [`GaitemHandle`]
    /// via [`swap_handle`](Self::swap_handle) rather than a bare assignment,
    /// and the ash gaitem's own `weapon_handle` back-pointer is set directly
    /// — a plain field write, not ref-counted, since it's only a
    /// back-pointer. Mirrors `CS::GaitemLookupResult::SetGemOwningWeaponHandle`.
    pub fn equip_ash_of_war(&mut self, weapon_handle: GaitemHandle, ash_handle: GaitemHandle) {
        let Some(current_ash) = self
            .gaitem_ins_by_handle(&weapon_handle)
            .and_then(|ins| ins.as_subclass::<CSWepGaitemIns>())
            .map(|weapon| weapon.gem_slot_table.gem_slots[0].gaitem_handle)
        else {
            return;
        };

        if current_ash != ash_handle {
            // Mirrors swap_handle's ref-count adjustment, done in two steps
            // since the slot being written (gem_slots[0].gaitem_handle) lives
            // inside `self.gaitems` itself — swap_handle needs an exclusive
            // `&mut GaitemHandle` into that same array, which can't coexist
            // with the `&mut self` calls it also needs to make internally.
            self.increase_ref_count(ash_handle);
            self.release_handle(current_ash);

            if let Some(weapon) = self
                .gaitem_ins_by_handle_mut(&weapon_handle)
                .and_then(|ins| ins.as_subclass_mut::<CSWepGaitemIns>())
            {
                weapon.gem_slot_table.gem_slots[0].gaitem_handle = ash_handle;
            }
        }

        let Some(ash) = self
            .gaitem_ins_by_handle_mut(&ash_handle)
            .and_then(|ins| ins.as_subclass_mut::<CSGemGaitemIns>())
        else {
            return;
        };
        ash.weapon_handle = weapon_handle;
    }
}

#[repr(C)]
#[derive(Superclass)]
#[superclass(children(CSWepGaitemIns, CSGemGaitemIns, CSProGaitemIns))]
pub struct CSGaitemIns {
    vftable: usize,
    pub gaitem_handle: GaitemHandle,
    /// The item this instance represents, category nibble included.
    ///
    /// Each category has its own setter, but they're all the same operation —
    /// `(id & 0xfffffff) | (category << 28)`:
    ///
    /// | setter | value |
    /// |---|---|
    /// | `SetItemIdWithWeaponCategory` | `id & 0xfffffff` |
    /// | `SetItemIdWithProtectorCategory` | `id & 0xfffffff \| 0x10000000` |
    /// | `SetItemIdWithGemCategory` | `id & 0xfffffff \| 0x80000000` |
    ///
    /// The weapon one only *looks* like it strips the category because
    /// [`ItemCategory::Weapon`] is 0, so its OR is a no-op — reading it as
    /// "store a bare param id" and applying that to the other two writes a
    /// Protector or Gem with a zeroed category nibble, which then resolves as
    /// a Weapon everywhere the id is consumed.
    pub item_id: OptionalItemId,
}

impl CSGaitemIns {
    /// Builds the [`CSGaitemIns`] base for a subclass, with `vftable` left
    /// as `0` — every subclass's own `new` must overwrite it with its own
    /// [`Subclass::vmt_va`] right after calling this, since a `CSGaitemIns`
    /// on its own (vtable pointing at the wrong, superclass-only layout)
    /// isn't a valid object.
    fn new_base(handle: GaitemHandle, item_id: ItemId) -> Self {
        Self {
            vftable: 0,
            gaitem_handle: handle,
            item_id: item_id.into(),
        }
    }
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

    /// The handle's payload: a pool index for indexed handles, or a param id
    /// for bare/partial ones.
    ///
    /// Only 23 bits wide, **not** 24 — bit 23 is [`is_indexed`](Self::is_indexed)
    /// and must never be written as part of the payload. The game enforces
    /// this in `RemoveCategoryFromGaitemHandleIfAny`, which masks its input
    /// with `0x7fffff` before `MakeBareGaitemHandle` ORs the category in, so
    /// a bare handle can never accidentally claim to be indexed.
    pub selector, _: 22, 0;
    _, set_selector: 22, 0;

    /// Generation counter for an indexed handle's pool slot, incremented
    /// every time that slot is freed so a stale handle to a recycled slot
    /// can be told apart from a live one.
    ///
    /// Mirrors the `+ 0x10000 & 0x7f0000` / `| gaitemIdx | 0x800000` pair the
    /// game applies in `RemoveCSGaitemIns`.
    pub generation, _: 22, 16;
    _, set_generation: 22, 16;

    /// Indicates if the gaitem handle refers to a GaitemIns available in CSGaitemImp.
    /// Will be true for Protectors, Weapons and Gems.
    pub is_indexed, _: 23;
    _, set_is_indexed: 23;

    /// Everything below the category nibble: the payload plus
    /// [`is_indexed`](Self::is_indexed).
    ///
    /// This is the part of a handle the game persists in a pool slot's
    /// descriptor and reuses verbatim across checkout/free cycles — the
    /// `& 0xffffff` that `CheckoutBareGaitemHandle` and `RemoveCSGaitemIns`
    /// apply before rewriting the category.
    pub uncategorized, _: 23, 0;
    _, set_uncategorized: 23, 0;

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
    /// Builds a handle from a payload and category.
    ///
    /// `selector` only occupies 23 bits, mirroring
    /// `RemoveCategoryFromGaitemHandleIfAny`: bit 23 is
    /// [`is_indexed`](Self::is_indexed), so letting a payload bit land there
    /// would produce a bare handle that claims to be indexed, whose
    /// [`index`](Self::index) then points at an unrelated `CSGaitemIns` in
    /// the pool. Anything wider is truncated by the field itself.
    pub fn from_parts(selector: u32, category: GaitemCategory) -> Self {
        let mut handle = GaitemHandle(0);
        handle.set_selector(selector);
        debug_assert_eq!(
            handle.selector(),
            selector,
            "gaitem handle selector {selector:#x} doesn't fit in 23 bits",
        );

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
                // Indexed: the payload is a pool index plus the generation
                // counter that distinguishes reuses of that slot.
                true => write!(
                    f,
                    "GaitemHandle({},gen {},{:?})",
                    self.index(),
                    self.generation(),
                    category
                ),
                // Bare: the payload is the item's param id, and there's no
                // pool slot behind it.
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

impl CSWepGaitemIns {
    fn new(handle: GaitemHandle, item_id: ItemId) -> Self {
        Self {
            gaitem_ins: CSGaitemIns {
                vftable: Self::vmt_va() as usize,
                ..CSGaitemIns::new_base(handle, item_id)
            },
            durability: 0,
            reinforcement_param_id: 0,
            gem_slot_table: CSGemSlotTable::new(),
        }
    }
}

#[repr(C)]
pub struct CSGemSlotTable {
    vtable: usize,
    pub gem_slots: [CSGemSlot; 1],
}

impl CSGemSlotTable {
    /// `CSGemSlotTable` doesn't derive [`Subclass`], since it isn't a
    /// subclass of [`CSGaitemIns`], so its vtable is looked up directly by
    /// RVA instead of through [`Subclass::vmt_va`].
    fn new() -> Self {
        let vtable = Program::current()
            .rva_to_va(rva::get().csgem_slot_table_vmt)
            .expect("csgem_slot_table_vmt RVA not found in executable")
            as usize;

        Self {
            vtable,
            gem_slots: [CSGemSlot::new(GaitemHandle(0))],
        }
    }
}

#[repr(C)]
pub struct CSGemSlot {
    vtable: usize,
    /// Refers to the actual gem entry in the CSGaitemImp.
    pub gaitem_handle: GaitemHandle,
}

impl CSGemSlot {
    /// See [`CSGemSlotTable::new`] for why this is looked up directly rather
    /// than through [`Subclass::vmt_va`].
    fn new(gaitem_handle: GaitemHandle) -> Self {
        let vtable = Program::current()
            .rva_to_va(rva::get().csgem_slot_vmt)
            .expect("csgem_slot_vmt RVA not found in executable") as usize;

        Self {
            vtable,
            gaitem_handle,
        }
    }
}

#[repr(C)]
#[derive(Subclass)]
pub struct CSGemGaitemIns {
    pub gaitem_ins: CSGaitemIns,
    /// Handle of the weapon this gem is attached to
    pub weapon_handle: GaitemHandle,
}

impl CSGemGaitemIns {
    fn new(handle: GaitemHandle, item_id: ItemId) -> Self {
        Self {
            gaitem_ins: CSGaitemIns {
                vftable: Self::vmt_va() as usize,
                ..CSGaitemIns::new_base(handle, item_id)
            },
            weapon_handle: GaitemHandle(0),
        }
    }
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

impl CSProGaitemIns {
    fn new(handle: GaitemHandle, item_id: ItemId) -> Self {
        Self {
            gaitem_ins: CSGaitemIns {
                vftable: Self::vmt_va() as usize,
                ..CSGaitemIns::new_base(handle, item_id)
            },
            durability: 0,
            reinforcement: 0,
        }
    }
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

impl CSGaitemGameData {
    /// Marks `item_id` as acquired in the save-persistent "have you ever
    /// picked this up" tracker — mirrors `CS::CSGaitemGameData::UpdateItem`
    /// with its `acquired` argument hardcoded `true` (the only way real
    /// `AddInventoryEquip` ever calls it). `gaitem_entries` is kept sorted by
    /// `item_id`'s raw numeric value — the real function binary-searches it —
    /// so this finds `item_id`'s insertion
    /// point and either flips an existing entry's `already_acquired` flag or
    /// inserts a fresh `true` entry, shifting every following entry over by
    /// one — [`DLFixedVector`] itself only supports appending, so the shift
    /// is done by hand here. No-op if the vector is already full and
    /// `item_id` isn't already present.
    pub fn mark_acquired(&mut self, item_id: ItemId) {
        let raw = item_id.into_inner();
        let entries = self.gaitem_entries.as_mut_slice();

        let insertion_point = entries.partition_point(|entry| entry.item_id.into_inner() < raw);

        if let Some(existing) = entries.get_mut(insertion_point)
            && existing.item_id.into_inner() == raw
        {
            existing.already_acquired = true;
            return;
        }

        if self
            .gaitem_entries
            .push(CSGaitemGameDataEntry {
                item_id: item_id.into(),
                already_acquired: true,
            })
            .is_err()
        {
            return;
        }

        let entries = self.gaitem_entries.as_mut_slice();
        entries[insertion_point..].rotate_right(1);
    }
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
