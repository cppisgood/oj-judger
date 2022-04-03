use std::{
    env,
    error::Error,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration, mem,
};

use oj_judger::{config, run_command::Command};
use rayon::ThreadPoolBuilder;
use serde::Deserialize;
use tokio::{runtime::Builder, time};
use tracing::debug;
use tracing_subscriber::fmt;

#[derive(Debug, Deserialize)]
pub struct JudgeInfo {
    language: String,
    code: String,
    problem_id: String,
    data_version: String,

    cpu_time_limit: u64,  // ms
    real_time_limit: u64, // ms
    memory_limit: u64,    // kb
}

// #[tokio::main]
fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    fmt::init();

    let config = config::get_config();

    let nat_url = env::var("NATS_URL").expect("NATS_URL must be set");
    let nc = nats::connect(&nat_url)?;
    let subject = config.get_string("nats.judge_queue_subject")?;
    let sub = nc.queue_subscribe(&subject, "default_queue")?;

    let thread_number = config.get_int("worker.thread_number")? as usize;

    let pool = ThreadPoolBuilder::new()
        .num_threads(thread_number)
        .build()?;

    let free_thread_number = Arc::new(Mutex::new(thread_number));

    loop {
        let mut free = free_thread_number.lock().unwrap();
        if *free > 0 {
            debug!("free: {}", *free);
            *free -= 1;
            mem::drop(free);
            if let Some(msg) = sub.next() {
                let judge_info = serde_json::from_slice::<JudgeInfo>(&msg.data);
                match judge_info {
                    Ok(judge_info) => {
                        let free_thread_number = Arc::clone(&free_thread_number);
                        pool.spawn(move || {
                            thread::sleep(Duration::from_secs(10));
                            *free_thread_number.lock().unwrap() += 1;
                            debug!("wuhu, i am free {:?}", thread::current().id());
                        });

                        debug!("{:?}", judge_info);

                    }
                    Err(_) => {
                        debug!("bad judge info: {:?}", msg.data);
                        
                    }
                }

            }
        }
    }

    // let pool = ThreadPoolBuilder::new().num_threads(8).build()?;
    // // calc();
    // println!("{}", pool.current_num_threads());

    // let (tx, rx) = mpsc::channel();

    // for i in 0..10 {
    //     let tx = tx.clone();
    //     pool.spawn(move || {
    //         let res = calc();
    //         println!("{}: {}", i, res);
    //         tx.send(res).unwrap();
    //         // thread::sleep(Duration::from_secs(10));
    //     });
    //     println!("{}: {:?}", i, pool.current_thread_has_pending_tasks());
    // }

    // let res: Vec<u64> = rx.into_iter().collect();
    // println!("{:?}", res);

    // let handle = thread::spawn(|| -> Result<(), Box<dyn Error + Sync + Send>> {
    //     let rt = Builder::new_multi_thread()
    //         .worker_threads(4)
    //         .enable_time()
    //         .build()?;

    //     for i in 0..10 {
    //         rt.spawn_blocking(move || {
    //             println!("{}", i);
    //             thread::sleep(Duration::from_secs(20));
    //         });
    //     }
    //     Ok(())
    // });
    // handle.join().unwrap().unwrap();

    Ok(())
}

// fn main() {
//     dotenv::dotenv().ok();
//     fmt::init();

//     let res = Command::new("/bin/ls").args(vec!["."])
//     .jail_path("jail")
//     .exec_path("./bin")
//     .run();
//     debug!("{:?}", res);
// }

// fn run() {}

// run_option {
//     cmd: i32,
//     args: i32,
//     jail_path: i32,
//     exec_path: i32,
//     uid: i32,
//     memory_limit: i32,
//     time_limit: i32,
//     syscall_limit: i32,
//     stdin_redirect: i32,
//     stdout_redirect: i32,
// }

// run_result {
//     result: i32,
//     exit_code: i32,
//     over_memory_limit: i32,
//     over_time_limit: i32,
//     over_syscall_limit: i32,
// }
