use syscallz::Syscall;

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
    Syscall::clone,  // TODO
    Syscall::getpid, // TODO
];
