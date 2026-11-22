//! Files a run keeps by writing a temp sibling and renaming it over the
//! path: a game's `--save`, the hot-reload snapshot, a draft's snapshot.

use std::path::Path;

/// Refuse a path whose rename would replace something that is not a file.
///
/// `rename(2)` replaces whatever inode is at the path. That is right for a
/// regular file, but a FIFO became a 0600 file holding the save, and run
/// as root `--save /dev/full` turned the box's `/dev/full` into one (#1079).
/// Nothing at the path, a regular file and a symlink (renamed over as a
/// link, or resolved first by the caller) are fine.
///
/// # Errors
/// What is at the path, when it is not one of those.
pub fn check_replaceable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::FileTypeExt;
    let Ok(meta) = std::fs::symlink_metadata(path) else { return Ok(()) };
    let kind = meta.file_type();
    let what = if kind.is_file() || kind.is_symlink() {
        return Ok(());
    } else if kind.is_dir() {
        "Is a directory"
    } else if kind.is_fifo() {
        "is a FIFO, not a regular file; a save would replace it"
    } else if kind.is_char_device() || kind.is_block_device() {
        "is a device, not a regular file; a save would replace it"
    } else if kind.is_socket() {
        "is a socket, not a regular file; a save would replace it"
    } else {
        "is not a regular file; a save would replace it"
    };
    Err(std::io::Error::other(what))
}

/// Resolve a `--save` path that is a symlink to the file it points at.
///
/// mtg-runner's since #215; the draft runner's and the hosted table's
/// snapshot writes replaced the link the same way (#1079's review).
///
/// The atomic write renames onto the path, and `rename(2)` does not follow
/// a final symlink: every save landed *next to* the link and destroyed the
/// link on the first write, while the target the operator pointed at stayed
/// empty. `--log`, which opens rather than renames, followed the link as
/// anyone would expect (issue #215). Following it here restores the
/// atomicity guarantee (the rename happens in the target's own directory)
/// and makes the startup writability probe test the directory that will
/// actually be written.
pub fn resolve_symlink(flag: &str, path: &str) -> String {
    use std::path::PathBuf;
    let mut current = PathBuf::from(path);
    // Bounded, so a symlink loop is a refusal rather than a hang.
    for _ in 0..8 {
        let Ok(meta) = std::fs::symlink_metadata(&current) else { break };
        if !meta.file_type().is_symlink() {
            break;
        }
        let Ok(target) = std::fs::read_link(&current) else { break };
        current = if target.is_absolute() {
            target
        } else {
            current.parent().unwrap_or(Path::new(".")).join(target)
        };
    }
    let resolved = current.to_string_lossy().into_owned();
    if resolved != path {
        crate::stderr_line!("note: {flag} '{path}' is a symlink; writing through it to '{resolved}'");
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::check_replaceable;

    #[test]
    fn only_a_file_a_link_or_nothing_is_replaceable() {
        let dir = std::env::temp_dir().join(format!("mtg-kept-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("save.json");
        assert!(check_replaceable(&file).is_ok(), "nothing there yet");
        std::fs::write(&file, "{}").unwrap();
        assert!(check_replaceable(&file).is_ok(), "a regular file");
        assert_eq!(check_replaceable(&dir).unwrap_err().to_string(), "Is a directory");
        let dev = check_replaceable(std::path::Path::new("/dev/null")).unwrap_err().to_string();
        assert!(dev.contains("is a device"), "{dev}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
