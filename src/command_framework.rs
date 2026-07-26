// The 17 Goldleaf command handlers, ported from CommandFramework.java.

use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::LocalKey;

use crate::app::UsbToUi;
use crate::command_block::{
    CommandBlock, FILE_MODE_APPEND, FILE_MODE_READ, FILE_MODE_WRITE, PATH_TYPE_DIRECTORY,
    PATH_TYPE_FILE, RESULT_EXCEPTION, RESULT_INVALID_FILE_MODE, RESULT_INVALID_INDEX,
    RESULT_SELECTION_CANCELLED,
};
use crate::config::Config;
use crate::filesystem;
use crate::logging::log;
use crate::usb::UsbTransport;

const MAX_BUFFER_SIZE: u64 = 1 << 30;

thread_local! {
    static READ_FILE: RefCell<Option<File>> = const { RefCell::new(None) };
    static WRITE_FILE: RefCell<Option<File>> = const { RefCell::new(None) };
}

fn with_file(file: &'static LocalKey<RefCell<Option<File>>>) -> Option<File> {
    file.with(|c| c.borrow_mut().take())
}

fn set_file(file: &'static LocalKey<RefCell<Option<File>>>, f: Option<File>) {
    file.with(|c| *c.borrow_mut() = f);
}

fn is_valid_file_mode(mode: u32) -> bool {
    matches!(mode, FILE_MODE_READ | FILE_MODE_WRITE | FILE_MODE_APPEND)
}

pub fn handle(
    id: u32,
    block: &mut CommandBlock,
    usb: &mut UsbTransport,
    cfg: &Arc<Mutex<Config>>,
    sender: &Sender<UsbToUi>,
) -> bool {
    match id {
        1 => cmd_get_drive_count(block, usb, sender),
        2 => cmd_get_drive_info(block, usb, sender),
        3 => cmd_stat_path(block, usb, sender),
        4 => cmd_get_file_count(block, usb, sender),
        5 => cmd_get_file(block, usb, sender),
        6 => cmd_get_directory_count(block, usb, sender),
        7 => cmd_get_directory(block, usb, sender),
        8 => cmd_start_file(block, usb, sender),
        9 => cmd_read_file(block, usb, sender),
        10 => cmd_write_file(block, usb, sender),
        11 => cmd_end_file(block, usb, sender),
        12 => cmd_create(block, usb, sender),
        13 => cmd_delete(block, usb, sender),
        14 => cmd_rename(block, usb, sender),
        15 => cmd_get_special_path_count(block, usb, cfg, sender),
        16 => cmd_get_special_path(block, usb, cfg, sender),
        17 => cmd_select_file(block, usb, sender),
        _ => false,
    }
}

// --- Handlers ----------------------------------------------------------------

fn cmd_get_drive_count(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let drives = filesystem::list_drives();
    log(s, &format!("[cf] GetDriveCount() -> count: {}", drives.len()));
    b.response_start();
    b.write_u32(drives.len() as u32);
    b.response_end(u)
}

fn cmd_get_drive_info(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let idx = b.read_u32();
    let drives = filesystem::list_drives();
    if (idx as usize) < drives.len() {
        let drive_path = drives[idx as usize].clone();
        let drive_name = filesystem::drive_label(&drive_path);
        log(
            s,
            &format!(
                "[cf] GetDriveInfo(idx: {idx}) -> path: '{drive_path}', name: '{drive_name}'"
            ),
        );
        b.response_start();
        b.write_string(&drive_name);
        b.write_string(&drive_path);
        b.write_u64(0); // total size (TODO)
        b.write_u64(0); // free size (TODO)
        b.response_end(u)
    } else {
        b.respond_failure(u, RESULT_INVALID_INDEX)
    }
}

fn cmd_stat_path(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let raw = b.read_string();
    let path = filesystem::denormalize_path(&raw);
    let p = Path::new(&path);
    let (path_type, file_size) = match p.metadata() {
        Ok(m) if m.is_file() => (PATH_TYPE_FILE, m.len()),
        Ok(m) if m.is_dir() => (PATH_TYPE_DIRECTORY, 0u64),
        _ => (0u32, 0u64),
    };
    log(
        s,
        &format!("[cf] StatPath(path: '{path}') -> path_type: {path_type}, file_size: {file_size}"),
    );
    b.response_start();
    b.write_u32(path_type);
    b.write_u64(file_size);
    b.response_end(u)
}

fn cmd_get_file_count(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let path = filesystem::denormalize_path(&b.read_string());
    let files = filesystem::get_files_in(&path);
    log(s, &format!("[cf] GetFileCount(path: '{path}') -> count: {}", files.len()));
    b.response_start();
    b.write_u32(files.len() as u32);
    b.response_end(u)
}

