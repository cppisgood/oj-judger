use std::{fs, error::Error};

pub struct CompareOption {
    allow_trailing_space: bool,
    allow_trailing_blank_line: bool,
}

impl CompareOption {
    fn default() -> Self {
        CompareOption {
            allow_trailing_space: true,
            allow_trailing_blank_line: true,
        }
    }
}

// considering str_a is std
fn compare_two_str(str_a: &str, str_b: &str, compare_option: Option<CompareOption>) -> bool {
    let compare_option = compare_option.or(Some(CompareOption::default())).unwrap();
    let (str_a, str_b) = if compare_option.allow_trailing_blank_line {
        (str_a.trim_end_matches('\n'), str_b.trim_end_matches('\n'))
    } else {
        (str_a, str_b)
    };
    
    let mut iter_a = str_a.split('\n');
    let mut iter_b = str_b.split('\n');

    let cmp = if compare_option.allow_trailing_space {
        |a: &str, b: &str| a.trim_end_matches(' ') == b.trim_end_matches(' ')
    } else {
        |a: &str, b: &str| a == b
    };

    let mut res =  true;
    while let Some(a) = iter_a.next() {
        if let Some(b) = iter_b.next() {
            res &= cmp(a, b);
        } else {
            return false;
        }
        if !res {
            break;
        }
    }
    res
}



pub fn compare_two_file(file_a: &str, file_b: &str, compare_option: Option<CompareOption>) -> Result<bool, Box<dyn Error>> {
    let file_a = fs::read_to_string(file_a)?;
    let file_b = fs::read_to_string(file_b)?;
    Ok(compare_two_str(&file_a, &file_b, compare_option))
}

#[test]
fn test() {
    let s = "123\n456\n\n789";
    let a: Vec<_> = s.split_terminator("\n").collect();
    println!("{:?}", a);
    let a: Vec<_> = s.split("\n").collect();
    println!("{:?}", a);

}
