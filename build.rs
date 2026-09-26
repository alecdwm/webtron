use std::env;
use std::process::Command;

/// The client's sources, which the release build is made from.
const CLIENT_SOURCES: &[&str] = &[
    "build.rs",
    "client/src",
    "client/public",
    "client/index.html",
    "client/package.json",
    "client/bun.lock",
    "client/tsconfig.json",
    "client/tsconfig.app.json",
    "client/tsconfig.node.json",
    "client/vite.config.mts",
];

fn main() {
    for path in CLIENT_SOURCES {
        println!("cargo:rerun-if-changed={}", path);
    }

    let debug = env::var("PROFILE").expect("PROFILE env variable required") == "debug";

    if debug {
        return;
    };

    let out = Command::new("bun")
        .current_dir("client")
        .arg("install")
        .arg("--frozen-lockfile")
        .status()
        .expect("Failed to fetch client dependencies");
    assert!(out.success());

    let out = Command::new("bun")
        .current_dir("client")
        .arg("run")
        .arg("build")
        .status()
        .expect("Failed to build client");
    assert!(out.success());
}
