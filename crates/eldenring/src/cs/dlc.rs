use shared::singleton;

#[repr(C)]
#[singleton("CSDlc")]
pub struct CSDlcImp {
    _vtable: usize,
    dlc_platform: usize,
    pub bonus_gesture: bool,
    pub sote: bool,
    pub sote_gesture: bool,
    unused_dlcs: [bool; 0x2F],
    unk42: bool,
    unk43: bool,
    pub applied_dlc_event_flags: bool,
    unk45: bool,
    unk48: usize,
}
