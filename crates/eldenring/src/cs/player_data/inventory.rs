use std::ptr::NonNull;

use fromsoftware_shared::program::Program;
use pelite::pe::Pe;
use thiserror::Error;

use crate::dlkr::MainHeapAllocator;
use crate::{
    ArrayWithHeader, DLList,
    cs::{
        CSGaitemImp, CSMenuManImp, ChrAsmArmStyle, ChrAsmSlot, EquipDataItem, EquipGameData,
        EquipParamGem, EquipParamGoods, EquipParamWeapon, GaitemHandle, GameDataMan, ItemCategory,
        ItemId, ItemIdError, OptionalItemId, ReinforceParamWeapon, SoloParamRepository,
    },
    rva,
};
use shared::{
    FromStatic, IsEmpty, MaybeEmpty, NonEmptyIteratorExt, NonEmptyIteratorMutExt, OwnedPtr,
};

#[repr(C)]
pub struct InventoryItemListAccessor {
    pub head: NonNull<MaybeEmpty<EquipInventoryDataListEntry>>,
    pub length: NonNull<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    Key,
    Normal,
}

#[repr(C)]
pub struct InventoryItemsData {
    pub global_capacity: u32,

    pub normal_items_capacity: u32,
    pub normal_items_head: OwnedPtr<MaybeEmpty<EquipInventoryDataListEntry>, MainHeapAllocator>,

    /// How many normal-item entries are occupied. A count of live entries,
    /// not an extent: slots are sparse, and this tracks occupancy across the
    /// gaps rather than the highest slot in use (that's
    /// [`EquipInventoryData::highest_item_slot`]).
    pub normal_items_len: u32,

    pub key_items_capacity: u32,
    pub key_items_head: OwnedPtr<MaybeEmpty<EquipInventoryDataListEntry>, MainHeapAllocator>,

    /// Same occupancy semantics as [`normal_items_len`](Self::normal_items_len).
    pub key_items_len: u32,

    pub multiplay_key_items_capacity: u32,

    /// Holds key items that are available in multiplayer.
    ///
    /// Populated by `SwapKeyItemsAccessor` when entering multiplayer: it
    /// walks `key_items` and copies across every `Goods` entry whose
    /// `goodsType` is `GREAT_RUNE`, `REGENERATIVE_MATERIAL` or
    /// `WONDROUS_PHYSICK_TEAR`, preserving each one's quantity and `sort_id`.
    pub multiplay_key_items_head:
        OwnedPtr<MaybeEmpty<EquipInventoryDataListEntry>, MainHeapAllocator>,

    /// Same occupancy semantics as [`normal_items_len`](Self::normal_items_len).
    pub multiplay_key_items_len: u32,

    /// Pointers to the active normal item list and its length. All
    /// inventory reads and writes go through this. Always the same as
    /// `normal_items`.
    pub normal_items_accessor: InventoryItemListAccessor,

    /// Pointers to the active key item list and its length. All inventory
    /// reads and writes go through this. In single-player this points at
    /// `key_items`; in multiplayer it switches to `multiplay_key_items`.
    pub key_items_accessor: InventoryItemListAccessor,

    pub item_id_mapping_indices: OwnedPtr<[i16; 2017], MainHeapAllocator>,
    pub item_id_mapping_pool_len: u32,
    pub item_id_mapping: OwnedPtr<ArrayWithHeader<ItemIdMapping>, MainHeapAllocator>,
    /// Index of the latest `item_id_mapping` free head, or -1 if none.
    pub item_id_mapping_free_head: i16,
}

impl InventoryItemsData {
    /// Returns an iterator over all the non-empty entries in the player's
    /// inventory, key items first.
    pub fn items(&self) -> impl Iterator<Item = &EquipInventoryDataListEntry> {
        self.current_key_entries()
            .iter()
            .chain(self.normal_entries().iter())
            .non_empty()
    }

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

