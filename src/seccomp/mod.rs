use syscallz::{Action, Context, Syscall};

pub mod cpp;

pub fn syscall_limit(language: &str) {
    let allowed_syscalls = match language {
        "cpp" => cpp::ALLOWED_SYSCALLS,
        _ => {
            panic!()
        }
    };

    let mut ctx = Context::init().expect("context init failed");
    for syscall in allowed_syscalls.iter() {
        ctx.allow_syscall(*syscall).expect("allow syscall failed");
    }

    ctx.load().expect("context load failed");
}
