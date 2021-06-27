use std::{env, path::PathBuf};

fn sdk_path(target: &str) -> Result<String, std::io::Error> {
    // Use environment variable if set
    println!("cargo:rerun-if-env-changed=VIDEO_TOOLBOX_SDK_PATH");
    if let Ok(path) = std::env::var("VIDEO_TOOLBOX_SDK_PATH") {
        return Ok(path);
    }

    use std::process::Command;

    let sdk = if target.contains("apple-darwin") {
        "macosx"
    } else if target == "x86_64-apple-ios" || target == "i386-apple-ios" {
        "iphonesimulator"
    } else if target == "aarch64-apple-ios"
        || target == "armv7-apple-ios"
        || target == "armv7s-apple-ios"
    {
        "iphoneos"
    } else {
        unreachable!();
    };

    let output = Command::new("xcrun").args(&["--sdk", sdk, "--show-sdk-path"]).output()?.stdout;
    let prefix_str = std::str::from_utf8(&output).expect("invalid output from `xcrun`");
    Ok(prefix_str.trim_end().to_string())
}

fn build(sdk_path: Option<&str>, target: &str) {
    let mut headers = vec![];

    println!("cargo:rustc-link-lib=framework=VideoToolbox");
    headers.push("VideoToolbox/VideoToolbox.h");

    println!("cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS");

    let out_dir =
        PathBuf::from(env::var("OUT_DIR").expect("Expected an OUT_DIR environment variable"));

    let mut builder = bindgen::Builder::default();

    let target = if target == "aarch64-apple-ios" {
        "arm64-apple-ios"
    } else if target == "aarch64-apple-darwin" {
        "arm64-apple-darwin"
    } else {
        target
    };

    builder = builder.size_t_is_usize(true);
    builder = builder.clang_args(&[&format!("--target={}", target)]);

    if let Some(sdk_path) = sdk_path {
        builder = builder.clang_args(&["-isysroot", sdk_path]);
    }

    if target.contains("apple-ios") {
        builder = builder.blacklist_item("timezone");
        builder = builder.blacklist_item("obj_object");
    }

    builder = builder.whitelist_type("VT.*");
    builder = builder.whitelist_function("VT.*");

    builder = builder.whitelist_type("CV.*");
    builder = builder.whitelist_function("CV.*");

    builder = builder.whitelist_type("CFAllocator.*");
    builder = builder.whitelist_function("CFAllocator.*");

    builder = builder.whitelist_type("OpaqueVT.*");
    builder = builder.whitelist_function("OpaqueVT.*");

    builder = builder.whitelist_type("CFObject.*");
    builder = builder.whitelist_function("CFObject.*");

    builder = builder.whitelist_type("CFDictionary.*");
    builder = builder.whitelist_function("CFDictionary.*");

    builder = builder.whitelist_type("CMSampleBuffer.*");
    builder = builder.whitelist_function("CMSampleBuffer.*");

    builder = builder.whitelist_type("CMBlockBuffer.*");
    builder = builder.whitelist_function("CMBlockBuffer.*");

    builder = builder.whitelist_type("kVTVideoEncoderSpecification.*");
    builder = builder.whitelist_function("kVTVideoEncoderSpecification.*");

    builder = builder.whitelist_type("kCFTypeDictionary.*");
    builder = builder.whitelist_function("kCFTypeDictionary.*");
    builder = builder.whitelist_var("kCFTypeDictionary.*");

    builder = builder.whitelist_type("kCFAlloc.*");
    builder = builder.whitelist_function("kCFAlloc.*");
    builder = builder.whitelist_var("kCFAlloc.*");

    builder = builder.whitelist_type("kCMVideo.*");
    builder = builder.whitelist_function("kCMVideo.*");
    builder = builder.whitelist_var("kCMVideo.*");

    let meta_header: Vec<_> = headers.iter().map(|h| format!("#include <{}>\n", h)).collect();

    builder = builder.header_contents("video_toolbox.h", &meta_header.concat());

    builder = builder.trust_clang_mangling(false).derive_default(true);

    let bindings = builder.generate().expect("Unable to generate VideoToolbox bindings");

    bindings
        .write_to_file(out_dir.join("video_toolbox.rs"))
        .expect("Failed to write VideoToolbox bindings");
}

fn main() {
    let target = std::env::var("TARGET").unwrap();
    if !(target.contains("apple-darwin") || target.contains("apple-ios")) {
        panic!("video-toolbox-sys requires macos or ios target");
    }

    let directory = sdk_path(&target).ok();

    build(directory.as_ref().map(String::as_ref), &target);
}
