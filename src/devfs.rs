use esp_idf_svc::sys::{
    dirent, esp_vfs_register, esp_vfs_t, esp_vfs_t__bindgen_ty_13, esp_vfs_t__bindgen_ty_15,
    esp_vfs_t__bindgen_ty_18, EspError, DIR, DT_REG, ESP_VFS_FLAG_DEFAULT,
};
use std::{
    ffi::{CStr, CString},
    fmt, io,
    path::PathBuf,
    ptr::{self, NonNull},
    str::FromStr,
    sync::Mutex,
    vec::IntoIter,
};

#[allow(clippy::cast_possible_truncation)]
const FILETYPE_REGULAR: u8 = DT_REG as _;

static DEVFS: Mutex<DevFs> = Mutex::new(DevFs::new());

type OpenDirImpl = esp_vfs_t__bindgen_ty_13;
type CloseDirImpl = esp_vfs_t__bindgen_ty_18;
type ReaddirRImpl = esp_vfs_t__bindgen_ty_15;

pub struct DirHandle {
    ptr: NonNull<DIR>,
    iter: IntoIter<u16>,
}

#[derive(Debug)]
pub struct DevFs {
    content: Vec<dirent>,
    dirhandles: Vec<DirHandle>,
}

impl DevFs {
    pub const fn new() -> Self {
        Self {
            content: Vec::new(),
            dirhandles: Vec::new(),
        }
    }

    pub fn setup(&mut self) {
        self.content.push(dirent {
            d_ino: 1,
            d_type: FILETYPE_REGULAR,
            d_name: filename("test"),
        });

        let config = Self::create_vfs_config();
        let error = unsafe { esp_vfs_register(c"/dev".as_ptr(), &config, ptr::null_mut()) };
        EspError::check_and_return(error, ()).unwrap();
    }

    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn opendir(&mut self, path: PathBuf) -> NonNull<DIR> {
        log::info!("Opening directory: {}", path.display());

        let handle = Box::new(DIR {
            dd_vfs_idx: 0, // managed by ESP-IDF
            dd_rsv: 0,
        });

        let handleptr = Box::into_non_null(handle);

        self.dirhandles.push(DirHandle {
            ptr: handleptr,
            iter: self.create_dir_iterator(),
        });
        handleptr
    }

    pub(crate) fn closedir(&mut self, handle: NonNull<DIR>) -> Result<(), io::Error> {
        let handle_index = unsafe { handle.as_ref().dd_vfs_idx };
        let olen = self.dirhandles.len();

        log::info!("closedir(handle={handle_index}): deleting handle");
        self.dirhandles
            .retain(|candidate| candidate.idx() != handle_index);

        let nlen = self.dirhandles.len();

        if nlen != (olen - 1) {
            log::error!("closedir(handle): illegal directory handle index");
            return Err(io::Error::other("illegal directory handle index"));
        }

        log::info!("closedir(handle={handle_index}): successfully deleted handle");

        Ok(())
    }

    pub(crate) fn readdir_r(
        &mut self,
        dir: NonNull<DIR>,
        mut entry: NonNull<dirent>,
        out_dirent: NonNull<*mut dirent>,
    ) -> Result<(), io::Error> {
        let dir_handle = unsafe { dir.as_ref() };

        log::info!(
            "readdir_r(handle={}): iterating directory contents",
            dir_handle.dd_vfs_idx
        );

        let Some(handle) = self
            .dirhandles
            .iter_mut()
            .find(|candidate| candidate.idx() == dir_handle.dd_vfs_idx)
        else {
            log::error!(
                "readdir_r(handle={}): illegal handle",
                dir_handle.dd_vfs_idx
            );
            return Err(io::Error::other("illegal handle"));
        };

        let Some(next_item) = handle.iter.next() else {
            log::info!(
                "readdir_r(handle={}): reached end of iterator",
                dir_handle.dd_vfs_idx
            );

            unsafe { *out_dirent.as_ptr() = ptr::null_mut() };

            return Ok(());
        };

        log::info!(
            "readdir_r(handle={}): continuing iteration",
            dir_handle.dd_vfs_idx
        );

        let next_entry = self.find_file_by_inode(next_item).unwrap();

        unsafe {
            entry.as_mut().d_ino = next_entry.d_ino;
            entry.as_mut().d_type = next_entry.d_type;
            entry.as_mut().d_name = next_entry.d_name;

            let out_dirent_ptr = out_dirent.as_ptr();

            (*out_dirent_ptr) = entry.as_mut();
        }
        Ok(())
    }

