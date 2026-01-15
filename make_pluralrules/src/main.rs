use clap::Parser;
use make_pluralrules::generate_rs;
use std::process::Command;

use std::fs;
use std::io::Write;

#[derive(Parser)]
#[command(name = "CLDR Plural Rules Rust Generator")]
#[command(version = "0.1.0")]
#[command(about = "Generates Rust code for CLDR plural rules.")]
struct Args {
    /// Input CLDR JSON plural rules files
    #[arg(short, long, required = true)]
    input: Vec<String>,

    /// Output RS file
    #[arg(short, long, required = true)]
    output: String,

    /// Do not format the output
    #[arg(short, long)]
    ugly: bool,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let input_jsons = args
        .input
        .iter()
        .map(|path| fs::read_to_string(path).expect("file not found"))
        .collect::<Vec<_>>();
    let complete_rs_code = generate_rs(&input_jsons);

    let mut file = fs::File::create(&args.output)?;
    file.write_all(complete_rs_code.as_bytes())?;

    if !args.ugly {
        Command::new("rustfmt")
            .args([&args.output])
            .output()
            .expect("Failed to format the output using `rustfmt`");
    }

    Ok(())
}
