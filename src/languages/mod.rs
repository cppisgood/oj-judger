mod cpp;
pub use cpp::Cpp;

use syscallz::Syscall;

pub trait Language {
    fn allowed_syscalls() -> &'static [Syscall];
}
