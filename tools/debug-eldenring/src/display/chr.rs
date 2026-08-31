use std::str::FromStr;

use hudhook::imgui::{TableColumnSetup, Ui};

use debug::UiExt;
use eldenring::cs::{
    CSGaitemImp, ChrAsm, ChrAsmEquipEntries, ChrAsmEquipment, ChrAsmSlot, ChrIns, ChrInsExt,
    ChrInsSubclassMut, EquipGameData, EquipInventoryData, EquipInventoryDataListEntry,
    EquipItemData, EquipMagicData, InventoryItemsData, ItemReplenishStateTracker,
    PlayerDataAttackRating, PlayerGameData, PlayerIns,
};
use fromsoftware_shared::{FromStatic, MaybeEmpty, NonEmptyIteratorExt};

use crate::display::{DebugDisplay, DisplayUiExt, StatefulDebugDisplay};

#[derive(Default)]
pub struct ChrInsState {
    new_speffect: String,
}

impl StatefulDebugDisplay for PlayerIns {
    type State = ChrInsState;

    fn render_debug_mut(&mut self, ui: &Ui, state: &mut Self::State) {
        chr_ins_common_debug(&mut self.chr_ins, ui, state);

        ui.nested("ChrAsm", &self.chr_asm);
        ui.nested("PlayerGameData", unsafe { self.player_game_data.as_ref() });
        ui.nested(
            "Session Player Entry",
            self.session_manager_player_entry.as_ref(),
        );
        ui.display(
            "Invincibility timer",
            self.invincibility_timer_for_net_player,
        );
        ui.display("Locked on enemy", self.locked_on_enemy);
        ui.display("Block position", self.block_position);
    }
}

impl DebugDisplay for ChrAsm {
    fn render_debug(&self, ui: &Ui) {
        ui.nested("ChrAsmEquipment", &self.equipment);

        // One row per slot, with the handle resolved through `CSGaitemImp`
        // rather than shown raw. Each equipped slot is really three values
        // that have to agree — the handle here, the param id beside it, and
        // the gaitem the handle points at — and `equip_slot` writes all three
        // from one inventory entry. When they disagree, the slot is holding a
        // stale or misresolved reference, which is exactly what a weapon
        // appearing in a protector slot looks like.
        //
        // "Gaitem Item ID" is what the pool actually holds for that handle;
        // "Param ID" is what `ChrAsm` recorded. Those two are written
        // together and should always match.
        let gaitem = unsafe { CSGaitemImp::instance() }.ok();

        ui.header("Equipped slots", || {
            ui.table(
                "chr-asm-equipped-slots",
                [
                    TableColumnSetup::new("Index"),
                    TableColumnSetup::new("Slot"),
                    TableColumnSetup::new("Gaitem Handle"),
                    TableColumnSetup::new("Gaitem Item ID"),
                    TableColumnSetup::new("Param ID"),
                ],
                self.gaitem_handles
                    .iter()
                    .zip(self.equipment_param_ids.iter())
                    .enumerate(),
                |ui, _, (index, (handle, param_id))| {
                    ui.table_next_column();
                    ui.text(index.to_string());

                    ui.table_next_column();
                    match ChrAsmSlot::from_index(index as u32) {
                        Ok(slot) => ui.text(format!("{slot:?}")),
                        Err(err) => ui.text(err.to_string()),
                    }

                    ui.table_next_column();
                    ui.text(handle.to_string());

                    ui.table_next_column();
                    if handle.0 == 0 {
                        ui.text("<empty>");
                    } else if !handle.is_indexed() {
                        // Goods/Accessory handles are bare — no pool entry
                        // backs them, so there's nothing to resolve.
                        ui.text("<not indexed>");
                    } else {
                        match gaitem.and_then(|g| g.gaitem_ins_by_handle(handle)) {
                            Some(ins) => ui.text(format!("{:?}", ins.item_id)),
                            None => ui.text("<unresolved>"),
                        }
                    }

                    ui.table_next_column();
                    ui.text(param_id.to_string());
                },
            );
        });
    }
}

impl DebugDisplay for ChrAsmEquipment {
    fn render_debug(&self, ui: &Ui) {
        ui.debug("Arm style", self.arm_style);
        ui.debug(
            "Left-hand weapon slot",
            self.selected_slots.left_weapon_slot,
        );
        ui.debug(
            "Right-hand weapon slot",
            self.selected_slots.right_weapon_slot,
        );
        ui.debug("Left-hand arrow slot", self.selected_slots.left_arrow_slot);
        ui.debug(
            "Right-hand arrow slot",
            self.selected_slots.right_arrow_slot,
        );
        ui.debug("Left-hand bolt slot", self.selected_slots.left_bolt_slot);
        ui.debug("Right-hand bolt slot", self.selected_slots.right_bolt_slot);
    }
}

