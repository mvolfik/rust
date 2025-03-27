use crate::ffi::{CStr, CString, OsStr, OsString};
use crate::hash::Hash;
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, SeekFrom, const_error};
use crate::mem::MaybeUninit;
use crate::os::helenos::ffi::OsStrExt;
use crate::path::{Path, PathBuf};
use crate::sync::Arc;
use crate::sys::common::small_c_string::run_path_with_cstr;
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

#[derive(Debug)]
pub struct ReadDir {
    base: Arc<PathBuf>, // to be shared with the DirEntry to build FullPath, if needed
    dir: *mut libc::DIR,
}

impl Drop for ReadDir {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.dir) };
    }
}

pub struct DirEntry {
    base: Arc<PathBuf>,
    name: CString,
}

#[derive(Clone, Debug)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
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
        let entry = unsafe { libc::readdir(self.dir) };
        if entry.is_null() {
            return None;
        }

        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };

        Some(Ok(DirEntry { base: self.base.clone(), name: name.to_owned() }))
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
        Err(const_error!(io::ErrorKind::Unsupported, "metadata unimplemented"))
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        Err(const_error!(io::ErrorKind::Unsupported, "file_type unimplemented"))
    }
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
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
        self.create = create;
    }
    pub fn create_new(&mut self, create_new: bool) {
        self.create_new = create_new;
    }

    /// Create `(r|w|x)[b|t][+][x]` string from the flags
    ///
    /// The possible mode combinations were enumerated with this Python script (which
    /// should be equivalent to the [`parse_mode`] function from HelenOS):
    ///
    /// [`parse_mode`]: <https://github.com/HelenOS/helenos/blob/32254d6ae36aa180fa53338ca60684a9c87d7947/uspace/lib/c/generic/io/io.c#L189>
    /// ```python
    /// # (r|w|a)[+][x]
    /// def parse_mode(fmode: str):
    ///     plus = len(fmode) > 1 and fmode[1] == "+"
    ///     ex = fmode[-1] == "x"
    ///
    ///     append = None
    ///     read = None
    ///     write = None
    ///     create = None
    ///     truncate = None
    ///     excl = None
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
    /// for p in ["", "+"]:
    ///     for x in ["", "x"]:
    ///         s = f"{c}{p}{x}"
    ///         try:
    ///             print(f"{s}: {parse_mode(s)}")
    ///         except ValueError as e:
    ///             print(f"{s}: {e}")
    /// ```
    ///
    /// This yields only the following valid combinations:
    /// ```text
    /// a:   {'append': True, 'read': None,  'write': True,  'create': True, 'truncate': None, 'excl': None}
    ///
    /// wx:  {'append': None, 'read': False, 'write': True,  'create': True, 'truncate': True, 'excl': True}
    /// w+x: {'append': None, 'read': True,  'write': True,  'create': True, 'truncate': None, 'excl': True}
    ///
    /// w:   {'append': None, 'read': False, 'write': True,  'create': True, 'truncate': True, 'excl': False}
    ///
    /// r:   {'append': None, 'read': True,  'write': False, 'create': None, 'truncate': None, 'excl': None}
    ///
    /// r+:  {'append': None, 'read': True,  'write': True,  'create': None, 'truncate': None, 'excl': None}
    /// w+:  {'append': None, 'read': True,  'write': True,  'create': True, 'truncate': None, 'excl': False}
    /// ```
    fn to_mode_str(&self) -> io::Result<[u8; 4]> {
        if self.append {
            // write can be any, append takes precedence
            if self.read {
                return Err(const_error!(
                    io::ErrorKind::InvalidInput,
                    "file opened with `append`+`read` is not supported on HelenOS"
                ));
            }
            if !self.create {
                return Err(const_error!(
                    io::ErrorKind::InvalidInput,
                    "file opened with `append` must be in create mode on HelenOS"
                ));
            }
            if self.truncate {
                return Err(const_error!(
                    io::ErrorKind::InvalidInput,
                    "file opened with `append` must not be in truncate mode on HelenOS"
                ));
            }
            if self.create_new {
                return Err(const_error!(
                    io::ErrorKind::InvalidInput,
                    "file opened with `append`+`create_new` is not supported on HelenOS"
                ));
            }
            return Ok(*b"a\0\0\0");
        }
        // append is false
        if self.create_new {
            // create, truncate are ignored
            if !self.write {
                return Err(const_error!(
                    io::ErrorKind::InvalidInput,
                    "file opened with `create_new` must be in write mode on HelenOS"
                ));
            }
            if self.read {
                return Ok(*b"w+x\0");
            }
            return Ok(*b"wx\0\0");
        }
        // create_new is false
        if !self.read {
            if !self.write {
                return Err(const_error!(
                    io::ErrorKind::InvalidInput,
                    "one of `read`,`write`,`append` must be set when opening a file"
                ));
            }
            if !self.truncate {
                return Err(const_error!(
                    io::ErrorKind::InvalidInput,
                    "file opened with `write` but not `read` must be in truncate mode"
                ));
            }
            if !self.create {
                return Err(const_error!(
                    io::ErrorKind::InvalidInput,
                    "file opened with `write` but not `read` must be in create mode"
                ));
            }
            return Ok(*b"w+x\0");
        }
        // read is true
        if !self.write {
            if self.truncate {
                return Err(const_error!(
                    io::ErrorKind::InvalidInput,
                    "file opened with `read` but not `write` must not be in truncate mode"
                ));
            }
            if self.create {
                return Err(const_error!(
                    io::ErrorKind::InvalidInput,
                    "file opened with `read` but not `write` must not be in create mode"
                ));
            }
            return Ok(*b"r\0\0\0");
        }
        if self.truncate {
            return Err(const_error!(
                io::ErrorKind::InvalidInput,
                "file opened with `read`+`write` can't be in truncate mode"
            ));
        }
        if self.create {
            return Ok(*b"w+\0\0");
        }
        return Ok(*b"r+\0\0");
    }
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        let mode_str = opts.to_mode_str()?;
        run_path_with_cstr(path, &|path| {
            let file = unsafe { libc::fopen(path.as_ptr(), mode_str.as_ptr() as *const i8) };
            if file.is_null() { Err(io::Error::last_os_error()) } else { Ok(File(file)) }
        })
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        Err(const_error!(io::ErrorKind::Unsupported, "file_attr unimplemented"))
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

    pub fn try_lock(&self) -> io::Result<bool> {
        Err(const_error!(io::ErrorKind::Unsupported, "try_lock unimplemented"))
    }

    pub fn try_lock_shared(&self) -> io::Result<bool> {
        Err(const_error!(io::ErrorKind::Unsupported, "try_lock_shared unimplemented"))
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

    pub fn duplicate(&self) -> io::Result<File> {
        Err(const_error!(io::ErrorKind::Unsupported, "duplicate unimplemented"))
    }

    pub fn set_permissions(&self, _perm: FilePermissions) -> io::Result<()> {
        Err(const_error!(io::ErrorKind::Unsupported, "set_permissions unimplemented"))
    }

    pub fn set_times(&self, _times: FileTimes) -> io::Result<()> {
        Err(const_error!(io::ErrorKind::Unsupported, "set_times unimplemented"))
    }
}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder {}
    }

    pub fn mkdir(&self, _p: &Path) -> io::Result<()> {
        unsupported()
    }
}

