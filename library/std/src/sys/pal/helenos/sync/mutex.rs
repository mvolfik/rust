use crate::cell::UnsafeCell;
use crate::mem;
use crate::pin::Pin;

pub struct Mutex {
    inner: UnsafeCell<libc::fibril_mutex_t>,
}

impl Mutex {
    pub fn new() -> Mutex {
        // This creates an instance in an undefined state - I hope the compiler allows that on C structs?
        // We can unfortunately only properly initialize it after pinning it in memory.
        Mutex { inner: UnsafeCell::new(unsafe { mem::zeroed() }) }
    }

    pub(super) fn raw(&self) -> *mut libc::fibril_mutex_t {
        self.inner.get()
    }

    /// # Safety
    /// May only be called once per instance of `Self`.
    pub unsafe fn init(self: Pin<&mut Self>) {
        unsafe { libc::fibril_mutex_initialize(self.raw()) }
    }

    /// # Safety
    /// `init` must have been called on this instance.
    pub unsafe fn lock(self: Pin<&Self>) {
        unsafe { libc::fibril_mutex_lock(self.raw()) }
    }

    /// # Safety
    /// `init` must have been called on this instance.
    pub unsafe fn try_lock(self: Pin<&Self>) -> bool {
        unsafe { libc::fibril_mutex_trylock(self.raw()) }
    }

    /// # Safety
    /// The mutex must be locked by the current thread.
    pub unsafe fn unlock(self: Pin<&Self>) {
        unsafe { libc::fibril_mutex_unlock(self.raw()) }
    }
}

impl !Unpin for Mutex {}

unsafe impl Send for Mutex {}
unsafe impl Sync for Mutex {}
