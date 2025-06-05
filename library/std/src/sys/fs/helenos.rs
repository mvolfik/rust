use crate::ffi::{CStr, CString, OsStr, OsString, c_char};
use crate::fs::TryLockError;
use crate::hash::Hash;
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, SeekFrom, const_error};
use crate::mem::MaybeUninit;
use crate::os::helenos::ffi::OsStrExt;
use crate::path::{Path, PathBuf};
use crate::sync::Arc;
use crate::sys::common::small_c_string::run_path_with_cstr;
pub use crate::sys::fs::common::{copy, exists, remove_dir_all};
use crate::sys::time::SystemTime;
use crate::sys::{cvt, cvt_nz, unsupported};

#[derive(Debug)]
pub struct File(*mut libc::FILE);

unsafe impl Send for File {}

impl Drop for File {
    fn drop(&mut self) {
        unsafe { libc::fclose(self.0) };
    }
}

#[derive(Clone)]
pub struct FileAttr(libc::vfs_stat_t);

unsafe impl Send for FileAttr {}

#[derive(Clone, Debug)]
pub struct ReadDir {
    entries: Vec<DirEntry>,
}

#[derive(Clone, Debug)]
pub struct DirEntry {
    base: Arc<PathBuf>, // the base is shared among all entries
    name: CString,
}

#[derive(Clone, Debug)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: Option<bool>,
    create_new: bool,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct FileTimes {}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct FilePermissions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileType {
    is_file: bool,
    is_dir: bool,
}

#[derive(Debug)]
pub struct DirBuilder {}

impl FileAttr {
    pub fn size(&self) -> u64 {
        self.0.size as u64
    }

    pub fn perm(&self) -> FilePermissions {
        FilePermissions
    }

    pub fn file_type(&self) -> FileType {
        FileType { is_file: self.0.is_file, is_dir: self.0.is_directory }
    }

    pub fn modified(&self) -> io::Result<SystemTime> {
        Err(const_error!(io::ErrorKind::Unsupported, "file times not supported on HelenOS"))
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        Err(const_error!(io::ErrorKind::Unsupported, "file times not supported on HelenOS"))
    }

    pub fn created(&self) -> io::Result<SystemTime> {
        Err(const_error!(io::ErrorKind::Unsupported, "file times not supported on HelenOS"))
    }
}

impl FilePermissions {
    pub fn readonly(&self) -> bool {
        false
    }

    pub fn set_readonly(&mut self, _readonly: bool) {
        // noop - we return an error for anything that assigns file permissions to a file
    }
}

