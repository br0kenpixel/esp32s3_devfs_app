// sources:
// - https://ftp.gnu.org/old-gnu/Manuals/glibc-2.2.3/html_node/libc_260.html

#![feature(box_vec_non_null)]

use std::fs;

mod devfs;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    log::info!("Registering devfs");
    devfs::setup();

    list_devices();

    null_device();
}

fn null_device() {
    let file = fs::OpenOptions::new().read(true).open("/dev/null");

    match file {
        Ok(file) => {
            todo!()
        }
        Err(why) => {
            log::error!("Failed to open /dev/null: {why}");
        }
    }
}

fn list_devices() {
    let files = fs::read_dir("/dev");

    match files {
        Ok(dir) => {
            log::info!("Directory read successfull, listing files");

            let mut dir = dir.peekable();

            if dir.peek().is_none() {
                log::warn!("No files to list");
            }

            for item in dir {
                match item {
                    Ok(entry) => {
                        log::info!("Entry: {entry:?}");
                    }
                    Err(why) => {
                        log::error!("Failed to read entry: {why}");
                    }
                }
            }

            log::info!("Done listing files");
        }
        Err(why) => {
            log::error!("Failed to list /dev: {why}");
        }
    }
}