fn cmd_get_file(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let path = filesystem::denormalize_path(&b.read_string());
    let idx = b.read_u32();
    let files = filesystem::get_files_in(&path);
    if (idx as usize) < files.len() {
        let file = files[idx as usize].clone();
        log(s, &format!("[cf] GetFile(path: '{path}', idx: {idx}) -> file: '{file}'"));
        b.response_start();
        b.write_string(&file);
        b.response_end(u)
    } else {
        b.respond_failure(u, RESULT_INVALID_INDEX)
    }
}

fn cmd_get_directory_count(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let path = filesystem::denormalize_path(&b.read_string());
    let dirs = filesystem::get_directories_in(&path);
    log(s, &format!("[cf] GetDirectoryCount(path: '{path}') -> count: {}", dirs.len()));
    b.response_start();
    b.write_u32(dirs.len() as u32);
    b.response_end(u)
}

fn cmd_get_directory(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let path = filesystem::denormalize_path(&b.read_string());
    let idx = b.read_u32();
    let dirs = filesystem::get_directories_in(&path);
    if (idx as usize) < dirs.len() {
        let dir = dirs[idx as usize].clone();
        log(s, &format!("[cf] GetDirectory(path: '{path}', idx: {idx}) -> dir: '{dir}'"));
        b.response_start();
        b.write_string(&dir);
        b.response_end(u)
    } else {
        b.respond_failure(u, RESULT_INVALID_INDEX)
    }
}

fn cmd_start_file(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let path = filesystem::denormalize_path(&b.read_string());
    let mode = b.read_u32();
    log(s, &format!("[cf] StartFile(path: '{path}', mode: {mode})"));
    if !is_valid_file_mode(mode) {
        return b.respond_failure(u, RESULT_INVALID_FILE_MODE);
    }
    let res: std::io::Result<()> = (|| {
        match mode {
            FILE_MODE_READ => {
                READ_FILE.with(|c| *c.borrow_mut() = None);
                let f = OpenOptions::new().read(true).open(&path)?;
                READ_FILE.with(|c| *c.borrow_mut() = Some(f));
            }
            FILE_MODE_WRITE => {
                WRITE_FILE.with(|c| *c.borrow_mut() = None);
                let f = OpenOptions::new().write(true).create(true).truncate(true).open(&path)?;
                WRITE_FILE.with(|c| *c.borrow_mut() = Some(f));
            }
            FILE_MODE_APPEND => {
                let f = OpenOptions::new().write(true).create(true).append(true).open(&path)?;
                WRITE_FILE.with(|c| *c.borrow_mut() = Some(f));
            }
            _ => unreachable!(),
        }
        Ok(())
    })();
    match res {
        Ok(()) => b.respond_empty(u),
        Err(_) => b.respond_failure(u, RESULT_EXCEPTION),
    }
}

fn cmd_read_file(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let path = filesystem::denormalize_path(&b.read_string());
    let offset = b.read_u64();
    let size = b.read_u64();
    if size > MAX_BUFFER_SIZE {
        return b.respond_failure(u, RESULT_EXCEPTION);
    }
    let res: std::io::Result<(u64, Vec<u8>)> = (|| {
        let mut is_own = false;
        let mut file = match with_file(&READ_FILE) {
            Some(f) => f,
            None => {
                is_own = true;
                OpenOptions::new().read(true).open(&path)?
            }
        };
        file.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0u8; size as usize];
        let n = file.read(&mut buf)?;
        if !is_own {
            set_file(&READ_FILE, Some(file));
        }
        Ok((n as u64, buf))
    })();
    match res {
        Ok((n, data)) => {
            log(s, &format!("[cf] ReadFile(path: '{path}', offset: {offset}, size: {size}) -> read_size: {n}"));
            b.response_start();
            b.write_u64(n);
            if !b.response_end(u) {
                return false;
            }
            b.send_buffer(u, &data)
        }
        Err(_) => b.respond_failure(u, RESULT_EXCEPTION),
    }
}

