use std::{
    ops::{Index, IndexMut},
    ptr::NonNull,
    sync::atomic::AtomicU32,
};

use bitfield::bitfield;
use vtable_rs::VPtr;

use crate::{
    DLMap,
    dlkr::MainHeapAllocator,
    dltx::DLString,
    dlut::{DLReferenceCountObject, DLReferenceCountObjectVmt, DLReferencePointer},
    fd4::FD4Time,
};
use shared::OwnedPtr;

use super::CSEzDraw;

#[vtable_rs::vtable]
pub trait CSFD4FadeSystemVmt {
    fn destructor(&mut self, delete_self: u32);
    fn update(&mut self, delta_time: &FD4Time);

    /// Debug-draws a swatch of [`CSFD4FadeSystem::blended_color`].
    fn debug_draw_blended_color(&self, draw: &mut CSEzDraw, x: i32, y: i32);

    fn set_fade_plate_for_slot(
        &mut self,
        slot: FadePlateId,
        plate: Option<NonNull<CSFD4FadePlate>>,
    );
    fn remove_fade_plate_slot(&mut self, slot: FadePlateId);

    /// Releases every plate in the fade-plate map and resets it to empty.
    fn clear_fade_plate_map(&mut self);
}

#[repr(C)]
/// Controls fades in the game. Used for cutscene transitions and such.
///
/// Source of name: RTTI
#[shared::singleton("CSFade")]
pub struct CSFade {
    vftable: usize,
    pub fade_system: OwnedPtr<CSFD4FadeSystem, MainHeapAllocator>,
    /// The 9 fade plates, indexed by [`FadePlateId`]. Co-owned with
    /// [`CSFD4FadeSystem::fade_plate_map`] via [`DLReferencePointer`], so a plate can be removed
    /// from the active render set while staying alive and addressable here.
    pub fade_plates: [DLReferencePointer<CSFD4FadePlate>; 9],
    /// Global
    pub fade_system_active: bool,
    /// Multiplier applied to the `FadePlateId::InGameBokeh`] plate's alpha
    /// before rendering.
    pub bokeh_blend_multiplier: f32,
}

impl CSFade {
    /// The color actually shown on screen right now.
    pub fn get_combined_fade_color(&self) -> CSFD4FadePlateColor {
        if self.fade_system_active {
            self.fade_system.blended_color
        } else {
            CSFD4FadePlateColor {
                r: 0.9375,
                g: 0.9727,
                b: 1.0,
                a: 1.0,
            }
        }
    }
}

#[repr(C)]
/// Source of name: RTTI
pub struct CSFD4FadeSystem {
    vftable: VPtr<dyn CSFD4FadeSystemVmt, Self>,
    /// The active set of plates composited into [`Self::blended_color`] each frame, keyed by
    /// [`FadePlateId`].
    pub fade_plate_map: DLMap<FadePlateId, DLReferencePointer<CSFD4FadePlate>>,
    /// The final on-screen tint, alpha-composited across every plate in [`Self::fade_plate_map`].
    pub blended_color: CSFD4FadePlateColor,
}

#[repr(C)]
/// A fade plate
///
/// Source of name: RTTI
pub struct CSFD4FadePlate {
    vftable: VPtr<dyn DLReferenceCountObjectVmt, Self>,
    reference_count: AtomicU32,
    /// The color actually drawn this frame.
    /// Interpolated from [`Self::transition_color`] to
    /// [`Self::target_color`] over [`Self::fade_duration`].
    pub current_color: CSFD4FadePlateColor,
    /// The color the last transition started from.
    pub transition_color: CSFD4FadePlateColor,
    /// The color the current transition is interpolating towards.
    pub target_color: CSFD4FadePlateColor,
    /// Seconds remaining until [`Self::current_color`] reaches [`Self::target_color`].
    pub fade_timer: FD4Time,
    /// Total duration the current transition was started with.
    pub fade_duration: FD4Time,
    /// Easing curve applied to interpolation progress between [`Self::transition_color`] and
    /// [`Self::target_color`].
    pub easing: FadePlateEasing,
    pub force_flags: FadePlateForceFlags,
    /// Debug string for this fade plate.
    pub title: DLString,
    unk98: CSFD4FadePlateColor,
    unka8: FD4Time,
    unkb8: u8,
}

impl CSFD4FadePlate {
    /// Sets [`Self::current_color`]/[`Self::transition_color`]/[`Self::target_color`] to `color`
    /// immediately, with no transition.
    pub fn set_color_instant(&mut self, color: CSFD4FadePlateColor) {
        self.current_color = color;
        self.transition_color = color;
        self.target_color = color;
        self.fade_timer.time = 0.0;
        self.fade_duration.time = 0.0;
    }

    /// The color composited to screen this frame.
    pub fn effective_color(&self) -> CSFD4FadePlateColor {
        if self.force_flags.force_cleared() || self.force_flags.debug_force_cleared() {
            CSFD4FadePlateColor {
                r: 0.9375,
                g: 0.9727,
                b: 1.0,
                a: 1.0,
            }
        } else {
            self.current_color
        }
    }
}

impl DLReferenceCountObject for CSFD4FadePlate {
    fn vtable(&self) -> VPtr<dyn DLReferenceCountObjectVmt, Self> {
        self.vftable
    }

    fn reference_count(&self) -> &AtomicU32 {
        &self.reference_count
    }
}

bitfield! {
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FadePlateForceFlags(u32);
    impl Debug;

    /// Pauses [`CSFD4FadePlate::current_color`] interpolation while set.
    pub interpolation_paused, set_interpolation_paused: 0;
    /// Forces [`CSFD4FadePlate::effective_color`] to the cleared preset while set.
    pub force_cleared, set_force_cleared: 1;
    /// Debug-only equivalent of [`Self::interpolation_paused`].
    pub debug_interpolation_paused, set_debug_interpolation_paused: 2;
    /// Debug-only equivalent of [`Self::force_cleared`].
    pub debug_force_cleared, debug_set_force_cleared: 3;
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CSFD4FadePlateColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<&CSFD4FadePlateColor> for [f32; 4] {
    fn from(val: &CSFD4FadePlateColor) -> Self {
        [val.r, val.g, val.b, val.a]
    }
}

impl From<[f32; 4]> for CSFD4FadePlateColor {
    fn from(val: [f32; 4]) -> Self {
        Self {
            r: val[0],
            g: val[1],
            b: val[2],
            a: val[3],
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FadePlateEasing {
    Linear = 0,
    EaseIn = 1,
    EaseOut = 2,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FadePlateId {
    Title = 0,
    MapIn = 1,
    InGame = 2,
    Cutscene = 3,
    InCutscene = 4,
    Ending = 5,
    Event = 6,
    InGameChroma = 7,
    InGameBokeh = 8,
}

impl Index<FadePlateId> for [DLReferencePointer<CSFD4FadePlate>; 9] {
    type Output = DLReferencePointer<CSFD4FadePlate>;

    fn index(&self, index: FadePlateId) -> &Self::Output {
        &self[index as usize]
    }
}

impl IndexMut<FadePlateId> for [DLReferencePointer<CSFD4FadePlate>; 9] {
    fn index_mut(&mut self, index: FadePlateId) -> &mut Self::Output {
        &mut self[index as usize]
    }
}