pub fn readdir(p: &Path) -> io::Result<ReadDir> {
    let dir = run_path_with_cstr(p, &|p| unsafe { Ok(libc::opendir(p.as_ptr())) })?;
    if dir.is_null() {
        Err(io::Error::last_os_error())
    } else {
        Ok(ReadDir { base: Arc::new(p.to_path_buf()), dir })
    }
}

pub fn unlink(_p: &Path) -> io::Result<()> {
    unsupported()
}

pub fn rename(_old: &Path, _new: &Path) -> io::Result<()> {
    unsupported()
}

pub fn set_perm(_p: &Path, _perm: FilePermissions) -> io::Result<()> {
    Err(const_error!(io::ErrorKind::Unsupported, "file permissions not supported on HelenOS"))
}

pub fn rmdir(_p: &Path) -> io::Result<()> {
    unsupported()
}

pub fn remove_dir_all(_path: &Path) -> io::Result<()> {
    unsupported()
}

pub fn exists(_path: &Path) -> io::Result<bool> {
    unsupported()
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

pub fn lstat(_p: &Path) -> io::Result<FileAttr> {
    unsupported()
}

pub fn canonicalize(_p: &Path) -> io::Result<PathBuf> {
    unsupported()
}

pub fn copy(_from: &Path, _to: &Path) -> io::Result<u64> {
    unsupported()
}
