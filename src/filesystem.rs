// Filesystem helpers ported from xortroll.goldleaf.quark.fs.FileSystem.
// Made Mac-correct: drives = home directory + mounted volumes under /Volumes.

use std::fs;
use std::path::Path;

pub fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
}

pub fn is_windows() -> bool {
    cfg!(windows)
}

pub fn list_drives() -> Vec<String> {
    let mut drives = vec![home_dir()];

    if is_windows() {
        // Mirror the Java behaviour: enumerate letter roots on Windows.
        for c in b'A'..=b'Z' {
            let root = format!("{}:\\", c as char);
            if Path::new(&root).exists() {
                drives.push((c as char).to_string());
            }
        }
    } else if cfg!(target_os = "macos") {
        drives.push("/".to_string());
        if let Ok(entries) = fs::read_dir("/Volumes") {
            for e in entries.flatten() {
                let name = e.file_name();
                let name = name.to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                drives.push(format!("/Volumes/{name}"));
            }
        }
    } else {
        // Linux/others: best-effort enumeration of real block-device mounts via
        // /proc/mounts. Only mounts whose source is a /dev/ node are listed, so
        // virtual filesystems (proc, sysfs, tmpfs, ...) are skipped. Matches the
        // Java Quark behaviour which filtered on FileStore.name().startsWith("/dev/").
        if let Ok(text) = fs::read_to_string("/proc/mounts") {
            for line in text.lines() {
                let mut parts = line.split_whitespace();
                let source = parts.next().unwrap_or("");
                if let Some(mount) = parts.next() {
                    if source.starts_with("/dev/") && !mount.is_empty() {
                        drives.push(mount.to_string());
                    }
                }
            }
        }
    }
    drives
}

pub fn drive_label(drive: &str) -> String {
    let home = home_dir();
    if drive == home {
        return "Home directory".to_string();
    }

    if is_windows() {
        return format!("Drive ({})", drive);
    }

    if drive == "/" {
        return "Root directory".to_string();
    }

    Path::new(drive)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("Drive ({drive})"))
}

pub fn get_files_in(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(path) {
        for e in rd.flatten() {
            if let Ok(md) = e.metadata() {
                if md.is_file() {
                    out.push(e.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    out
}

pub fn get_directories_in(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(path) {
        for e in rd.flatten() {
            if let Ok(md) = e.metadata() {
                if md.is_dir() {
                    out.push(e.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    out
}

// Path normalization shared with the Switch side. On non-Windows the protocol
// uses ':' as a separator that gets expanded back to '/'.
pub fn normalize_path(path: &str) -> String {
    let s = path.replace('\\', "/");
    if is_windows() {
        s.replace("//", ":")
    } else {
        s.replace("//", "/")
    }
}

pub fn denormalize_path(path: &str) -> String {
    if is_windows() {
        path.replace('/', "\\")
    } else {
        path.replace(':', "/")
    }
}

pub fn delete_path(path: &Path) {
    if path.is_dir() {
        if let Ok(rd) = fs::read_dir(path) {
            for e in rd.flatten() {
                delete_path(&e.path());
            }
        }
    }
    let _ = fs::remove_file(path);
}