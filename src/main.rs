fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("tig-rs {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    println!("tig-rs: TUI not implemented yet");
}
