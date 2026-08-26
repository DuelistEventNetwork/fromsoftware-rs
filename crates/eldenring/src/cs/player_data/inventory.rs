use thiserror::Error;

use fromsoftware_shared::program::Program;
use pelite::pe::Pe;

use crate::cs::{
    CSGaitemImp, CSMenuManImp, ChrAsmArmStyle, ChrAsmSlot, EquipDataItem, EquipGameData,
    EquipInventoryData, EquipInventoryDataListEntry, EquipParamGem, EquipParamWeapon,
    GaitemCategory, GaitemHandle, GameDataMan, ItemCategory, ItemId, ItemIdError, OptionalItemId,
    ReinforceParamWeapon, SoloParamRepository,
};
use crate::rva;
use shared::FromStatic;

/// An error giving an item can fail with.
#[derive(Debug, Error)]
pub enum GiveItemError {
    /// The [`CSGaitemImp`] singleton isn't available yet.
    #[error("CSGaitem singleton not available")]
    GaitemSingletonUnavailable,
    /// A [`GiveItemRequest`] variant's `param_id` (or, for
    /// [`Weapon`](GiveItemRequest::Weapon), `ash_of_war_param_id`) couldn't
    /// form a valid [`ItemId`] with that variant's implied category.
    #[error("invalid param id: {0}")]
    InvalidParamId(#[from] ItemIdError),
}

pub enum GiveItemRequest {
    Weapon {
        param_id: u32,
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

/// The outcome of a successful [`EquipGameData::give_item`] call.
pub struct GaveItem {
    /// How many were actually given. Always 0 or 1 for
    /// [`Weapon`]/[`Protector`]/[`Accessory`], possibly more for
    /// [`Goods`] (and possibly less than the
    /// requested quantity, if the inventory/stack was full).
    ///
    /// [`Weapon`]: GiveItemRequest::Weapon
    /// [`Protector`]: GiveItemRequest::Protector
    /// [`Accessory`]: GiveItemRequest::Accessory
    /// [`Goods`]: GiveItemRequest::Goods
    pub quantity: u32,
    /// For [`Weapon`]/[`Protector`]/[`Accessory`] requests: the
    /// exact new copy's [`GaitemHandle`] and inventory slot.
    ///
    /// [`Weapon`]: GiveItemRequest::Weapon
    /// [`Protector`]: GiveItemRequest::Protector
    /// [`Accessory`]: GiveItemRequest::Accessory
    pub indexed: Option<(GaitemHandle, u32)>,
}

impl EquipGameData {
    pub fn give_item(&mut self, request: GiveItemRequest) -> Result<GaveItem, GiveItemError> {
        let gaitem = unsafe { CSGaitemImp::instance_mut() }
            .map_err(|_| GiveItemError::GaitemSingletonUnavailable)?;

        let item_id = request.item_id()?;
        if EquipInventoryData::is_stackable(item_id)
            && let Some(slot) = self.equip_inventory_data.items_data.find_item_idx(item_id)
        {
            let quantity = match request {
                GiveItemRequest::Goods { quantity, .. } => quantity,
                _ => 1,
            };
            let added = self
                .equip_inventory_data
                .add_to_stack(item_id, slot, quantity);
            if added > 0 {
                self.mark_item_acquired(item_id);
            }
            return Ok(GaveItem {
                quantity: added,
                indexed: None,
            });
        }

        match request {
            GiveItemRequest::Goods { .. } => {
                let item_id = request.item_id()?;
                let GiveItemRequest::Goods { quantity, .. } = request else {
                    unreachable!()
                };
                Ok(GaveItem {
                    quantity: self.give_goods(item_id, quantity, gaitem),
                    indexed: None,
                })
            }
            GiveItemRequest::Weapon {
                ash_of_war_param_id,
                ..
            } => {
                let item_id = request.item_id()?;
                let indexed = self.give_weapon_indexed(item_id, ash_of_war_param_id, gaitem);
                Ok(GaveItem {
                    quantity: indexed.is_some() as u32,
                    indexed,
                })
            }
            GiveItemRequest::Protector { .. } | GiveItemRequest::Accessory { .. } => {
                let item_id = request.item_id()?;
                let indexed = self.give_indexed_item(item_id, gaitem);
                Ok(GaveItem {
                    quantity: indexed.is_some() as u32,
                    indexed,
                })
            }
        }
    }

    fn give_weapon_indexed(
        &mut self,
        item_id: ItemId,
        ash_of_war_param_id: Option<u32>,
        gaitem: &mut CSGaitemImp,
    ) -> Option<(GaitemHandle, u32)> {
        let ash_of_war_param_id =
            ash_of_war_param_id.filter(|ash| Self::is_ash_of_war_valid_for_weapon(item_id, *ash));

        let item_id = match ash_of_war_param_id {
            Some(_) => item_id,
            None => Self::without_affinity(item_id).unwrap_or(item_id),
        };

        let given = self.give_indexed_item(item_id, gaitem)?;

        if let Some(ash_param_id) = ash_of_war_param_id
            && let Ok(ash_item_id) = ItemId::new(ItemCategory::Gem, ash_param_id)
            && let Some(ash_handle) = gaitem.allocate_indexed_gaitem(ash_item_id)
        {
            gaitem.equip_ash_of_war(given.0, ash_handle);
        }

        Some(given)
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

    fn give_goods(&mut self, item_id: ItemId, quantity: u32, gaitem: &mut CSGaitemImp) -> u32 {
        if quantity == 0 {
            return 0;
        }

        if EquipInventoryData::is_stackable(item_id) {
            let Some(handle) = Self::allocate_gaitem_for_give(item_id, gaitem) else {
                return 0;
            };

            let given = self
                .equip_inventory_data
                .give_stackable(item_id, quantity, handle);
            if given > 0 {
                self.mark_item_acquired(item_id);
            }
            given
        } else {
            let mut given = 0;
            for _ in 0..quantity {
                if self.give_indexed_item(item_id, gaitem).is_none() {
                    break;
                }
                given += 1;
            }
            given
        }
    }

    fn allocate_gaitem_for_give(item_id: ItemId, gaitem: &mut CSGaitemImp) -> Option<GaitemHandle> {
        match item_id.category() {
            ItemCategory::Goods => {
                Some(gaitem.allocate_partial_gaitem(GaitemCategory::Goods, item_id.param_id()))
            }
            ItemCategory::Accessory => {
                Some(gaitem.allocate_partial_gaitem(GaitemCategory::Accessory, item_id.param_id()))
            }
            ItemCategory::Weapon | ItemCategory::Protector | ItemCategory::Gem => {
                gaitem.allocate_indexed_gaitem(item_id)
            }
        }
    }

    fn give_indexed_item(
        &mut self,
        item_id: ItemId,
        gaitem: &mut CSGaitemImp,
    ) -> Option<(GaitemHandle, u32)> {
        let handle = Self::allocate_gaitem_for_give(item_id, gaitem)?;

        self.heal_next_sort_id();
        let sort_id = self.equip_inventory_data.next_sort_id;
        self.equip_inventory_data.next_sort_id += 1;

        let is_key_item = EquipInventoryData::is_key_item(item_id);
        let inserted = self.equip_inventory_data.insert_entry(
            EquipInventoryDataListEntry {
                gaitem_handle: handle,
                item_id: item_id.into(),
                quantity: 1,
                sort_id,
                is_new: true,
                pot_group: -1,
            },
            is_key_item,
        );

        match inserted {
            Some(slot) => {
                self.mark_item_acquired(item_id);
                self.add_recent_item_index(slot);
                if item_id.category() == ItemCategory::Weapon {
                    self.update_matching_weapon_level(item_id);
                }
                Some((handle, slot))
            }
            None => {
                gaitem.release_handle(handle);
                None
            }
        }
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
        let Some(reinforce) = repo.get::<ReinforceParamWeapon>(weapon.reinforce_type_id() as u32)
        else {
            return;
        };
        let max_reinforce_level = reinforce.max_reinforce_level();

        let player_game_data = unsafe { self.player_game_data.as_mut() };
        if max_reinforce_level > player_game_data.matching_weapon_level {
            player_game_data.matching_weapon_level = max_reinforce_level;
        }
    }

    /// Removes the entry at a specific inventory `slot` outright, whatever it
    /// holds, releasing its gaitem handle and dropping every reference to the
    /// slot (equip slot, quick slot, pouch slot, great rune) first.
    ///
    /// Returns the quantity the removed entry held, or `None` if the slot was
    /// already empty.
    pub fn remove_item_at_slot(&mut self, gaitem: &mut CSGaitemImp, slot: u32) -> Option<u32> {
        let quantity = self
            .equip_inventory_data
            .items_data
            .entry_at_slot(slot)
            .and_then(|entry| entry.as_option())
            .map(|entry| entry.quantity)?;

        self.clear_references_to_slot(gaitem, slot);

        let handle = self.equip_inventory_data.remove_entry(slot)?;
        gaitem.release_handle(handle);

        Some(quantity)
    }

    /// Removes up to `quantity` of `item_id` from this equipment's owner,
    /// unequipping it first if it's currently equipped, mirroring the
    /// effects of the game's own item-removal logic without calling into the
    /// game's own code.
    ///
    /// Returns the quantity actually removed, which may be less than
    /// `quantity` if fewer copies are held.
    ///
    /// Mirrors the negative-quantity branch of `AddOrRemoveItem`.
    pub fn remove_item(&mut self, item_id: ItemId, quantity: u32) -> u32 {
        if quantity == 0 {
            return 0;
        }

        let Ok(gaitem) = (unsafe { CSGaitemImp::instance_mut() }) else {
            return 0;
        };

        if EquipInventoryData::is_stackable(item_id) {
            let Some(slot) = self.equip_inventory_data.items_data.find_item_idx(item_id) else {
                return 0;
            };

            let emptying = self
                .equip_inventory_data
                .items_data
                .entry_at_slot(slot)
                .and_then(|e| e.as_option())
                .is_some_and(|entry| quantity >= entry.quantity);

            if emptying {
                self.clear_references_to_slot(gaitem, slot);
            }

            let (removed, freed_handle) =
                self.equip_inventory_data.take_stackable(item_id, quantity);

            if let Some(handle) = freed_handle {
                gaitem.release_handle(handle);
            }
            removed
        } else {
            let mut removed = 0;
            for _ in 0..quantity {
                let Some(slot) = self.equip_inventory_data.items_data.find_item_idx(item_id) else {
                    break;
                };

                self.clear_references_to_slot(gaitem, slot);

                let Some(handle) = self.equip_inventory_data.remove_entry(slot) else {
                    break;
                };
                gaitem.release_handle(handle);

                removed += 1;
            }

            removed
        }
    }

    fn clear_references_to_slot(&mut self, gaitem: &mut CSGaitemImp, slot: u32) {
        let index = slot as i32;

        if let Some(equip_slot) = self.find_equipped_slot(slot) {
            unsafe { self.unequip_slot(gaitem, equip_slot) };
        }

        for i in 0..self.equip_item_data.quick_slots.len() {
            if self.equip_item_data.quick_slots[i].index == index {
                self.equip_item_data.quick_slots[i] = EquipDataItem {
                    gaitem_handle: GaitemHandle(0),
                    index: -1,
                };
                self.equipment_entries.quick_tems[i] = OptionalItemId::NONE;
            }
        }

        for i in 0..self.equip_item_data.pouch_slots.len() {
            if self.equip_item_data.pouch_slots[i].index == index {
                self.equip_item_data.pouch_slots[i] = EquipDataItem {
                    gaitem_handle: GaitemHandle(0),
                    index: -1,
                };
                self.equipment_entries.pouch[i] = OptionalItemId::NONE;
            }
        }

        if self.equip_item_data.great_rune.index == index {
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

    /// Tells other players in the session that this character's equipment
    /// changed, so they render the new loadout.
    ///
    /// Calls `BroadcastPacket12CharacterData` (`0x140ca11c0`), which fills a
    /// packet from the main player's equipment and hands it to
    /// `CSSessionManagerImp::P2PBroadcast`. The real equip, unequip and
    /// item-removal paths all reach this; without it remote players keep
    /// seeing whatever was equipped when they last got an update.
    ///
    /// Takes no arguments and guards itself: it returns early when there's no
    /// main player game data or the character has no event id, so calling it
    /// outside a session is harmless.
    ///
    /// Only meaningful for the main player, so this is a no-op otherwise.
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
    /// Mirrors `CS::EquipGameData::RevalidateSelectedQuickSlot`
    /// (`0x140249a90`): if the selected inventory index no longer maps to a
    /// quick slot, fall back to whatever occupies the slot it used to be in,
    /// and failing that advance to the next occupied slot. The real equip,
    /// unequip and auto-equip paths all call this.
    pub fn revalidate_selected_quick_slot(&mut self) {
        let selected = self.equip_item_data.selected_quick_slot;

        // The slot the selected item currently sits in, if any.
        let slot_of_selected = self
            .equip_item_data
            .quick_slots
            .iter()
            .position(|entry| entry.index == selected);

        if selected != -1 {
            if slot_of_selected.is_some() {
                return;
            }
            // The item moved: keep the same slot position if something else
            // now occupies it.
            if let Some(entry) = self
                .equip_item_data
                .quick_slots
                .iter()
                .find(|entry| entry.index != -1)
            {
                self.equip_item_data.selected_quick_slot = entry.index;
                return;
            }
        }

        self.select_next_occupied_quick_slot();
    }

    /// Moves the selection to the next quick slot holding something, or
    /// clears it when every slot is empty.
    ///
    /// Mirrors `EquipItemData::SelectNextOccupiedQuickSlot` (`0x14024f7e0`).
    fn select_next_occupied_quick_slot(&mut self) {
        let next = self
            .equip_item_data
            .quick_slots
            .iter()
            .map(|entry| entry.index)
            .find(|index| *index != -1);

        self.equip_item_data.selected_quick_slot = next.unwrap_or(-1);
    }

    /// Records the inventory index the player last equipped, which the
    /// equipment menu uses to restore its cursor.
    ///
    /// The real equip path writes `CSMenuMan::lastEquippedItemIndex` at every
    /// exit.
    fn set_last_equipped_item_index(inventory_slot: u32) {
        let Ok(menu_man) = (unsafe { CSMenuManImp::instance_mut() }) else {
            return;
        };
        menu_man.last_equipped_item_index = inventory_slot as i32;
    }

    /// Finds the [`ChrAsmSlot`] currently equipped to the given global
    /// inventory `slot`, if any.
    pub fn find_equipped_slot(&self, slot: u32) -> Option<ChrAsmSlot> {
        self.equipment_item_idx_list
            .iter()
            .position(|&idx| idx == slot)
            .and_then(|i| ChrAsmSlot::from_index(i as u32).ok())
    }

    /// Equips the already-owned inventory entry at `inventory_slot` into
    /// `slot`, unequipping whatever's already there first if the slot is
    /// occupied. A no-op if `inventory_slot` doesn't hold a real entry.
    ///
    /// **Deliberately separate from [`give_item`](Self::give_item)** — an
    /// explicit second step the caller must take after giving an item, not
    /// something `give_item` does as a side effect, mirroring the real
    /// game's own give/equip split (`AddInventoryEquip`/`UpdateAutoEquip`
    /// only auto-equip a narrow set of cases — arrow/bolt weapons, quick-slot
    /// goods — everything else needs an explicit equip action).
    ///
    /// Mirrors the real equip chain traced via Ghidra
    /// (`pc_eldenring_runtime.1.16.2.exe`): `EquipItemStruct::EquipItem`
    /// (`0x1407a3320`, the menu's top-level equip action) →
    /// `EquipItemToChrAsmSlot` (`0x140787c30`) →
    /// `CS::EquipGameData::SetEquipmentEntries` (`0x140249160`) →
    /// `CS::ChrAsm::EquipItem` (`0x1403bf3c0`, invoked via a deferred
    /// `std::function` `SetEquipmentEntries` constructs and calls inline).
    /// Confirmed **not** relevant to interactability (traced this session,
    /// all no-ops or UI/network side effects outside what's covered below):
    /// `EquipItemToChrAsmSlot`'s own popup-warning pre-check
    /// (`FUN_140787ab0`, "already equipped elsewhere" dialog),
    /// `GLOBAL_CSMenuMan`'s last-interacted-item-index field write, a
    /// per-slot "changed" flag in `EquipGameData` (undocumented, presumably
    /// VFX/SFX trigger), `BroadCastEquipmentChange` (confirmed multiplayer
    /// session-only — a no-op with no active session), and `FUN_140249a90`
    /// (re-validates the *quick-slot* selection index, unrelated to `ChrAsm`
    /// slots).
    ///
    /// The reverse of [`unequip_slot`](Self::unequip_slot): writes real
    /// values into the same four arrays that method clears, plus
    /// `CS::ChrAsm::EquipItem`'s two additional real writes for weapon/bolt
    /// slots (`slot`'s discriminant `< 12`, i.e. `WeaponLeft1..=Bolt3`):
    /// resetting two-handing back to `OneHanded` if `slot` is the currently
    /// active weapon slot for either hand, and clearing that slot's
    /// loaded-bolt/arrow state.
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

        // The real path checks whether this entry is already equipped
        // somewhere before writing: re-equipping into the slot it already
        // occupies toggles it off, and equipping it into a *different* slot
        // vacates the old one first. Skipping this leaves one inventory entry
        // referenced by two `ChrAsm` slots.
        //
        // The "nothing equipped" placeholders are exempt. One such entry is
        // shared by every empty slot of its kind, so displacing it would
        // empty an unrelated slot — and since `unequip_slot` equips a
        // placeholder by calling back into this method, it would also
        // recurse.
        let is_placeholder = Self::default_item_for_empty_slot(slot) == Some(item_id);
        if !is_placeholder
            && let Some(current) = self.find_equipped_slot(inventory_slot)
        {
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

        if (slot as usize) < 12 {
            let equipment = &mut self.chr_asm.equipment;
            if slot == equipment.active_left_weapon_slot()
                || slot == equipment.active_right_weapon_slot()
            {
                equipment.arm_style = ChrAsmArmStyle::OneHanded;
            }
            self.chr_asm.bolt_loaded_states[slot as usize] = false;
        }

        // The tail of the real equip path, which runs after the arrays are
        // written: menu cursor, quick-slot selection, then the network
        // broadcast.
        Self::set_last_equipped_item_index(inventory_slot);
        self.revalidate_selected_quick_slot();
        self.broadcast_equipment_change();

        true
    }

    /// Unequips the given [`ChrAsmSlot`], releasing the gaitem handle
    /// reference it held.
    ///
    /// Mirrors `CS::EquipGameData::UnequipSlot` (`0x140247160`), which does
    /// **not** treat every slot the same way. Weapon and protector slots are
    /// never left empty: the game equips a *default* item into them instead —
    /// unarmed fists (`GetDefaultUnarmedParamId`, `0x140248270` — param id
    /// [`UNARMED_PARAM_ID`]) for weapon slots, and the bare-skin protector
    /// rows (`GetDefaultItemIdForEmptyProtectorSlot`, `0x140d473d0` —
    /// [`EMPTY_PROTECTOR_ITEM_IDS`]) for the four armor slots. Only ammo and
    /// accessory slots get a true clear (handle `0`, item index `-1`).
    ///
    /// Writing an empty handle into a weapon or protector slot instead
    /// produces a state the game never creates, which the equipment menu then
    /// renders from stale/mismatched data.
    ///
    /// The default item has to be *owned* to be equipped, since the slot
    /// refers to it by inventory index: the real code looks it up with
    /// `GetItemInventoryIdx` and, only if it isn't already held, gives it via
    /// `AddInventoryEquip` first. This does the same.
    pub unsafe fn unequip_slot(&mut self, gaitem: &mut CSGaitemImp, slot: ChrAsmSlot) {
        if let Some(default_item_id) = Self::default_item_for_empty_slot(slot) {
            let owned = self
                .equip_inventory_data
                .items_data
                .find_item_idx(default_item_id);

            let inventory_slot = match owned {
                Some(inventory_slot) => Some(inventory_slot),
                None => self
                    .give_indexed_item(default_item_id, gaitem)
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
