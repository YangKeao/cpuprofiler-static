use std::path::{Path, PathBuf};
use std::{fs, io};

#[cfg(any(feature = "static_gperftools", feature = "static_unwind"))]
use std::process::Command;

#[cfg(any(feature = "static_gperftools", feature = "static_unwind"))]
fn get_c_flags() -> std::ffi::OsString {
    let original_cflags = std::env::var("CFLAGS").unwrap_or_default();
    format!("{} -fPIC", original_cflags).into()
}

#[cfg(any(feature = "static_gperftools", feature = "static_unwind"))]
fn get_cxx_flags() -> std::ffi::OsString {
    let original_cflags = std::env::var("CXXFLAGS").unwrap_or_default();
    format!("{} -fPIC", original_cflags).into()
}

#[cfg(any(feature = "static_gperftools", feature = "static_unwind"))]
fn is_directory_empty<P: AsRef<Path>>(p: P) -> std::io::Result<bool> {
    let mut entries = fs::read_dir(p)?;
    Ok(entries.next().is_none())
}

#[cfg(any(feature = "static_gperftools", feature = "static_unwind"))]
fn check() {
    let libs = vec!["third_party/gperftools", "third_party/libunwind"];

    for lib in libs {
        if is_directory_empty(lib).unwrap_or(true) {
            panic!(
                "Can't find library {}. You need to run `git submodule \
                 update --init --recursive` first to build the project.",
                lib
            );
        }
    }
}

#[cfg(feature = "static_gperftools")]
fn build_gperftools(source_root: &PathBuf) -> std::io::Result<PathBuf> {
    check();

    let target_gperftool_source_dir = {
        let mut source_root = source_root.clone();
        source_root.push("gperftools");
        source_root
    };

    let mut autogen = Command::new("./autogen.sh")
        .current_dir(&target_gperftool_source_dir)
        .spawn()?;
    autogen.wait()?;

    let mut configure = Command::new("./configure")
        .args(&[
            "--disable-heap-profiler",
            "--disable-heap-checker",
            "--disable-debugalloc",
            "--disable-shared",
        ])
        .env("CFLAGS", &get_c_flags())
        .env("CXXFLAGS", &get_cxx_flags())
        .current_dir(&target_gperftool_source_dir)
        .spawn()?;
    configure.wait()?;

    let cpu_num = num_cpus::get();
    let mut make = Command::new("make")
        .args(&[format!("-j{}", cpu_num), "--keep-going".to_owned()])
        .current_dir(&target_gperftool_source_dir)
        .spawn()?;
    make.wait()?;

    Ok(target_gperftool_source_dir)
}

#[cfg(not(feature = "static_gperftools"))]
fn build_gperftools(_: &PathBuf) -> std::io::Result<PathBuf> {
    unreachable!();
}

#[cfg(feature = "static_unwind")]
fn build_unwind(source_root: &PathBuf) -> std::io::Result<PathBuf> {
    check();

    let target_unwind_source_dir = {
        let mut source_root = source_root.clone();
        source_root.push("libunwind");
        source_root
    };

    let mut autogen = Command::new("./autogen.sh")
        .current_dir(&target_unwind_source_dir)
        .spawn()?;
    autogen.wait()?;

    let mut configure = Command::new("./configure")
        .args(&[
            "--disable-shared",
            "--disable-minidebuginfo",
            "--disable-zlibdebuginfo",
        ])
        .env("CFLAGS", &get_c_flags())
        .env("CXXFLAGS", &get_cxx_flags())
        .current_dir(&target_unwind_source_dir)
        .spawn()?;
    configure.wait()?;

    let cpu_num = num_cpus::get();
    let mut make = Command::new("make")
        .args(&[format!("-j{}", cpu_num), "--keep-going".to_owned()])
        .current_dir(&target_unwind_source_dir)
        .spawn()?;
    make.wait()?;

    Ok(target_unwind_source_dir)
}

#[cfg(not(feature = "static_unwind"))]
fn build_unwind(_: &PathBuf) -> std::io::Result<PathBuf> {
    unreachable!();
}

fn copy_source_files() -> std::io::Result<PathBuf> {
    let mut copy_options = fs_extra::dir::CopyOptions::new();
    copy_options.overwrite = true;
    copy_options.copy_inside = true;

    let third_party_source_dir = {
        let mut current = std::env::current_dir()?;
        current.push("third_party");
        current
    };

    let target_third_party_source_dir: PathBuf = {
        let out_dir: String = std::env::var("OUT_DIR").unwrap();
        PathBuf::from(out_dir)
    };

    match fs_extra::dir::copy(
        &third_party_source_dir,
        &target_third_party_source_dir,
        &copy_options,
    ) {
        Ok(_) => {
            let mut target = target_third_party_source_dir.clone();
            target.push("third_party");
            Ok(target)
        }
        Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
    }
}

fn main() -> std::io::Result<()> {
    let source_root = copy_source_files()?;

    if cfg!(feature = "static_gperftools") {
        let gperftools = build_gperftools(&source_root)?;
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=static=profiler");
        println!(
            "cargo:rustc-link-search=native={}/.libs",
            gperftools.display()
        );
    }

    if cfg!(feature = "static_unwind") {
        let unwind = build_unwind(&source_root)?;
        println!("cargo:rustc-link-lib=static=unwind");
        println!(
            "cargo:rustc-link-search=native={}/src/.libs",
            unwind.display()
        );
    }
    Ok(())
}
