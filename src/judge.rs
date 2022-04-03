use std::fs::File;
use std::io::Write;
use std::os::unix::prelude::AsRawFd;
use std::{fs, panic};

use axum::{http::StatusCode, response::IntoResponse, routing, Json, Router};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::run_command::{ExecResult, RunResult};
use crate::{compare, config, run_command::Command, utils};

#[derive(Debug, Deserialize, Serialize)]
pub struct JudgeInfo {
    pub submission_id: String,

    pub language: String,
    pub code: String,
    pub problem_id: String,
    pub data_version: String,

    pub cpu_time_limit: u64,  // ms
    pub real_time_limit: u64, // ms
    pub memory_limit: u64,    // kb
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SingleJudgeStatus {
    Accepted,
    WrongAnswer,
    RuntimeError,
    TimeLimitExceeded,
    MemoryLimitExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleJudgeResult {
    status: SingleJudgeStatus,
    exit_code: u32,
    cpu_time: u64,
    real_time: u64,
    memory: u64,
}

impl SingleJudgeResult {
    fn from_run_result(status: SingleJudgeStatus, run_result: &RunResult) -> Self {
        SingleJudgeResult {
            status,
            exit_code: run_result.exit_code,
            cpu_time: run_result.cpu_time,
            real_time: run_result.real_time,
            memory: run_result.memory,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum JudgeStatus {
    Accepted,
    CompileError,
    WrongAnswer,
    RuntimeError,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    SystemError,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JudgeResult {
    status: JudgeStatus,
    exit_code: u32,
    cpu_time: u64,
    real_time: u64,
    memory: u64,
    results: Vec<SingleJudgeResult>,
    msg: Option<String>,
}

impl From<SingleJudgeStatus> for JudgeStatus {
    fn from(status: SingleJudgeStatus) -> Self {
        match status {
            SingleJudgeStatus::Accepted => JudgeStatus::Accepted,
            SingleJudgeStatus::WrongAnswer => JudgeStatus::WrongAnswer,
            SingleJudgeStatus::RuntimeError => JudgeStatus::RuntimeError,
            SingleJudgeStatus::TimeLimitExceeded => JudgeStatus::TimeLimitExceeded,
            SingleJudgeStatus::MemoryLimitExceeded => JudgeStatus::MemoryLimitExceeded,
        }
    }
}

impl JudgeResult {
    fn new(
        status: JudgeStatus,
        exit_code: u32,
        cpu_time: u64,
        real_time: u64,
        memory: u64,
        results: Vec<SingleJudgeResult>,
        msg: Option<String>,
    ) -> Self {
        JudgeResult {
            status,
            exit_code,
            cpu_time,
            real_time,
            memory,
            results,
            msg,
        }
    }
}

fn judge(judge_info: JudgeInfo) -> JudgeResult {
    debug!("{:?}", judge_info);
    let config = config::get_config();

    let jail_path = config.get_string("sandbox.jail_path").unwrap();

    // write code to file
    {
        let src_file_name = config
            .get_string(&format!("language.{}.src_file_name", judge_info.language))
            .unwrap();
        let src_file_path = format!("{}{}", jail_path, src_file_name);
        let mut src_file = File::create(&src_file_path).unwrap();
        write!(src_file, "{}", judge_info.code).unwrap();
    }

    // compile src code if need
    if let Ok(compile_cmd) =
        config.get_string(&format!("language.{}.compile_cmd", judge_info.language))
    {
        let args: Vec<_> = compile_cmd.split(" ").collect();
        let cmd_path = args[0];
        let output_file_path = config.get_string("judger.compile_output_file").unwrap();
        let output_file = File::create(&output_file_path).unwrap();
        let output_fd = output_file.as_raw_fd();
        let res = Command::new(cmd_path)
            .args(args)
            .exec_path(&jail_path)
            .stdout(output_fd as u32)
            .run()
            .unwrap();
        debug!("compile result: {:?}", res);
        if res.result != ExecResult::Ok {
            let compile_error_msg = fs::read_to_string(&output_file_path).unwrap();
            return JudgeResult::new(
                JudgeStatus::CompileError,
                0,
                0,
                0,
                0,
                vec![],
                Some(compile_error_msg),
            );
        }
    }

    // run code
    let run_cmd = config
        .get_string(&format!("language.{}.run_cmd", judge_info.language))
        .unwrap();
    let args: Vec<_> = run_cmd.split(" ").collect();
    let cmd_path = args[0];
    let data_path = {
        let data_path = config.get_string("data.data_path").unwrap();
        format!("{}{}", data_path, judge_info.problem_id)
    };
    // filter *.in file only
    let paths = fs::read_dir(&data_path)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|path| path.path().extension().unwrap() == "in");
    let mut results: Vec<SingleJudgeResult> = Vec::new();
    let mut max_cpu_time = 0;
    let mut max_real_time = 0;
    let mut max_memory = 0;
    for input_file_path in paths {
        debug!("input_file_path: {:?}", input_file_path.path().display());

        let input_file = File::open(input_file_path.path()).unwrap();
        let input_fd = input_file.as_raw_fd();

        let output_file_path = config.get_string("judger.user_output_file").unwrap();
        let output_file = File::create(&output_file_path).unwrap();
        let output_fd = output_file.as_raw_fd();

        let uid = config.get_int("judger.exec_user_uid").unwrap();

        let res = Command::new(cmd_path)
            .args(args.clone())
            .uid(uid as u32)
            .cpu_time(judge_info.cpu_time_limit)
            .real_time(judge_info.real_time_limit)
            .memory(judge_info.memory_limit)
            .jail_path(&jail_path)
            .stdin(input_fd as u32)
            .stdout(output_fd as u32)
            .run()
            .expect("run command error");
        debug!("run: {:?}", res);

        let single_judge_result = match res.result {
            ExecResult::Ok => {
                let std_out_file_path = input_file_path
                    .path()
                    .with_extension("out")
                    .into_os_string()
                    .into_string()
                    .unwrap();
                debug!("{} {}", std_out_file_path, output_file_path);
                let ok =
                    compare::compare_two_file(&std_out_file_path, &output_file_path, None).unwrap();
                if ok {
                    SingleJudgeResult::from_run_result(SingleJudgeStatus::Accepted, &res)
                } else {
                    SingleJudgeResult::from_run_result(SingleJudgeStatus::WrongAnswer, &res)
                }
            }
            ExecResult::CpuTimeLimitExceeded => {
                SingleJudgeResult::from_run_result(SingleJudgeStatus::TimeLimitExceeded, &res)
            }
            ExecResult::RealTimeLimitExceeded => {
                SingleJudgeResult::from_run_result(SingleJudgeStatus::TimeLimitExceeded, &res)
            }
            ExecResult::MemoryLimitExceeded => {
                SingleJudgeResult::from_run_result(SingleJudgeStatus::MemoryLimitExceeded, &res)
            }
            ExecResult::SyscallLimitExceeded => {
                SingleJudgeResult::from_run_result(SingleJudgeStatus::RuntimeError, &res)
            }
            ExecResult::RuntimeError => {
                SingleJudgeResult::from_run_result(SingleJudgeStatus::RuntimeError, &res)
            }
        };
        max_cpu_time = max_cpu_time.max(single_judge_result.cpu_time);
        max_real_time = max_real_time.max(single_judge_result.real_time);
        max_memory = max_memory.max(single_judge_result.memory);

        results.push(single_judge_result.clone());

        let status = JudgeStatus::from(single_judge_result.status);
        if status != JudgeStatus::Accepted {
            return JudgeResult::new(
                status,
                single_judge_result.exit_code,
                max_cpu_time,
                max_real_time,
                max_memory,
                results,
                None,
            );
        }
    }
    JudgeResult::new(
        JudgeStatus::Accepted,
        0,
        max_cpu_time,
        max_real_time,
        max_memory,
        results,
        None,
    )
}

pub async fn judge_handler(Json(judge_info): Json<JudgeInfo>) -> impl IntoResponse {
    let res = panic::catch_unwind(|| judge(judge_info));
    match res {
        Ok(res) => (StatusCode::OK, utils::gen_response(0, res)),
        Err(e) => {
            debug!("{:?}", e);
            (
                StatusCode::OK,
                utils::gen_response(
                    0,
                    JudgeResult::new(JudgeStatus::SystemError, 0, 0, 0, 0, vec![], None),
                ),
            )
        }
    }
}

pub fn get_router() -> Router {
    // Router::new()
    Router::new().route("/", routing::post(judge_handler))
}

#[test]
fn test() {}
