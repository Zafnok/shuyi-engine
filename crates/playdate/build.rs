//! Declares `--cfg shuyi_playdate_sdk`, which `shuyi export playdate` sets
//! on the build that links the SDK. A cfg and no cargo feature: a build with
//! `--all-features` has `std`, whose panic handler and allocator the SDK's
//! would clash with.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(shuyi_playdate_sdk)");
}
