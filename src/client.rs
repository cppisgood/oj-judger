use std::fs::read_to_string;

use oj_judger::proto::judger::judger_client::JudgerClient;
use oj_judger::proto::judger::JudgeRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = JudgerClient::connect("http://[::1]:50051").await?;

    let f = read_to_string("tmp.cpp").unwrap();

    let request = tonic::Request::new(JudgeRequest {
        problem_id: "1000".to_string(),
        src_code: f,
        language: "cpp".to_string(),
        cpu_time_limit: 10000,
        real_time_limit: 100000,
        memory_limit: 1024 * 1024,
        special_judge: false,
        spj_code: "".to_string(),
        spj_language: "".to_string(),
        data_last_modify: "".to_string(),
    });

    let response = client.judge(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
