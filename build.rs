use std::path::{PathBuf, Path};
use std::{io, fs};
use std::process::Command;

fn is_directory_empty<P: AsRef<Path>>(p: P) -> Result<bool, io::Error> {
    let mut entries = fs::read_dir(p)?;
    Ok(entries.next().is_none())
}

fn prepare_gperftool() {
    let libs = vec![
        "third_party/gperftools",
        "third_party/libunwind",
    ];

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

#[cfg(feature = "gperftools")]
fn build_gperftools(source_root: &PathBuf) -> std::io::Result<PathBuf> {
    prepare_gperftool();

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

#[cfg(not(feature = "gperftools"))]
fn build_gperftools(_: &PathBuf) -> std::io::Result<PathBuf> {
    unreachable!();
}

#[cfg(feature = "unwind")]
fn build_unwind(source_root: &PathBuf) -> std::io::Result<PathBuf> {
    prepare_gperftool();

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
        .args(&["--disable-shared", "--disable-minidebuginfo", "--disable-zlibdebuginfo"])
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

#[cfg(not(feature = "unwind"))]
fn build_unwind(_: &PathBuf) -> std::io::Result<PathBuf> {
    unreachable!();
}

fn copy_source_files() -> std::io::Result<PathBuf> {
    let third_party_source_dir = {
        let mut current =  std::env::current_dir()?;
        current.push("third_party");
        current
    };

    let target_third_party_source_dir: PathBuf = {
        let out_dir: String = std::env::var("OUT_DIR").unwrap();
        PathBuf::from(format!("{}/third_party", out_dir))
    };

    let mut copy = Command::new("cp")
        .args(&[
            "-r",
            &format!("{}", third_party_source_dir.display()),
            &format!("{}", target_third_party_source_dir.display()),
        ])
        .spawn()?;
    copy.wait()?;

    Ok(target_third_party_source_dir)
}

fn main() -> std::io::Result<()> {
    let source_root = copy_source_files()?;

    if cfg!(feature = "gperftools") {
        let gperftools = build_gperftools(&source_root)?;
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=static=profiler");
        println!(
            "cargo:rustc-link-search=native={}/.libs",
            gperftools.display()
        );
    }

    if cfg!(feature = "unwind") {
        let unwind = build_unwind(&source_root)?;
        println!("cargo:rustc-link-lib=static=unwind");
        println!(
            "cargo:rustc-link-search=native={}/src/.libs",
            unwind.display()
        );
    }
    Ok(())
}