impl DebugDisplay for ChrAsmEquipEntries {
    fn render_debug(&self, ui: &Ui) {
        // Indexed by `ChrAsmSlot`, in the same order as
        // `ChrAsm::gaitem_handles` — the two are written together by
        // `equip_slot`, so this table lines up row-for-row with `ChrAsm`'s
        // and any disagreement between them is a half-applied equip.
        //
        // Ids are shown whole rather than through `param_id()`, since the
        // category nibble is what distinguishes e.g. a Protector from a
        // Weapon and is exactly what goes wrong when an id is built or
        // stored incorrectly.
        ui.header("Slots", || {
            ui.table(
                "chr-asm-equip-entries-slots",
                [
                    TableColumnSetup::new("Index"),
                    TableColumnSetup::new("Slot"),
                    TableColumnSetup::new("Item ID"),
                ],
                (0..22u32).filter_map(|index| {
                    let slot = ChrAsmSlot::from_index(index).ok()?;
                    Some((index, slot, self[slot]))
                }),
                |ui, _, (index, slot, item_id)| {
                    ui.table_next_column();
                    ui.text(index.to_string());

                    ui.table_next_column();
                    ui.text(format!("{slot:?}"));

                    ui.table_next_column();
                    ui.text(format!("{item_id:?}"));
                },
            );
        });

        ui.list("Quick Items", self.quick_tems.iter(), |ui, index, item| {
            ui.text(format!("{}: {:?}", index, item));
        });

        ui.list("Pouch", self.pouch.iter(), |ui, i, item| {
            ui.text(format!("{}: {:?}", i, item));
        });
    }
}

