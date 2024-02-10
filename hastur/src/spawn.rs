use futures::{
    channel::oneshot,
    future::{poll_fn, BoxFuture, FutureExt},
    select_biased, Future,
};

use std::panic::AssertUnwindSafe;

use crate::inbox::{self, Exit};
use crate::kernel::{self, ExitReason};
use crate::pid::{myself, MonitorRef, Pid, PID};

use derive_builder::Builder;

#[derive(Default, Builder, Debug)]
pub struct SpawnOpt {
    #[builder(setter(into), default = "false")]
    link: bool,
    #[builder(setter(into), default = "false")]
    monitor: bool,
}

pub fn __spawn_opt<P>(
    proc: P,
    opt: SpawnOpt,
) -> (Pid, Option<MonitorRef>, impl Future<Output = ExitReason>)
where
    P: Future + Send + 'static,
{
    let link = if opt.link { Some(myself()) } else { None };

    let (pid, monitor_ref, join_handle) = if opt.monitor {
        let monitor_ref = MonitorRef::new();

        let (pid, join_handle) = spawn_int(proc, link, Some((monitor_ref, myself())));

        (pid, Some(monitor_ref), join_handle)
    } else {
        let (pid, join_handle) = spawn_int(proc, link, None);

        (pid, None, join_handle)
    };
    if opt.link {
        kernel::get_mut(&myself()).link(pid);
    }

    (pid, monitor_ref, join_handle)
}

pub fn spawn_opt<P>(proc: P, opt: SpawnOpt) -> Pid
where
    P: Future + Send + 'static,
{
    __spawn_opt(proc, opt).0
}

pub fn __spawn<P>(proc: P) -> (Pid, impl Future<Output = ExitReason>)
where
    P: Future + Send + 'static,
{
    spawn_int(proc, None, None)
}

pub fn spawn<P>(proc: P) -> Pid
where
    P: Future + Send + 'static,
{
    spawn_int(proc, None, None).0
}

pub fn __spawn_link<P>(proc: P) -> (Pid, impl Future<Output = ExitReason>)
where
    P: Future + Send + 'static,
{
    spawn_int(proc, Some(myself()), None)
}

pub fn spawn_link<P>(proc: P) -> Pid
where
    P: Future + Send + 'static,
{
    __spawn_link(proc).0
}

fn spawn_int<F>(
    future: F,
    link_to: Option<Pid>,
    monitor: Option<(MonitorRef, Pid)>,
) -> (Pid, BoxFuture<'static, ExitReason>)
where
    F: Future + Send + 'static,
{
    let pid = Pid::new();

    let span = tracing::span!(parent: None, tracing::Level::DEBUG, "process", ?pid);

    tracing::trace!(parent: &span, event = "spawn", ?link_to, ?monitor);

    let (self_exit_sender, self_exit_receiver) = oneshot::channel();

    let context = kernel::Kernel::new(pid, self_exit_sender);

    if let Some(link_to) = link_to {
        context.link(link_to);
    }

    if let Some((monitor_ref, monitor_pid)) = monitor {
        context.monitor(monitor_ref, monitor_pid);
    }

    kernel::place(pid, context);

    inbox::create(pid);

    let task = async move {
        let mut future = future.boxed();

        let future = poll_fn(move |cx| {
            PID.with(|cell| cell.set(Some(pid)));
            future.as_mut().poll(cx)
        });

        let mut future = AssertUnwindSafe(future).catch_unwind().fuse();
        let mut self_exit_receiver = self_exit_receiver.fuse();

        let reason = loop {
            select_biased! {
                reason = self_exit_receiver => {
                    let reason = reason.unwrap();
                    tracing::trace!(event="receive_self_exit", ?reason);
                    break reason;
                },

                exit = inbox::receive_exit(pid).fuse() => {
                    static EXIT: &str = "receive_exit";

                    let Exit(from, reason) = exit;

                    let trap_exit = kernel::get(&pid).get_trap_exit();

                    if trap_exit {
                        if reason == ExitReason::Kill {
                            tracing::trace!(event=EXIT, ?from, ?reason, trap_exit, outcome = "exit");
                            break reason;
                        }else{
                            tracing::trace!(event=EXIT, ?from, ?reason, trap_exit, outcome = "send");
                            inbox::send(pid, Exit(pid, reason));
                        }
                    }else{
                        if reason != ExitReason::Normal {
                            tracing::trace!(event=EXIT, ?from, ?reason, trap_exit, outcome = "exit");
                            break reason;
                        }else{
                            tracing::trace!(event=EXIT, ?from, ?reason, trap_exit, outcome = "ignore");
                        }
                    }
                },

                result = future => {
                    let reason =
                        match result {
                            Ok(_) => {
                                ExitReason::Normal
                            },
                            Err(_) => {
                                ExitReason::Panic
                            }
                        };

                    break reason;
                }
            };
        };

        tracing::trace!(event = "exit", ?reason);

        inbox::drop(&pid);
        let context = kernel::remove(&pid);

        context.for_each_linked(|linked| {
            inbox::send_exit(&linked, Exit(pid, reason));
        });

        reason
    };

    cfg_if::cfg_if! {
       if #[cfg(debug_assertions)] {
           use tracing_futures::Instrument;
           let task = task.instrument(span);
       }
    }

    if async_metronome::is_context() {
        (pid, async_metronome::spawn(task).boxed())
    } else {
        use async_std::task;
        (pid, task::spawn(task).boxed())
    }
}