    pub fn normal_entries(&self) -> &[MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts(
                self.normal_items_head.as_ptr(),
                self.normal_items_capacity as usize,
            )
        }
    }

    pub fn normal_entries_mut(&mut self) -> &mut [MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.normal_items_head.as_ptr(),
                self.normal_items_capacity as usize,
            )
        }
    }

    pub fn is_normal_items_full(&self) -> bool {
        self.normal_items_len >= self.normal_items_capacity
            && self.normal_entries().iter().all(|e| !e.is_empty())
    }

    /// A slice over the active key item entries, resolved through the
    /// accessor — key array in single-player, multiplay array in a session.
    pub fn current_key_entries(&self) -> &[MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts(
                self.key_items_accessor.head.as_ptr(),
                self.key_items_capacity as usize,
            )
        }
    }

    pub fn current_key_entries_mut(&mut self) -> &mut [MaybeEmpty<EquipInventoryDataListEntry>] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.key_items_accessor.head.as_ptr(),
                self.key_items_capacity as usize,
            )
        }
    }

    pub fn is_key_items_full(&self) -> bool {
        self.key_items_len >= self.key_items_capacity
            && self.current_key_entries().iter().all(|e| !e.is_empty())
    }

    /// Whether [`key_items_accessor`](Self::key_items_accessor) currently
    /// points at the multiplayer array.
    pub fn is_multiplayer_key_accessor(&self) -> bool {
        self.key_items_accessor.head.as_ptr() == self.multiplay_key_items_head.as_ptr()
    }

    pub fn entry_at_slot(&self, slot: u32) -> Option<&MaybeEmpty<EquipInventoryDataListEntry>> {
        let (container, offset) = self.container_for(slot)?;
        match container {
            Container::Key => self.current_key_entries().get(offset as usize),
            Container::Normal => self.normal_entries().get(offset as usize),
        }
    }

    pub fn entry_at_slot_mut(
        &mut self,
        slot: u32,
    ) -> Option<&mut MaybeEmpty<EquipInventoryDataListEntry>> {
        let (container, offset) = self.container_for(slot)?;
        match container {
            Container::Key => self.current_key_entries_mut().get_mut(offset as usize),
            Container::Normal => self.normal_entries_mut().get_mut(offset as usize),
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

    /// Pop one entry from the free list. Returns `None` if the pool is full.
    ///
    /// The popped entry's chain field holds the *next* free entry, 1-based
    /// with `0` meaning "no next" — so the new head is
    /// `next_chain_idx().unwrap_or(-1)`, matching
    /// `InsertItemIntoLookupMap`'s `((mapping >> 0xc) & 0xfff) - 1`, which
    /// lands on `-1` for the final entry.
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

    /// Upserts `(item_id -> inventory slot index)`, keeping the **minimum**
    /// slot index for each item_id.
    pub fn update_item_id_mapping(&mut self, item_id: ItemId, inventory_slot: i16) {
        let slot_bits = (inventory_slot as u16) & 0xfff;
        debug_assert_eq!(
            slot_bits, inventory_slot as u16,
            "inventory slot {inventory_slot} doesn't fit in the 12-bit item_slot field",
        );

        match self.find_chain_entry(item_id) {
            Some((_, cur_idx)) => {
                if let Some(e) = self.mapping_entry_mut(cur_idx)
                    && slot_bits < e.mapping.item_slot()
                {
                    e.mapping.set_item_slot(slot_bits);
                }
            }
            None => {
                let new_idx = match self.pop_free_entry() {
                    Some(i) => i,
                    None => return,
                };

                let bucket = Self::bucket_for(item_id);
                let first_raw = self.item_id_mapping_indices[bucket];

                if first_raw < 0 {
                    self.item_id_mapping_indices[bucket] = new_idx;
                } else {
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
    /// The map tracks the *lowest* slot holding a given id, so when the
    /// copy being removed is the one the map points at, the entry is
    /// repointed at the next-lowest remaining copy rather than unlinked —
    /// otherwise surviving copies become invisible to
    /// [`find_item_idx`](Self::find_item_idx).
    pub fn remove_item_id_mapping(&mut self, item_id: ItemId) {
        let (prev_idx, cur_idx) = match self.find_chain_entry(item_id) {
            Some(pair) => pair,
            None => return,
        };

        if let Some(lowest) = self.lowest_slot_holding(item_id) {
            if let Some(e) = self.mapping_entry_mut(cur_idx) {
                e.mapping.set_item_slot(lowest as u16);
            }
            return;
        }

        let next = self.mapping_entry(cur_idx).and_then(|e| e.next_chain_idx());
        let free_head = self.item_id_mapping_free_head;

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
    fn lowest_slot_holding(&self, item_id: ItemId) -> Option<u32> {
        let key = self
            .current_key_entries()
            .iter()
            .position(|e| {
                e.as_option()
                    .is_some_and(|e| e.item_id.as_valid() == Some(item_id))
            })
            .map(|i| self.global_slot(Container::Key, i as u32));

        key.or_else(|| {
            self.normal_entries()
                .iter()
                .position(|e| {
                    e.as_option()
                        .is_some_and(|e| e.item_id.as_valid() == Some(item_id))
                })
                .map(|i| self.global_slot(Container::Normal, i as u32))
        })
    }

    /// O(1) lookup of an item's first inventory slot via the hash table.
    pub fn find_item_idx(&self, item_id: ItemId) -> Option<u32> {
        let (_, cur_idx) = self.find_chain_entry(item_id)?;
        Some(self.mapping_entry(cur_idx)?.mapping.item_slot() as u32)
    }

    /// The container and offset within it that global `slot` addresses,
    /// resolved through the accessors. Mirrors the game's own addressing in
    /// `RebuildLookupMapping`/`RemoveItemEntryBySlot`.
    fn container_for(&self, slot: u32) -> Option<(Container, u32)> {
        if slot < self.key_items_capacity {
            Some((Container::Key, slot))
        } else {
            let offset = slot - self.key_items_capacity;
            (offset < self.normal_items_capacity).then_some((Container::Normal, offset))
        }
    }

    fn global_slot(&self, container: Container, offset: u32) -> u32 {
        match container {
            Container::Key => offset,
            Container::Normal => self.key_items_capacity + offset,
        }
    }

    fn find_empty_offset(&self, container: Container) -> Option<u32> {
        match container {
            Container::Key => self
                .current_key_entries()
                .iter()
                .position(|e| e.is_empty())
                .map(|i| i as u32),
            Container::Normal => self
                .normal_entries()
                .iter()
                .position(|e| e.is_empty())
                .map(|i| i as u32),
        }
    }

    pub fn empty_key_slot_count(&self) -> u32 {
        self.current_key_entries()
            .iter()
            .filter(|e| e.is_empty())
            .count() as u32
    }

    pub fn empty_normal_slot_count(&self) -> u32 {
        self.normal_entries()
            .iter()
            .filter(|e| e.is_empty())
            .count() as u32
    }

    fn accessor_mut(&mut self, container: Container) -> &mut InventoryItemListAccessor {
        match container {
            Container::Key => &mut self.key_items_accessor,
            Container::Normal => &mut self.normal_items_accessor,
        }
    }

    /// Detaches the entry at `slot`, leaving it empty and dropping it from
    /// the id lookup map. The caller now owns the returned entry's handle
    /// and must place it via [`attach`](Self::attach) or release it —
    /// dropping the entry leaks the gaitem slot.
    #[must_use]
    pub fn detach(&mut self, slot: u32) -> Option<EquipInventoryDataListEntry> {
        let (container, offset) = self.container_for(slot)?;
        let cell = match container {
            Container::Key => self.current_key_entries_mut().get_mut(offset as usize)?,
            Container::Normal => self.normal_entries_mut().get_mut(offset as usize)?,
        };
        let entry = cell.as_option().copied()?;
        cell.clear();

        unsafe {
            *self.accessor_mut(container).length.as_mut() -= 1;
        }

        self.remove_item_id_mapping(entry.item()?);
        Some(entry)
    }

    /// Writes `entry` into the first free slot of `container`, taking
    /// ownership of its handle. Returns the new global slot on success, or
    /// hands `entry` back unconsumed if `container` is full.
    pub fn attach(
        &mut self,
        container: Container,
        entry: EquipInventoryDataListEntry,
    ) -> Result<u32, EquipInventoryDataListEntry> {
        let Some(item_id) = entry.item() else {
            return Err(entry);
        };
        let Some(offset) = self.find_empty_offset(container) else {
            return Err(entry);
        };
        let global_slot = self.global_slot(container, offset);

        let cell = match container {
            Container::Key => &mut self.current_key_entries_mut()[offset as usize],
            Container::Normal => &mut self.normal_entries_mut()[offset as usize],
        };
        *cell = MaybeEmpty::new(entry);

        unsafe {
            *self.accessor_mut(container).length.as_mut() += 1;
        }

        self.update_item_id_mapping(item_id, global_slot as i16);
        Ok(global_slot)
    }
}

#[repr(C)]
pub struct EquipInventoryData {
    vftable: usize,
    pub items_data: InventoryItemsData,
    /// The highest global slot index in use, **not** a count of items —
    /// inventory slots are sparse, and this ignores the gaps between them.
    /// Not a true maximum after removals in the middle of the range: it
    /// only steps down when the removed slot happens to be this one.
    pub highest_item_slot: u32,
    pub next_sort_id: u32,
    pub pot_items_count: [u32; 16],
    pub pot_items_capacity: [u32; 16],
    /// List of item indices to show in the "Recent Items" tab. Capped at 64
    /// and shown in the UI back to front.
    pub recent_item_indices: DLList<u32>,
    /// True will allow consumables to stack up to 600, like in storage box.
    pub unlimited_consumables: bool,
    pub limited_pots: bool,
    unk122: u8,
    unk123: u8,
    unk124: u32,
}

pub struct RemovedItem {
    pub quantity: u32,
    pub emptied: bool,
}

/// Weapon category value for arrows, the only weapon-category items that
/// stack.
const WEAPON_CATEGORY_ARROW: u8 = 0xd;
/// Weapon category value for bolts, the only other weapon-category items
/// that stack.
const WEAPON_CATEGORY_BOLT: u8 = 0xe;

impl EquipInventoryData {
    /// Detaches the entry at `slot`, maintaining `highest_item_slot`,
    /// `pot_items_count` and `pot_items_capacity` alongside
    /// [`InventoryItemsData::detach`].
    #[must_use]
    pub fn detach(&mut self, slot: u32) -> Option<EquipInventoryDataListEntry> {
        let entry = self.items_data.detach(slot)?;

        if entry.pot_group >= 0 {
            let array = self.pot_array_for(entry.item());
            array[entry.pot_group as usize] =
                array[entry.pot_group as usize].saturating_sub(entry.quantity);
        }
        if slot == self.highest_item_slot {
            self.highest_item_slot = self.highest_item_slot.saturating_sub(1);
        }

        Some(entry)
    }

    /// Writes `entry` into the first free slot of `container`, maintaining
    /// `highest_item_slot`, `pot_items_count` and `pot_items_capacity`
    /// alongside [`InventoryItemsData::attach`]. Carries the same
    /// unreachable-slot warning as that method.
    pub fn attach(
        &mut self,
        container: Container,
        entry: EquipInventoryDataListEntry,
    ) -> Result<u32, EquipInventoryDataListEntry> {
        let pot_group = entry.pot_group;
        let quantity = entry.quantity;
        let item_id = entry.item();
        let slot = self.items_data.attach(container, entry)?;

        if self.highest_item_slot < slot {
            self.highest_item_slot = slot;
        }
        if pot_group >= 0 {
            self.pot_array_for(item_id)[pot_group as usize] += quantity;
        }

        Ok(slot)
    }

    /// `pot_items_capacity` for a pot-group container item
    /// (`goodsType == REGENERATIVE_MATERIAL`), `pot_items_count` for
    /// everything else in a pot group (the consumables). Mirrors the same
    /// split `UpdatePotsStates` makes between `IsRegenerativeMaterial` and
    /// `IsPotConsumable`.
    fn pot_array_for(&mut self, item_id: Option<ItemId>) -> &mut [u32; 16] {
        const REGENERATIVE_MATERIAL: u8 = 0xb;
        let is_container = item_id
            .and_then(|id| {
                unsafe { SoloParamRepository::instance() }
                    .ok()
                    .map(|r| (id, r))
            })
            .and_then(|(id, repo)| repo.get::<EquipParamGoods>(id.param_id()))
            .is_some_and(|goods| goods.goods_type() == REGENERATIVE_MATERIAL);

        if is_container {
            &mut self.pot_items_capacity
        } else {
            &mut self.pot_items_count
        }
    }

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

    pub fn max_stack_for(&self, item_id: ItemId) -> u32 {
        let Ok(repo) = (unsafe { SoloParamRepository::instance() }) else {
            return 0;
        };

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
    /// [`unlimited_consumables`](Self::unlimited_consumables), which is
    /// what the storage box sets. Despite the name it isn't a blanket
    /// "unlimited": each category answers differently.
    fn unlimited_consumables_max_stack(&self, item_id: ItemId) -> u32 {
        let Ok(repo) = (unsafe { SoloParamRepository::instance() }) else {
            return 0;
        };

        match item_id.category() {
            ItemCategory::Weapon => repo
                .get::<EquipParamWeapon>((item_id.param_id() / 100) * 100)
                .filter(|weapon| {
                    matches!(
                        weapon.weapon_category(),
                        WEAPON_CATEGORY_ARROW | WEAPON_CATEGORY_BOLT
                    )
                })
                .map_or(1, |_| 600),
            ItemCategory::Goods => repo
                .get::<EquipParamGoods>(item_id.param_id())
                .map_or(99, |goods| goods.max_repository_num() as u32),
            ItemCategory::Gem => repo
                .get::<EquipParamGem>(item_id.param_id())
                .map_or(0, |_| 1),
            ItemCategory::Protector | ItemCategory::Accessory => 1,
        }
    }

    /// How many more of `item_id` can actually be added, accounting for
    /// what's already held. For a pot-group item under `limited_pots` (and
    /// not `unlimited_consumables` — that takes over `max_stack_for` first)
    /// this is already the group's remaining headroom, shared across every
    /// item in the group, so the entry's own quantity is not deducted
    /// again; for everything else it's a per-entry stack ceiling minus
    /// what's held.
    pub fn headroom_for(&self, item_id: ItemId, slot: Option<u32>) -> u32 {
        let max = self.max_stack_for(item_id);

        if !self.unlimited_consumables && self.limited_pots && Self::pot_group_for(item_id) >= 0 {
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
    /// [`headroom_for`](Self::headroom_for) and the free slots.
    /// [`is_stackable`](Self::is_stackable) items merge into an existing
    /// entry; everything else takes one entry per copy.
    pub fn add_item(
        &mut self,
        gaitem: &mut CSGaitemImp,
        item_id: ItemId,
        quantity: u32,
    ) -> AddedItem {
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
                self.pot_array_for(Some(item_id))[pot_group as usize] += added;
            }
            return AddedItem {
                quantity: added,
                slots: vec![slot],
                created_entries: 0,
            };
        }

        if Self::is_stackable(item_id) {
            let wanted = self.headroom_for(item_id, None).min(quantity);
            return match self.insert_new_entry(gaitem, item_id, wanted) {
                Some((_, slot)) => AddedItem {
                    quantity: wanted,
                    slots: vec![slot],
                    created_entries: 1,
                },
                None => AddedItem::default(),
            };
        }

        let mut slots = Vec::new();
        while (slots.len() as u32) < quantity
            && let Some((_, slot)) = self.insert_new_entry(gaitem, item_id, 1)
        {
            slots.push(slot);
        }

        AddedItem {
            quantity: slots.len() as u32,
            created_entries: slots.len() as u32,
            slots,
        }
    }

    /// Creates one entry holding `quantity` of `item_id`, allocating the
    /// gaitem handle it takes ownership of. Returns the entry's handle and
    /// slot, or `None` if the inventory or gaitem pool was full.
    pub(crate) fn insert_new_entry(
        &mut self,
        gaitem: &mut CSGaitemImp,
        item_id: ItemId,
        quantity: u32,
    ) -> Option<(GaitemHandle, u32)> {
        if quantity == 0 {
            return None;
        }

        let handle = match gaitem.allocate_for(item_id)? {
            crate::cs::ItemHandle::NonIndexed(handle) => handle,
            crate::cs::ItemHandle::Indexed(owned) => owned.into_raw(),
        };

        let pot_group = Self::pot_group_for(item_id);
        let sort_id = self.next_sort_id;
        self.next_sort_id += 1;

        let is_key_item = Self::is_key_item(item_id);
        let container = if is_key_item {
            Container::Key
        } else {
            Container::Normal
        };

        let slot = match self.attach(
            container,
            EquipInventoryDataListEntry {
                gaitem_handle: handle,
                item_id: item_id.into(),
                quantity,
                sort_id,
                is_new: true,
                pot_group,
            },
        ) {
            Ok(slot) => slot,
            Err(_) => {
                gaitem.release_handle(handle);
                return None;
            }
        };

        Some((handle, slot))
    }

    /// Removes up to `quantity` from the entry at `slot`, releasing its
    /// gaitem handle if the stack reaches zero.
    pub fn remove_at(
        &mut self,
        gaitem: &mut CSGaitemImp,
        slot: u32,
        quantity: u32,
    ) -> Option<RemovedItem> {
        let held = self
            .items_data
            .entry_at_slot(slot)
            .and_then(|e| e.as_option())
            .map(|e| e.quantity)?;

        let removed = quantity.min(held);
        if removed == 0 {
            return Some(RemovedItem {
                quantity: 0,
                emptied: false,
            });
        }

        if removed < held {
            let entry = self
                .items_data
                .entry_at_slot_mut(slot)
                .and_then(|e| e.as_option_mut())?;
            entry.quantity -= removed;
            let pot_group = entry.pot_group;
            let item_id = entry.item();
            if pot_group >= 0 {
                let array = self.pot_array_for(item_id);
                array[pot_group as usize] = array[pot_group as usize].saturating_sub(removed);
            }
            return Some(RemovedItem {
                quantity: removed,
                emptied: false,
            });
        }

        let entry = self.detach(slot)?;
        gaitem.release_handle(entry.gaitem_handle);
        Some(RemovedItem {
            quantity: removed,
            emptied: true,
        })
    }

    /// Removes up to `quantity` of `item_id` from the lowest slot holding
    /// it.
    pub fn remove_item(&mut self, gaitem: &mut CSGaitemImp, item_id: ItemId, quantity: u32) -> u32 {
        let Some(slot) = self.items_data.find_item_idx(item_id) else {
            return 0;
        };
        self.remove_at(gaitem, slot, quantity)
            .map_or(0, |r| r.quantity)
    }

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

bitfield::bitfield! {
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
    pub fn next_mapping_item(&self) -> i16 {
        self.mapping.mapping_index() as i16 - 1
    }

    /// The index of the item slot. First checked against the key items
    /// capacity to see if it's contained there; if not, subtract the key
    /// items capacity to get the index into the normal items list.
    pub fn item_slot(&self) -> i16 {
        self.mapping.item_slot() as i16
    }

    pub fn is_free(&self) -> bool {
        self.mapping.is_free()
    }

    /// Next entry in the collision chain, 0-based. `None` = end of chain.
    /// The stored value is 1-based (0 means no next entry).
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
#[derive(Clone, Copy)]
pub struct EquipInventoryDataListEntry {
    /// Handle to the gaitem instance describing additional properties,
    /// like durability and gems in the case of weapons.
    pub gaitem_handle: GaitemHandle,
    /// The item this entry holds, or [`OptionalItemId::NONE`] when empty.
    pub item_id: OptionalItemId,
    pub quantity: u32,
    /// Sort ID used to sort items by acquisition order.
    pub sort_id: u32,
    /// Whether the item is newly acquired and should be highlighted if
    /// "Mark New Items" is enabled.
    pub is_new: bool,
    /// [pot group] of the item, or -1 if not a pot item.
    ///
    /// [pot group]: crate::param::EQUIP_PARAM_GOODS_ST::pot_group_id
    pub pot_group: i32,
}

unsafe impl IsEmpty for EquipInventoryDataListEntry {
    fn is_empty(value: &MaybeEmpty<EquipInventoryDataListEntry>) -> bool {
        // Safety: `item_id` is `OptionalItemId`, valid for every bit
        // pattern, so it's readable whether or not the entry is occupied.
        !unsafe { value.as_non_null().as_ref() }.item_id.is_valid()
    }
}

impl EquipInventoryDataListEntry {
    pub fn item(&self) -> Option<ItemId> {
        self.item_id.as_valid()
    }
}

impl Default for EquipInventoryDataListEntry {
    /// An empty entry: a null `gaitem_handle` and a `NONE` `item_id`. Both
    /// fields matter — the game tests emptiness two different ways
    /// depending on the code path (`RebuildLookupMapping` on `itemId`,
    /// `InsertKeyItem`/`InsertNormalItem` on `IsGaItemHandleNull`), and
    /// clearing only one leaves a slot invisible to one but permanently
    /// unavailable to the other, still carrying a dangling handle.
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

#[derive(Debug, Default, Clone)]
pub struct AddedItem {
    /// Falls short of the requested quantity when the stack, the inventory,
    /// or the gaitem pool filled up.
    pub quantity: u32,
    /// Slots now holding the item: one for a stacking item, one per copy
    /// otherwise.
    pub slots: Vec<u32>,
    /// `slots.len()` when the entries are new, `0` when an existing stack
    /// was topped up.
    pub created_entries: u32,
}

/// An error giving an item can fail with.
#[derive(Debug, Error)]
pub enum GiveItemError {
    #[error("CSGaitem singleton not available")]
    GaitemSingletonUnavailable,
    #[error("invalid param id: {0}")]
    InvalidParamId(#[from] ItemIdError),
}

pub enum GiveItemRequest {
    Weapon {
        param_id: u32,
        /// Only honoured for arrows and bolts, the sole stacking weapons.
        /// Anything else occupies one entry per copy and gives a single item.
        quantity: Option<u32>,
        ash_of_war_param_id: Option<u32>,
    },
    Protector {
        param_id: u32,
    },
    Accessory {
        param_id: u32,
    },
    Goods {
        param_id: u32,
        quantity: u32,
    },
}

impl GiveItemRequest {
    fn category(&self) -> ItemCategory {
        match self {
            Self::Weapon { .. } => ItemCategory::Weapon,
            Self::Protector { .. } => ItemCategory::Protector,
            Self::Accessory { .. } => ItemCategory::Accessory,
            Self::Goods { .. } => ItemCategory::Goods,
        }
    }

    fn param_id(&self) -> u32 {
        match self {
            Self::Weapon { param_id, .. }
            | Self::Protector { param_id }
            | Self::Accessory { param_id }
            | Self::Goods { param_id, .. } => *param_id,
        }
    }

    pub fn item_id(&self) -> Result<ItemId, ItemIdError> {
        ItemId::new(self.category(), self.param_id())
    }
}

pub struct GaveItem {
    pub quantity: u32,
    pub slots: Vec<u32>,
}

impl GaveItem {
    pub fn slot(&self) -> Option<u32> {
        self.slots.first().copied()
    }
}

impl EquipGameData {
    /// Gives an item and mounts its ash of war.
    pub fn give_item(
        &mut self,
        gaitem: &mut CSGaitemImp,
        request: GiveItemRequest,
    ) -> Result<GaveItem, GiveItemError> {
        let item_id = request.item_id()?;
        let quantity = match request {
            GiveItemRequest::Goods { quantity, .. } => quantity,
            GiveItemRequest::Weapon { quantity, .. } => quantity.unwrap_or(1),
            _ => 1,
        };

        let (item_id, ash_of_war_param_id) = match request {
            GiveItemRequest::Weapon {
                ash_of_war_param_id,
                ..
            } => Self::resolve_weapon(item_id, ash_of_war_param_id),
            _ => (item_id, None),
        };

        self.heal_next_sort_id();

        let added = self
            .equip_inventory_data
            .add_item(gaitem, item_id, quantity);
        if added.quantity == 0 {
            return Ok(GaveItem {
                quantity: 0,
                slots: Vec::new(),
            });
        }

        // A topped-up stack isn't new to the player.
        if added.created_entries > 0 {
            for slot in &added.slots {
                self.add_recent_item_index(*slot);
            }
        }

        // Each copy carries its own ash.
        if let Some(ash_param_id) = ash_of_war_param_id
            && let Ok(ash_item_id) = ItemId::new(ItemCategory::Gem, ash_param_id)
        {
            for slot in &added.slots {
                let Some(weapon_handle) = self
                    .equip_inventory_data
                    .items_data
                    .entry_at_slot(*slot)
                    .and_then(|entry| entry.as_option())
                    .map(|entry| entry.gaitem_handle)
                else {
                    continue;
                };
                let Some(ash_handle) = gaitem.allocate_indexed_gaitem(ash_item_id) else {
                    break;
                };
                gaitem.equip_ash_of_war(weapon_handle, ash_handle.into_raw());
            }
        }

        self.mark_item_acquired(item_id);
        if item_id.category() == ItemCategory::Weapon {
            self.update_matching_weapon_level(item_id);
        }

        Ok(GaveItem {
            quantity: added.quantity,
            slots: added.slots,
        })
    }

    /// The weapon id to give and the ash to mount on it. Drops the affinity
    /// when the ash isn't valid for the weapon, since the game has no
    /// infused weapon without one.
    fn resolve_weapon(item_id: ItemId, ash_of_war_param_id: Option<u32>) -> (ItemId, Option<u32>) {
        let ash_of_war_param_id =
            ash_of_war_param_id.filter(|ash| Self::is_ash_of_war_valid_for_weapon(item_id, *ash));

        match ash_of_war_param_id {
            Some(_) => (item_id, ash_of_war_param_id),
            None => (Self::without_affinity(item_id).unwrap_or(item_id), None),
        }
    }

    fn without_affinity(item_id: ItemId) -> Option<ItemId> {
        if item_id.category() != ItemCategory::Weapon {
            return None;
        }
        let affinity_id = (item_id.param_id() % 10000) / 100;
        if affinity_id == 0 {
            return None;
        }
        ItemId::new(ItemCategory::Weapon, item_id.param_id() - affinity_id * 100).ok()
    }

    fn is_ash_of_war_valid_for_weapon(weapon_id: ItemId, ash_param_id: u32) -> bool {
        let Ok(repo) = (unsafe { SoloParamRepository::instance() }) else {
            return true;
        };
        let Some(weapon) = repo.get::<EquipParamWeapon>(weapon_id.base_weapon_param_id()) else {
            return true;
        };
        let Some(gem) = repo.get::<EquipParamGem>(ash_param_id) else {
            return false;
        };

        let affinity_id = weapon.gem_check_affinity_id(weapon_id.param_id());
        let max_gem_rank = repo
            .get::<ReinforceParamWeapon>(weapon.reinforce_type_id() as u32)
            .map(|r| r.enable_gem_rank())
            .unwrap_or(0);

        weapon.can_mount_gem(gem, affinity_id, max_gem_rank)
    }

    fn heal_next_sort_id(&mut self) {
        let max_existing = self
            .equip_inventory_data
            .items_data
            .items()
            .map(|entry| entry.sort_id)
            .max();
        if let Some(max_existing) = max_existing
            && max_existing >= self.equip_inventory_data.next_sort_id
        {
            self.equip_inventory_data.next_sort_id = max_existing + 1;
        }
    }

    fn mark_item_acquired(&self, item_id: ItemId) {
        if !self.is_main_player || item_id.category() == ItemCategory::Gem {
            return;
        }
        let Ok(game_data_man) = (unsafe { GameDataMan::instance_mut() }) else {
            return;
        };

        let normalized_param_id = if item_id.category() == ItemCategory::Weapon {
            item_id.base_weapon_param_id()
        } else {
            item_id.param_id()
        };
        let Ok(normalized_id) = ItemId::new(item_id.category(), normalized_param_id) else {
            return;
        };

        game_data_man.gaitem_game_data.mark_acquired(normalized_id);
    }

    fn add_recent_item_index(&mut self, inventory_slot: u32) {
        const RECENT_ITEMS_CAP: usize = 64;
        let list = &mut self.equip_inventory_data.recent_item_indices;
        list.push_front(inventory_slot);
        while list.len() > RECENT_ITEMS_CAP {
            list.pop_back();
        }
    }

    fn update_matching_weapon_level(&mut self, item_id: ItemId) {
        if !self.is_main_player {
            return;
        }
        let Ok(repo) = (unsafe { SoloParamRepository::instance() }) else {
            return;
        };
        let Some(weapon) = repo.get::<EquipParamWeapon>(item_id.base_weapon_param_id()) else {
            return;
        };
        let reinforce_row = weapon.reinforce_type_id() as u32 + item_id.weapon_upgrade_level();
        let Some(reinforce) = repo.get::<ReinforceParamWeapon>(reinforce_row) else {
            return;
        };
        let level = reinforce.max_reinforce_level();

        let player_game_data = unsafe { self.player_game_data.as_mut() };
        if level > player_game_data.matching_weapon_level {
            player_game_data.matching_weapon_level = level;
        }
    }

    /// Removes the entry at `slot` outright, releasing its gaitem handle and
    /// dropping every reference to the slot (equip slot, quick slot, pouch
    /// slot, great rune) first. Returns the quantity it held, or `None` if
    /// the slot was already empty.
    pub fn remove_item_at_index(
        &mut self,
        gaitem: &mut CSGaitemImp,
        slot: u32,
        quantity: u32,
    ) -> Option<u32> {
        let held = self
            .equip_inventory_data
            .items_data
            .entry_at_slot(slot)
            .and_then(|entry| entry.as_option())
            .map(|entry| entry.quantity)?;

        let emptying = quantity >= held;
        if emptying {
            self.clear_references_to_slot(gaitem, slot);
        }

        self.equip_inventory_data
            .remove_at(gaitem, slot, quantity)
            .map(|r| r.quantity)
    }

    /// Removes up to `quantity` of `item_id`, unequipping it first if
    /// currently equipped.
    pub fn remove_item(&mut self, gaitem: &mut CSGaitemImp, item_id: ItemId, quantity: u32) -> u32 {
        if quantity == 0 {
            return 0;
        }

        let mut removed = 0;
        while removed < quantity {
            let Some(slot) = self.equip_inventory_data.items_data.find_item_idx(item_id) else {
                break;
            };
            let Some(took) = self.remove_item_at_index(gaitem, slot, quantity - removed) else {
                break;
            };
            if took == 0 {
                break;
            }
            removed += took;
        }

        removed
    }

    /// Detaches every equip/quick/pouch/great-rune reference to `slot`.
    pub fn clear_references_to_slot(&mut self, gaitem: &mut CSGaitemImp, slot: u32) {
        let index = slot as i32;

        if let Some(equip_slot) = self.find_equipped_slot(slot) {
            unsafe { self.unequip_slot(gaitem, equip_slot) };
        }

        for i in 0..self.equip_item_data.quick_slots.len() {
            if self.equip_item_data.quick_slots[i].index == index {
                gaitem.release_handle(self.equip_item_data.quick_slots[i].gaitem_handle);
                self.equip_item_data.quick_slots[i] = EquipDataItem {
                    gaitem_handle: GaitemHandle(0),
                    index: -1,
                };
                self.equipment_entries.quick_tems[i] = OptionalItemId::NONE;
            }
        }

        for i in 0..self.equip_item_data.pouch_slots.len() {
            if self.equip_item_data.pouch_slots[i].index == index {
                gaitem.release_handle(self.equip_item_data.pouch_slots[i].gaitem_handle);
                self.equip_item_data.pouch_slots[i] = EquipDataItem {
                    gaitem_handle: GaitemHandle(0),
                    index: -1,
                };
                self.equipment_entries.pouch[i] = OptionalItemId::NONE;
            }
        }

        if self.equip_item_data.great_rune.index == index {
            gaitem.release_handle(self.equip_item_data.great_rune.gaitem_handle);
            self.equip_item_data.great_rune = EquipDataItem {
                gaitem_handle: GaitemHandle(0),
                index: -1,
            };
        }

        // Clearing a quick slot can strand the selection on it, and dropping
        // an equipped item changes the loadout other players see.
        self.revalidate_selected_quick_slot();
        self.broadcast_equipment_change();
    }

    /// Points a quick slot at `inventory_slot`, taking a reference on its
    /// handle. Goods-only, matching the quick slots' own contents.
    pub fn set_quick_slot(
        &mut self,
        gaitem: &mut CSGaitemImp,
        position: usize,
        inventory_slot: u32,
    ) {
        let Some(handle) = self
            .equip_inventory_data
            .items_data
            .entry_at_slot(inventory_slot)
            .and_then(|e| e.as_option())
            .map(|e| e.gaitem_handle)
        else {
            return;
        };
        gaitem.increase_ref_count(handle);
        gaitem.release_handle(self.equip_item_data.quick_slots[position].gaitem_handle);
        self.equip_item_data.quick_slots[position] = EquipDataItem {
            gaitem_handle: handle,
            index: inventory_slot as i32,
        };
    }

    /// Points a pouch slot at `inventory_slot`. Goods-only.
    pub fn set_pouch_slot(
        &mut self,
        gaitem: &mut CSGaitemImp,
        position: usize,
        inventory_slot: u32,
    ) {
        let Some(handle) = self
            .equip_inventory_data
            .items_data
            .entry_at_slot(inventory_slot)
            .and_then(|e| e.as_option())
            .map(|e| e.gaitem_handle)
        else {
            return;
        };
        gaitem.increase_ref_count(handle);
        gaitem.release_handle(self.equip_item_data.pouch_slots[position].gaitem_handle);
        self.equip_item_data.pouch_slots[position] = EquipDataItem {
            gaitem_handle: handle,
            index: inventory_slot as i32,
        };
    }

    /// Points the great rune slot at `inventory_slot`. Goods-only.
    pub fn set_great_rune(&mut self, gaitem: &mut CSGaitemImp, inventory_slot: u32) {
        let Some(handle) = self
            .equip_inventory_data
            .items_data
            .entry_at_slot(inventory_slot)
            .and_then(|e| e.as_option())
            .map(|e| e.gaitem_handle)
        else {
            return;
        };
        gaitem.increase_ref_count(handle);
        gaitem.release_handle(self.equip_item_data.great_rune.gaitem_handle);
        self.equip_item_data.great_rune = EquipDataItem {
            gaitem_handle: handle,
            index: inventory_slot as i32,
        };
    }

    /// Tells other players in the session that this character's equipment
    /// changed, so they render the new loadout. Only meaningful for the
    /// main player.
    pub fn broadcast_equipment_change(&self) {
        if !self.is_main_player {
            return;
        }

        let Ok(va) = Program::current().rva_to_va(rva::get().broadcast_equipment_change) else {
            return;
        };

        // SAFETY: the RVA resolves to `BroadcastPacket12CharacterData`, which
        // takes no parameters and reads its state from globals.
        unsafe {
            let broadcast: extern "C" fn() = std::mem::transmute(va);
            broadcast();
        }
    }

    /// Re-points the selected quick slot when the item behind it moved.
    ///
    /// `selected_quick_slot` holds a slot position, not an inventory index,
    /// so the item is resolved through the slot first: if it still occupies
    /// a slot nothing changes, otherwise the same position is reused when
    /// something else moved into it, and failing that the search advances
    /// from that position.
    pub fn revalidate_selected_quick_slot(&mut self) {
        let position = self.selected_quick_slot_position();
        let item_index = self.selected_quick_slot_item_index();

        if item_index != -1 {
            if self.quick_slot_position_of(item_index).is_some() {
                return;
            }
            if let Some(position) = position
                && self.equip_item_data.quick_slots[position].index != -1
            {
                self.equip_item_data.selected_quick_slot = position as i32;
                return;
            }
        }

        self.select_next_occupied_quick_slot(position);
    }

    fn selected_quick_slot_position(&self) -> Option<usize> {
        let selected = self.equip_item_data.selected_quick_slot;
        (selected >= 0 && (selected as usize) < self.equip_item_data.quick_slots.len())
            .then_some(selected as usize)
    }

    fn selected_quick_slot_item_index(&self) -> i32 {
        match self.selected_quick_slot_position() {
            Some(position) => self.equip_item_data.quick_slots[position].index,
            None => -1,
        }
    }

    fn quick_slot_position_of(&self, item_index: i32) -> Option<usize> {
        if item_index == -1 {
            return None;
        }
        self.equip_item_data
            .quick_slots
            .iter()
            .position(|entry| entry.index == item_index)
    }

    /// Moves the selection to the next occupied slot after `from`,
    /// wrapping, or clears it when every slot is empty.
    fn select_next_occupied_quick_slot(&mut self, from: Option<usize>) {
        let slots = &self.equip_item_data.quick_slots;
        let count = slots.len();
        let start = from.unwrap_or(count - 1);

        let next = (1..=count)
            .map(|offset| (start + offset) % count)
            .find(|position| slots[*position].index != -1);

        self.equip_item_data.selected_quick_slot = next.map_or(-1, |position| position as i32);
    }

    fn set_last_equipped_item_index(inventory_slot: u32) {
        let Ok(menu_man) = (unsafe { CSMenuManImp::instance_mut() }) else {
            return;
        };
        menu_man.last_equipped_item_index = inventory_slot as i32;
    }

    pub fn find_equipped_slot(&self, slot: u32) -> Option<ChrAsmSlot> {
        self.equipment_item_idx_list
            .iter()
            .position(|&idx| idx == slot)
            .and_then(|i| ChrAsmSlot::from_index(i as u32).ok())
    }

    /// Equips the already-owned inventory entry at `inventory_slot` into
    /// `slot`, unequipping whatever's already there first if occupied. A
    /// no-op if `inventory_slot` doesn't hold a real entry.
    ///
    /// Giving an item does not equip it, so this is a separate step after
    /// [`give_item`](Self::give_item).
    pub unsafe fn equip_slot(
        &mut self,
        gaitem: &mut CSGaitemImp,
        slot: ChrAsmSlot,
        inventory_slot: u32,
    ) -> bool {
        let Some(item_id) = self
            .equip_inventory_data
            .items_data
            .entry_at_slot(inventory_slot)
            .and_then(|entry| entry.as_option())
            .and_then(|entry| entry.item_id.as_valid())
        else {
            return false;
        };

        if !slot.accepts(item_id.category()) {
            return false;
        }

        // Re-equipping into the slot it already occupies toggles it off,
        // and equipping into a different slot vacates the old one first.
        // The "nothing equipped" placeholders are exempt — one such entry
        // is shared by every empty slot of its kind, so displacing it would
        // empty an unrelated slot, and `unequip_slot` would recurse back in
        // through here.
        let is_placeholder = Self::default_item_for_empty_slot(slot) == Some(item_id);
        if !is_placeholder && let Some(current) = self.find_equipped_slot(inventory_slot) {
            unsafe { self.unequip_slot(gaitem, current) };
            if current == slot {
                return true;
            }
        }

        let Some(entry_handle) = self
            .equip_inventory_data
            .items_data
            .entry_at_slot(inventory_slot)
            .and_then(|entry| entry.as_option())
            .map(|entry| entry.gaitem_handle)
        else {
            return false;
        };

        gaitem.swap_handle(
            &mut self.chr_asm.gaitem_handles[slot as usize],
            entry_handle,
        );
        self.chr_asm.equipment_param_ids[slot as usize] = item_id.param_id() as i32;
        self.equipment_entries[slot] = item_id.into();
        self.equipment_item_idx_list[slot as usize] = inventory_slot;

        // Weapon and ammo slots (`WeaponLeft1..=Bolt3`) drop two-handing and
        // whatever was loaded.
        if (slot as usize) < 12 {
            let equipment = &mut self.chr_asm.equipment;
            if slot == equipment.active_left_weapon_slot()
                || slot == equipment.active_right_weapon_slot()
            {
                equipment.arm_style = ChrAsmArmStyle::OneHanded;
            }
            self.chr_asm.bolt_loaded_states[slot as usize] = false;
        }

        Self::set_last_equipped_item_index(inventory_slot);
        self.revalidate_selected_quick_slot();
        self.broadcast_equipment_change();

        true
    }

    /// Unequips `slot`, releasing the gaitem handle reference it held.
    ///
    /// Only ammo and accessory slots are truly cleared. Weapon and
    /// protector slots are never left empty: they get the default item for
    /// the slot instead, given first if it isn't already owned — a state
    /// the game never produces and the equipment menu misrenders otherwise.
    pub unsafe fn unequip_slot(&mut self, gaitem: &mut CSGaitemImp, slot: ChrAsmSlot) {
        if let Some(default_item_id) = Self::default_item_for_empty_slot(slot) {
            let owned = self
                .equip_inventory_data
                .items_data
                .find_item_idx(default_item_id);

            let inventory_slot = match owned {
                Some(inventory_slot) => Some(inventory_slot),
                None => self
                    .equip_inventory_data
                    .insert_new_entry(gaitem, default_item_id, 1)
                    .map(|(_, inventory_slot)| inventory_slot),
            };

            if let Some(inventory_slot) = inventory_slot {
                unsafe { self.equip_slot(gaitem, slot, inventory_slot) };
                return;
            }
        }

        gaitem.swap_handle(
            &mut self.chr_asm.gaitem_handles[slot as usize],
            GaitemHandle(0),
        );
        self.chr_asm.equipment_param_ids[slot as usize] = -1;
        self.equipment_entries[slot] = OptionalItemId::NONE;
        self.equipment_item_idx_list[slot as usize] = u32::MAX;

        // Only this branch needs them: the default-item path above returns
        // through `equip_slot`, which fires the same side effects itself.
        self.revalidate_selected_quick_slot();
        self.broadcast_equipment_change();
    }

    fn default_item_for_empty_slot(slot: ChrAsmSlot) -> Option<ItemId> {
        const EMPTY_PROTECTOR_ITEM_IDS: [u32; 4] = [10000, 10100, 10200, 10300];
        const UNARMED_PARAM_ID: u32 = 110000;

        let param_id = match slot {
            ChrAsmSlot::WeaponLeft1
            | ChrAsmSlot::WeaponRight1
            | ChrAsmSlot::WeaponLeft2
            | ChrAsmSlot::WeaponRight2
            | ChrAsmSlot::WeaponLeft3
            | ChrAsmSlot::WeaponRight3 => {
                return ItemId::new(ItemCategory::Weapon, UNARMED_PARAM_ID).ok();
            }
            ChrAsmSlot::ProtectorHead => EMPTY_PROTECTOR_ITEM_IDS[0],
            ChrAsmSlot::ProtectorChest => EMPTY_PROTECTOR_ITEM_IDS[1],
            ChrAsmSlot::ProtectorHands => EMPTY_PROTECTOR_ITEM_IDS[2],
            ChrAsmSlot::ProtectorLegs => EMPTY_PROTECTOR_ITEM_IDS[3],
            _ => return None,
        };

        ItemId::new(ItemCategory::Protector, param_id).ok()
    }
}
