use crate::ffi::CStr;
use crate::io;
use crate::num::NonZero;
use crate::sync::{Arc, Condvar, Mutex};
use crate::time::Duration;

pub const DEFAULT_MIN_STACK_SIZE: usize = 1 * 1024 * 1024;

pub struct Thread {
    id: libc::fid_t,
    done: Arc<(Mutex<bool>, Condvar)>,
}

unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}

impl Thread {
    // unsafe: see thread::Builder::spawn_unchecked for safety requirements
    pub unsafe fn new(stack: usize, p: Box<dyn FnOnce()>) -> io::Result<Thread> {
        let done = Arc::new((Mutex::new(false), Condvar::new()));
        let done_copy = Arc::clone(&done);
        let wrapper: Box<dyn FnOnce()> = Box::new(move || {
            p();
            *done_copy.0.lock().unwrap() = true;
            done_copy.1.notify_all();
        });
        let p = Box::into_raw(Box::new(wrapper));
        let res = libc::fibril_create_generic(thread_start, p as *mut _, stack);
        if res.is_null() {
            drop(Box::from_raw(p));
            return Err(io::Error::new(io::ErrorKind::Other, "fibril_create failed"));
        }
        libc::fibril_start(res);
        return Ok(Thread { id: res, done });

        extern "C" fn thread_start(main: *mut libc::c_void) -> libc::errno_t {
            unsafe {
                Box::from_raw(main as *mut Box<dyn FnOnce()>)();
            }
            libc::EOK
        }
    }

    pub fn yield_now() {
        unsafe { libc::fibril_yield() };
    }

    pub fn set_name(_name: &CStr) {
        // nope
    }

    pub fn sleep(dur: Duration) {
        unsafe { libc::fibril_usleep(dur.as_micros() as i64) };
    }

    pub fn join(self) {
        let (lock, cvar) = &*self.done;
        let mut done = lock.lock().unwrap();
        while !*done {
            done = cvar.wait(done).unwrap();
        }
    }
}

pub fn available_parallelism() -> io::Result<NonZero<usize>> {
    super::unsupported()
}