impl DebugDisplay for PlayerGameData {
    fn render_debug(&self, ui: &Ui) {
        ui.display("Player ID", self.player_id);
        ui.display("Character ID", self.character_id);
        ui.display("Character Event ID", self.character_event_id);
        ui.display("Game Data Man Index", self.game_data_man_index);
        ui.debug("Character Type", self.chr_type);
        ui.debug("Multiplay Role", self.multiplay_role);
        ui.debug("Sell Region", self.sell_region);
        ui.display("Is My World", self.is_my_world);
        ui.display("Is Main Player", self.is_main_player);
        ui.display("Is Voice Chat Enabled", self.is_voice_chat_enabled);

        ui.header("Character", || {
            ui.display("Level", self.level);
            ui.display("Gender", self.gender);
            ui.display("Archetype", self.archetype);
            ui.display("Voice Type", self.voice_type);
            ui.display("Starting Gift", self.starting_gift);
            ui.display("Vow Type", self.vow_type);
            ui.display("Unlocked Magic Slots", self.unlocked_magic_slots);
            ui.display("Unlocked Talisman Slots", self.unlocked_talisman_slots);
            ui.display("Scadutree Blessing", self.scadutree_blessing);
            ui.display("Reversed Spirit Ash", self.reversed_spirit_ash);
        });

        ui.header("Resources", || {
            ui.display(
                "HP",
                format!(
                    "{} / {} (base {})",
                    self.current_hp, self.current_max_hp, self.base_max_hp
                ),
            );
            ui.display(
                "FP",
                format!(
                    "{} / {} (base {})",
                    self.current_fp, self.current_max_fp, self.base_max_fp
                ),
            );
            ui.display(
                "Stamina",
                format!(
                    "{} / {} (base {})",
                    self.current_stamina, self.current_max_stamina, self.base_max_stamina
                ),
            );
            ui.display("HP Flask Max", self.max_hp_flask);
            ui.display("HP Estus Rate", self.hp_estus_rate);
            ui.display("HP Estus Additional", self.hp_estus_additional);
            ui.display("FP Flask Max", self.max_fp_flask);
            ui.display("FP Estus Rate", self.fp_estus_rate);
            ui.display("FP Estus Additional", self.fp_estus_additional);
        });

        ui.header("Attributes", || {
            ui.display(
                "Vigor",
                format!("{} (effective {})", self.vigor, self.effective_vigor),
            );
            ui.display(
                "Mind",
                format!("{} (effective {})", self.mind, self.effective_mind),
            );
            ui.display(
                "Endurance",
                format!(
                    "{} (effective {})",
                    self.endurance, self.effective_endurance
                ),
            );
            ui.display(
                "Strength",
                format!("{} (effective {})", self.strength, self.effective_strength),
            );
            ui.display(
                "Dexterity",
                format!(
                    "{} (effective {})",
                    self.dexterity, self.effective_dexterity
                ),
            );
            ui.display(
                "Intelligence",
                format!(
                    "{} (effective {})",
                    self.intelligence, self.effective_intelligence
                ),
            );
            ui.display(
                "Faith",
                format!("{} (effective {})", self.faith, self.effective_faith),
            );
            ui.display(
                "Arcane",
                format!("{} (effective {})", self.arcane, self.effective_arcane),
            );
            ui.display("Effective Vitality", self.effective_vitality);
        });

        ui.header("Combat", || {
            ui.display("Poise", self.poise);
            ui.display("Discovery", self.discovery);
            ui.display("Max Equip Load", self.max_equip_load);
            ui.display("Base Durability", self.base_durability);
            ui.display("Damage Negation Physical", self.damage_negation_physical);
            ui.display("Damage Negation Strike", self.damage_negation_strike);
            ui.display("Damage Negation Slash", self.damage_negation_slash);
            ui.display("Damage Negation Pierce", self.damage_negation_pierce);
            ui.display("Damage Negation Magic", self.damage_negation_magic);
            ui.display("Damage Negation Fire", self.damage_negation_fire);
            ui.display("Damage Negation Lightning", self.damage_negation_lightning);
            ui.display("Damage Negation Holy", self.damage_negation_holy);
            ui.nested("Attack Rating", &self.attack_rating);
        });

        ui.header("Status Resistances", || {
            const NAMES: [&str; 7] = [
                "Poison", "Rot", "Bleed", "Death", "Frost", "Sleep", "Madness",
            ];
            let base = [
                self.poison_resist,
                self.rot_resist,
                self.bleed_resist,
                self.death_resist,
                self.frost_resist,
                self.sleep_resist,
                self.madness_resist,
            ];
            for (i, name) in NAMES.iter().enumerate() {
                ui.display(
                    *name,
                    format!(
                        "{} / {} (resist {})",
                        self.resistance_gauges[i], self.resistance_gauge_max[i], base[i]
                    ),
                );
            }
            ui.display("Resist Curse Item Count", self.resist_curse_item_count);
            ui.display("Pending Block Clear Bonus", self.pending_block_clear_bonus);
        });

        ui.header("Status Proc Timers", || {
            const NAMES: [&str; 7] = [
                "Poison", "Rot", "Bleed", "Death", "Frost", "Sleep", "Madness",
            ];
            for (i, name) in NAMES.iter().enumerate() {
                ui.display(
                    *name,
                    format!(
                        "{:.2} / {:.2}",
                        self.proc_status_timers[i], self.proc_status_timer_max[i]
                    ),
                );
            }
        });

        ui.header("Progression", || {
            ui.display("Rune Count", self.rune_count);
            ui.display("Rune Memory", self.rune_memory);
            ui.display("Reached Max Rune Memory", self.reached_max_rune_memory);
            ui.display("Base Hero Point", self.base_hero_point);
            ui.display("Base Hero Point 2", self.base_hero_point_2);
            ui.display("Matching Weapon Level", self.matching_weapon_level);
            ui.display(
                "Matchmaking Spirit Ashes Level",
                self.matchmaking_spirit_ashes_level,
            );
        });

        ui.header("Multiplayer", || {
            ui.debug(
                "Furlcalling Finger Active",
                self.furlcalling_finger_remedy_active,
            );
            ui.debug("Rune Arc Active", self.rune_arc_active);
            ui.debug("White Ring Active", self.white_ring_active);
            ui.debug("Blue Ring Active", self.blue_ring_active);
            ui.display("Total Summon Count", self.total_summon_count);
            ui.display("Coop Success Count", self.coop_success_count);
            ui.display("Invasions Success Count", self.invasions_success_count);
            ui.display("Solo Breakin Point", self.solo_breakin_point);
            ui.display("Invaders Killed", self.invaders_killed);
            ui.display(
                "Is Using Festering Bloody Finger",
                self.is_using_festering_bloody_finger,
            );
            ui.debug("Used Invasion Item Type", self.used_invasion_item_type);
            ui.display("Mount Handle", self.mount_handle);
            ui.display("Sign Cooldown Enabled", self.sign_cooldown_enabled);
        });

        ui.header("Quickmatch", || {
            ui.display("Kill Count", self.quickmatch_kill_count);
            ui.display("Team", self.quick_match_team);
            ui.debug("Desired Team", self.quick_match_desired_team);
            ui.display("Spawn Slot Fraction", self.quick_match_spawn_slot_fraction);
            ui.display("Is Lead", self.is_quick_match_lead);
            ui.display("Is In Game", self.is_quick_match_in_game);
            ui.display("Is Host", self.is_quick_match_host);
            ui.display("Map Load Ready", self.quick_match_map_load_ready);
            ui.display("Duel Points", self.quick_match_duel_points);
            ui.display(
                "United Combat Points",
                self.quick_match_united_combat_points,
            );
            ui.display("Spirit Ashes Points", self.quick_match_spirit_ashes_points);
            ui.display("Duel Rank", self.quickmatch_duel_rank);
            ui.display("United Combat Rank", self.quickmatch_united_combat_rank);
            ui.display("Spirit Ashes Rank", self.quickmatch_spirit_ashes_rank);
            ui.display("Host Scadutree Blessing", self.host_scadutree_blessing);
            ui.display("Host Scaling Applied", self.host_scaling_applied);
        });

        ui.nested("EquipGameData", &self.equipment);
        ui.nested_opt("Storage Box EquipInventoryData", self.storage.as_ref());
    }
}

