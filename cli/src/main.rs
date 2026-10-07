mod commandline;
use commandline as cmd;

mod parse_repmgr_conf;

use std::fs::File;
use std::process::exit;

fn main() {
    let options = cmd::CommandLine::parse_cli();

    let file = File::open(options.config_file).unwrap_or_else(|e| {
        eprintln!("Wrong file: {e}");
        exit(1);
    });

    let config = parse_repmgr_conf::parse_file(file);
    println!("{:?}", config);
}
