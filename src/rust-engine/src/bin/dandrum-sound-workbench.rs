fn main() {
    let result = dandrum_engine::sound_workbench::run_sound_workbench(std::env::args());
    print!("{}", result.stdout);
    eprint!("{}", result.stderr);
    if result.exit_code != 0 {
        std::process::exit(result.exit_code);
    }
}
