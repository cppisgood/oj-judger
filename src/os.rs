use std::env;
use std::error::Error;
use std::os::unix::fs as osfs;

pub fn chroot(path: &str) -> Result<(), Box<dyn Error>> {
    osfs::chroot(path)?;
    env::set_current_dir("/")?;
    Ok(())
}
