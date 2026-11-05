use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let native_bin = root.join("build/nodaysidle-browser");

    if !native_bin.exists() {
        println!("Building native Qt6 WebEngine browser...");
        let status = Command::new("./bin/build").current_dir(&root).status();
        if let Err(e) = status {
            eprintln!("Failed to execute build script: {e}");
            std::process::exit(1);
        }
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let err = Command::new(&native_bin).args(&args).exec();

    eprintln!("Failed to launch native Qt6 WebEngine browser: {err}");
    std::process::exit(1);
}
