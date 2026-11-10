fn main() -> std::process::ExitCode {
    squigit_qt::run(std::env::args().skip(1).collect())
}
