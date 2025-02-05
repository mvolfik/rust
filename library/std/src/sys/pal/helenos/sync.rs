use crate::mem::MaybeUninit;
use crate::cell::UnsafeCell;
use crate::io::Error;
use crate::pin::Pin;

pub struct Mutex {
    inner: MaybeUninit<UnsafeCell<libc::fibril_mutex_t>>,
}

impl Mutex {
    pub fn new() -> Mutex {
        Mutex { inner: MaybeUninit::uninit() }
    }

    fn raw(&self) -> *mut libc::fibril_mutex_t {
        UnsafeCell::raw_get(self.inner.as_ptr())
    }

    /// # Safety
    /// May only be called once per instance of `Self`.
    pub unsafe fn init(self: Pin<&mut Self>) {
        unsafe { libc::fibril_mutex_initialize(self.raw()) }
    }

    /// # Safety
    /// * Locking mutex without calling `init` first causes undefined behaviour.
    /// * Destroying a locked mutex causes undefined behaviour.
    pub unsafe fn lock(self: Pin<&Self>) {
        unsafe { libc::fibril_mutex_lock(self.raw()) }
    }

    /// # Safety
    /// * Locking mutex without calling `init` first causes undefined behaviour.
    /// * Destroying a locked mutex causes undefined behaviour.
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

impl Drop for Mutex {
    fn drop(&mut self) {
        // TODO: ????
    }
}
