use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "wangzi vm-lua")]
pub struct Opt {
    #[structopt(short = "f", long, parse(from_os_str))]
    pub file: Vec<PathBuf>,
    #[structopt(short = "e", long)]
    pub command: Vec<String>,
    #[structopt(short = "b", long)]
    pub bench: bool,
    #[structopt(short = "j", long)]
    pub jit: bool,
    #[structopt(short = "l", long, default_value = "lua")]
    pub language: String,
}
