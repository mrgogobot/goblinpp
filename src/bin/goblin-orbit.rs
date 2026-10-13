use clap::Parser;

#[derive(Parser)]
#[command(name = "goblin-orbit", version = goblinpp::VERSION, about = "Read-only scoped baseline checker; not a compromise detector")]
struct Cli {
    #[command(flatten)]
    orbit: goblinpp::orbit_cli::OrbitArgs,
}

fn main() {
    let code = match Cli::try_parse() {
        Ok(cli) => match goblinpp::orbit_cli::dispatch(cli.orbit) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("{}", error.pretty());
                2
            }
        },
        Err(error) => {
            let _ = error.print();
            error.exit_code()
        }
    };
    std::process::exit(code);
}
