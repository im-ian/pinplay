use std::process::ExitCode;

fn main() -> ExitCode {
    match pinplay::app::run_from_env() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("pinplay: {error}");
            ExitCode::FAILURE
        }
    }
}
