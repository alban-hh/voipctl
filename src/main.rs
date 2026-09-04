use clap::Parser;

fn main() {
    let cli = voipctl::cli::Cli::parse();
    let json = cli.json;
    if let Err(error) = voipctl::app::run(cli) {
        if json {
            eprintln!("{}", serde_json::json!({"error":format!("{error:#}")}));
        } else {
            eprintln!("error: {error:#}");
        }
        std::process::exit(1);
    }
}
