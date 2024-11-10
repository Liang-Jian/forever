
use std::fs::File;
use std::io::{self, BufRead};

fn process_file_and_generate_esl_ids(file_path: &str) -> Vec<String> {
    let mut esl_ids = Vec::new();

    if let Ok(file) = File::open(file_path) {
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            if let Ok(content) = line {
                let _s = format!("{:08x}", content[8..].parse::<u32>().unwrap());
                let esl_id_11 = format!(
                    "{}-{}-{}-{}\n",
                    &_s[0..2],
                    &_s[2..4],
                    &_s[4..6],
                    &_s[6..8]
                )
                .to_uppercase();
                esl_ids.push(esl_id_11);
            }
        }
    }

    esl_ids
}