impl DebugDisplay for PlayerDataAttackRating {
    fn render_debug(&self, ui: &Ui) {
        ui.display("Left Primary", self.left_armament_primary);
        ui.display("Right Primary", self.right_armament_primary);
        ui.display("Left Secondary", self.left_armament_secondary);
        ui.display("Right Secondary", self.right_armament_secondary);
        ui.display("Left Tertiary", self.left_armament_tertiary);
        ui.display("Right Tertiary", self.right_armament_tertiary);
    }
}

impl DebugDisplay for EquipGameData {
    fn render_debug(&self, ui: &Ui) {
        ui.display("Is main player", self.is_main_player);
        ui.debug("Last add item result", self.last_add_item_result);
        ui.debug("Broken equipment slots", self.broken_equipment_slots);

        ui.nested("ChrAsm", &self.chr_asm);

        // The fourth array `equip_slot` writes, alongside
        // `chr_asm.gaitem_handles`, `chr_asm.equipment_param_ids` and
        // `equipment_entries` — it maps each `ChrAsmSlot` back to the global
        // inventory slot the equipped item lives in, and `u32::MAX` means
        // nothing equipped. "Resolves To" is what that inventory slot
        // actually holds now, so a row whose resolved item disagrees with
        // the same slot in `ChrAsm` is pointing at a stale entry.
        ui.header("Equipment Item Index List", || {
            ui.table(
                "equip-game-data-equipment-item-idx-list",
                [
                    TableColumnSetup::new("Index"),
                    TableColumnSetup::new("Slot"),
                    TableColumnSetup::new("Inventory Slot"),
                    TableColumnSetup::new("Resolves To"),
                    TableColumnSetup::new("Consistent"),
                ],
                self.equipment_item_idx_list.iter().enumerate(),
                |ui, _, (index, inventory_slot)| {
                    ui.table_next_column();
                    ui.text(index.to_string());

                    let slot = ChrAsmSlot::from_index(index as u32);

                    ui.table_next_column();
                    match &slot {
                        Ok(slot) => ui.text(format!("{slot:?}")),
                        Err(err) => ui.text(err.to_string()),
                    }

                    ui.table_next_column();
                    if *inventory_slot == u32::MAX {
                        ui.text("<none>");
                    } else {
                        ui.text(inventory_slot.to_string());
                    }

                    ui.table_next_column();
                    if *inventory_slot == u32::MAX {
                        ui.text("<none>");
                    } else {
                        ui.text(resolved_slot_item(
                            &self.equip_inventory_data,
                            *inventory_slot as i32,
                        ));
                    }

                    // All three of `equipment_entries`, the equipped gaitem
                    // handle, and the entry `equipment_item_idx_list` names
                    // are written from one inventory slot by
                    // `SetEquipmentEntries`/`ChrAsm::EquipItem`, so none of
                    // them can legitimately disagree. The gaitem handle is
                    // checked too, not just the id: a slot whose entries and
                    // index agree can still hold a handle belonging to a
                    // completely different item, which is invisible if only
                    // the ids are compared.
                    ui.table_next_column();
                    match slot {
                        Ok(slot) => {
                            let entry = self.equipment_entries[slot].as_valid();
                            let inventory_entry = (*inventory_slot != u32::MAX)
                                .then(|| {
                                    self.equip_inventory_data
                                        .items_data
                                        .entry_at_slot(*inventory_slot)
                                        .and_then(|e| e.as_option())
                                })
                                .flatten();
                            let resolved = inventory_entry.and_then(|e| e.item_id.as_valid());

                            let equipped_handle = self.chr_asm.gaitem_handles[slot as usize];
                            let entry_handle = inventory_entry.map(|e| e.gaitem_handle);

                            let mut problems = Vec::new();
                            if entry != resolved {
                                problems.push(format!("entries={entry:?} idx_list->{resolved:?}"));
                            }
                            if let Some(entry_handle) = entry_handle
                                && entry_handle != equipped_handle
                            {
                                problems.push(format!(
                                    "handle={equipped_handle:?} entry has {entry_handle:?}"
                                ));
                            }

                            if problems.is_empty() {
                                ui.text("ok");
                            } else {
                                ui.text(format!("MISMATCH {}", problems.join("; ")));
                            }
                        }
                        Err(_) => ui.text("-"),
                    }
                },
            );
        });

        ui.nested("Equipment Entries", &self.equipment_entries);

        ui.header("Physick Tears", || {
            ui.list("Tears", self.physick_tears.iter(), |ui, i, tear| {
                ui.text(format!("{i}: {tear:?}"));
            });
            ui.debug("Extra tear", self.extra_physick_tear);
        });

        ui.nested("EquipInventoryData", &self.equip_inventory_data);
        ui.nested("EquipMagicData", &self.equip_magic_data);
        ui.nested("EquipItemData", &self.equip_item_data);
        ui.nested_opt(
            "Item Replenish State Tracker",
            self.item_replenish_state_tracker.as_ref(),
        );

        if let Some(qm_item_backup_vector) = self.qm_item_backup_vector.as_ref() {
            ui.header("QM Item Backup Vector", || {
                ui.table(
                    "equip-game-data-qm-item-backup-vector",
                    [
                        TableColumnSetup::new("Index"),
                        TableColumnSetup::new("Item ID"),
                        TableColumnSetup::new("Quantity"),
                    ],
                    qm_item_backup_vector.iter(),
                    |ui, i, item| {
                        ui.table_next_column();
                        ui.text(format!("{i}"));
                        ui.table_next_column();
                        ui.text(format!("{:?}", item.item_id));
                        ui.table_next_column();
                        ui.text(format!("{}", item.quantity));
                    },
                );
            });
        } else {
            ui.text("QM Item Backup Vector: None");
        }
    }
}

