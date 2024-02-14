// use std::{env, path::Path};

fn main() {
    //я пробовал так выключить nodefault
        // println!("cargo:default-linker-libraries=yes");
        // println!("cargo:default-linker-libraries=true");
        // println!("default-linker-libraries=true");
    println!("cargo:rustc-link-arg=-L.");
    println!("cargo:rustc-link-arg=-logt_voxel_meshify");
    // println!("cargo:rustc-link-arg=-lstdc++");
    // println!("cargo:rustc-link-arg=-lc");
    // println!("cargo:rustc-link-arg=-lgcc");
    println!("cargo:rustc-link-arg=-lm"); //for math.h
    // let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    // println!("cargo:rustc-link-search=native={}", Path::new(&dir).join("lib").display());
}