fn main() {
    if let Err(err) = research_artifact_uploader::cli::run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
