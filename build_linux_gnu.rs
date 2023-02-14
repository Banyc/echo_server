#!/usr/bin/env rust-script

use std::{env, process};

fn main() {
    env::set_var(
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
        "x86_64-unknown-linux-gnu-gcc",
    );
    let output = process::Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg("x86_64-unknown-linux-gnu")
        .output()
        .unwrap();
    println!("{:?}", output);
}
