use std::{
    ffi::CString,
    fs::File,
    os::unix::io::AsRawFd,
    thread::sleep,
    time::{Duration, SystemTime},
};

use ipc_channel::ipc;
use libc;
use nix::{
    sys::wait::{waitpid, WaitStatus},
    unistd::{dup2, execve, fork, setuid, ForkResult, Uid},
};
use oj_judger::{
    cgroups::Cgroup,
    config::get_config,
    os::chroot,
    seccomp, timer,
};

fn main() {
    let now = SystemTime::now();
    let (tx, rx) = ipc::channel().unwrap();

    match unsafe { fork().expect("fork error") } {
        ForkResult::Child => {
            // let fd = File::open("data/1000/1.in").unwrap();
            // dup2(fd.as_raw_fd(), std::io::stdin().as_raw_fd()).unwrap();
            // let fd = File::create("data/1000/1.out").unwrap();
            // dup2(fd.as_raw_fd(), std::io::stdout().as_raw_fd()).unwrap();
            rx.recv().unwrap();

            // setuid(Uid::from_raw(1002)).expect("setuid failed");
            let args = [CString::new("/").unwrap()];
            // seccomp::syscall_limit("cpp");
            execve::<CString, CString>(
                CString::new("/usr/bin/ls")
                    .expect("new CString failed")
                    .as_c_str(),
                &args,
                &[],
            )
            .expect("g execve failed");
        }
        ForkResult::Parent { child } => {
            let cg = Cgroup::new(Some(1024 * 1024 * 1024 / 4), Some(1));
            cg.add_task(child.as_raw() as u64).expect("add task failed");

            tx.send(true).unwrap();

            match waitpid(child, None) {
                Ok(status) => match status {
                    WaitStatus::Exited(_, exit_code) => match exit_code {
                        0 => println!("exit success with code {}", exit_code),
                        _ => {
                            println!("runtime error");
                        }
                    },
                    other => {
                        println!("exit with {:?}", other);
                    }
                },
                Err(e) => {
                    println!("wait failed {}", e);
                }
            }

            // let exec_time = now.elapsed().unwrap().as_millis();
            // println!("{}", exec_time);
        }
    }
}
