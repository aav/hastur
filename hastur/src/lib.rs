#![recursion_limit = "256"]

mod inbox;
mod kernel;
mod pid;
mod spawn;

pub use inbox::{Envelope, SaveQueue, __receive, __selective_restore, send, send_exit, send_raw};
pub use pid::{cpid, myself, MonitorRef, Pid};
pub use spawn::{
    __spawn, __spawn_link, __spawn_opt, spawn, spawn_link, spawn_opt, SpawnOpt, SpawnOptBuilder,
};

pub use kernel::{exit, get_trap_exit, link, trap_exit, ExitReason};

pub use hastur_macro::receive;
