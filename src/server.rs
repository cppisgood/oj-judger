use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::prelude::AsRawFd;

use tonic::{transport::Server, Request, Response, Status};

use oj_judger::proto::judger::judger_server::{Judger, JudgerServer};
use oj_judger::proto::judger::{JudgeReply, JudgeRequest};

use oj_judger::run_command::Command;
use oj_judger::utils;

#[derive(Debug, Default)]
pub struct MyJudger {}

#[tonic::async_trait]
impl Judger for MyJudger {
    async fn judge(&self, request: Request<JudgeRequest>) -> Result<Response<JudgeReply>, Status> {
        judge(request)
    }
}
fn judge(request: Request<JudgeRequest>) -> Result<Response<JudgeReply>, Status> {
    let request = request.get_ref();

    // TODO
    // if !check(request) {
    //     reply.error_msg = "bad request";
    //     return reply;
    // }
    let file_path = format!("{}_{}.cpp", request.problem_id, utils::unix_time());
    {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&file_path)
            .unwrap();
        f.write(request.src_code.as_bytes()).unwrap();
    }

    let compile_cmd = Command::new("/usr/bin/g++")
        .args(vec!["g++", &file_path, "-o", "tmp.zmm"])
        .cpu_time(request.cpu_time_limit)
        .memory(request.memory_limit)
        .process(10)
        .run()
        .unwrap();
    println!("{:?}", compile_cmd);

    // TODO
    // compile(request.src_code, request.language, config.exec_path);
    // if (request.special_judge) {
    //     compile(request.spj_code, request.spj_language, config.spj_path);
    // }

    let f_in = OpenOptions::new().read(true).open("1.in").unwrap();
    let f_out = OpenOptions::new()
        .create(true)
        .write(true)
        .open("1.out")
        .unwrap();

    // let exe_path = format!("./{}")
    let run_result = Command::new("./tmp.zmm")
        .cpu_time(request.cpu_time_limit)
        .real_time(request.real_time_limit)
        .memory(request.memory_limit)
        .stdin(f_in.as_raw_fd() as u32)
        .stdout(f_out.as_raw_fd() as u32)
        .run()
        .unwrap();
    println!("{:?}", run_result);

    let reply = JudgeReply {
        error_msg: "".to_string(),
        result: compile_cmd.result as u64,
        cpu_time: run_result.cpu_time,
        memory: run_result.memory,
        details: vec![],
    };
    Ok(Response::new(reply))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let judger = MyJudger::default();

    Server::builder()
        .add_service(JudgerServer::new(judger))
        .serve(addr)
        .await?;

    Ok(())
}
