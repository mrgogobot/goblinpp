mod cube;
fn main() {
    if let Err(e) = cube::cli() {
        eprintln!("EXTRACTION_ERROR: {e}\nNo successful extraction is implied by partial output.");
        std::process::exit(1);
    }
}
