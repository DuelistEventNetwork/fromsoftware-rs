use bitfield::bitfield;
use num_enum::TryFromPrimitive;

#[shared::singleton("MsgRepository")]
#[repr(C)]
pub struct MsgRepositoryImp;

/// Number of tag slots [`MsgTagManImp`] holds.
pub const MSG_TAG_COUNT: usize = 0x230;

/// Depth limit `CS::MsgTagManImp::SubstituteMsgTags` applies to nested
/// substitutions, after which remaining spans are left unsubstituted.
pub const MSG_TAG_MAX_RECURSION: u32 = 0x32;

#[repr(C)]
/// One substitution the tag engine can perform.
///
/// A menu string containing `<?name?>` has that span replaced by this entry's
/// value. A `callback` runs if present, otherwise `value` is returned as-is.
pub struct MsgTag {
    /// Tag name matched against the text between `<?` and `?>`, up to an
    /// optional `@` that separates arguments.
    pub name: *const u16,
    /// Substitution returned when [`callback`] is null.
    ///
    /// Points at a constructor-allocated buffer when [`capacity`] is non-zero.
    /// Otherwise `CS::MsgTagManImp::SetMsgTagString` repoints this at the
    /// caller's string rather than copying, so the pointee isn't owned here.
    ///
    /// [`callback`]: Self::callback
    /// [`capacity`]: Self::capacity
    pub value: *const u16,
    /// Characters [`value`]'s buffer holds, excluding the terminator, or `0`
    /// for tags that don't own a buffer. Setters skip the tag when not positive.
    ///
    /// [`value`]: Self::value
    pub capacity: u32,
    /// Numeric value the formatting callbacks render into [`value`], read as
    /// an `i32` by [`CB_Integer`] and as an `f32` by [`CB_Float`].
    ///
    /// [`value`]: Self::value
    /// [`CB_Integer`]: MsgTagCallback::Integer
    /// [`CB_Float`]: MsgTagCallback::Float
    pub numeric_value: u32,
    /// Precomputed hash of [`name`], compared before `wcsncmp` runs.
    ///
    /// [`name`]: Self::name
    pub name_hash: MsgTagNameHash,
    pub flags: MsgTagFlags,
    unk1d: [u8; 0x3],
    /// Called with the tag's arguments to produce the substitution.
    ///
    /// Null for tags whose value is set rather than computed. See
    /// [`MsgTagCallback`] for the five the game installs.
    pub callback: Option<
        unsafe extern "C" fn(
            *mut MsgTagManImp,
            tag: MsgTagType,
            args: *const u16,
            args_len: i32,
        ) -> *const u16,
    >,
}