impl DebugDisplay for ItemReplenishStateTracker {
    fn render_debug(&self, ui: &Ui) {
        ui.table(
            "item-replenish-state-tracker-entries",
            [
                TableColumnSetup::new("Index"),
                TableColumnSetup::new("Item ID"),
                TableColumnSetup::new("Auto Replenish"),
            ],
            self.entries().iter(),
            |ui, index, item| {
                ui.table_next_column();
                ui.text(index.to_string());

                ui.table_next_column();
                ui.text(format!("{:?}", item.item_id));

                ui.table_next_column();
                ui.text(item.auto_replenish.to_string());
            },
        );
        ui.text(format!("Count: {}", self.count));
    }
}

impl DebugDisplay for EquipMagicData {
    fn render_debug(&self, ui: &Ui) {
        ui.text(format!("Selected slot: {}", self.selected_slot));

        ui.header("EquipDataItem", || {
            ui.table(
                "equip-magic-data-entries",
                [
                    TableColumnSetup::new("Index"),
                    TableColumnSetup::new("Param ID"),
                    TableColumnSetup::new("Charges"),
                ],
                self.entries.iter(),
                |ui, index, item| {
                    ui.table_next_column();
                    ui.text(index.to_string());

                    ui.table_next_column();
                    ui.text(item.param_id.to_string());

                    ui.table_next_column();
                    ui.text(item.charges.to_string());
                },
            );
        });
    }
}

impl DebugDisplay for EquipItemData {
    fn render_debug(&self, ui: &Ui) {
        ui.display("Selected quick slot", self.selected_quick_slot);

        let inventory = unsafe { self.inventory.as_ref() };
        let entries = unsafe { self.equip_entries.as_ref() };

        // Each of these slots is a *pair*: an `EquipDataItem` (gaitem handle +
        // inventory slot) here, and the mirrored item id in
        // `ChrAsmEquipEntries`. The two must agree, and the inventory slot
        // must still hold the item the handle refers to.
        //
        // `CS::EquipGameData::RemoveItem` clears both halves of any slot
        // pointing at a removed entry (`FUN_140250060` for quick,
        // `FUN_140250100` for pouch, `FUN_140250030` for the great rune).
        // Skip that and the index goes on pointing at whatever later reuses
        // the slot — which the UI then renders in place of the real item.
        // "Resolves to" below is what the inventory actually holds at that
        // index right now, so a mismatch against "Mirrored ID" is a stale
        // reference.
        equip_data_item_table(
            ui,
            "Quick slots",
            "equip-item-data-quick-slots",
            &self.quick_slots,
            &entries.quick_tems,
            inventory,
        );

        equip_data_item_table(
            ui,
            "Pouch slots",
            "equip-item-data-pouch-slots",
            &self.pouch_slots,
            &entries.pouch,
            inventory,
        );

        ui.header("Great rune", || {
            ui.display("Gaitem handle", self.great_rune.gaitem_handle.to_string());
            ui.display("Inventory slot", self.great_rune.index);
            ui.display(
                "Resolves to",
                resolved_slot_item(inventory, self.great_rune.index),
            );
        });

        // Same object `EquipGameData` renders directly — shown again here
        // because reaching it through this back-pointer is what proves the
        // pointer is still valid.
        ui.nested("Equipment Entries (via back-pointer)", entries);
    }
}

