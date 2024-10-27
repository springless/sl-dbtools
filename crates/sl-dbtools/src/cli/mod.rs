use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    name: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

impl Args {
    pub fn run(&self) {
        for _ in 0..self.count {
            println!("Hello {}", self.name);
        }
    }
}

