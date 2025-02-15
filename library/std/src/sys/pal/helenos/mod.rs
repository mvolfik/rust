use crate::io::ErrorKind;

#[path = "../unix/args.rs"]
pub mod args;
#[path = "../unsupported/env.rs"]
pub mod env;
#[path = "../unsupported/fs.rs"]
pub mod fs;
pub mod os;
#[path = "../unsupported/pipe.rs"]
pub mod pipe;
#[path = "../unsupported/process.rs"]
pub mod process;
pub mod stdio;
#[path = "../unsupported/thread.rs"]
pub mod thread;
#[path = "../unsupported/time.rs"]
pub mod time;

pub fn unsupported<T>() -> crate::io::Result<T> {
    Err(unsupported_err())
}

pub fn unsupported_err() -> crate::io::Error {
    crate::io::const_error!(
        crate::io::ErrorKind::Unsupported,
        "operation not supported on HelenOS yet",
    )
}

// SAFETY: must be called only once during runtime cleanup.
// NOTE: this is not guaranteed to run, for example when the program aborts.
pub unsafe fn cleanup() {}

pub unsafe fn init(argc: isize, argv: *const *const u8, _sigpipe: u8) {
    args::init(argc, argv);
}

pub fn abort_internal() -> ! {
    unsafe { libc::abort() }
}

pub fn decode_error_kind(_errno: i32) -> ErrorKind { ErrorKind::Uncategorized }

pub(crate) fn is_interrupted(_errno: i32) -> bool { false }

// pub mod sync;