/// What the inventory currently holds at global slot `index`, for
/// cross-checking a stored slot reference against reality.
fn resolved_slot_item(inventory: &EquipInventoryData, index: i32) -> String {
    let Ok(slot) = u32::try_from(index) else {
        return "<none>".to_string();
    };

    match inventory
        .items_data
        .entry_at_slot(slot)
        .and_then(|entry| entry.as_option())
    {
        Some(entry) => format!("{:?}", entry.item_id),
        None => "<empty slot>".to_string(),
    }
}

/// Renders a quick-slot or pouch table alongside its mirrored ids, so the two
/// halves of each slot can be compared at a glance.
fn equip_data_item_table(
    ui: &Ui,
    label: &str,
    id: &str,
    slots: &[eldenring::cs::EquipDataItem],
    mirrored: &[eldenring::cs::OptionalItemId],
    inventory: &EquipInventoryData,
) {
    let gaitem = unsafe { CSGaitemImp::instance() }.ok();

    ui.header(label, || {
        ui.table(
            id,
            [
                TableColumnSetup::new("Slot"),
                TableColumnSetup::new("Gaitem Handle"),
                TableColumnSetup::new("Gaitem Item ID"),
                TableColumnSetup::new("Inventory Slot"),
                TableColumnSetup::new("Resolves To"),
                TableColumnSetup::new("Mirrored ID"),
            ],
            slots.iter().enumerate(),
            |ui, _, (index, item)| {
                ui.table_next_column();
                ui.text(index.to_string());

                ui.table_next_column();
                ui.text(item.gaitem_handle.to_string());

                ui.table_next_column();
                if item.gaitem_handle.0 == 0 {
                    ui.text("<empty>");
                } else if !item.gaitem_handle.is_indexed() {
                    ui.text("<not indexed>");
                } else {
                    match gaitem.and_then(|g| g.gaitem_ins_by_handle(&item.gaitem_handle)) {
                        Some(ins) => ui.text(format!("{:?}", ins.item_id)),
                        None => ui.text("<unresolved>"),
                    }
                }

                ui.table_next_column();
                ui.text(item.index.to_string());

                ui.table_next_column();
                ui.text(resolved_slot_item(inventory, item.index));

                ui.table_next_column();
                match mirrored.get(index) {
                    Some(id) => ui.text(format!("{id:?}")),
                    None => ui.text("<out of range>"),
                }
            },
        );
    });
}

impl DebugDisplay for EquipInventoryData {
    fn render_debug(&self, ui: &Ui) {
        ui.nested("InventoryItemsData", &self.items_data);

        // Bounds several of the game's own scans (`GetQuantityByItemId`,
        // `AdjustQuantityBy`, `GetInventoryItemEntryByIndex` all walk
        // `0..=highest_item_slot`), so an entry past it is present in memory
        // but invisible to them. Raised by `InsertItem`, and lowered by one
        // by `EquipInventoryData::RemoveItem` only when the removed slot is
        // exactly this one — an upper bound, not a tight maximum.
        ui.display("Highest item slot", self.highest_item_slot);
        ui.display("Next sort ID", self.next_sort_id);
        ui.display("Unlimited Consumables", self.unlimited_consumables);
        ui.display("Limited Consumables", self.limited_pots);

        ui.header("Pot Groups", || {
            ui.table(
                "equip-inventory-data-pot-groups",
                [
                    TableColumnSetup::new("Group"),
                    TableColumnSetup::new("Count"),
                    TableColumnSetup::new("Capacity"),
                ],
                self.pot_items_count
                    .iter()
                    .zip(self.pot_items_capacity.iter())
                    .enumerate()
                    .filter(|(_, (count, capacity))| **count != 0 || **capacity != 0),
                |ui, _, (group, (count, capacity))| {
                    ui.table_next_column();
                    ui.text(group.to_string());

                    ui.table_next_column();
                    ui.text(count.to_string());

                    ui.table_next_column();
                    ui.text(capacity.to_string());
                },
            );
        });

        ui.list(
            "Recent Item Indices",
            self.recent_item_indices.iter(),
            |ui, i, item| {
                ui.text(format!("{}: {:?}", i, item));
            },
        );
    }
}