bitfield! {
    #[repr(C)]
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MsgTagFlags(u8);
    impl Debug;
    bool;
    /// Makes the next formatting callback emit an empty string and clear this
    /// bit, blanking the tag for one resolve.
    pub blank_once, set_blank_once: 0;
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Cheap rejection test the resolver applies before comparing tag names.
///
/// Compared as a single `u32`, so a slot only reaches `wcsncmp` when both
/// halves already agree.
pub struct MsgTagNameHash {
    /// Number of UTF-16 code units in the name.
    pub len: u16,
    /// Sum of the name's UTF-16 code units, wrapping on overflow.
    pub sum: u16,
}

impl MsgTagNameHash {
    /// Computes the hash the resolver would build for `name`.
    pub fn of(name: &[u16]) -> Self {
        Self {
            len: name.len() as u16,
            sum: name.iter().fold(0u16, |sum, c| sum.wrapping_add(*c)),
        }
    }
}

/// Number of short-tag slots [`MsgTagManImp`] holds.
pub const MENU_SHORT_TAG_COUNT: usize = 15;

#[repr(C)]
/// A three-letter tag naming a block of FMG entries rather than one value.
///
/// Where a [`MsgTag`] carries the substitution itself, these carry the id the
/// menu offsets from, so `em2` covers the enemy names starting at 30000 and
/// `upr` those at 970000.
pub struct MenuShortTag {
    /// Three-letter name, matched the same way [`MsgTag::name`] is.
    pub name: *const u16,
    /// Precomputed hash of [`name`], built by the same routine that fills
    /// [`MsgTag::name_hash`].
    ///
    /// [`name`]: Self::name
    pub name_hash: MsgTagNameHash,
    /// First FMG entry id of the block this tag names.
    pub base_msg_id: u32,
}

/// The five substitution callbacks the game installs on [`MsgTag::callback`].
///
/// Named after the marker string each returns when it can't produce a value,
/// which is what shows up in the menu when a tag is used wrongly.
///
/// [`Integer`], [`Float`] and [`ImgTag`] render [`MsgTag::numeric_value`] and
/// ignore the tag's arguments. [`SingleId`] and [`System`] instead read the
/// argument after `@`, so they back the `<?tag@N?>` form.
///
/// [`Integer`]: Self::Integer
/// [`Float`]: Self::Float
/// [`ImgTag`]: Self::ImgTag
/// [`SingleId`]: Self::SingleId
/// [`System`]: Self::System
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MsgTagCallback {
    /// `?CB_Integer?` — formats [`MsgTag::numeric_value`] as a decimal
    /// integer, zero-padded to [`MsgTag::capacity`] digits.
    Integer,
    /// `?CB_Float?` — formats [`MsgTag::numeric_value`] as `%0.1f`. Used by
    /// the stats shown with one decimal, such as `weight` and the damage-cut
    /// percentages.
    Float,
    /// `?CB_SingleId?` — parses the text after `@` as a row id and looks up
    /// the matching name, so `<?keyicon@N?>` yields the icon for key `N`.
    ///
    /// Which table is consulted depends on the tag: the item name categories
    /// go to [`MsgRepositoryImp`], while `keyicon`, `kgicon`, `keyActName`
    /// and the `kg*` button tags resolve through `KeyAssignMenuItemParam`.
    SingleId,
    /// `?CB_System?` — builds a value from live system state rather than a
    /// param row. Backs `errcodeEOS`, `errcodeSteam`, `ssInfoMsg`, `keyMove`
    /// and `keyControlCamera`, the last two assembling a movement or camera
    /// key glyph from the current keybinds.
    System,
    /// `?CB_imgTag?` — resolves the evaluation image for `codenameIcon` by
    /// indexing a five-entry table with [`MsgTag::numeric_value`].
    ImgTag,
}

#[repr(C)]
#[shared::singleton("MsgTagMan")]
/// Resolves `<?tag?>` spans in menu strings.
///
/// `CS::MenuString::FormatTag` runs every string the menu displays through
/// this, so a tag placed in an FMG entry is substituted at display time.
/// Substitutions are re-scanned up to [`MSG_TAG_MAX_RECURSION`] deep, so a
/// tag's value may contain further tags.
pub struct MsgTagManImp {
    vftable: usize,
    /// Fixed-size table addressed by tag index, not a map: lookup by name is
    /// a linear scan comparing [`MsgTagNameHash`] and then the name itself.
    ///
    /// Every slot is populated by the constructor and indexed by its
    /// [`MsgTagType`] discriminant.
    pub tags: [MsgTag; MSG_TAG_COUNT],
    /// `DLTX::DlFixedString<wchar_t, 128>` scratch buffer the button-glyph
    /// tags build their result in, since the glyph is assembled from several
    /// strings rather than stored in the slot.
    ///
    /// Only `CB_SingleId` uses it, and it returns a pointer into this buffer,
    /// so the result lives only until the next such tag is resolved.
    unk5788: [u8; 0x130],
    /// The short tags, which name a base message id instead of a value.
    pub short_tags: [MenuShortTag; MENU_SHORT_TAG_COUNT],
    /// Maps a tag index onto its slot in the debug value cache, or `0xff`
    /// when it has not been assigned one.
    unk59a8: [u8; 0x230],
    /// Debug value cache, letting the debug menu show and edit a tag's last
    /// numeric value.
    unk5bd8: [u8; 0xbe8],
    /// Whether the debug value cache is in use.
    unk67c0: bool,
    /// `FD4DebugMenu` node registered when the debug menu manager exists.
    unk67c8: usize,
    unk67d0: [u8; 0x8],
}

impl MsgTagManImp {
    /// The tag occupying `index`.
    pub fn tag(&self, tag: MsgTagType) -> &MsgTag {
        &self.tags[tag as usize]
    }

    /// The tag's name, or `None` when the slot is unpopulated.
    pub fn name(&self, index: usize) -> Option<&[u16]> {
        let tag = self.tags.get(index)?;
        unsafe { nul_terminated(tag.name) }
    }

    /// The tag's current value, or `None` when it has none.
    ///
    /// Reflects only the stored value, so a tag that computes its
    /// substitution through a callback reports what was last written to the
    /// slot rather than what the callback would return.
    pub fn value(&self, index: usize) -> Option<&[u16]> {
        let tag = self.tags.get(index)?;
        unsafe { nul_terminated(tag.value) }
    }

    /// The tag's writable buffer, or `None` for tags that don't own one.
    ///
    /// Excludes the terminator slot, so the whole slice may be written and a
    /// `\0` still fits after it, which is what
    /// [`set_string`](Self::set_string) relies on.
    pub fn buffer_mut(&mut self, tag: MsgTagType) -> Option<&mut [u16]> {
        let slot = &mut self.tags[tag as usize];
        if slot.capacity < 1 || slot.value.is_null() {
            return None;
        }
        // SAFETY: `capacity` is the buffer's length in code units, and the
        // constructor allocates one more for the terminator.
        Some(unsafe {
            std::slice::from_raw_parts_mut(slot.value.cast_mut(), slot.capacity as usize)
        })
    }

    /// Writes `value` into the tag, mirroring
    /// `CS::MsgTagManImp::SetMsgTagString`.
    ///
    /// Copies into the tag's own buffer, truncating to [`MsgTag::capacity`]
    /// and terminating. Returns the number of code units written, or `None`
    /// for a tag that owns no buffer — use [`point_at`](Self::point_at) for
    /// those.
    pub fn set_string(&mut self, tag: MsgTagType, value: &str) -> Option<usize> {
        let capacity = self.tags[tag as usize].capacity;
        if capacity < 1 {
            return None;
        }

        let encoded: Vec<u16> = value.encode_utf16().take(capacity as usize).collect();
        let written = encoded.len();
        let buffer = self.buffer_mut(tag)?;
        buffer[..written].copy_from_slice(&encoded);
        // The buffer has room for a terminator past `capacity`.
        unsafe { buffer.as_mut_ptr().add(written).write(0) };
        Some(written)
    }

    /// Repoints a bufferless tag at `value` without copying.
    ///
    /// The game does this for tags whose [`MsgTag::capacity`] is zero. Returns
    /// `false` for a tag that owns a buffer, since overwriting its pointer
    /// would leak the allocation — use [`set_string`](Self::set_string) there.
    ///
    /// # Safety
    /// `value` must be nul-terminated and stay alive and unmoved for as long as
    /// the tag may be resolved, which is until the tag is set again.
    pub unsafe fn point_at(&mut self, tag: MsgTagType, value: *const u16) -> bool {
        let slot = &mut self.tags[tag as usize];
        if slot.capacity >= 1 {
            return false;
        }
        slot.value = value;
        true
    }

    /// Sets the numeric value the formatting callbacks render.
    ///
    /// Backs [`MsgTagCallback::Integer`] and [`MsgTagCallback::Float`], which
    /// read this rather than [`MsgTag::value`].
    pub fn set_numeric(&mut self, tag: MsgTagType, value: u32) {
        self.tags[tag as usize].numeric_value = value;
    }

    /// Finds a tag by name, the way the resolver does.
    ///
    /// Matching is case-sensitive, so `wpCriticalAtkPlus` and
    /// `WpCriticalAtkPlus` resolve to different tags.
    pub fn find(&self, name: &str) -> Option<MsgTagType> {
        let wanted: Vec<u16> = name.encode_utf16().collect();
        let hash = MsgTagNameHash::of(&wanted);

        let index = self.tags.iter().position(|tag| {
            tag.name_hash == hash && unsafe { nul_terminated(tag.name) } == Some(wanted.as_slice())
        })?;

        MsgTagType::try_from(index as u32).ok()
    }
}

/// Reads a nul-terminated UTF-16 string, or `None` if the pointer is null.
///
/// # Safety
/// `ptr` must either be null or point at a nul-terminated UTF-16 string that
/// outlives the returned slice.
unsafe fn nul_terminated<'a>(ptr: *const u16) -> Option<&'a [u16]> {
    if ptr.is_null() {
        return None;
    }
    let len = unsafe { (0..).take_while(|i| *ptr.add(*i) != 0).count() };
    Some(unsafe { std::slice::from_raw_parts(ptr, len) })
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, TryFromPrimitive)]
/// Every tag the constructor populates, named by the text that appears
/// between `<?` and `?>`.
///
/// The discriminant is the tag's index in [`MsgTagManImp::tags`]. Names are
/// matched case-sensitively.
pub enum MsgTagType {
    /// `<?OKBtn?>`
    OKBtn = 0,
    /// `<?NGBtn?>`
    NGBtn = 1,
    /// `<?WepName?>`
    WepName = 2,
    /// `<?Test?>`
    Test = 3,
    /// `<?pcName?>`
    PcName = 4,
    /// `<?pcGender?>`
    PcGender = 5,
    /// `<?pcOath?>`
    PcOath = 6,
    /// `<?pcVitality?>`
    PcVitality = 7,
    /// `<?pcWill?>`
    PcWill = 8,
    /// `<?pcEndur?>`
    PcEndur = 9,
    /// `<?pcStrength?>`
    PcStrength = 10,
    /// `<?pcDexter?>`
    PcDexter = 11,
    /// `<?pcDurability?>`
    PcDurability = 12,
    /// `<?pcIntellect?>`
    PcIntellect = 13,
    /// `<?pcForce?>`
    PcForce = 14,
    /// `<?pcLuck?>`
    PcLuck = 15,
    /// `<?pcHeroic?>`
    PcHeroic = 16,
    /// `<?pcLevel?>`
    PcLevel = 17,
    /// `<?pcGift?>`
    PcGift = 18,
    /// `<?pcAppearance?>`
    PcAppearance = 19,
    /// `<?pcFeature?>`
    PcFeature = 20,
    /// `<?pcFaceType?>`
    PcFaceType = 21,
    /// `<?pcHairType?>`
    PcHairType = 22,
    /// `<?pcHairEyeColor?>`
    PcHairEyeColor = 23,
    /// `<?pcHP?>`
    PcHP = 24,
    /// `<?pcMP?>`
    PcMP = 25,
    /// `<?pcStamina?>`
    PcStamina = 26,
    /// `<?pcMaxHp?>`
    PcMaxHp = 27,
    /// `<?pcMaxMp?>`
    PcMaxMp = 28,
    /// `<?pcMaxStamina?>`
    PcMaxStamina = 29,
    /// `<?pcItemOccur?>`
    PcItemOccur = 30,
    /// `<?equipWeight?>`
    EquipWeight = 31,
    /// `<?maxEquipWeight?>`
    MaxEquipWeight = 32,
    /// `<?atckR1?>`
    AtckR1 = 33,
    /// `<?atckR2?>`
    AtckR2 = 34,
    /// `<?atckL1?>`
    AtckL1 = 35,
    /// `<?atckL2?>`
    AtckL2 = 36,
    /// `<?defPhysicalTotal?>`
    DefPhysicalTotal = 37,
    /// `<?defPhysicalPC?>`
    DefPhysicalPC = 38,
    /// `<?defBluntTotal?>`
    DefBluntTotal = 39,
    /// `<?defCutTotal?>`
    DefCutTotal = 40,
    /// `<?defStabTotal?>`
    DefStabTotal = 41,
    /// `<?defMagicTotal?>`
    DefMagicTotal = 42,
    /// `<?defMagicPC?>`
    DefMagicPC = 43,
    /// `<?defFireTotal?>`
    DefFireTotal = 44,
    /// `<?defFirePC?>`
    DefFirePC = 45,
    /// `<?defThunderboltTotal?>`
    DefThunderboltTotal = 46,
    /// `<?defThunderboltPC?>`
    DefThunderboltPC = 47,
    /// `<?defSAToughnessTotal?>`
    DefSAToughnessTotal = 48,
    /// `<?resistBleedingTotal?>`
    ResistBleedingTotal = 49,
    /// `<?resistPoisonTotal?>`
    ResistPoisonTotal = 50,
    /// `<?resistPlagueTotal?>`
    ResistPlagueTotal = 51,
    /// `<?resistCurseTotal?>`
    ResistCurseTotal = 52,
    /// `<?magicSlotNum?>`
    MagicSlotNum = 53,
    /// `<?miracleSlotNum?>`
    MiracleSlotNum = 54,
    /// `<?souls?>`
    Souls = 55,
    /// `<?lvUpAfterSouls?>`
    LvUpAfterSouls = 56,
    /// `<?lvUpCostSouls?>`
    LvUpCostSouls = 57,
    /// `<?lvUpPcVitality?>`
    LvUpPcVitality = 58,
    /// `<?lvUpPcWill?>`
    LvUpPcWill = 59,
    /// `<?lvUpPcEndur?>`
    LvUpPcEndur = 60,
    /// `<?lvUpPcStrength?>`
    LvUpPcStrength = 61,
    /// `<?lvUpPcDexter?>`
    LvUpPcDexter = 62,
    /// `<?lvUpPcDurability?>`
    LvUpPcDurability = 63,
    /// `<?lvUpPcIntellect?>`
    LvUpPcIntellect = 64,
    /// `<?lvUpPcForce?>`
    LvUpPcForce = 65,
    /// `<?lvUpPcLuck?>`
    LvUpPcLuck = 66,
    /// `<?lvUpPcHeroic?>`
    LvUpPcHeroic = 67,
    /// `<?lvUpPcLevel?>`
    LvUpPcLevel = 68,
    /// `<?lvUpPcMaxHp?>`
    LvUpPcMaxHp = 69,
    /// `<?lvUpPcMaxMp?>`
    LvUpPcMaxMp = 70,
    /// `<?lvUpPcMaxStamina?>`
    LvUpPcMaxStamina = 71,
    /// `<?lvUpPcItemOccur?>`
    LvUpPcItemOccur = 72,
    /// `<?lvUpMaxEquipWeight?>`
    LvUpMaxEquipWeight = 73,
    /// `<?lvUpAtkR1?>`
    LvUpAtkR1 = 74,
    /// `<?lvUpAtkR2?>`
    LvUpAtkR2 = 75,
    /// `<?lvUpAtkL1?>`
    LvUpAtkL1 = 76,
    /// `<?lvUpAtkL2?>`
    LvUpAtkL2 = 77,
    /// `<?lvUpDefPhysicalTotal?>`
    LvUpDefPhysicalTotal = 78,
    /// `<?lvUpDefPhysicalPC?>`
    LvUpDefPhysicalPC = 79,
    /// `<?lvUpDefBlunt?>`
    LvUpDefBlunt = 80,
    /// `<?lvUpDefCut?>`
    LvUpDefCut = 81,
    /// `<?lvUpDefStab?>`
    LvUpDefStab = 82,
    /// `<?lvUpDefMagicTotal?>`
    LvUpDefMagicTotal = 83,
    /// `<?lvUpDefMagicPC?>`
    LvUpDefMagicPC = 84,
    /// `<?lvUpDefFireTotal?>`
    LvUpDefFireTotal = 85,
    /// `<?lvUpDefFirePC?>`
    LvUpDefFirePC = 86,
    /// `<?lvUpDefThunderboltTotal?>`
    LvUpDefThunderboltTotal = 87,
    /// `<?lvUpDefThunderboltPC?>`
    LvUpDefThunderboltPC = 88,
    /// `<?lvUpResistBleedingTotal?>`
    LvUpResistBleedingTotal = 89,
    /// `<?lvUpResistPoisonTotal?>`
    LvUpResistPoisonTotal = 90,
    /// `<?lvUpResistPlagueTotal?>`
    LvUpResistPlagueTotal = 91,
    /// `<?lvUpResistCurseTotal?>`
    LvUpResistCurseTotal = 92,
    /// `<?lvUpDefSAToughnessTotal?>`
    LvUpDefSAToughnessTotal = 93,
    /// `<?lvUpMagicSlotNum?>`
    LvUpMagicSlotNum = 94,
    /// `<?lvUpMiracleSlotNum?>`
    LvUpMiracleSlotNum = 95,
    /// `<?lvUpWpPhysicalAtkPlus?>`
    LvUpWpPhysicalAtkPlus = 96,
    /// `<?lvUpWpAttributeAtkPlus?>`
    LvUpWpAttributeAtkPlus = 97,
    /// `<?lvUpWpCriticalAtkPlus?>`
    LvUpWpCriticalAtkPlus = 98,
    /// `<?lvUpWpMagicAtkPlus?>`
    LvUpWpMagicAtkPlus = 99,
    /// `<?lvUpWpFireAtkPlus?>`
    LvUpWpFireAtkPlus = 100,
    /// `<?lvUpWpThunderboltAtkPlus?>`
    LvUpWpThunderboltAtkPlus = 101,
    /// `<?lvUpWpMagicAdjust?>`
    LvUpWpMagicAdjust = 102,
    /// `<?lvUpWpMiracleAdjust?>`
    LvUpWpMiracleAdjust = 103,
    /// `<?itemListName1?>`
    ItemListName1 = 104,
    /// `<?itemListName2?>`
    ItemListName2 = 105,
    /// `<?itemListName3?>`
    ItemListName3 = 106,
    /// `<?itemListName4?>`
    ItemListName4 = 107,
    /// `<?itemListName5?>`
    ItemListName5 = 108,
    /// `<?itemListNum1?>`
    ItemListNum1 = 109,
    /// `<?itemListNum2?>`
    ItemListNum2 = 110,
    /// `<?itemListNum3?>`
    ItemListNum3 = 111,
    /// `<?itemListNum4?>`
    ItemListNum4 = 112,
    /// `<?itemListNum5?>`
    ItemListNum5 = 113,
    /// `<?itemListLineMsg1?>`
    ItemListLineMsg1 = 114,
    /// `<?itemListLineMsg2?>`
    ItemListLineMsg2 = 115,
    /// `<?itemListLineMsg3?>`
    ItemListLineMsg3 = 116,
    /// `<?itemListLineMsg4?>`
    ItemListLineMsg4 = 117,
    /// `<?itemListLineMsg5?>`
    ItemListLineMsg5 = 118,
    /// `<?itemName?>`
    ItemName = 119,
    /// `<?itemLineMsg?>`
    ItemLineMsg = 120,
    /// `<?itemDetail?>`
    ItemDetail = 121,
    /// `<?itemNum?>`
    ItemNum = 122,
    /// `<?buySouls?>`
    BuySouls = 123,
    /// `<?sellSouls?>`
    SellSouls = 124,
    /// `<?duration?>`
    Duration = 125,
    /// `<?maxDuration?>`
    MaxDuration = 126,
    /// `<?weight?>`
    Weight = 127,
    /// `<?itemNameBeforeForge?>`
    ItemNameBeforeForge = 128,
    /// `<?itemNameAfterForge?>`
    ItemNameAfterForge = 129,
    /// `<?wpAtkType?>`
    WpAtkType = 130,
    /// `<?wpType?>`
    WpType = 131,
    /// `<?wpPhysicalAtk?>`
    WpPhysicalAtk = 132,
    /// `<?wpAttributeAtk?>`
    WpAttributeAtk = 133,
    /// `<?wpCriticalAtk?>`
    WpCriticalAtk = 134,
    /// `<?wpPhysicalAtkPlus?>`
    WpPhysicalAtkPlus = 135,
    /// `<?wpAttributeAtkPlus?>`
    WpAttributeAtkPlus = 136,
    /// `<?wpCriticalAtkPlus?>`
    WpCriticalAtkPlus = 137,
    /// `<?wpShotRange?>`
    WpShotRange = 138,
    /// `<?wpMagicAdjust?>`
    WpMagicAdjust = 139,
    /// `<?wpMiracleAdjust?>`
    WpMiracleAdjust = 140,
    /// `<?wpCutPhysical?>`
    WpCutPhysical = 141,
    /// `<?wpCutMagic?>`
    WpCutMagic = 142,
    /// `<?wpCutFire?>`
    WpCutFire = 143,
    /// `<?wpCutThunderbolt?>`
    WpCutThunderbolt = 144,
    /// `<?wpHitRes?>`
    WpHitRes = 145,
    /// `<?atkPlusPcStrength?>`
    AtkPlusPcStrength = 146,
    /// `<?atkPlusPcDexter?>`
    AtkPlusPcDexter = 147,
    /// `<?atkPlusPcIntellect?>`
    AtkPlusPcIntellect = 148,
    /// `<?atkPlusPcForce?>`
    AtkPlusPcForce = 149,
    /// `<?reqPcStrength?>`
    ReqPcStrength = 150,
    /// `<?reqPcDexter?>`
    ReqPcDexter = 151,
    /// `<?reqPcIntellect?>`
    ReqPcIntellect = 152,
    /// `<?reqPcForce?>`
    ReqPcForce = 153,
    /// `<?effectBleeding?>`
    EffectBleeding = 154,
    /// `<?effectPoison?>`
    EffectPoison = 155,
    /// `<?effectPlague?>`
    EffectPlague = 156,
    /// `<?effectCurse?>`
    EffectCurse = 157,
    /// `<?wpMagicAtk?>`
    WpMagicAtk = 158,
    /// `<?wpFireAtk?>`
    WpFireAtk = 159,
    /// `<?wpThunderboltAtk?>`
    WpThunderboltAtk = 160,
    /// `<?wpMagicAtkPlus?>`
    WpMagicAtkPlus = 161,
    /// `<?wpFireAtkPlus?>`
    WpFireAtkPlus = 162,
    /// `<?wpThunderboltAtkPlus?>`
    WpThunderboltAtkPlus = 163,
    /// `<?WpCriticalAtkPlus?>`
    ///
    /// A separate tag from [`WpCriticalAtkPlus`], which the game spells with a
    /// lowercase leading `w`. Matching is case-sensitive, so the two resolve
    /// independently.
    ///
    /// [`WpCriticalAtkPlus`]: Self::WpCriticalAtkPlus
    WpCriticalAtkPlusCapitalized = 164,
    /// `<?forgedWpPhysicalAtk?>`
    ForgedWpPhysicalAtk = 165,
    /// `<?forgedWpAttributeAtk?>`
    ForgedWpAttributeAtk = 166,
    /// `<?forgedWpCriticalAtk?>`
    ForgedWpCriticalAtk = 167,
    /// `<?forgedWpPhysicalAtkPlus?>`
    ForgedWpPhysicalAtkPlus = 168,
    /// `<?forgedWpAttributeAtkPlus?>`
    ForgedWpAttributeAtkPlus = 169,
    /// `<?forgedWpCriticalAtkPlus?>`
    ForgedWpCriticalAtkPlus = 170,
    /// `<?forgedWpShotRange?>`
    ForgedWpShotRange = 171,
    /// `<?forgedWpMagicAdjust?>`
    ForgedWpMagicAdjust = 172,
    /// `<?forgedWpCutPhysical?>`
    ForgedWpCutPhysical = 173,
    /// `<?forgedWpCutMagic?>`
    ForgedWpCutMagic = 174,
    /// `<?forgedWpCutFire?>`
    ForgedWpCutFire = 175,
    /// `<?forgedWpCutThunderbolt?>`
    ForgedWpCutThunderbolt = 176,
    /// `<?forgedWpHitRes?>`
    ForgedWpHitRes = 177,
    /// `<?forgedAtkPlusPcStrength?>`
    ForgedAtkPlusPcStrength = 178,
    /// `<?forgedAtkPlusPcDexter?>`
    ForgedAtkPlusPcDexter = 179,
    /// `<?forgedAtkPlusPcIntellect?>`
    ForgedAtkPlusPcIntellect = 180,
    /// `<?forgedAtkPlusPcForce?>`
    ForgedAtkPlusPcForce = 181,
    /// `<?forgedReqPcStrength?>`
    ForgedReqPcStrength = 182,
    /// `<?forgedReqPcDexter?>`
    ForgedReqPcDexter = 183,
    /// `<?forgedReqPcIntellect?>`
    ForgedReqPcIntellect = 184,
    /// `<?forgedReqPcForce?>`
    ForgedReqPcForce = 185,
    /// `<?forgedEffectBleeding?>`
    ForgedEffectBleeding = 186,
    /// `<?forgedEffectPoison?>`
    ForgedEffectPoison = 187,
    /// `<?forgedEffectPlague?>`
    ForgedEffectPlague = 188,
    /// `<?forgedEffectCurse?>`
    ForgedEffectCurse = 189,
    /// `<?reqGender?>`
    ReqGender = 190,
    /// `<?defPhysical?>`
    DefPhysical = 191,
    /// `<?defBlunt?>`
    DefBlunt = 192,
    /// `<?defCut?>`
    DefCut = 193,
    /// `<?defStab?>`
    DefStab = 194,
    /// `<?defMagic?>`
    DefMagic = 195,
    /// `<?defFire?>`
    DefFire = 196,
    /// `<?defThunderbolt?>`
    DefThunderbolt = 197,
    /// `<?resistBleeding?>`
    ResistBleeding = 198,
    /// `<?resistPoison?>`
    ResistPoison = 199,
    /// `<?resistPlague?>`
    ResistPlague = 200,
    /// `<?resistCurse?>`
    ResistCurse = 201,
    /// `<?forgedDefPhysical?>`
    ForgedDefPhysical = 202,
    /// `<?forgedDefBlunt?>`
    ForgedDefBlunt = 203,
    /// `<?forgedDefCut?>`
    ForgedDefCut = 204,
    /// `<?forgedDefStab?>`
    ForgedDefStab = 205,
    /// `<?forgedDefMagic?>`
    ForgedDefMagic = 206,
    /// `<?forgedDefFire?>`
    ForgedDefFire = 207,
    /// `<?forgedDefThunderbolt?>`
    ForgedDefThunderbolt = 208,
    /// `<?forgedResistBleeding?>`
    ForgedResistBleeding = 209,
    /// `<?forgedResistPoison?>`
    ForgedResistPoison = 210,
    /// `<?forgedResistPlague?>`
    ForgedResistPlague = 211,
    /// `<?forgedResistCurse?>`
    ForgedResistCurse = 212,
    /// `<?reqSlot?>`
    ReqSlot = 213,
    /// `<?costMP?>`
    CostMP = 214,
    /// `<?equipMagicInfo1?>`
    EquipMagicInfo1 = 215,
    /// `<?equipMagicInfo2?>`
    EquipMagicInfo2 = 216,
    /// `<?equipMagicInfo3?>`
    EquipMagicInfo3 = 217,
    /// `<?equipMagicInfo4?>`
    EquipMagicInfo4 = 218,
    /// `<?equipMagicInfo5?>`
    EquipMagicInfo5 = 219,
    /// `<?equipMagicInfo6?>`
    EquipMagicInfo6 = 220,
    /// `<?equipMagicInfo7?>`
    EquipMagicInfo7 = 221,
    /// `<?equipMagicInfo8?>`
    EquipMagicInfo8 = 222,
    /// `<?equipMagicInfo9?>`
    EquipMagicInfo9 = 223,
    /// `<?equipMagicInfo10?>`
    EquipMagicInfo10 = 224,
    /// `<?equipMagicInfo11?>`
    EquipMagicInfo11 = 225,
    /// `<?equipMagicInfo12?>`
    EquipMagicInfo12 = 226,
    /// `<?magicCount1?>`
    MagicCount1 = 227,
    /// `<?magicCount2?>`
    MagicCount2 = 228,
    /// `<?magicCount3?>`
    MagicCount3 = 229,
    /// `<?magicCount4?>`
    MagicCount4 = 230,
    /// `<?magicCount5?>`
    MagicCount5 = 231,
    /// `<?magicCount6?>`
    MagicCount6 = 232,
    /// `<?magicCount7?>`
    MagicCount7 = 233,
    /// `<?magicCount8?>`
    MagicCount8 = 234,
    /// `<?magicCount9?>`
    MagicCount9 = 235,
    /// `<?magicCount10?>`
    MagicCount10 = 236,
    /// `<?magicCount11?>`
    MagicCount11 = 237,
    /// `<?magicCount12?>`
    MagicCount12 = 238,
    /// `<?magicMaxCount1?>`
    MagicMaxCount1 = 239,
    /// `<?magicMaxCount2?>`
    MagicMaxCount2 = 240,
    /// `<?magicMaxCount3?>`
    MagicMaxCount3 = 241,
    /// `<?magicMaxCount4?>`
    MagicMaxCount4 = 242,
    /// `<?magicMaxCount5?>`
    MagicMaxCount5 = 243,
    /// `<?magicMaxCount6?>`
    MagicMaxCount6 = 244,
    /// `<?magicMaxCount7?>`
    MagicMaxCount7 = 245,
    /// `<?magicMaxCount8?>`
    MagicMaxCount8 = 246,
    /// `<?magicMaxCount9?>`
    MagicMaxCount9 = 247,
    /// `<?magicMaxCount10?>`
    MagicMaxCount10 = 248,
    /// `<?magicMaxCount11?>`
    MagicMaxCount11 = 249,
    /// `<?magicMaxCount12?>`
    MagicMaxCount12 = 250,
    /// `<?equipmentPart?>`
    EquipmentPart = 251,
    /// `<?equipItemName?>`
    EquipItemName = 252,
    /// `<?ringEffect?>`
    RingEffect = 253,
    /// `<?equipItemNum1?>`
    EquipItemNum1 = 254,
    /// `<?equipItemNum2?>`
    EquipItemNum2 = 255,
    /// `<?equipItemNum3?>`
    EquipItemNum3 = 256,
    /// `<?equipItemNum4?>`
    EquipItemNum4 = 257,
    /// `<?equipItemNum5?>`
    EquipItemNum5 = 258,
    /// `<?equipArrowNum1?>`
    EquipArrowNum1 = 259,
    /// `<?equipArrowNum2?>`
    EquipArrowNum2 = 260,
    /// `<?equipBoltNum1?>`
    EquipBoltNum1 = 261,
    /// `<?equipBoltNum2?>`
    EquipBoltNum2 = 262,
    /// `<?removeMagic?>`
    RemoveMagic = 263,
    /// `<?getItemName1?>`
    GetItemName1 = 264,
    /// `<?getItemName2?>`
    GetItemName2 = 265,
    /// `<?getItemName3?>`
    GetItemName3 = 266,
    /// `<?getItemName4?>`
    GetItemName4 = 267,
    /// `<?getItemName5?>`
    GetItemName5 = 268,
    /// `<?getItemNum1?>`
    GetItemNum1 = 269,
    /// `<?getItemNum2?>`
    GetItemNum2 = 270,
    /// `<?getItemNum3?>`
    GetItemNum3 = 271,
    /// `<?getItemNum4?>`
    GetItemNum4 = 272,
    /// `<?getItemNum5?>`
    GetItemNum5 = 273,
    /// `<?mapName?>`
    MapName = 274,
    /// `<?warpName1?>`
    WarpName1 = 275,
    /// `<?warpName2?>`
    WarpName2 = 276,
    /// `<?warpName3?>`
    WarpName3 = 277,
    /// `<?warpName4?>`
    WarpName4 = 278,
    /// `<?warpName5?>`
    WarpName5 = 279,
    /// `<?warpName6?>`
    WarpName6 = 280,
    /// `<?warpName7?>`
    WarpName7 = 281,
    /// `<?warpName8?>`
    WarpName8 = 282,
    /// `<?warpName9?>`
    WarpName9 = 283,
    /// `<?warpName10?>`
    WarpName10 = 284,
    /// `<?warpName11?>`
    WarpName11 = 285,
    /// `<?warpName12?>`
    WarpName12 = 286,
    /// `<?warpName13?>`
    WarpName13 = 287,
    /// `<?pcenvInitValue01?>`
    PcenvInitValue01 = 288,
    /// `<?pcenvValue01?>`
    PcenvValue01 = 289,
    /// `<?pcenvValue02?>`
    PcenvValue02 = 290,
    /// `<?pcenvValue03?>`
    PcenvValue03 = 291,
    /// `<?pcenvValue04?>`
    PcenvValue04 = 292,
    /// `<?pcenvValue05?>`
    PcenvValue05 = 293,
    /// `<?pcenvTimer1?>`
    PcenvTimer1 = 294,
    /// `<?keyCategory?>`
    KeyCategory = 295,
    /// `<?key1?>`
    Key1 = 296,
    /// `<?key2?>`
    Key2 = 297,
    /// `<?key3?>`
    Key3 = 298,
    /// `<?key4?>`
    Key4 = 299,
    /// `<?key5?>`
    Key5 = 300,
    /// `<?key6?>`
    Key6 = 301,
    /// `<?key7?>`
    Key7 = 302,
    /// `<?key8?>`
    Key8 = 303,
    /// `<?key9?>`
    Key9 = 304,
    /// `<?key10?>`
    Key10 = 305,
    /// `<?keyAction1?>`
    KeyAction1 = 306,
    /// `<?keyAction2?>`
    KeyAction2 = 307,
    /// `<?keyAction3?>`
    KeyAction3 = 308,
    /// `<?keyAction4?>`
    KeyAction4 = 309,
    /// `<?keyAction5?>`
    KeyAction5 = 310,
    /// `<?keyAction6?>`
    KeyAction6 = 311,
    /// `<?keyAction7?>`
    KeyAction7 = 312,
    /// `<?keyAction8?>`
    KeyAction8 = 313,
    /// `<?keyAction9?>`
    KeyAction9 = 314,
    /// `<?keyAction10?>`
    KeyAction10 = 315,
    /// `<?hostName?>`
    HostName = 316,
    /// `<?joinName?>`
    JoinName = 317,
    /// `<?leaveName?>`
    LeaveName = 318,
    /// `<?deadName?>`
    DeadName = 319,
    /// `<?enemyPC1?>`
    EnemyPC1 = 320,
    /// `<?enemyPC2?>`
    EnemyPC2 = 321,
    /// `<?enemyPC3?>`
    EnemyPC3 = 322,
    /// `<?remainSec?>`
    RemainSec = 323,
    /// `<?registerItemNum?>`
    RegisterItemNum = 324,
    /// `<?coliseumDeadNameA?>`
    ColiseumDeadNameA = 325,
    /// `<?coliseumDeadNameB?>`
    ColiseumDeadNameB = 326,
    /// `<?coliseumDeadNameC?>`
    ColiseumDeadNameC = 327,
    /// `<?coliseumDeadNameD?>`
    ColiseumDeadNameD = 328,
    /// `<?codenamePCName?>`
    CodenamePCName = 329,
    /// `<?codenameIcon?>`
    CodenameIcon = 330,
    /// `<?matchDuel_SecondHalfTime?>`
    MatchDuelSecondHalfTime = 331,
    /// `<?matchDuel_TotalTime?>`
    MatchDuelTotalTime = 332,
    /// `<?matchDM2_SecondHalfTime?>`
    MatchDM2SecondHalfTime = 333,
    /// `<?matchDM2_TotalTime?>`
    MatchDM2TotalTime = 334,
    /// `<?matchDM4_SecondHalfTime?>`
    MatchDM4SecondHalfTime = 335,
    /// `<?matchDM4_TotalTime?>`
    MatchDM4TotalTime = 336,
    /// `<?matchDM6_SecondHalfTime?>`
    MatchDM6SecondHalfTime = 337,
    /// `<?matchDM6_TotalTime?>`
    MatchDM6TotalTime = 338,
    /// `<?matchTM1vs1_SecondHalfTime?>`
    MatchTM1vs1SecondHalfTime = 339,
    /// `<?matchTM1vs1_TotalTime?>`
    MatchTM1vs1TotalTime = 340,
    /// `<?matchTM2vs2_SecondHalfTime?>`
    MatchTM2vs2SecondHalfTime = 341,
    /// `<?matchTM2vs2_TotalTime?>`
    MatchTM2vs2TotalTime = 342,
    /// `<?matchTM3vs3_SecondHalfTime?>`
    MatchTM3vs3SecondHalfTime = 343,
    /// `<?matchTM3vs3_TotalTime?>`
    MatchTM3vs3TotalTime = 344,
    /// `<?rank1?>`
    Rank1 = 345,
    /// `<?rank2?>`
    Rank2 = 346,
    /// `<?rank3?>`
    Rank3 = 347,
    /// `<?rank4?>`
    Rank4 = 348,
    /// `<?rank5?>`
    Rank5 = 349,
    /// `<?rank6?>`
    Rank6 = 350,
    /// `<?rank7?>`
    Rank7 = 351,
    /// `<?rank8?>`
    Rank8 = 352,
    /// `<?rank9?>`
    Rank9 = 353,
    /// `<?rank10?>`
    Rank10 = 354,
    /// `<?rankPlayerName1?>`
    RankPlayerName1 = 355,
    /// `<?rankPlayerName2?>`
    RankPlayerName2 = 356,
    /// `<?rankPlayerName3?>`
    RankPlayerName3 = 357,
    /// `<?rankPlayerName4?>`
    RankPlayerName4 = 358,
    /// `<?rankPlayerName5?>`
    RankPlayerName5 = 359,
    /// `<?rankPlayerName6?>`
    RankPlayerName6 = 360,
    /// `<?rankPlayerName7?>`
    RankPlayerName7 = 361,
    /// `<?rankPlayerName8?>`
    RankPlayerName8 = 362,
    /// `<?rankPlayerName9?>`
    RankPlayerName9 = 363,
    /// `<?rankPlayerName10?>`
    RankPlayerName10 = 364,
    /// `<?rankScore1?>`
    RankScore1 = 365,
    /// `<?rankScore2?>`
    RankScore2 = 366,
    /// `<?rankScore3?>`
    RankScore3 = 367,
    /// `<?rankScore4?>`
    RankScore4 = 368,
    /// `<?rankScore5?>`
    RankScore5 = 369,
    /// `<?rankScore6?>`
    RankScore6 = 370,
    /// `<?rankScore7?>`
    RankScore7 = 371,
    /// `<?rankScore8?>`
    RankScore8 = 372,
    /// `<?rankScore9?>`
    RankScore9 = 373,
    /// `<?rankScore10?>`
    RankScore10 = 374,
    /// `<?rankCategory?>`
    RankCategory = 375,
    /// `<?pcPlayTime?>`
    PcPlayTime = 376,
    /// `<?ptTime?>`
    PtTime = 377,
    /// `<?ptMinute?>`
    PtMinute = 378,
    /// `<?ptSecond?>`
    PtSecond = 379,
    /// `<?dataSize?>`
    DataSize = 380,
    /// `<?menuTitle?>`
    MenuTitle = 381,
    /// `<?pmOptionsText?>`
    PmOptionsText = 382,
    /// `<?pmOptions1?>`
    PmOptions1 = 383,
    /// `<?pmOptions2?>`
    PmOptions2 = 384,
    /// `<?pmOptions3?>`
    PmOptions3 = 385,
    /// `<?pmOptions4?>`
    PmOptions4 = 386,
    /// `<?pmOptions5?>`
    PmOptions5 = 387,
    /// `<?pmOptions6?>`
    PmOptions6 = 388,
    /// `<?pmOptions7?>`
    PmOptions7 = 389,
    /// `<?pmOptions8?>`
    PmOptions8 = 390,
    /// `<?pmOptions9?>`
    PmOptions9 = 391,
    /// `<?pmOptions10?>`
    PmOptions10 = 392,
    /// `<?pmOptions11?>`
    PmOptions11 = 393,
    /// `<?pmOptions12?>`
    PmOptions12 = 394,
    /// `<?pmOptions13?>`
    PmOptions13 = 395,
    /// `<?pmOptions14?>`
    PmOptions14 = 396,
    /// `<?pmOptions15?>`
    PmOptions15 = 397,
    /// `<?pmOptions16?>`
    PmOptions16 = 398,
    /// `<?pmOptions17?>`
    PmOptions17 = 399,
    /// `<?pmOptions18?>`
    PmOptions18 = 400,
    /// `<?pmOptions19?>`
    PmOptions19 = 401,
    /// `<?pmOptions20?>`
    PmOptions20 = 402,
    /// `<?goodsNameId?>`
    GoodsNameId = 403,
    /// `<?weaponNameId?>`
    WeaponNameId = 404,
    /// `<?magicNameId?>`
    MagicNameId = 405,
    /// `<?protectorNameId?>`
    ProtectorNameId = 406,
    /// `<?ringNameId?>`
    RingNameId = 407,
    /// `<?originId?>`
    OriginId = 408,
    /// `<?npcNameId?>`
    NpcNameId = 409,
    /// `<?mapNameId?>`
    MapNameId = 410,
    /// `<?systemMsgId?>`
    SystemMsgId = 411,
    /// `<?platformMsgId?>`
    PlatformMsgId = 412,
    /// `<?murdererName?>`
    MurdererName = 413,
    /// `<?lineHelp?>`
    LineHelp = 414,
    /// `<?keyGuide?>`
    KeyGuide = 415,
    /// `<?loadHintName?>`
    LoadHintName = 416,
    /// `<?loadHintCaption?>`
    LoadHintCaption = 417,
    /// `<?selectU?>`
    SelectU = 418,
    /// `<?selectD?>`
    SelectD = 419,
    /// `<?selectL?>`
    SelectL = 420,
    /// `<?selectR?>`
    SelectR = 421,
    /// `<?selectUD?>`
    SelectUD = 422,
    /// `<?selectLR?>`
    SelectLR = 423,
    /// `<?selectAll?>`
    SelectAll = 424,
    /// `<?pageUD?>`
    PageUD = 425,
    /// `<?pageUp?>`
    PageUp = 426,
    /// `<?pageDown?>`
    PageDown = 427,
    /// `<?conclusion?>`
    Conclusion = 428,
    /// `<?cancel?>`
    Cancel = 429,
    /// `<?viewChange?>`
    ViewChange = 430,
    /// `<?commando?>`
    Commando = 431,
    /// `<?shortCutLR?>`
    ShortCutLR = 432,
    /// `<?shortCutL?>`
    ShortCutL = 433,
    /// `<?shortCutR?>`
    ShortCutR = 434,
    /// `<?categoryChangeLR?>`
    CategoryChangeLR = 435,
    /// `<?categoryChangeL?>`
    CategoryChangeL = 436,
    /// `<?categoryChangeR?>`
    CategoryChangeR = 437,
    /// `<?startMenuSwitch?>`
    StartMenuSwitch = 438,
    /// `<?slectMenuSwitch?>`
    SlectMenuSwitch = 439,
    /// `<?spin?>`
    Spin = 440,
    /// `<?move?>`
    Move = 441,
    /// `<?spinReset?>`
    SpinReset = 442,
    /// `<?moneReset?>`
    MoneReset = 443,
    /// `<?actionHelp?>`
    ActionHelp = 444,
    /// `<?run?>`
    Run = 445,
    /// `<?dush?>`
    Dush = 446,
    /// `<?step?>`
    Step = 447,
    /// `<?select?>`
    Select = 448,
    /// `<?attackL?>`
    AttackL = 449,
    /// `<?attackR?>`
    AttackR = 450,
    /// `<?attackBoth?>`
    AttackBoth = 451,
    /// `<?defenceBoth?>`
    DefenceBoth = 452,
    /// `<?changeEquStyle?>`
    ChangeEquStyle = 453,
    /// `<?changeArmL?>`
    ChangeArmL = 454,
    /// `<?changeArmR?>`
    ChangeArmR = 455,
    /// `<?startMagic?>`
    StartMagic = 456,
    /// `<?changeMagic?>`
    ChangeMagic = 457,
    /// `<?startAction?>`
    StartAction = 458,
    /// `<?prepBow?>`
    PrepBow = 459,
    /// `<?attachBow?>`
    AttachBow = 460,
    /// `<?conceBow?>`
    ConceBow = 461,
    /// `<?zoomIn?>`
    ZoomIn = 462,
    /// `<?zoomOut?>`
    ZoomOut = 463,
    /// `<?releaseConceBow?>`
    ReleaseConceBow = 464,
    /// `<?startItem?>`
    StartItem = 465,
    /// `<?chamgeItem?>`
    ChamgeItem = 466,
    /// `<?gdsparam?>`
    Gdsparam = 467,
    /// `<?wepparam?>`
    Wepparam = 468,
    /// `<?mgcparam?>`
    Mgcparam = 469,
    /// `<?prtparam?>`
    Prtparam = 470,
    /// `<?acsparam?>`
    Acsparam = 471,
    /// `<?areaName?>`
    AreaName = 472,
    /// `<?blockName?>`
    BlockName = 473,
    /// `<?archetype?>`
    Archetype = 474,
    /// `<?npcName?>`
    NpcName = 475,
    /// `<?placeName?>`
    PlaceName = 476,
    /// `<?sysmsg?>`
    Sysmsg = 477,
    /// `<?platmsg?>`
    Platmsg = 478,
    /// `<?demandSoul?>`
    DemandSoul = 479,
    /// `<?title?>`
    Title = 480,
    /// `<?selectGesture?>`
    SelectGesture = 481,
    /// `<?inventoryNum0?>`
    InventoryNum0 = 482,
    /// `<?inventoryNum1?>`
    InventoryNum1 = 483,
    /// `<?inventoryNum2?>`
    InventoryNum2 = 484,
    /// `<?inventoryNum3?>`
    InventoryNum3 = 485,
    /// `<?repositoryNum0?>`
    RepositoryNum0 = 486,
    /// `<?repositoryNum1?>`
    RepositoryNum1 = 487,
    /// `<?repositoryNum2?>`
    RepositoryNum2 = 488,
    /// `<?repositoryNum3?>`
    RepositoryNum3 = 489,
    /// `<?optCameraLR?>`
    OptCameraLR = 490,
    /// `<?optCameraUD?>`
    OptCameraUD = 491,
    /// `<?optCameraSpeed?>`
    OptCameraSpeed = 492,
    /// `<?optVibration?>`
    OptVibration = 493,
    /// `<?optLockOnAuto?>`
    OptLockOnAuto = 494,
    /// `<?optCamAvoidWall?>`
    OptCamAvoidWall = 495,
    /// `<?optShowBlood?>`
    OptShowBlood = 496,
    /// `<?optSubtitles?>`
    OptSubtitles = 497,
    /// `<?optHUD?>`
    OptHUD = 498,
    /// `<?optBGMVol?>`
    OptBGMVol = 499,
    /// `<?optSEVol?>`
    OptSEVol = 500,
    /// `<?optVoiceVol?>`
    OptVoiceVol = 501,
    /// `<?optRegisterRanking?>`
    OptRegisterRanking = 502,
    /// `<?optBrightness?>`
    OptBrightness = 503,
    /// `<?evntAcquittalPrice?>`
    EvntAcquittalPrice = 504,
    /// `<?em2?>`
    Em2 = 505,
    /// `<?npl?>`
    Npl = 506,
    /// `<?nam?>`
    Nam = 507,
    /// `<?job?>`
    Job = 508,
    /// `<?etc?>`
    Etc = 509,
    /// `<?trp?>`
    Trp = 510,
    /// `<?dir?>`
    Dir = 511,
    /// `<?som?>`
    Som = 512,
    /// `<?cls?>`
    Cls = 513,
    /// `<?rat?>`
    Rat = 514,
    /// `<?got?>`
    Got = 515,
    /// `<?fel?>`
    Fel = 516,
    /// `<?hlp?>`
    Hlp = 517,
    /// `<?ssInfoMsg?>`
    SsInfoMsg = 518,
    /// `<?errcodeEOS?>`
    ErrcodeEOS = 519,
    /// `<?errcodeSteam?>`
    ErrcodeSteam = 520,
    /// `<?loopCount?>`
    LoopCount = 521,
    /// `<?nextLoopCount?>`
    NextLoopCount = 522,
    /// `<?kgicon?>`
    Kgicon = 523,
    /// `<?keyActName?>`
    KeyActName = 524,
    /// `<?keyActFullName?>`
    KeyActFullName = 525,
    /// `<?keyMove?>`
    KeyMove = 526,
    /// `<?keyControlCamera?>`
    KeyControlCamera = 527,
    /// `<?image?>`
    Image = 528,
    /// `<?keyicon?>`
    Keyicon = 529,
    /// `<?kgOk?>`
    KgOk = 530,
    /// `<?kgCancel?>`
    KgCancel = 531,
    /// `<?kgU?>`
    KgU = 532,
    /// `<?kgD?>`
    KgD = 533,
    /// `<?kgL?>`
    KgL = 534,
    /// `<?kgR?>`
    KgR = 535,
    /// `<?kgUD?>`
    KgUD = 536,
    /// `<?kgLR?>`
    KgLR = 537,
    /// `<?kgUDLR?>`
    KgUDLR = 538,
    /// `<?kgRU?>`
    KgRU = 539,
    /// `<?kgRD?>`
    KgRD = 540,
    /// `<?kgRL?>`
    KgRL = 541,
    /// `<?kgRR?>`
    KgRR = 542,
    /// `<?kgL1?>`
    KgL1 = 543,
    /// `<?kgR1?>`
    KgR1 = 544,
    /// `<?kgL1R1?>`
    KgL1R1 = 545,
    /// `<?kgL2?>`
    KgL2 = 546,
    /// `<?kgR2?>`
    KgR2 = 547,
    /// `<?kgL2R2?>`
    KgL2R2 = 548,
    /// `<?kgL3?>`
    KgL3 = 549,
    /// `<?kgR3?>`
    KgR3 = 550,
    /// `<?kgLStick?>`
    KgLStick = 551,
    /// `<?kgRStick?>`
    KgRStick = 552,
    /// `<?kgTouchPadL?>`
    KgTouchPadL = 553,
    /// `<?kgTouchPadR?>`
    KgTouchPadR = 554,
    /// `<?kgStart?>`
    KgStart = 555,
    /// `<?kgBack?>`
    KgBack = 556,
    /// `<?newItemIcon?>`
    NewItemIcon = 557,
    /// `<?dlcPlayerDopingLevel?>`
    DlcPlayerDopingLevel = 558,
    /// `<?dlcBuddyDopingLevel?>`
    DlcBuddyDopingLevel = 559,
}