    #[allow(clippy::needless_collect)]
    fn create_dir_iterator(&self) -> IntoIter<u16> {
        let inodes: Vec<u16> = self.content.iter().map(|entry| entry.d_ino).collect();
        inodes.into_iter()
    }

    fn find_file_by_inode(&self, inode: u16) -> Option<&dirent> {
        self.content
            .iter()
            .find(|candidate| candidate.d_ino == inode)
    }

    fn create_vfs_config() -> esp_vfs_t {
        esp_vfs_t {
            flags: unsafe { i32::try_from(ESP_VFS_FLAG_DEFAULT).unwrap_unchecked() },
            __bindgen_anon_13: OpenDirImpl {
                opendir: Some(Self::_vfs_opendir),
            },
            __bindgen_anon_18: CloseDirImpl {
                closedir: Some(Self::_vfs_closedir),
            },
            __bindgen_anon_15: ReaddirRImpl {
                readdir_r: Some(Self::_vfs_readdir_r),
            },
            ..Default::default()
        }
    }

    unsafe extern "C" fn _vfs_opendir(path: *const u8) -> *mut DIR {
        let path_str = unsafe { CStr::from_ptr(path) };
        let hpath = PathBuf::from_str(path_str.to_str().unwrap()).unwrap();

        let mut result = DEVFS.lock().unwrap().opendir(hpath);
        result.as_mut()
    }

    unsafe extern "C" fn _vfs_closedir(dir: *mut DIR) -> i32 {
        let Some(dirptr) = NonNull::new(dir) else {
            log::error!("_vfs_closedir(dir): dir is NULL");
            return -1;
        };

        let value = DEVFS.lock().unwrap().closedir(dirptr);
        match value {
            Ok(()) => 0,
            Err(why) => why.raw_os_error().unwrap_or(-1),
        }
    }

    unsafe extern "C" fn _vfs_readdir_r(
        dir: *mut DIR,
        entry: *mut dirent,
        result: *mut *mut dirent,
    ) -> i32 {
        let dirptr = NonNull::new(dir).unwrap();
        let entryptr = NonNull::new(entry).unwrap();
        let resultptr = NonNull::new(result).unwrap();

        let error = DEVFS.lock().unwrap().readdir_r(dirptr, entryptr, resultptr);
        match error {
            Ok(()) => 0,
            Err(why) => why.raw_os_error().unwrap_or(-1),
        }
    }
}

fn filename<S: AsRef<str>>(name: S) -> [u8; 256] {
    let name = name.as_ref();
    let cname = CString::new(name).unwrap();
    let mut result = [0; 256];

    for (i, byte) in cname.as_bytes().iter().enumerate() {
        result[i] = *byte;
    }

    assert_eq!(result.last(), Some(&0));

    result
}

pub fn setup() {
    DEVFS.lock().unwrap().setup();
}

impl DirHandle {
    pub const fn dir(&self) -> &DIR {
        unsafe { self.ptr.as_ref() }
    }

    pub const fn idx(&self) -> u16 {
        self.dir().dd_vfs_idx
    }
}

impl Drop for DirHandle {
    fn drop(&mut self) {
        // this will be dropped
        let _boxed = unsafe { Box::from_non_null(self.ptr) };
    }
}

impl fmt::Debug for DirHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entry = unsafe { self.ptr.as_ref() };

        writeln!(f, "DirectoryHandle {{")?;
        writeln!(f, "\tptr.dd_vfs_idx: {}", entry.dd_vfs_idx)?;
        writeln!(f, "\tptr.dd_rsv: {}", entry.dd_rsv)?;
        writeln!(f, "}}")
    }
}

unsafe impl Send for DirHandle {}
unsafe impl Sync for DirHandle {}
