use ipc_channel::ipc;
use nix::{
    sys::resource::{self, Resource},
    unistd::{self, ForkResult, Pid, Uid}, sched::{self, CloneFlags},
};
use std::{
    error::Error,
    ffi::CString,
    io,
    mem::MaybeUninit,
    os::unix::prelude::AsRawFd,
    process, thread,
    time::{Duration, SystemTime}, env,
};
use syscallz::Syscall;
use tracing::debug;

use crate::{cgroups::Cgroup, os, seccomp};

use libc;

const SIGSYS_EXIT_CODE: i32 = 159;
const SUCCESS_EXIT_CODE: i32 = 0;

#[derive(Debug)]
pub struct RunOption<'a> {
    pub cmd: &'a str,

    pub args: Option<Vec<&'a str>>,
    pub jail_path: Option<&'a str>,
    pub exec_path: Option<&'a str>,
    pub uid: Option<u32>,
    pub process_limit: Option<u32>,
    pub memory_limit: Option<u64>,    // kbyte
    pub cpu_time_limit: Option<u64>,  // ms
    pub real_time_limit: Option<u64>, // ms
    pub syscall_limit: Option<&'a [Syscall]>,
    pub stdin_redirect: Option<u32>,  // raw file descriptor
    pub stdout_redirect: Option<u32>, // raw file descriptor
}

