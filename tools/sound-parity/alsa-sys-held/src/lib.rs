//! Holds `alsa-sys`'s place (ADR-0074). CPAL's Linux path needs `alsa-sys`, and
//! quad-snd's needs `quad-alsa-sys`, and each declares `links = "alsa"`, which Cargo
//! allows only once in a build, whatever the target. CPAL is built only for Windows
//! here, so nothing is compiled against this. The root `Cargo.toml` patches it in.
