#[derive(clap::Parser)]
pub struct Args {
    #[command(subcommand)]
    pub commands: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    #[command(about = "run program in child process")]
    Run {
        /// Executable file to run
        #[arg(value_name = "FILE")]
        program_path: String,
    },
    #[command(about = "attach to process")]
    Attach {
        /// Executable file to run
        #[arg(value_name = "FILE")]
        program_path: String,

        /// Pid of process to attach
        #[arg(value_name = "PID")]
        pid: i32,
    },
}
