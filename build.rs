use actix_web_static_files::resource_dir;
use std::env;
use std::process::Command;

fn main() {
    let debug = env::var("PROFILE").expect("PROFILE env variable required") == "debug";

    if debug {
        return;
    };

    let out = Command::new("yarn")
        .current_dir("client")
        .arg("install")
        .status()
        .expect("Failed to fetch client dependencies");
    assert!(out.success());

    let out = Command::new("yarn")
        .current_dir("client")
        .arg("build")
        .status()
        .expect("Failed to build client");
    assert!(out.success());

    resource_dir("client/public")
        .build()
        .expect("Failed to bundle client resources");
}
