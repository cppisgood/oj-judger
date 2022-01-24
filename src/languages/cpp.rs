use crate::languages::Language;
use syscallz::Syscall;

pub struct Cpp;

pub const ALLOWED_SYSCALLS: &[Syscall] = &[
    Syscall::read,
    Syscall::write,
    Syscall::access,
    Syscall::futex,
    Syscall::mmap,
    Syscall::rt_sigprocmask,
    Syscall::execve, // TODO
    Syscall::brk,
    Syscall::arch_prctl,
    Syscall::newfstatat,
    Syscall::close,
    Syscall::pread64,
    Syscall::mprotect,
    Syscall::munmap,
    Syscall::exit_group,
    Syscall::set_tid_address, // ?
    Syscall::set_robust_list, // ?
    Syscall::rt_sigaction,    // ?
    Syscall::prlimit64,       // ?
    Syscall::lseek,
    Syscall::clock_gettime,
    Syscall::openat, // TODO
    Syscall::readlink,
    Syscall::getpid, // fk java
    Syscall::clone, // fk java
    Syscall::gettimeofday, // fk java
    Syscall::getdents64, // fk java
    Syscall::sysinfo, // fk java
    Syscall::sched_getaffinity, // fk java
    Syscall::clock_getres, // fk java
    Syscall::geteuid, // fk java
    Syscall::socket, // fk java
    Syscall::connect, // fk java
    Syscall::gettid, // fk java
    Syscall::rt_sigreturn, // fk java
    Syscall::fcntl, // fk java
    Syscall::prctl, // fk java
    Syscall::uname, // fk java
    Syscall::ioctl, // fk java
    Syscall::getuid, // fk java
    Syscall::getcwd, // fk java
    Syscall::faccessat2, // fk java
    Syscall::madvise, // fk java
    Syscall::getrusage, // fk java
    Syscall::exit, // fk java
];

impl Language for Cpp {
    fn allowed_syscalls() -> &'static [Syscall] {
        ALLOWED_SYSCALLS
    }
}
