use oj_judger::{
    config,
    judge::{self, JudgeInfo},
};
use rayon::ThreadPoolBuilder;
use std::{
    env,
    error::Error,
    mem,
    sync::{Arc, Mutex},
    thread,
};
use tracing::debug;
use tracing_subscriber::fmt;

pub fn init() {
    // TODO mount bin and lib to jail here
}

fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    fmt::init();

    init();

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
                        debug!("{:?}", judge_info);
                        let result = pool.install(move || {
                            let result = judge::judge(judge_info);
                            *free_thread_number.lock().unwrap() += 1;
                            debug!("wuhu, i am free {:?}", thread::current().id());
                            result
                        });
                        debug!("{:?}", result);
                        let result = serde_json::to_string(&result).unwrap();
                        nc.publish(&msg.reply.unwrap(), &result).unwrap();
                    }
                    Err(e) => {
                        debug!("bad judge info: {:?}", e);
                    }
                }
            }
        }
    }
}
