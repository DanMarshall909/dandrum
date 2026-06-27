fn main() {
    let result = dandrum_engine::cli::run(std::env::args());

    print!("{}", result.stdout);
    eprint!("{}", result.stderr);
    std::process::exit(result.exit_code);
}
