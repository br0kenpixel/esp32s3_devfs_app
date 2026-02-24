use core::ffi::{c_char, c_int};
use esp_idf_svc::sys::{
    dirent, esp_vfs_register, esp_vfs_t, esp_vfs_t__bindgen_ty_1, esp_vfs_t__bindgen_ty_13,
    esp_vfs_t__bindgen_ty_14, esp_vfs_t__bindgen_ty_15, esp_vfs_t__bindgen_ty_18,
    esp_vfs_t__bindgen_ty_3, esp_vfs_t__bindgen_ty_6, esp_vfs_t__bindgen_ty_7,
    esp_vfs_t__bindgen_ty_8, stat, EspError, DIR, DT_REG, ESP_VFS_FLAG_DEFAULT,
};
use std::{ffi::CStr, ptr};

unsafe extern "C" fn vfs_open(path: *const c_char, flags: c_int, mode: c_int) -> c_int {
    log::info!(
        "vfs_open(): path={:?}, flags={flags}, mode={mode}",
        unsafe { CStr::from_ptr(path) }
    );
    0
}

unsafe extern "C" fn vfs_write(fd: i32, data: *const std::ffi::c_void, len: usize) -> isize {
    log::info!("vfs_write(): fd={fd}, data_len={len}");
    0
}

unsafe extern "C" fn vfs_read(fd: c_int, dst: *mut core::ffi::c_void, size: usize) -> isize {
    log::info!("vfs_read(): fd={fd}, size={size}");
    0
}

unsafe extern "C" fn vfs_close(fd: c_int) -> c_int {
    log::info!("vfs_close(): fd={fd}");
    0
}

unsafe extern "C" fn vfs_fstat(fd: c_int, stat: *mut stat) -> i32 {
    log::info!("vfs_fstat(): fd={fd}, stat={stat:?}");
    0
}

/*unsafe extern "C" fn vfs_readdir(dir: *mut esp_idf_svc::sys::DIR) -> *mut esp_idf_svc::sys::dirent {
    log::info!(
        "vfs_readdir(): dd_vfs_idx={}, dd_rsv={}",
        (*dir).dd_vfs_idx,
        (*dir).dd_rsv
    );

    if (*dir).dd_vfs_idx != 0 {
        return ptr::null_mut();
    }

    let entries = vec![
        dirent {
            d_ino: 0,
            d_type: 0,
            d_name: filename("test"),
        },
        dirent {
            d_ino: 1,
            d_type: 0,
            d_name: filename("test2"),
        },
    ];

    entries.leak().as_mut_ptr()
}*/

unsafe extern "C" fn vfs_readdir_r(
    dirp: *mut esp_idf_svc::sys::DIR,
    entry: *mut dirent,
    result: *mut *mut dirent,
) -> i32 {
    let ent_name = CStr::from_bytes_until_nul(&((*entry).d_name)).unwrap();

    log::info!(
        "vfs_readdir_r(): dir.dd_vfs_idx={}, dir.dd_rsv={}, ent.d_ino={}, ent.d_type={}, ent.d_name={:?}",
        (*dirp).dd_vfs_idx,
        (*dirp).dd_rsv,
        (*entry).d_ino,
        (*entry).d_type,
        ent_name
    );

    if !ent_name.is_empty() {
        *result = ptr::null_mut();
        return 0;
    }

    0
}

unsafe extern "C" fn vfs_opendir(dir: *const u8) -> *mut DIR {
    log::info!("vfs_opendir()");

    let result = Box::new(DIR {
        dd_vfs_idx: 0,
        dd_rsv: 0,
    });

    Box::leak(result)
}

unsafe extern "C" fn vfs_closedir(dir: *mut DIR) -> i32 {
    log::info!("vfs_closedir()");

    let _ = Box::from_raw(dir);
    0
}

pub fn setup() {
    let vfs = esp_vfs_t {
        flags: ESP_VFS_FLAG_DEFAULT as i32,
        __bindgen_anon_1: esp_vfs_t__bindgen_ty_1 {
            write: Some(vfs_write),
        },
        __bindgen_anon_3: esp_vfs_t__bindgen_ty_3 {
            read: Some(vfs_read),
        },
        __bindgen_anon_6: esp_vfs_t__bindgen_ty_6 {
            open: Some(vfs_open),
        },
        __bindgen_anon_7: esp_vfs_t__bindgen_ty_7 {
            close: Some(vfs_close),
        },
        __bindgen_anon_8: esp_vfs_t__bindgen_ty_8 {
            fstat: Some(vfs_fstat),
        },
        //__bindgen_anon_14: esp_vfs_t__bindgen_ty_14 {
        //    readdir: Some(vfs_readdir),
        //},
        __bindgen_anon_13: esp_vfs_t__bindgen_ty_13 {
            opendir: Some(vfs_opendir),
        },
        __bindgen_anon_18: esp_vfs_t__bindgen_ty_18 {
            closedir: Some(vfs_closedir),
        },
        __bindgen_anon_15: esp_vfs_t__bindgen_ty_15 {
            readdir_r: Some(vfs_readdir_r),
        },
        ..Default::default()
    };

    let error = unsafe { esp_vfs_register(c"/dev".as_ptr(), &vfs, ptr::null_mut()) };

    EspError::check_and_return(error, ()).unwrap();
}

fn filename(name: &str) -> [u8; 256] {
    let mut buffer = [0; 256];

    for (i, byte) in name.bytes().enumerate() {
        buffer[i] = byte;
    }

    buffer
}
