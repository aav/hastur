use std::sync::atomic::{AtomicBool, Ordering};

use futures::{channel::oneshot, future::pending};

use dashmap::{
    mapref::one::{Ref, RefMut},
    DashMap, DashSet,
};

use crate::inbox;
use crate::pid::{myself, MonitorRef, Pid};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExitReason {
    Normal,
    Custom,
    NoProc(Pid),
    Panic,
    Kill,
    JoinError,
}

pub struct Kernel {
    pid: Pid,
    linked: DashSet<Pid>,
    monitors: DashMap<MonitorRef, Pid>,
    self_exit_sender: Option<oneshot::Sender<ExitReason>>,
    trap_exit: AtomicBool,
}

lazy_static::lazy_static! {
    static ref PKERNEL: DashMap<Pid, Kernel> = {
        DashMap::new()
    };
}

const NOKERNEL: &str = "nokernel";

pub(crate) fn get(pid: &Pid) -> Ref<'static, Pid, Kernel> {
    PKERNEL.get(pid).expect(NOKERNEL)
}

pub(crate) fn get_mut(pid: &Pid) -> RefMut<'static, Pid, Kernel> {
    PKERNEL.get_mut(pid).expect(NOKERNEL)
}

pub(crate) fn place(pid: Pid, kernel: Kernel) {
    PKERNEL.insert(pid, kernel);
}

pub(crate) fn remove(pid: &Pid) -> Kernel {
    PKERNEL.remove(pid).expect(NOKERNEL).1
}

impl Kernel {
    pub fn new(pid: Pid, self_exit_sender: oneshot::Sender<ExitReason>) -> Self {
        Kernel {
            pid,
            linked: DashSet::new(),
            monitors: DashMap::new(),

            trap_exit: AtomicBool::new(false),
            self_exit_sender: Some(self_exit_sender),
        }
    }

    pub fn trap_exit(&self, trap_exit: bool) {
        self.trap_exit.store(trap_exit, Ordering::Relaxed);
    }

    pub fn get_trap_exit(&self) -> bool {
        self.trap_exit.load(Ordering::Relaxed)
    }

    pub fn exit(&mut self, reason: ExitReason) {
        if let Some(sender) = self.self_exit_sender.take() {
            let _ = sender.send(reason);
        } else {
            unreachable!();
        }
    }

    pub fn link(&self, pid: Pid) {
        tracing::trace!(event = "link", pid1 = ?self.pid, pid2 = ?pid);

        get(&pid).linked.insert(self.pid);
        self.linked.insert(pid);
    }

    pub fn for_each_linked<F: Fn(&Pid)>(&self, f: F) {
        self.linked.iter().for_each(|pid| {
            f(&pid);
        });
    }

    pub fn monitor(&self, monitor_ref: MonitorRef, monitor_pid: Pid) {
        self.monitors.insert(monitor_ref, monitor_pid);
    }
}

pub fn link(to: Pid) {
    let myself = myself();

    if let Some(kernel) = PKERNEL.get(&to) {
        kernel.link(myself);
    } else {
        inbox::send_exit(&myself, inbox::Exit(myself, ExitReason::NoProc(to)));
    }
}

pub fn trap_exit(value: bool) {
    get_mut(&myself()).trap_exit(value);
}

pub fn get_trap_exit() -> bool {
    get_mut(&myself()).get_trap_exit()
}

pub async fn exit(reason: ExitReason) {
    get_mut(&myself()).exit(reason);

    // never return, assime that task will not be scheduled
    pending::<()>().await;
}
