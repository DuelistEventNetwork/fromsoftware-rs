use thiserror::Error;

use crate::cs::{
    CSGaitemImp, ChrAsmSlot, EquipGameData, EquipInventoryData, EquipInventoryDataListEntry,
    GaitemCategory, GaitemHandle, ItemCategory, ItemId, OptionalItemId,
};
use shared::FromStatic;

/// An error giving an item can fail with.
#[derive(Debug, Error)]
pub enum GiveItemError {
    /// The [`CSGaitemImp`] singleton isn't available yet.
    #[error("CSGaitem singleton not available")]
    GaitemSingletonUnavailable,
}

impl EquipGameData {
    /// Gives `quantity` of `item_id` to this equipment's owner, mirroring the
    /// effects of the game's own item-granting logic (item stacking rules,
    /// key-item deduplication, gaitem allocation) without calling into the
    /// game's own code.
    ///
    /// Returns the quantity actually added, which may be less than
    /// `quantity` (including 0) if the target stack or the inventory is
    /// full. Unlike the game, this never spawns a world item drop for any
    /// remainder that didn't fit — the caller can inspect the returned
    /// quantity and decide what to do about the difference, if any.
    ///
    /// Mirrors the positive-quantity branches of `AddOrRemoveItem`.
    pub fn give_item(&mut self, item_id: ItemId, quantity: u32) -> Result<u32, GiveItemError> {
        if quantity == 0 {
            return Ok(0);
        }

        let gaitem = unsafe { CSGaitemImp::instance_mut() }
            .map_err(|_| GiveItemError::GaitemSingletonUnavailable)?;

        if EquipInventoryData::is_stackable(item_id) {
            let handle = match item_id.category() {
                ItemCategory::Goods => {
                    gaitem.allocate_partial_gaitem(GaitemCategory::Goods, item_id.param_id())
                }
                _ => match gaitem.allocate_indexed_gaitem(item_id) {
                    Some(handle) => handle,
                    None => return Ok(0),
                },
            };

            Ok(self
                .equip_inventory_data
                .give_stackable(item_id, quantity, handle))
        } else {
            let is_key_item = EquipInventoryData::is_key_item(item_id);

            // Unstackable items get one inventory entry per copy, each
            // backed by its own gaitem instance, mirroring the per-unit loop
            // in `AddOrRemoveItem`'s unstackable branch.
            let mut given = 0;
            for _ in 0..quantity {
                let Some(handle) = gaitem.allocate_indexed_gaitem(item_id) else {
                    break;
                };

                let sort_id = self.equip_inventory_data.next_sort_id;
                self.equip_inventory_data.next_sort_id += 1;

                let inserted = self.equip_inventory_data.items_data.insert_entry(
                    EquipInventoryDataListEntry {
                        gaitem_handle: handle,
                        item_id,
                        quantity: 1,
                        sort_id,
                        is_new: true,
                        pot_group: -1,
                    },
                    is_key_item,
                );

                if inserted.is_none() {
                    // No inventory slot available for the gaitem we just
                    // allocated: release it so it doesn't leak, then stop.
                    gaitem.release_handle(handle);
                    break;
                }

                given += 1;
            }

            Ok(given)
        }
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

                if let Some(equip_slot) = find_equipped_slot(self, slot) {
                    unequip_slot(self, equip_slot, gaitem);
                }

                let Some(handle) = self.equip_inventory_data.items_data.remove_entry(slot) else {
                    break;
                };
                gaitem.release_handle(handle);

                removed += 1;
            }

            removed
        }
    }
}

/// Finds the [`ChrAsmSlot`] currently equipped to the given global inventory
/// `slot`, if any.
///
/// Mirrors `GetSlotOfInventoryId_`.
fn find_equipped_slot(equipment: &EquipGameData, slot: u32) -> Option<ChrAsmSlot> {
    equipment
        .equipment_item_idx_list
        .iter()
        .position(|&idx| idx == slot)
        .and_then(|i| ChrAsmSlot::from_index(i as u32).ok())
}

/// Clears the given equipped [`ChrAsmSlot`], releasing the gaitem handle
/// reference it held.
///
/// Mirrors `FUN_140247160`'s slot-clearing path (the unarmed/empty-slot
/// fallback and cosmetic re-equip logic it also performs aren't
/// reimplemented here, since they only affect rendering/animation state, not
/// inventory data).
fn unequip_slot(equipment: &mut EquipGameData, slot: ChrAsmSlot, gaitem: &mut CSGaitemImp) {
    gaitem.swap_handle(
        &mut equipment.chr_asm.gaitem_handles[slot as usize],
        GaitemHandle(0),
    );
    equipment.chr_asm.equipment_param_ids[slot as usize] = -1;
    equipment.equipment_entries[slot] = OptionalItemId::NONE;
    equipment.equipment_item_idx_list[slot as usize] = u32::MAX;
}