impl<'a> RunOption<'a> {
    fn default(cmd: &'a str) -> Self {
        Self {
            cmd,
            args: None,
            jail_path: None,
            exec_path: None,
            uid: None,
            process_limit: None,
            memory_limit: None,
            cpu_time_limit: None,
            real_time_limit: None,
            syscall_limit: None,
            stdin_redirect: None,
            stdout_redirect: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExecResult {
    Ok,
    CpuTimeLimitExceeded,
    RealTimeLimitExceeded,
    MemoryLimitExceeded,
    SyscallLimitExceeded,
    RuntimeError,
}

#[derive(Debug)]
pub struct RunResult {
    pub result: ExecResult,
    pub exit_code: u32,
    pub cpu_time: u64,
    pub real_time: u64,
    pub memory: u64,
}

impl RunResult {
    pub fn new(
        result: ExecResult,
        exit_code: u32,
        cpu_time: u64,
        real_time: u64,
        memory: u64,
    ) -> Self {
        Self {
            result,
            exit_code,
            real_time,
            cpu_time,
            memory,
        }
    }

    pub fn default() -> Self {
        Self {
            result: ExecResult::Ok,
            exit_code: 0,
            real_time: 0,
            cpu_time: 0,
            memory: 0,
        }
    }
}
pub struct Command<'a> {
    pub option: RunOption<'a>,
    pub result: Option<RunResult>,
}

impl<'a> Command<'a> {
    pub fn new(cmd: &'a str) -> Self {
        Self {
            option: RunOption::default(cmd),
            result: None,
        }
    }

    pub fn args(&mut self, args: Vec<&'a str>) -> &mut Self {
        self.option.args = Some(args);
        self
    }

    pub fn uid(&mut self, uid: u32) -> &mut Self {
        self.option.uid = Some(uid);
        self
    }

    pub fn jail_path(&mut self, jail_path: &'a str) -> &mut Self {
        self.option.jail_path = Some(jail_path);
        self
    }

    pub fn exec_path(&mut self, exec_path: &'a str) -> &mut Self {
        self.option.exec_path = Some(exec_path);
        self
    }

    pub fn process(&mut self, process: u32) -> &mut Self {
        self.option.process_limit = Some(process);
        self
    }

    pub fn memory(&mut self, memory: u64) -> &mut Self {
        self.option.memory_limit = Some(memory);
        self
    }

    pub fn cpu_time(&mut self, cpu_time: u64) -> &mut Self {
        self.option.cpu_time_limit = Some(cpu_time);
        self
    }

    pub fn real_time(&mut self, real_time: u64) -> &mut Self {
        self.option.real_time_limit = Some(real_time);
        self
    }

    pub fn syscall(&mut self, syscall: &'a [Syscall]) -> &mut Self {
        self.option.syscall_limit = Some(syscall);
        self
    }

    pub fn stdin(&mut self, fd: u32) -> &mut Self {
        self.option.stdin_redirect = Some(fd);
        self
    }

    pub fn stdout(&mut self, fd: u32) -> &mut Self {
        self.option.stdout_redirect = Some(fd);
        self
    }

    pub fn option(&mut self, option: RunOption<'a>) -> &mut Self {
        self.option = option;
        self
    }

    pub fn run(&mut self) -> Result<RunResult, Box<dyn Error>> {
        let (tx_cgroup, rx_cgroup) = ipc::channel()?;
        match unsafe { unistd::fork()? } {
            ForkResult::Child => {
                // new a network namspace for child process
                sched::unshare(CloneFlags::CLONE_NEWNET)?;

                unistd::setpgid(Pid::from_raw(0), Pid::from_raw(0))?;
                if let Some(fd) = self.option.stdin_redirect {
                    unistd::dup2(fd as i32, io::stdin().as_raw_fd())?;
                }
                if let Some(fd) = self.option.stdout_redirect {
                    unistd::dup2(fd as i32, io::stdout().as_raw_fd())?;
                }

                rx_cgroup.recv().unwrap(); // wait for parent create cgroup

                if let Some(jail_path) = &self.option.jail_path {
                    os::chroot(jail_path)?;
                }
                if let Some(exec_path) = &self.option.exec_path {
                    env::set_current_dir(exec_path)?;
                }
                if let Some(uid) = self.option.uid {
                    unistd::setuid(Uid::from_raw(uid))?;
                }
                if let Some(syscalls) = self.option.syscall_limit {
                    seccomp::syscall_limit(syscalls)?;
                }
                if let Some(cpu_time) = self.option.cpu_time_limit {
                    let time = Some((cpu_time / 1000).max(1));
                    resource::setrlimit(Resource::RLIMIT_CPU, time, time)?;
                }
                    let args = &mut match &self.option.args {
                        Some(args) => args
                            .into_iter()
                            .map(|s| CString::new(*s).unwrap())
                            .collect::<Vec<CString>>(),
                        None => vec![],
                    };
                unistd::execv::<CString>(CString::new(self.option.cmd)?.as_c_str(), &args)?;
            }
            ForkResult::Parent { child } => {
                debug!("{}", child);
                let cg = {
                    let memory_limit = match self.option.memory_limit {
                        Some(memory) => Some(memory * 1024),
                        None => None,
                    };

                    let cg = Cgroup::new(memory_limit, self.option.process_limit);
                    cg.add_task(child.as_raw() as u64).expect("add task failed");
                    tx_cgroup.send(true)?;
                    cg
                };

                if let Some(real_time) = self.option.real_time_limit {
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(real_time));
                        process::Command::new("kill")
                            .args(["-9", "--", &format!("-{}", child.as_raw())])
                            .output()
                            // .spawn()
                            .unwrap();
                    });
                }

                let now = SystemTime::now();
                let (status, usage) = unsafe {
                    let mut status = MaybeUninit::uninit();
                    let mut usage = MaybeUninit::uninit();
                    libc::wait4(child.as_raw(), status.as_mut_ptr(), 0, usage.as_mut_ptr());
                    (status.assume_init(), usage.assume_init())
                };
                debug!("{:?}", usage);
                let real_time = now.elapsed()?.as_millis() as u64;
                let cpu_time =
                    (usage.ru_utime.tv_sec * 1000 + usage.ru_utime.tv_usec / 1000) as u64;
                let memory = usage.ru_maxrss as u64;
                let mut res = ExecResult::Ok;

                if status != SUCCESS_EXIT_CODE {
                    res = ExecResult::RuntimeError;
                }
                if cg.oom_killed() {
                    res = ExecResult::MemoryLimitExceeded;
                }
                if let Some(memory_limit) = self.option.memory_limit {
                    if memory > memory_limit {
                        res = ExecResult::MemoryLimitExceeded;
                    }
                }
                // TODO move to config file
                let abs = 0;
                if let Some(real_time_limit) = self.option.real_time_limit {
                    if real_time + abs > real_time_limit {
                        res = ExecResult::RealTimeLimitExceeded;
                    }
                }
                if let Some(cpu_time_limit) = self.option.cpu_time_limit {
                    if cpu_time + abs > cpu_time_limit {
                        res = ExecResult::CpuTimeLimitExceeded;
                    }
                }
                if let Some(_) = self.option.syscall_limit {
                    if status == SIGSYS_EXIT_CODE {
                        res = ExecResult::SyscallLimitExceeded;
                    }
                }
                return Ok(RunResult::new(
                    res,
                    status as u32,
                    cpu_time,
                    real_time,
                    memory,
                ));
            }
        }

        Err("gg".into())
    }
}

#[test]
#[allow(unused_imports)]
pub fn test_run_command() {
    use crate::languages::Cpp;
    use crate::languages::Language;
    use std::fs::OpenOptions;

    // let syscalls = Cpp::allowed_syscalls();
    // let path = get_config().get_str("sandbox.jail_path").unwrap();
    // let out_file = OpenOptions::new()
    //     .write(true)
    //     .create(true)
    //     .open("1.out")
    //     .unwrap();
    // std::env::
    let res = Command::new("./tmp.zmm")
        .args(vec![
            "tmp", "go", "run",
            "tmp.go",
            // "g++",
            // "tmp.cpp",
            // "-o",
            // "./out/tmp.zmm",
        ])
        .memory(1024 * 1024)
        .cpu_time(200000)
        .real_time(500000)
        .uid(1002)
        .process(1)
        // .jail_path("./jail")
        .run();
    println!("{:?}", res);
    // println!("{:?}", std::env::var(key));
}
