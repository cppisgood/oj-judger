use crate::utils;
use cgroups_rs::{
    cgroup_builder::CgroupBuilder, hierarchies, memory::MemController, Cgroup as CG, CgroupPid,
    Controller, MaxValue,
};
use notify::{watcher, DebouncedEvent, INotifyWatcher, RecursiveMode, Watcher};
use std::{
    error::Error,
    fs::OpenOptions,
    io::Write,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

fn gen_cgroup_name() -> String {
    format!("oj-cg-{}", utils::unix_time())
}

pub struct Cgroup {
    pub cg: CG,
    _watcher: INotifyWatcher,
    rx: Receiver<DebouncedEvent>,
}

impl Cgroup {
    // fn controller<'a>(&'a self) -> &'a MemController {
    //     self.cg.controller_of().expect("get controller failed")
    // }

    pub fn new(memory_limit: Option<u64>, process_limit: Option<u32>) -> Self {
        let cg = {
            let mut cg = CgroupBuilder::new(&gen_cgroup_name());
            if let Some(memory_limit) = memory_limit {
                cg = cg.memory().memory_hard_limit(memory_limit as i64).done();
            }
            if let Some(process_limit) = process_limit {
                cg = cg
                    .pid()
                    .maximum_number_of_processes(MaxValue::Value(process_limit as i64))
                    .done();
            }
            let hier = hierarchies::auto();
            cg.build(hier)
        };

        {
            let path = cg
                .controller_of::<MemController>()
                .expect("get controller failed")
                .path()
                .join("memory.oom.group");
            let mut f = OpenOptions::new()
                .write(true)
                .open(path)
                .expect("open memory.oom.group failed");
            f.write("1".as_bytes())
                .expect("write memory.oom.group failed");
        }

        let (tx, rx) = mpsc::channel();
        let path = cg
            .controller_of::<MemController>()
            .expect("get controller failed")
            .path()
            .join("memory.events.local");
        let mut _watcher = watcher(tx, Duration::ZERO).expect("Cgroup memory watcher init falied");
        _watcher
            .watch(path, RecursiveMode::Recursive)
            .expect("watcher Cgroup memory failed");

        Cgroup { cg, _watcher, rx }
    }
    pub fn oom_killed(&self) -> bool {
        self.rx.try_recv().is_ok()
    }
    pub fn add_task(&self, pid: u64) -> Result<(), Box<dyn Error>> {
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

impl Drop for Cgroup {
    fn drop(&mut self) {
        while !self.cg.tasks().is_empty() {}
        self.delete().expect("drop Cgroup failed");
    }
}
