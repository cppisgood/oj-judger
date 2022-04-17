use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::os::unix::prelude::AsRawFd;
use std::str::FromStr;
use std::{fs, panic};

use axum::{http::StatusCode, response::IntoResponse, routing, Json, Router};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::run_command::{ExecResult, RunResult};
use crate::{compare, config, run_command::Command, utils};

#[derive(Debug, Deserialize, Serialize)]
pub struct JudgeInfo {
    pub submission_id: String,
    pub compile_cmd: Option<String>,
    pub run_cmd: String,
    pub src_file_name: String,

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
    Judging,
    MemoryLimitExceeded,
    RuntimeError,
    SystemError,
    TimeLimitExceeded,
    WrongAnswer,
}

impl Display for JudgeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for JudgeStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Accepted" => Ok(JudgeStatus::Accepted),
            "CompileError" => Ok(JudgeStatus::CompileError),
            "Judging" => Ok(JudgeStatus::Judging),
            "MemoryLimitExceeded" => Ok(JudgeStatus::MemoryLimitExceeded),
            "RuntimeError" => Ok(JudgeStatus::RuntimeError),
            "SystemError" => Ok(JudgeStatus::SystemError),
            "TimeLimitExceeded" => Ok(JudgeStatus::TimeLimitExceeded),
            "WrongAnswer" => Ok(JudgeStatus::WrongAnswer),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JudgeResult {
    pub submission_id: String,
    pub status: JudgeStatus,
    pub exit_code: u32,
    pub cpu_time: u64,
    pub real_time: u64,
    pub memory: u64,
    pub results: Vec<SingleJudgeResult>,
    pub msg: Option<String>,
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
        submission_id: String,
        status: JudgeStatus,
        exit_code: u32,
        cpu_time: u64,
        real_time: u64,
        memory: u64,
        results: Vec<SingleJudgeResult>,
        msg: Option<String>,
    ) -> Self {
        JudgeResult {
            submission_id,
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

pub fn judge(judge_info: JudgeInfo) -> JudgeResult {
    debug!("{:?}", judge_info);
    let config = config::get_config();

    let jail_path = config.get_string("sandbox.jail_path").unwrap();

    // write code to file
    {
        let src_file_name = judge_info.src_file_name;
        let src_file_path = format!("{}{}", jail_path, src_file_name);
        let mut src_file = File::create(&src_file_path).unwrap();
        write!(src_file, "{}", judge_info.code).unwrap();
    }

    // compile src code if need
    if let Some(compile_cmd) = judge_info.compile_cmd {
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
                judge_info.submission_id.clone(),
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
    let run_cmd = judge_info.run_cmd;
    let args: Vec<_> = run_cmd.split(" ").collect();
    let cmd_path = args[0];
    let data_path = {
        let data_path = config.get_string("data.data_path").unwrap();
        format!("{}{}", data_path, judge_info.problem_id)
    };
    // filter *.in file only
    debug!("{}", data_path);
    let paths = fs::read_dir(&data_path);
    let paths = match paths {
        Ok(paths) => paths.filter_map(Result::ok)
        .filter(|path| path.path().extension().unwrap() == "in"),
        Err(_) => return JudgeResult::new(
            judge_info.submission_id.clone(),
            JudgeStatus::SystemError,
            0,
            0,
            0,
            0,
            vec![],
            Some("data not found".to_string()),
        )
    };
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

        debug!("cmd_path: {}", cmd_path);
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
                judge_info.submission_id.clone(),
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
        judge_info.submission_id.clone(),
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
    let submission_id = judge_info.submission_id.clone();
    let res = panic::catch_unwind(|| judge(judge_info));
    match res {
        Ok(res) => (StatusCode::OK, utils::gen_response(0, res)),
        Err(e) => {
            debug!("{:?}", e);
            (
                StatusCode::OK,
                utils::gen_response(
                    0,
                    JudgeResult::new(
                        submission_id,
                        JudgeStatus::SystemError,
                        0,
                        0,
                        0,
                        0,
                        vec![],
                        None,
                    ),
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
