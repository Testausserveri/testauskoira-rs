use std::process::Command;

fn main() {
    let git_hash = std::env::var("GIT_HASH")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let output = Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output();
            if let Ok(o) = output {
                String::from_utf8(o.stdout)
                    .unwrap_or(String::from("NOCOMMITHASH"))
                    .trim()
                    .to_string()
            } else {
                String::from("NOCOMMITHASH")
            }
        });
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