/// Renders one inventory entry table, with **real global slot indices**
/// rather than positions within the filtered iterator.
///
/// The distinction matters: a global slot is what `equipment_item_idx_list`,
/// `EquipDataItem::index` and the item-id lookup map all store, so a table
/// keyed by anything else can't be cross-referenced against them. Normal
/// items start at `key_items_capacity`, key items at 0.
fn inventory_entry_table(
    ui: &Ui,
    id: &str,
    entries: &[MaybeEmpty<EquipInventoryDataListEntry>],
    first_slot: u32,
) {
    // "Gaitem Item ID" resolves the entry's handle through `CSGaitemImp` and
    // should always match the entry's own "Item ID" — the two are written
    // together when the entry is created. A mismatch means the handle points
    // at a pool slot that's since been reused, and `<unresolved>` means it
    // points at nothing at all.
    let gaitem = unsafe { CSGaitemImp::instance() }.ok();

    ui.table(
        id,
        [
            TableColumnSetup::new("Slot"),
            TableColumnSetup::new("Gaitem Handle"),
            TableColumnSetup::new("Gaitem Item ID"),
            TableColumnSetup::new("Item ID"),
            TableColumnSetup::new("Quantity"),
            TableColumnSetup::new("Sort ID"),
            TableColumnSetup::new("Pot Group"),
            TableColumnSetup::new("Is New"),
        ],
        entries
            .iter()
            .enumerate()
            .filter_map(|(offset, entry)| Some((first_slot + offset as u32, entry.as_option()?))),
        |ui, _, (slot, item)| {
            ui.table_next_column();
            ui.text(slot.to_string());

            ui.table_next_column();
            ui.text(item.gaitem_handle.to_string());

            ui.table_next_column();
            if !item.gaitem_handle.is_indexed() {
                ui.text("<not indexed>");
            } else {
                match gaitem.and_then(|g| g.gaitem_ins_by_handle(&item.gaitem_handle)) {
                    Some(ins) => ui.text(format!("{:?}", ins.item_id)),
                    None => ui.text("<unresolved>"),
                }
            }

            ui.table_next_column();
            ui.text(format!("{:?}", item.item_id));

            ui.table_next_column();
            ui.text(item.quantity.to_string());

            ui.table_next_column();
            ui.text(item.sort_id.to_string());

            ui.table_next_column();
            ui.text(item.pot_group.to_string());

            ui.table_next_column();
            ui.text(item.is_new.to_string());
        },
    );
}

impl DebugDisplay for InventoryItemsData {
    fn render_debug(&self, ui: &Ui) {
        // `*_len` are occupancy counts, not extents — slots are sparse, so
        // the highest occupied slot can sit well above the count. Both are
        // shown, since a mismatch between the count and the number of rows
        // below means the counters have drifted from reality.
        let key_capacity = self.key_items_capacity;

        let occupied_normal = self.normal_entries().iter().non_empty().count();
        let label = format!(
            "Normal Items ({} occupied, len {}, cap {}, slots {}..)",
            occupied_normal, self.normal_items_len, self.normal_items_capacity, key_capacity,
        );
        ui.header(&label, || {
            inventory_entry_table(
                ui,
                "inventory-items-data-normal-items",
                self.normal_entries(),
                key_capacity,
            );
        });

        let occupied_key = self.key_entries().iter().non_empty().count();
        let label = format!(
            "Key Items ({} occupied, len {}, cap {}, slots 0..)",
            occupied_key, self.key_items_len, key_capacity,
        );
        ui.header(&label, || {
            inventory_entry_table(ui, "inventory-items-data-key-items", self.key_entries(), 0);
        });

        // The accessor is what every read and write in the game actually goes
        // through: it points at `key_items` in singleplayer and swaps to
        // `multiplay_key_items` in multiplayer (`SwapKeyItemsAccessor`). If
        // this doesn't match the Key Items table above, the session is in
        // multiplayer.
        let accessor_matches_key = std::ptr::eq(
            self.key_entries().as_ptr(),
            self.current_key_entries().as_ptr(),
        );
        ui.display(
            "Key items accessor",
            if accessor_matches_key {
                "key_items (singleplayer)"
            } else {
                "multiplay_key_items"
            },
        );

        let occupied_mp = self.multiplay_key_entries().iter().non_empty().count();
        let label = format!(
            "Multiplay Key Items ({} occupied, len {}, cap {}, slots 0..)",
            occupied_mp, self.multiplay_key_items_len, self.multiplay_key_items_capacity,
        );
        ui.header(&label, || {
            inventory_entry_table(
                ui,
                "inventory-items-data-multiplay-key-items",
                self.multiplay_key_entries(),
                0,
            );
        });
        ui.header("Item ID Map", || {
            ui.table(
                "inventory-items-data-item-map",
                [
                    TableColumnSetup::new("Index"),
                    TableColumnSetup::new("Is Free"),
                    TableColumnSetup::new("Item Id"),
                    TableColumnSetup::new("Item Slot"),
                    TableColumnSetup::new("Next Index"),
                ],
                unsafe { self.item_id_mapping.as_slice() },
                |ui, index, item| {
                    ui.table_next_column();
                    ui.text(index.to_string());

                    ui.table_next_column();
                    ui.text(format!("{:?}", item.is_free()));

                    ui.table_next_column();
                    ui.text(format!("{:?}", item.item_id));

                    ui.table_next_column();
                    ui.text(item.item_slot().to_string());

                    ui.table_next_column();
                    ui.text(item.next_mapping_item().to_string());
                },
            );
        });
        ui.list(
            "Item ID Indices",
            self.item_id_mapping_indices.iter(),
            |ui, i, item| {
                ui.text(format!("{}: {:?}", i, item));
            },
        );
    }
}

