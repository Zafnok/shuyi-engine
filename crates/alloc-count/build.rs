//! Declares `--cfg shuyi_no_heap_count`, the opt-out of the global heap
//! counter, so `check-cfg` knows it.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(shuyi_no_heap_count)");
    println!("cargo::rerun-if-changed=build.rs");
}
