use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output();
    let git_hash = if let Ok(o) = output {
        String::from_utf8(o.stdout).unwrap_or(String::from("NOCOMMITHASH"))
    } else {
        String::from("NOCOMMITHASH")
    };
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