fn cmd_write_file(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let path = filesystem::denormalize_path(&b.read_string());
    let size = b.read_u64();
    if size > MAX_BUFFER_SIZE {
        return b.respond_failure(u, RESULT_EXCEPTION);
    }
    let data = match b.get_buffer(u, size as usize) {
        Some(d) => d,
        None => return b.respond_failure(u, RESULT_EXCEPTION),
    };
    let res: std::io::Result<()> = (|| {
        let mut is_own = false;
        let mut file = match with_file(&WRITE_FILE) {
            Some(f) => f,
            None => {
                is_own = true;
                let f = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&path)?;
                f
            }
        };
        file.write_all(&data)?;
        if !is_own {
            set_file(&WRITE_FILE, Some(file));
        }
        Ok(())
    })();
    match res {
        Ok(()) => {
            log(s, &format!("[cf] WriteFile(path: '{path}', size: {size}) -> written_size: {size}"));
            b.response_start();
            b.write_u64(size);
            b.response_end(u)
        }
        Err(_) => b.respond_failure(u, RESULT_EXCEPTION),
    }
}

fn cmd_end_file(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let mode = b.read_u32();
    log(s, &format!("[cf] EndFile(mode: {mode})"));
    if !is_valid_file_mode(mode) {
        return b.respond_failure(u, RESULT_INVALID_FILE_MODE);
    }
    if mode == FILE_MODE_READ {
        set_file(&READ_FILE, None);
    } else {
        set_file(&WRITE_FILE, None);
    }
    b.respond_empty(u)
}

fn cmd_create(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let path = filesystem::denormalize_path(&b.read_string());
    let path_type = b.read_u32();
    log(s, &format!("[cf] Create(path: '{path}', path_type: {path_type})"));
    let p = Path::new(&path);
    let res: std::io::Result<()> = match path_type {
        PATH_TYPE_FILE => OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(p)
            .map(|_| ()),
        PATH_TYPE_DIRECTORY => std::fs::create_dir(p),
        _ => Ok(()),
    };
    match res {
        Ok(()) => b.respond_empty(u),
        Err(_) => b.respond_failure(u, RESULT_EXCEPTION),
    }
}

fn cmd_delete(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let path = filesystem::denormalize_path(&b.read_string());
    log(s, &format!("[cf] Delete(path: '{path}')"));
    filesystem::delete_path(Path::new(&path));
    b.respond_empty(u)
}

fn cmd_rename(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let path = filesystem::denormalize_path(&b.read_string());
    let new_name = filesystem::denormalize_path(&b.read_string());
    log(s, &format!("[cf] Rename(path: '{path}', new_name: '{new_name}')"));
    let p = Path::new(&path);
    let dest = match p.parent() {
        Some(parent) => parent.join(&new_name),
        None => PathBuf::from(&new_name),
    };
    match std::fs::rename(p, &dest) {
        Ok(()) => b.respond_empty(u),
        Err(_) => b.respond_failure(u, RESULT_EXCEPTION),
    }
}

fn cmd_get_special_path_count(
    b: &mut CommandBlock,
    u: &mut UsbTransport,
    cfg: &Arc<Mutex<Config>>,
    s: &Sender<UsbToUi>,
) -> bool {
    let count = cfg.lock().map(|c| c.paths.len()).unwrap_or(0) as u32;
    log(s, &format!("[cf] GetSpecialPathCount() -> count: {count}"));
    b.response_start();
    b.write_u32(count);
    b.response_end(u)
}

fn cmd_get_special_path(
    b: &mut CommandBlock,
    u: &mut UsbTransport,
    cfg: &Arc<Mutex<Config>>,
    s: &Sender<UsbToUi>,
) -> bool {
    let idx = b.read_u32();
    let entries = cfg.lock().map(|c| c.entries()).unwrap_or_default();
    if (idx as usize) < entries.len() {
        let (name, path) = entries[idx as usize].clone();
        let normalized = filesystem::normalize_path(&path);
        log(
            s,
            &format!("[cf] GetSpecialPath(idx: {idx}) -> name: '{name}', path: '{normalized}'"),
        );
        b.response_start();
        b.write_string(&name);
        b.write_string(&normalized);
        b.response_end(u)
    } else {
        log(s, &format!("[cf] GetSpecialPath(idx: {idx}) -> invalid index"));
        b.respond_failure(u, RESULT_INVALID_INDEX)
    }
}

fn cmd_select_file(b: &mut CommandBlock, u: &mut UsbTransport, s: &Sender<UsbToUi>) -> bool {
    let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
    let _ = s.send(UsbToUi::PickFile(tx));
    match rx.recv() {
        Ok(Some(path)) => {
            let s_path = path.to_string_lossy().to_string();
            let normalized = filesystem::normalize_path(&s_path);
            log(s, &format!("[cf] SelectFile() -> path: '{normalized}'"));
            b.response_start();
            b.write_string(&normalized);
            b.response_end(u)
        }
        _ => {
            log(s, "[cf] SelectFile() -> cancelled");
            b.respond_failure(u, RESULT_SELECTION_CANCELLED)
        }
    }
}