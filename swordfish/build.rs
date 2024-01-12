use std::process::Command;


// Example custom build script.
fn main() {
    // Get the git commit hash (short)
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .expect("Failed to execute git command");
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!(
        "cargo:rustc-env=BUILD_PROFILE={}",
        std::env::var("PROFILE").unwrap()
    );
}
