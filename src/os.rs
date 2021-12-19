use std::env;
use std::os::unix::fs as osfs;

pub fn chroot() {
    osfs::chroot("/home/zmm/fakeroot").unwrap();
    env::set_current_dir("/").unwrap();
}
