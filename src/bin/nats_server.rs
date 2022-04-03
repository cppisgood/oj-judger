use oj_judger::{config, judge::JudgeInfo};
use nats;
use tracing::debug;
// use tracing::debug;
use tracing_subscriber::fmt;
use std::{env, error::Error, io};

fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    fmt::init();
    let nat_url = env::var("NATS_URL").expect("NATS_URL must be set");

    let config = config::get_config();

    let nc = nats::connect(&nat_url)?;
    let subject = config.get_string("nats.judge_queue_subject")?;

    // let mut round = 0;
    loop {
        let n: usize =  {
            let mut buf = String::new();
            io::stdin().read_line(&mut buf)?;
            buf.trim().parse()?
        };

        let judge_info = JudgeInfo {
            submission_id: "s1".to_string(),
            language: "python3".to_string(),
            code: "print('hello ningoj')".to_string(),
            problem_id: "1000".to_string(),
            data_version: n.to_string(),
            cpu_time_limit: 10000,
            real_time_limit: 10000,
            memory_limit: 102400,
        };

        let judge_info = serde_json::to_string(&judge_info)?;

        // round += 1;
        for i in 0..n {
            debug!("send {}", i);
            nc.publish(&subject, &judge_info)?;
            // nc.pu
        }
        // nc.publish(&subject, &judge_info)?;
    }
}