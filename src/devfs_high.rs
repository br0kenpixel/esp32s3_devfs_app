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

const FILETYPE_REGULAR: u8 = DT_REG as _;

static DEVFS: Mutex<DevFs> = Mutex::new(DevFs::new());

type OpenDirImpl = esp_vfs_t__bindgen_ty_13;
type CloseDirImpl = esp_vfs_t__bindgen_ty_18;
type ReaddirRImpl = esp_vfs_t__bindgen_ty_15;

pub struct DirHandle {
    ptr: NonNull<DIR>,
    iter: IntoIter<dirent>,
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

        let new_handle_id = self
            .dirhandles
            .last()
            .map_or(1, |handle| unsafe { handle.ptr.as_ref() }.dd_vfs_idx + 1);

        log::info!("opendir(): creating new handle with index {new_handle_id}");

        let handle = Box::new(DIR {
            dd_vfs_idx: new_handle_id,
            dd_rsv: 0,
        });

        let handleptr = Box::into_non_null(handle);

        self.dirhandles.push(DirHandle {
            ptr: handleptr,
            iter: self.content.clone().into_iter(),
        });
        handleptr
    }

    pub(crate) fn closedir(&mut self, handle: NonNull<DIR>) -> Result<(), io::Error> {
        let handle_index = unsafe { handle.as_ref().dd_vfs_idx };
        let olen = self.dirhandles.len();

        log::info!("closedir(handle={handle_index}): deleting handle");

        dbg!(&self.dirhandles);
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

    fn create_vfs_config() -> esp_vfs_t {
        esp_vfs_t {
            flags: unsafe { i32::try_from(ESP_VFS_FLAG_DEFAULT).unwrap_unchecked() },
            __bindgen_anon_13: OpenDirImpl {
                opendir: Some(Self::_vfs_opendir),
            },
            __bindgen_anon_18: CloseDirImpl {
                closedir: Some(Self::_vfs_closedir),
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
