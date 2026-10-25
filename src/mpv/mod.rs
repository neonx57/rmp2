pub mod commands;
pub mod events;
pub mod ipc;
pub mod process;
pub mod properties;

pub use events::MpvMsg;
pub use properties::{LiveInfo, live_info, probe_metadata};

use crate::mpv::ipc::Pending;
use std::os::unix::net::UnixStream;
use std::process::Child;
use std::sync::atomic::AtomicU32;

pub const P_PAUSE: u32 = 1;
pub const P_TIME_POS: u32 = 2;
pub const P_CACHE: u32 = 3;

pub struct Mpv {
    pub(super) sock: UnixStream,
    pub(super) next_id: AtomicU32,
    pub(super) pending: Pending,
    pub(super) child: Child,
    pub(super) ipc_path: String,
}
