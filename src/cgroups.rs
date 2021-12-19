use cgroups_rs::{
    cgroup_builder::CgroupBuilder, hierarchies, memory::MemController, Cgroup as CG, CgroupPid,
    Controller,
};
use notify::{watcher, DebouncedEvent, INotifyWatcher, RecursiveMode, Watcher};
use std::{
    error::Error,
    sync::mpsc::{self, Receiver},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub trait Cgroup {
    fn add_task(&self, pid: u64) -> Result<(), Box<dyn Error>>;
    fn delete(&self) -> Result<(), Box<dyn Error>>;
}

fn gen_cgroup_name() -> String {
    format!(
        "oj-cg-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("gen_cgroup_name failed")
            .as_millis()
    )
}

pub struct CGMemory {
    cg: CG,
    _watcher: INotifyWatcher,
    rx: Receiver<DebouncedEvent>,
}

impl CGMemory {
    // fn controller<'a>(&'a self) -> &'a MemController {
    //     self.cg.controller_of().expect("get controller failed")
    // }

    pub fn new(memory_limit: u64) -> Self {
        let hier = hierarchies::auto();
        let cg = CgroupBuilder::new(&gen_cgroup_name())
            .memory()
            .memory_hard_limit(memory_limit as i64)
            .done()
            .build(hier);

        let (tx, rx) = mpsc::channel();
        let mut _watcher = watcher(tx, Duration::ZERO).expect("CGMemory watcher init falied");
        _watcher
            .watch(
                cg.controller_of::<MemController>()
                    .expect("get controller failed")
                    .path()
                    .join("memory.events.local"),
                RecursiveMode::Recursive,
            )
            .expect("watcher CGMemory failed");
        CGMemory { cg, _watcher, rx }
    }
    pub fn oom_killed(&self) -> bool {
        self.rx.try_recv().is_ok()
    }
}

impl Cgroup for CGMemory {
    fn add_task(&self, pid: u64) -> Result<(), Box<dyn Error>> {
        match self
            .cg
            .controller_of::<MemController>()
            .expect("get controller failed")
            .add_task(&CgroupPid::from(pid))
        {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }

    fn delete(&self) -> Result<(), Box<dyn Error>> {
        match self.cg.delete() {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }
}

impl Drop for CGMemory {
    fn drop(&mut self) {
        self.cg.delete().expect("drop CGmemory failed");
    }
}
