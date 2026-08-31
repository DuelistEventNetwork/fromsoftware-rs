use thiserror::Error;

use fromsoftware_shared::program::Program;
use pelite::pe::Pe;

use crate::cs::{
    CSGaitemImp, CSMenuManImp, ChrAsmArmStyle, ChrAsmSlot, EquipDataItem, EquipGameData,
    EquipInventoryData, EquipParamGem, EquipParamWeapon,
    GaitemHandle, GameDataMan, ItemCategory, ItemId, ItemIdError, OptionalItemId,
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
    /// Falls short of the requested quantity when the stack, the inventory or
    /// the gaitem pool filled up.
    pub quantity: u32,
    /// Slots now holding the item: one for a stacking item, one per copy
    /// otherwise.
    pub slots: Vec<u32>,
}

impl GaveItem {
    /// The slot to equip, or `None` if nothing was given.
    pub fn slot(&self) -> Option<u32> {
        self.slots.first().copied()
    }
}

impl EquipGameData {
    /// Gives an item and mounts its ash of war.
    ///
    /// [`EquipInventoryData::add_item`] does the inventory work; this adds what
    /// `AddInventoryEquip` does over `InsertItem` — marking the
    /// item acquired and raising the matchmaking weapon level.
    pub fn give_item(&mut self, request: GiveItemRequest) -> Result<GaveItem, GiveItemError> {
        let gaitem = unsafe { CSGaitemImp::instance_mut() }
            .map_err(|_| GiveItemError::GaitemSingletonUnavailable)?;

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

        let added = self.equip_inventory_data.add_item(item_id, quantity);
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
                gaitem.equip_ash_of_war(weapon_handle, ash_handle);
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
    /// when the ash isn't valid for the weapon, since the game has no infused
    /// weapon without one.
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

            self.equip_inventory_data.remove_item(item_id, quantity)
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
    /// Calls `BroadcastPacket12CharacterData`, which fills a
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
    ///. `selected_quick_slot` holds a slot position, not an
    /// inventory index, so the item is resolved through the slot first: if it
    /// still occupies a slot nothing changes, otherwise the same position is
    /// reused when something else moved into it, and failing that the search
    /// advances from that position.
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

    /// The selected slot's position, or `None` when nothing is selected.
    fn selected_quick_slot_position(&self) -> Option<usize> {
        let selected = self.equip_item_data.selected_quick_slot;
        (selected >= 0 && (selected as usize) < self.equip_item_data.quick_slots.len())
            .then_some(selected as usize)
    }

    /// The inventory index the selected slot points at.
    ///
    /// Mirrors `EquipItemData::GetSelectedQuickslotItemIndex`.
    fn selected_quick_slot_item_index(&self) -> i32 {
        match self.selected_quick_slot_position() {
            Some(position) => self.equip_item_data.quick_slots[position].index,
            None => -1,
        }
    }

    /// The position of the slot holding `item_index`.
    ///
    /// Mirrors `EquipItemData::GetQuickSlotIndexByInventoryIndex`
    ///.
    fn quick_slot_position_of(&self, item_index: i32) -> Option<usize> {
        if item_index == -1 {
            return None;
        }
        self.equip_item_data
            .quick_slots
            .iter()
            .position(|entry| entry.index == item_index)
    }

    /// Moves the selection to the next occupied slot after `from`, wrapping,
    /// or clears it when every slot is empty.
    ///
    /// Mirrors `EquipItemData::SelectNextOccupiedQuickSlot`
    /// and `FindNextOccupiedQuickSlot`: the search starts at
    /// `from + 1` and wraps, and an unset selection starts from the last slot
    /// so the scan begins at position 0.
    fn select_next_occupied_quick_slot(&mut self, from: Option<usize>) {
        let slots = &self.equip_item_data.quick_slots;
        let count = slots.len();
        let start = from.unwrap_or(count - 1);

        let next = (1..=count)
            .map(|offset| (start + offset) % count)
            .find(|position| slots[*position].index != -1);

        self.equip_item_data.selected_quick_slot = next.map_or(-1, |position| position as i32);
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
    /// The reverse of [`unequip_slot`](Self::unequip_slot). Giving an item
    /// does not equip it, so this is a separate step after
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
    /// Only ammo and accessory slots are truly cleared. Weapon and protector
    /// slots are never left empty: they get the default item for the slot
    /// instead, given first if it isn't already owned. Clearing them outright
    /// is a state the game never produces and the equipment menu misrenders.
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
                    .insert_new_entry(default_item_id, 1)
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
