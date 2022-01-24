use std::fs::File;

pub fn compare_two_file(file_a: &str, file_b: &str) -> bool {
    
    let file_a = File::open(file_a).unwrap();
    let file_b = File::open(file_b).unwrap();

    

    todo!()
}