impl FileTimes {
    pub fn set_accessed(&mut self, _t: SystemTime) {}
    pub fn set_modified(&mut self, _t: SystemTime) {}
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn is_symlink(&self) -> bool {
        false
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        self.entries.pop().map(Result::Ok)
    }
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.base.join(OsStr::from_bytes(self.name.to_bytes()))
    }

    pub fn file_name(&self) -> OsString {
        OsStr::from_bytes(self.name.to_bytes()).to_os_string()
    }

    pub fn metadata(&self) -> io::Result<FileAttr> {
        stat(&self.path())
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        self.metadata().map(|attr| attr.file_type())
    }
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: None,
            create_new: false,
        }
    }

    pub fn read(&mut self, read: bool) {
        self.read = read;
    }
    pub fn write(&mut self, write: bool) {
        self.write = write;
    }
    pub fn append(&mut self, append: bool) {
        self.append = append;
    }
    pub fn truncate(&mut self, truncate: bool) {
        self.truncate = truncate;
    }
    pub fn create(&mut self, create: bool) {
        self.create = Some(create);
    }
    pub fn create_new(&mut self, create_new: bool) {
        self.create_new = create_new;
    }

    /// HelenOS API expects a string of the form `(r|w|a)[+][x]` to open files,
    /// but then, internally, this string is converted by [`parse_mode`] back
    /// to a set of flags exactly equivalent to the ones used here in Rust.
    ///
    /// We assume the implementation of the flags in HelenOS is correct, so
    /// we really want to write an inverse function to `parse_mode` to
    /// convert the flags to a string here.
    ///
    /// We created the following Python script (which contains a copy of `parse_mode`
    /// from HelenOS) to enumerate the valid combinations of flags and their meaning.
    ///
    /// [`parse_mode`]: <https://github.com/HelenOS/helenos/blob/32254d6ae36aa180fa53338ca60684a9c87d7947/uspace/lib/c/generic/io/io.c#L189>
    /// ```python
    /// # (r|w|a)[+][x]
    /// def parse_mode(fmode: str):
    ///     plus = "+" in fmode
    ///     ex = fmode[-1] == "x"
    ///
    ///     append = False
    ///     read = False
    ///     write = False
    ///     create = False
    ///     truncate = False
    ///     excl = False
    ///
    ///     if fmode[0] == "r":
    ///         read = True
    ///         write = plus
    ///         if ex:
    ///             raise ValueError("Invalid mode")
    ///
    ///     elif fmode[0] == "w":
    ///         write = True
    ///         read = plus
    ///         create = True
    ///         excl = ex
    ///         if not plus:
    ///             truncate = True
    ///
    ///     elif fmode[0] == "a":
    ///         if plus:
    ///             raise ValueError("Invalid mode")
    ///         if ex:
    ///             raise ValueError("Invalid mode")
    ///         append = True
    ///         write = True
    ///         create = True
    ///         read = plus
    ///
    ///     return {
    ///         "append": append,
    ///         "read": read,
    ///         "write": write,
    ///         "create": create,
    ///         "truncate": truncate,
    ///         "excl": excl
    ///     }
    ///
    /// for c in ["r", "w", "a"]:
    ///     for p in ["", "+"]:
    ///         for x in ["", "x"]:
    ///             s = f"{c}{p}{x}"
    ///             try:
    ///                 print(f"{s}: {parse_mode(s)}")
    ///             except ValueError as e:
    ///                 print(f"{s}: {e}")
    /// ```
    ///
    /// This yields only the following valid combinations:
    /// ```text
    /// a:   {'append': True,  'read': False, 'write': True,  'create': True,  'truncate': False, 'excl': False}
    ///
    /// wx:  {'append': False, 'read': False, 'write': True,  'create': True,  'truncate': True,  'excl': True}
    /// w+x: {'append': False, 'read': True,  'write': True,  'create': True,  'truncate': False, 'excl': True}
    ///
    /// w:   {'append': False, 'read': False, 'write': True,  'create': True,  'truncate': True,  'excl': False}
    ///
    /// r:   {'append': False, 'read': True,  'write': False, 'create': False, 'truncate': False, 'excl': False}
    ///
    /// r+:  {'append': False, 'read': True,  'write': True,  'create': False, 'truncate': False, 'excl': False}
    /// w+:  {'append': False, 'read': True,  'write': True,  'create': True,  'truncate': False, 'excl': False}
    /// ```
    fn to_mode_str(&self) -> io::Result<[u8; 4]> {
        let create = self.create.unwrap_or(if self.append { true } else { false });
        match (self.append, self.read, self.write, create, self.truncate, self.create_new) {
            // app, rea, writ, crea, trun, excl
            // all disabled
            (false, false, false, _, _, _) => Err(const_error!(
                io::ErrorKind::InvalidInput,
                "one of `read`,`write`,`append` must be set when opening a file"
            )),
            // append mode, Rust says append implies write
            (true, false, _, true, false, false) => Ok(*b"a\0\0\0"),
            (true, _, _, _, _, _) => Err(const_error!(
                io::ErrorKind::InvalidInput,
                "file opened with `append` must be !create on HelenOS, and nonoe of `read,truncate,exclusive` can be set"
            )),
            // exclusive create mode
            // create,truncate are irrelevant
            (_, false, true, _, _, true) => Ok(*b"wx\0\0"),
            (_, true, true, _, _, true) => Ok(*b"w+x\0"),
            (_, _, _, _, _, true) => Err(const_error!(
                io::ErrorKind::InvalidInput,
                "file opened with `create_new` must have `write` on HelenOS"
            )),
            // write only
            (_, false, true, true, true, _) => Ok(*b"w\0\0\0"),
            (_, false, true, _, _, _) => Err(const_error!(
                io::ErrorKind::InvalidInput,
                "file opened write-only must have `create+truncate` on HelenOS"
            )),
            // read only
            (_, true, false, false, false, _) => Ok(*b"r\0\0\0"),
            (_, true, false, _, _, _) => Err(const_error!(
                io::ErrorKind::InvalidInput,
                "file opened with `read` but not `write` can't truncate or create"
            )),
            // read+write
            (_, true, true, true, false, _) => Ok(*b"w+\0\0"),
            (_, true, true, false, false, _) => Ok(*b"r+\0\0"),
            (_, true, true, _, _, _) => Err(const_error!(
                io::ErrorKind::InvalidInput,
                "file opened with `read`+`write` can't be in truncate mode"
            )),
        }
    }
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        let mode_str = opts.to_mode_str()?;
        run_path_with_cstr(path, &|path| {
            let file = unsafe { libc::fopen(path.as_ptr(), mode_str.as_ptr() as *const c_char) };
            if file.is_null() { Err(io::Error::last_os_error()) } else { Ok(File(file)) }
        })
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        let mut handle = 0;
        cvt_nz(unsafe { libc::vfs_fhandle(self.0, &mut handle) })?;

        let mut stat_val = MaybeUninit::uninit();
        cvt_nz(unsafe { libc::vfs_stat(handle, stat_val.as_mut_ptr()) })?;
        Ok(FileAttr(unsafe { stat_val.assume_init() }))
    }

    pub fn fsync(&self) -> io::Result<()> {
        Err(const_error!(io::ErrorKind::Unsupported, "fsync unimplemented"))
    }

    pub fn datasync(&self) -> io::Result<()> {
        Err(const_error!(io::ErrorKind::Unsupported, "datasync unimplemented"))
    }

    pub fn lock(&self) -> io::Result<()> {
        Err(const_error!(io::ErrorKind::Unsupported, "lock unimplemented"))
    }

    pub fn lock_shared(&self) -> io::Result<()> {
        Err(const_error!(io::ErrorKind::Unsupported, "lock_shared unimplemented"))
    }

    pub fn try_lock(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(const_error!(io::ErrorKind::Unsupported, "try_lock unimplemented")))
    }

    pub fn try_lock_shared(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(const_error!(
            io::ErrorKind::Unsupported,
            "try_lock_shared unimplemented"
        )))
    }

    pub fn unlock(&self) -> io::Result<()> {
        Err(const_error!(io::ErrorKind::Unsupported, "unlock unimplemented"))
    }

    pub fn truncate(&self, _size: u64) -> io::Result<()> {
        Err(const_error!(io::ErrorKind::Unsupported, "truncate unimplemented"))
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let ret =
            unsafe { libc::fread(buf.as_mut_ptr() as *mut libc::c_void, 1, buf.len(), self.0) };
        // apparently helenos doesn't provide a way to detect failures from fread
        Ok(ret)
    }

    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        crate::io::default_read_vectored(|buf| self.read(buf), bufs)
    }

    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn read_buf(&self, mut cursor: BorrowedCursor<'_>) -> io::Result<()> {
        unsafe {
            let ret = libc::fread(
                cursor.as_mut().as_mut_ptr() as *mut libc::c_void,
                1,
                cursor.capacity(),
                self.0,
            );
            cursor.advance_unchecked(ret);
        }
        Ok(())
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        Ok(unsafe { libc::fwrite(buf.as_ptr() as *const libc::c_void, 1, buf.len(), self.0) }
            as usize)
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        crate::io::default_write_vectored(|buf| self.write(buf), bufs)
    }

    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn flush(&self) -> io::Result<()> {
        cvt_nz(unsafe { libc::fflush(self.0) })
    }

    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        let (whence, offset) = match pos {
            SeekFrom::Start(pos) => (libc::SEEK_SET, pos as i64),
            SeekFrom::End(pos) => (libc::SEEK_END, pos),
            SeekFrom::Current(pos) => (libc::SEEK_CUR, pos),
        };
        let ret = unsafe { libc::fseek(self.0, offset.try_into().unwrap(), whence) };
        assert_eq!(cvt(ret)?, 0);
        cvt(unsafe { libc::ftell(self.0) }).map(|x| x as u64)
    }

    pub fn tell(&self) -> io::Result<u64> {
        self.seek(SeekFrom::Current(0))
    }

    pub fn duplicate(&self) -> io::Result<File> {
        Err(const_error!(io::ErrorKind::Unsupported, "duplicate unimplemented"))
    }

    pub fn set_permissions(&self, _perm: FilePermissions) -> io::Result<()> {
        Ok(())
    }

    pub fn set_times(&self, _times: FileTimes) -> io::Result<()> {
        Err(const_error!(io::ErrorKind::Unsupported, "set_times unimplemented"))
    }
}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder {}
    }

    pub fn mkdir(&self, p: &Path) -> io::Result<()> {
        run_path_with_cstr(p, &|path| {
            cvt_nz(unsafe {
                libc::vfs_link_path(
                    path.as_ptr(),
                    libc::vfs_file_kind_t::KIND_DIRECTORY,
                    crate::ptr::null_mut(),
                )
            })
        })
    }
}

