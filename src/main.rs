use clap::{Parser, Subcommand};
mod api;
mod tui;

#[derive(Parser)]
#[command(name = "acdc")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Test,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => tui::run().unwrap(),
        Some(Commands::Test) => println!("Subcommand was used"),
    }

    api::test("nginx").unwrap();
}
