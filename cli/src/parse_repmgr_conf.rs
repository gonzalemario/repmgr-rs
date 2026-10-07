use std::fs::File;
use std::io::{BufRead, BufReader};

use std::collections::HashMap;

use std::process;

fn parse_value_thats_next_to_key(value: String) -> String {
    /*
     * a line in the config might be something like:
     * conninfo='db=random host=non-local user=notme' #This is a comment in the config
     * or a simple
     * node_name='supername'
     *
     * hence, we must extract the values inside the single quotes only and ignore the
     * rest, if anything
     */

    let parts_of_value: Vec<&str> = value.split("'").collect();
    if parts_of_value.len() < 3 {
        eprintln!("Malformed line {value}");
        process::exit(-1);
    }
    String::from(parts_of_value[1])
}

fn handle_line(mut splitted_line: Vec<&str>, config: &mut HashMap<String, String>) {
    let key = splitted_line.remove(0).trim().to_string();
    let mut value = splitted_line.remove(0).trim().to_string();
    value = parse_value_thats_next_to_key(value);

    config.insert(key, value);
}

pub fn parse_file(file: File) -> HashMap<String, String> {
    let buffer = BufReader::new(file);
    let mut rp_global_config = HashMap::new();

    for line in buffer.lines() {
        let line = line.expect("error reading line");
        if line.trim().starts_with('#') || line.trim().is_empty() {
            continue;
        }

        handle_line(line.splitn(2, '=').collect(), &mut rp_global_config);
    }

    rp_global_config
}