impl StatefulDebugDisplay for ChrIns {
    type State = ChrInsState;

    fn render_debug_mut(&mut self, ui: &Ui, state: &mut Self::State) {
        match ChrInsSubclassMut::from(self) {
            ChrInsSubclassMut::PlayerIns(player) => player.render_debug_mut(ui, state),
            mut chr_ins => chr_ins_common_debug(chr_ins.superclass_mut(), ui, state),
        }
    }
}

fn chr_ins_common_debug(chr_ins: &mut ChrIns, ui: &Ui, state: &mut ChrInsState) {
    ui.display("Team", chr_ins.team_type);
    ui.debug("Chr Type", chr_ins.chr_type);
    ui.display("Field Ins Handle", chr_ins.field_ins_handle);
    ui.display("P2P Entity Handle", &chr_ins.p2p_entity_handle);

    ui.display("Block ID", chr_ins.block_id);
    ui.display("Block ID Override", chr_ins.block_id_override);
    ui.display("Block ID Origin", chr_ins.block_origin);
    ui.display("Block ID Origin Override", chr_ins.block_origin_override);

    ui.nested("Chunk Position", chr_ins.chunk_position);
    ui.nested("Initial Position", chr_ins.initial_position);
    ui.nested("Initial Orientation", chr_ins.initial_orientation_euler);

    ui.debug("Backread State", chr_ins.backread_state);
    ui.debug("Activation Flags", chr_ins.chr_activation_flags);
    ui.debug("Flags 1c8", chr_ins.chr_flags1c8);
    ui.debug("Debug Flags", chr_ins.debug_flags);

    ui.display("Last hit by", chr_ins.last_hit_by);
    ui.debug("TAE use item", chr_ins.tae_queued_use_item);

    ui.header("Special Effect", || {
        ui.input_text("", &mut state.new_speffect).build();
        ui.same_line();
        let id = i32::from_str(&state.new_speffect);
        ui.disabled(id.is_err(), || {
            if ui.button("Apply") {
                chr_ins.apply_speffect(id.unwrap(), false);
            }
        });

        let mut remove = None;
        ui.table(
            "chr-ins-special-effects",
            [
                TableColumnSetup::new("ID"),
                TableColumnSetup::new("Timer"),
                TableColumnSetup::new("Removal timer"),
                TableColumnSetup::new("Duration"),
                TableColumnSetup::new("Interval Timer"),
                TableColumnSetup::new(""),
            ],
            chr_ins.special_effect.entries(),
            |ui, _i, entry| {
                ui.table_next_column();
                ui.text(format!("{}", entry.param_id));

                ui.table_next_column();
                ui.text(format!("{}", entry.interval_timer));

                ui.table_next_column();
                ui.text(format!("{}", entry.removal_timer));

                ui.table_next_column();
                ui.text(format!("{}", entry.duration));

                ui.table_next_column();
                ui.text(format!("{}", entry.interval_timer));

                ui.table_next_column();
                if ui.button("Remove") {
                    // We can't directly call `chr_ins.remove_speffect` here
                    // because `chr_ins` is already borrowed for the iteration.
                    remove = Some(entry.param_id);
                }
            },
        );

        if let Some(speffect) = remove {
            chr_ins.remove_speffect(speffect);
        }
    });

    ui.nested("Modules", &chr_ins.modules);
}
