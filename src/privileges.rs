use nix::unistd::Uid;
use std::io;

pub fn require_root() -> io::Result<()> {
    if Uid::effective().is_root() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "This command must be run with sudo/root.",
        ))
    }
}
