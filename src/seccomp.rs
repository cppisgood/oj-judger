use std::error::Error;

use syscallz::{Context, Syscall};

pub fn syscall_limit<'a>(allowed_syscalls: &'a [Syscall]) -> Result<(), Box<dyn Error>> {
    let mut ctx = Context::init()?;
    for syscall in allowed_syscalls.iter() {
        ctx.allow_syscall(*syscall)?;
    }

    ctx.load()?;
    Ok(())
}
