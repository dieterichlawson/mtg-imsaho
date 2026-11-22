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
