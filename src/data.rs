// use crate::config::get_config;
// // use crate::proto::judger::JudgeRequest;
// use std::path::Path;

// pub fn check_datas(request: JudgeRequest) -> Result<String, &'static str> {
//     let path = get_config().get_str("data.data_path").unwrap();
//     let path = Path::new(&path);
//     let path = path.join(request.problem_id);

//     if !path.exists() {
//         return Err("no data");
//     }

//     // TODO check data version

//     Ok(path.into_os_string().into_string().unwrap())
// }
