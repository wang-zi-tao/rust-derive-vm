use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "wangzi vm-lua")]
pub struct Opt {
    #[structopt(short = "f", long, parse(from_os_str))]
    pub file: Vec<PathBuf>,
    #[structopt(short = "e", long)]
    pub command: Vec<String>,
}
