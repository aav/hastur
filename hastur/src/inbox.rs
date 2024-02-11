use atomic_refcell::AtomicRefCell;

use futures::{
    future::{poll_fn, Future},
    task::{AtomicWaker, Poll},
};
use std::any::{Any, TypeId};
use std::collections::VecDeque;

use crate::kernel::ExitReason;
use crate::pid::{myself, Pid};
use crossbeam::queue::SegQueue;

use dashmap::DashMap;

use tracing::{self, instrument};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exit(pub Pid, pub ExitReason);

pub struct Envelope {
    size: usize,
    message: Box<dyn Any>,
}

unsafe impl Send for Envelope {}
unsafe impl Sync for Envelope {}

impl Envelope {
    pub fn new<M: Sized + Send + 'static>(message: M) -> Self {
        Self {
            size: std::mem::size_of::<M>(),
            message: Box::new(message),
        }
    }

    pub fn downcast<T: Any>(self) -> Option<T> {
        self.message.downcast::<T>().ok().map(|x| *x)
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.message.downcast_ref::<T>()
    }

    pub fn id(&self) -> TypeId {
        self.message.type_id()
    }

    pub fn is<T: Any>(&self) -> bool {
        self.message.is::<T>()
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl<T: PartialEq + Send + 'static> PartialEq<T> for Envelope {
    fn eq(&self, other: &T) -> bool {
        if let Some(this) = self.downcast_ref::<T>() {
            this == other
        } else {
            false
        }
    }
}

impl std::fmt::Debug for Envelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Envelope")
            .field("size", &self.size)
            .field("typeid", &self.id())
            .finish()
    }
}

pub type SaveQueue = VecDeque<Envelope>;

struct Inbox {
    message_queue: SegQueue<Envelope>,
    save_queue: AtomicRefCell<SaveQueue>,
    exit_queue: SegQueue<Exit>,

    waker: AtomicWaker,
}

impl Inbox {
    fn new() -> Self {
        let message_queue = SegQueue::new();
        let waker = AtomicWaker::new();
        let save_queue = AtomicRefCell::new(SaveQueue::new());

        let exit_queue = SegQueue::new();

        Self {
            message_queue,
            waker,
            save_queue,
            exit_queue,
        }
    }
}

lazy_static::lazy_static! {
    static ref PINBOX: DashMap<Pid, Inbox> = {
        DashMap::new()
    };
}

pub(crate) fn create(pid: Pid) {
    PINBOX.insert(pid, Inbox::new());
}

pub(crate) fn drop(pid: &Pid) {
    PINBOX.remove(pid);
}

static NOPROC: &str = "noproc";

pub fn send<T: Send + 'static>(to: Pid, message: T) {
    if let Some(inbox) = PINBOX.get(&to) {
        inbox.message_queue.push(Envelope::new(message));
        inbox.waker.wake();
    } else {
        tracing::warn!(NOPROC);
    }
}

#[instrument(level = "debug", skip(envelope))]
pub fn send_raw(to: Pid, envelope: Envelope) {
    if let Some(inbox) = PINBOX.get(&to) {
        inbox.message_queue.push(envelope);
        inbox.waker.wake();
    } else {
        tracing::warn!(NOPROC);
    }
}

pub fn __receive() -> impl Future<Output = Envelope> {
    let myself = myself();

    poll_fn(move |context| {
        let inbox = PINBOX.get(&myself).expect(NOPROC);

        if !inbox.exit_queue.is_empty() {
            // yield if there are exits
            Poll::Pending
        } else {
            match inbox.save_queue.borrow_mut().pop_back() {
                Some(envelope) => Poll::Ready(envelope),

                None => {
                    if !inbox.message_queue.is_empty() {
                        Poll::Ready(inbox.message_queue.pop().unwrap())
                    } else {
                        inbox.waker.register(context.waker());
                        Poll::Pending
                    }
                }
            }
        }
    })
}

pub fn __selective_restore(mut save: SaveQueue) {
    let inbox = PINBOX.get(&myself()).expect(NOPROC);

    inbox.save_queue.borrow_mut().append(&mut save);
}

pub fn send_exit(to: &Pid, exit: Exit) -> bool {
    if let Some(inbox) = PINBOX.get(to) {
        inbox.exit_queue.push(exit);
        inbox.waker.wake();
        true
    } else {
        tracing::trace!(event = "send_exit", what = NOPROC, ?to, ?exit);
        false
    }
}

pub(crate) fn receive_exit(pid: Pid) -> impl Future<Output = Exit> {
    poll_fn(move |context| {
        let inbox = PINBOX.get(&pid).expect(NOPROC);

        if !inbox.exit_queue.is_empty() {
            Poll::Ready(inbox.exit_queue.pop().unwrap())
        } else {
            inbox.waker.register(context.waker());
            Poll::Pending
        }
    })
}
