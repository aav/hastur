use std::cell::Cell;
use std::sync::atomic::{AtomicU32, Ordering};

static PIDGEN: AtomicU32 = AtomicU32::new(0);
static MONGEN: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pid(u32);

impl Pid {
    pub(crate) fn new() -> Self {
        Self(PIDGEN.fetch_add(1, Ordering::Relaxed))
    }
}

impl std::fmt::Display for Pid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pid<{}>", self.0)
    }
}

impl std::fmt::Debug for Pid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

thread_local! {
    pub static PID: Cell<Option<Pid>> = Cell::new(None);
}

pub fn myself() -> Pid {
    PID.with(|cell| cell.get().expect("noproc"))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MonitorRef(u32);

impl MonitorRef {
    pub(crate) fn new() -> Self {
        Self(MONGEN.fetch_add(1, Ordering::Relaxed))
    }
}

impl std::fmt::Display for MonitorRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MonitorRef<{}>", self.0)
    }
}

impl std::fmt::Debug for MonitorRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

pub fn cpid() -> u32 {
    PIDGEN.load(Ordering::Relaxed)
}
