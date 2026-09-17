#[cfg(feature = "home-mixer")]
fn main() {
    if let Err(error) = eoro::home_mixer::run_cli(std::env::args().skip(1)) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "home-mixer"))]
fn main() {}
