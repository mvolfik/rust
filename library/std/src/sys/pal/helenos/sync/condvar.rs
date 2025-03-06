use super::Mutex;
use crate::cell::UnsafeCell;
use crate::mem;
use crate::pin::Pin;
use crate::time::Duration;

pub struct Condvar {
    inner: UnsafeCell<libc::fibril_condvar_t>,
}

impl Condvar {
    pub(crate) const PRECISE_TIMEOUT: bool = true;
    pub fn new() -> Condvar {
        // This creates an instance in an undefined state - I hope the compiler allows that on C structs?
        // We can unfortunately only properly initialize it after pinning it in memory.
        Condvar { inner: UnsafeCell::new(unsafe { mem::zeroed() }) }
    }

    fn raw(&self) -> *mut libc::fibril_condvar_t {
        self.inner.get()
    }

    /// # Safety
    /// May only be called once per instance of `Self`.
    pub unsafe fn init(self: Pin<&mut Self>) {
        unsafe { libc::fibril_condvar_initialize(self.raw()) }
    }

    /// # Safety
    /// `init` must have been called on this instance.
    pub unsafe fn notify_one(self: Pin<&Self>) {
        unsafe { libc::fibril_condvar_signal(self.raw()) }
    }

    /// # Safety
    /// `init` must have been called on this instance.
    pub unsafe fn notify_all(self: Pin<&Self>) {
        unsafe { libc::fibril_condvar_broadcast(self.raw()) }
    }

    /// # Safety
    /// * `init` must have been called on this instance.
    /// * `mutex` must be locked by the current thread.
    /// * This condition variable may only be used with the same mutex.
    pub unsafe fn wait(self: Pin<&Self>, mutex: Pin<&Mutex>) {
        unsafe { libc::fibril_condvar_wait(self.raw(), mutex.raw()) }
    }

    /// # Safety
    /// * `init` must have been called on this instance.
    /// * `mutex` must be locked by the current thread.
    /// * This condition variable may only be used with the same mutex.
    pub unsafe fn wait_timeout(&self, mutex: Pin<&Mutex>, dur: Duration) -> bool {
        let dur: libc::c_longlong = dur
            .as_micros()
            .try_into()
            .expect("Cannot specify condvar timeout duration that doesn't fit long long");
        let r = unsafe { libc::fibril_condvar_wait_timeout(self.raw(), mutex.raw(), dur) };
        r == 0
    }
}

impl !Unpin for Condvar {}

unsafe impl Sync for Condvar {}
unsafe impl Send for Condvar {}