pub fn readdir(p: &Path) -> io::Result<ReadDir> {
    let dir = run_path_with_cstr(p, &|p| unsafe { Ok(libc::opendir(p.as_ptr())) })?;
    if dir.is_null() {
        return Err(io::Error::last_os_error());
    }
    // unfortunately, the iteration can't be done lazily, because it triggers issues
    // when the directory is modified while we are iterating over it
    let mut entries = Vec::new();
    let base = Arc::new(p.to_path_buf());
    loop {
        let entry = unsafe { libc::readdir(dir) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_owned();
        entries.push(DirEntry { base: base.clone(), name });
    }
    unsafe { libc::closedir(dir) };
    // we will later pop() from the end, let's return them in the system order
    entries.reverse();
    Ok(ReadDir { entries })
}

pub fn unlink(p: &Path) -> io::Result<()> {
    run_path_with_cstr(p, &|path| cvt_nz(unsafe { libc::vfs_unlink_path(path.as_ptr()) }))
}

pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    run_path_with_cstr(old, &|old| {
        run_path_with_cstr(new, &|new| {
            cvt_nz(unsafe { libc::vfs_rename_path(old.as_ptr(), new.as_ptr()) })
        })
    })
}

pub fn set_perm(_p: &Path, _perm: FilePermissions) -> io::Result<()> {
    Ok(()) // no permissions on HelenOS
}

pub fn rmdir(p: &Path) -> io::Result<()> {
    unlink(p)
}

pub fn readlink(_p: &Path) -> io::Result<PathBuf> {
    unsupported()
}

pub fn symlink(_original: &Path, _link: &Path) -> io::Result<()> {
    unsupported()
}

pub fn link(_src: &Path, _dst: &Path) -> io::Result<()> {
    unsupported()
}

pub fn stat(p: &Path) -> io::Result<FileAttr> {
    run_path_with_cstr(p, &|path| unsafe {
        let mut stat_val = MaybeUninit::uninit();
        cvt_nz(libc::vfs_stat_path(path.as_ptr(), stat_val.as_mut_ptr()))?;
        Ok(FileAttr(stat_val.assume_init()))
    })
}

pub fn lstat(p: &Path) -> io::Result<FileAttr> {
    stat(p) // HelenOS has no symlinks
}

pub fn canonicalize(_p: &Path) -> io::Result<PathBuf> {
    unsupported()
}