const _: () = {
    assert!(std::mem::size_of::<MsgTag>() == 0x28);
    assert!(std::mem::size_of::<MsgTagManImp>() == 0x67d8);
    assert!(std::mem::offset_of!(MsgTagManImp, tags) == 8);

    assert!(std::mem::offset_of!(MsgTag, name) == 0x00);
    assert!(std::mem::offset_of!(MsgTag, value) == 0x08);
    assert!(std::mem::offset_of!(MsgTag, capacity) == 0x10);
    assert!(std::mem::offset_of!(MsgTag, numeric_value) == 0x14);
    assert!(std::mem::offset_of!(MsgTag, name_hash) == 0x18);
    assert!(std::mem::offset_of!(MsgTag, flags) == 0x1c);
    assert!(std::mem::offset_of!(MsgTag, callback) == 0x20);

    assert!(std::mem::size_of::<MenuShortTag>() == 0x10);
    assert!(std::mem::offset_of!(MenuShortTag, name) == 0x00);
    assert!(std::mem::offset_of!(MenuShortTag, name_hash) == 0x08);
    assert!(std::mem::offset_of!(MenuShortTag, base_msg_id) == 0x0c);

    assert!(std::mem::offset_of!(MsgTagManImp, short_tags) == 0x58b8);

    assert!(MsgTagType::DlcBuddyDopingLevel as usize == MSG_TAG_COUNT - 1);
};
