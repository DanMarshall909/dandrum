use std::path::Path;

fn main() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/sound-design/tb303-acid-poc.yaml");
    let result = dandrum_engine::sound_workbench::run_sound_workbench([
        "dandrum-sound-workbench".to_string(),
        "render".to_string(),
        fixture.display().to_string(),
        "--output-wav".to_string(),
        "/tmp/dandrum-acid.wav".to_string(),
        "--output-metrics".to_string(),
        "/tmp/dandrum-acid.csv".to_string(),
    ]);

    print!("{}", result.stdout);
    eprint!("{}", result.stderr);
    if result.exit_code != 0 {
        std::process::exit(result.exit_code);
    }
}
