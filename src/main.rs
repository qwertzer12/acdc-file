use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
mod api;
mod tui;

#[derive(Parser)]
#[command(name = "acdc")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, default_value = "false")]
    console: bool,
}

#[derive(Subcommand)]
enum Commands {
    Test {
        #[arg(default_value = "nginx")]
        repo: String,
    },
    SearchTags {
        #[arg(default_value = "library")]
        namespace: String,
        #[arg(default_value = "nginx")]
        repo: String,
        #[arg(default_value = "")]
        query: String,
        #[arg(short, long, default_value_t = 15)]
        limit: usize,
    },
    AutoTags {
        #[arg(default_value = "")]
        image: String,
        #[arg(default_value = "")]
        query: String,
        #[arg(short, long, default_value_t = 15)]
        limit: usize,
    },
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => {
            if cli.console {
                println!("Console mode enabled (no TUI)."); // Placeholder for console mode logic
            } else {
                tui::run().unwrap();
            }
        }
        Some(Commands::Test { repo }) => {
            api::test(&repo).unwrap();
        }
        Some(Commands::SearchTags {
            namespace,
            repo,
            query,
            limit,
        }) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let tags = runtime
                .block_on(api::search_docker_hub_tags(&namespace, &repo, &query, limit))
                .unwrap();

            println!(
                "search namespace={namespace} repo={repo} query='{query}' limit={limit}"
            );
            for tag in tags {
                println!("{tag}");
            }
        }
        Some(Commands::AutoTags {
            image,
            query,
            limit,
        }) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let result = runtime
                .block_on(api::auto_search_docker_hub_tags(&image, &query, limit))
                .unwrap();

            match result {
                Some((resolved, tags)) => {
                    println!(
                        "auto image='{image}' -> namespace={} repo={} query='{}' limit={}",
                        resolved.namespace, resolved.repo, query, limit
                    );
                    for tag in tags {
                        println!("{tag}");
                    }
                }
                None => {
                    println!("No repository found for image term '{image}'");
                }
            }
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "acdc", &mut std::io::stdout());
        }
    }
}
