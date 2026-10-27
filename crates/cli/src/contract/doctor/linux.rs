//! `doctor` on a Linux host: the C linker, `pkg-config` and the system
//! libraries the backend links and opens, one check a row of [`ROWS`].
//! A failed check's help is one install command for the distribution's
//! package manager (read from `/etc/os-release`), or one per manager when
//! the distribution is unknown. `scripts/setup-linux.sh` installs the same
//! packages, in the same order (a test compares them).

use std::path::Path;

use super::{Check, Probe, fail, first_line, pass};

/// How `doctor` finds what a row needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Find {
    /// A program and its arguments that succeed when it is there; the
    /// first line they print is what was found.
    Run(&'static str, &'static [&'static str]),
    /// A shared library, by its soname: in `ldconfig -p`, or as a file in
    /// one of [`LIB_DIRS`].
    Soname(&'static str),
}

/// One thing a Linux build needs, and its package on each manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Row {
    /// The check's name.
    pub(crate) name: &'static str,
    /// How `doctor` finds it.
    pub(crate) find: Find,
    /// Its package on Debian and Ubuntu.
    pub(crate) apt: &'static str,
    /// Its package on Fedora and RHEL.
    pub(crate) dnf: &'static str,
    /// Its package on Arch.
    pub(crate) pacman: &'static str,
}

/// What a Linux build needs, in order: the first four to build, the last
/// four to open a window.
pub(crate) const ROWS: &[Row] = &[
    // To build: rustc links every binary and build script with `cc`.
    Row {
        name: "cc",
        find: Find::Run("cc", &["--version"]),
        apt: "build-essential",
        dnf: "gcc",
        pacman: "base-devel",
    },
    // To build: `libudev-sys` (under gilrs) finds libudev with it.
    Row {
        name: "pkg-config",
        find: Find::Run("pkg-config", &["--version"]),
        apt: "pkg-config",
        dnf: "pkgconf-pkg-config",
        pacman: "pkgconf",
    },
    // To build: `quad-alsa-sys` (under quad-snd) links `libasound`.
    Row {
        name: "lib-alsa",
        find: Find::Run("pkg-config", &["--modversion", "alsa"]),
        apt: "libasound2-dev",
        dnf: "alsa-lib-devel",
        pacman: "alsa-lib",
    },
    // To build: `libudev-sys` (under gilrs) links libudev.
    Row {
        name: "lib-udev",
        find: Find::Run("pkg-config", &["--modversion", "libudev"]),
        apt: "libudev-dev",
        dnf: "systemd-devel",
        pacman: "systemd-libs",
    },
    // To open a window: miniquad's X11 backend loads it.
    Row {
        name: "lib-x11",
        find: Find::Soname("libX11.so.6"),
        apt: "libx11-dev",
        dnf: "libX11-devel",
        pacman: "libx11",
    },
    // To open a window: miniquad's X11 backend loads it.
    Row {
        name: "lib-xi",
        find: Find::Soname("libXi.so.6"),
        apt: "libxi-dev",
        dnf: "libXi-devel",
        pacman: "libxi",
    },
    // To open a window: miniquad's X11 backend loads it.
    Row {
        name: "lib-xkbcommon",
        find: Find::Soname("libxkbcommon.so.0"),
        apt: "libxkbcommon-dev",
        dnf: "libxkbcommon-devel",
        pacman: "libxkbcommon",
    },
    // To open a window: miniquad's GLX loads it.
    Row {
        name: "lib-gl",
        find: Find::Soname("libGL.so.1"),
        apt: "libgl1-mesa-dev",
        dnf: "mesa-libGL-devel",
        pacman: "libglvnd",
    },
];

/// Where a soname is looked for when `ldconfig -p` doesn't name it.
pub(crate) const LIB_DIRS: [&str; 4] = [
    "/usr/lib/x86_64-linux-gnu",
    "/usr/lib/aarch64-linux-gnu",
    "/usr/lib64",
    "/usr/lib",
];

/// The file naming the distribution.
const OS_RELEASE: &str = "/etc/os-release";

/// What the help says after the install command.
const SCRIPT_HINT: &str = "; in a game made with `shuyi new`, `bash scripts/setup-linux.sh` \
                           installs them all";

/// A distribution's package manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Manager {
    Apt,
    Dnf,
    Pacman,
}

impl Manager {
    /// The command that installs `row`'s package, without `sudo`.
    fn install(self, row: &Row) -> String {
        match self {
            Self::Apt => format!("apt-get install -y {}", row.apt),
            Self::Dnf => format!("dnf install -y {}", row.dnf),
            Self::Pacman => format!("pacman -S --needed {}", row.pacman),
        }
    }
}

/// The package manager of the distribution `os_release` describes: its
/// `ID`, then each word of its `ID_LIKE`; none for any other.
fn manager(os_release: &str) -> Option<Manager> {
    let value = |key: &str| {
        os_release.lines().find_map(|line| {
            let value = line.trim().strip_prefix(key)?.strip_prefix('=')?;
            Some(value.trim().trim_matches(['"', '\'']).to_owned())
        })
    };
    let ids = [value("ID"), value("ID_LIKE")];
    ids.iter()
        .flatten()
        .flat_map(|ids| ids.split_whitespace())
        .find_map(|id| match id {
            "debian" | "ubuntu" => Some(Manager::Apt),
            "fedora" | "rhel" | "centos" => Some(Manager::Dnf),
            "arch" => Some(Manager::Pacman),
            _ => None,
        })
}

/// The help of a failed `row`: its install command for `manager`, or one
/// per manager, each with `sudo` unless `root`.
fn help(row: &Row, manager: Option<Manager>, root: bool) -> String {
    let sudo = if root { "" } else { "sudo " };
    let command = |m: Manager| format!("`{sudo}{}`", m.install(row));
    let commands = match manager {
        Some(m) => command(m),
        None => format!(
            "{}, {} or {}",
            command(Manager::Apt),
            command(Manager::Dnf),
            command(Manager::Pacman)
        ),
    };
    format!("{commands}{SCRIPT_HINT}")
}

/// Where `soname` is: the path `ldconfig` (its `-p` output) gives, or
/// else a file in one of [`LIB_DIRS`].
fn soname_at(probe: &dyn Probe, ldconfig: Option<&str>, soname: &str) -> Option<String> {
    let listed = ldconfig.and_then(|text| {
        text.lines().find_map(|line| {
            let (name, rest) = line.trim().split_once(' ')?;
            (name == soname).then(|| rest.rsplit("=> ").next().unwrap_or(rest).trim().to_owned())
        })
    });
    listed.or_else(|| {
        LIB_DIRS
            .iter()
            .map(|dir| format!("{dir}/{soname}"))
            .find(|path| probe.exists(Path::new(path)))
    })
}

/// A check for each row of [`ROWS`], in order, on the Linux machine
/// `probe` looks at.
pub(super) fn checks(probe: &dyn Probe) -> Vec<Check> {
    let manager = probe
        .read(Path::new(OS_RELEASE))
        .as_deref()
        .and_then(manager);
    let root = probe
        .output("id", &["-u"])
        .is_some_and(|uid| uid.trim() == "0");
    let ldconfig = probe.output("ldconfig", &["-p"]);
    ROWS.iter()
        .map(|row| {
            let found = match row.find {
                Find::Run(program, args) => probe.output(program, args).map(|t| first_line(&t)),
                Find::Soname(soname) => soname_at(probe, ldconfig.as_deref(), soname),
            };
            match found {
                Some(found) => pass(row.name, found),
                None => fail(row.name, None, help(row, manager, root)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::tests::{Fake, rust};
    use super::super::{DoctorInput, checks as doctor_checks, doctor, text};
    use super::*;

    const UBUNTU: &str = "PRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\nNAME=\"Ubuntu\"\nID=ubuntu\n\
                          ID_LIKE=debian\n";

    /// A Linux machine with Rust, rustup's targets, these os-release
    /// lines, and every row's program, module and file but `missing`.
    fn linux(os_release: Option<&str>, missing: &str) -> Fake {
        let mut machine = rust("x86_64-unknown-linux-gnu").with(
            "rustup target list --installed",
            "x86_64-unknown-linux-gnu\nwasm32-unknown-unknown\n",
        );
        if let Some(text) = os_release {
            machine = machine.reading(OS_RELEASE, text);
        }
        for row in ROWS.iter().filter(|row| row.name != missing) {
            match row.find {
                Find::Run(program, args) => {
                    let command = std::iter::once(program)
                        .chain(args.iter().copied())
                        .collect::<Vec<_>>()
                        .join(" ");
                    machine = machine.with(&command, &format!("{} 1.0\nmore\n", row.name));
                }
                Find::Soname(soname) => {
                    machine
                        .files
                        .push(PathBuf::from(format!("/usr/lib/x86_64-linux-gnu/{soname}")));
                }
            }
        }
        machine
    }

    fn names(checks: &[Check]) -> Vec<&str> {
        checks.iter().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn an_ubuntu_host_with_everything_passes_every_row_in_order() {
        let answer = doctor(&linux(Some(UBUNTU), ""), &DoctorInput::default());
        assert!(answer.ok, "{answer:?}");
        let output = answer.result.unwrap();
        assert_eq!(
            names(&output.checks),
            [
                "rust",
                "cargo",
                "rustup",
                "cc",
                "pkg-config",
                "lib-alsa",
                "lib-udev",
                "lib-x11",
                "lib-xi",
                "lib-xkbcommon",
                "lib-gl",
                "wasm-target"
            ]
        );
        assert!(output.checks.iter().all(|c| c.ok && c.help.is_none()));
        let found = |name: &str| {
            let check = output.checks.iter().find(|c| c.name == name).unwrap();
            check.found.as_deref().unwrap()
        };
        assert_eq!(found("cc"), "cc 1.0");
        assert_eq!(found("lib-alsa"), "lib-alsa 1.0");
        assert_eq!(found("lib-gl"), "/usr/lib/x86_64-linux-gnu/libGL.so.1");
        assert!(answer.limits.is_empty(), "{:?}", answer.limits);
    }

    #[test]
    fn each_row_missing_on_ubuntu_says_its_apt_command() {
        for row in ROWS {
            let output = doctor_checks(&linux(Some(UBUNTU), row.name));
            let failed: Vec<&Check> = output.checks.iter().filter(|c| !c.ok).collect();
            assert_eq!(failed.len(), 1, "{failed:?}");
            assert_eq!(
                (failed[0].name.as_str(), &failed[0].found),
                (row.name, &None)
            );
            assert_eq!(
                failed[0].help.as_deref(),
                Some(
                    format!(
                        "`sudo apt-get install -y {}`; in a game made with `shuyi new`, `bash \
                         scripts/setup-linux.sh` installs them all",
                        row.apt
                    )
                    .as_str()
                ),
            );
            // As root, no sudo.
            let root = linux(Some(UBUNTU), row.name).with("id -u", "0\n");
            let check = doctor_checks(&root)
                .checks
                .into_iter()
                .find(|c| c.name == row.name)
                .unwrap();
            assert_eq!(
                check.help.as_deref(),
                Some(
                    format!(
                        "`apt-get install -y {}`; in a game made with `shuyi new`, `bash \
                         scripts/setup-linux.sh` installs them all",
                        row.apt
                    )
                    .as_str()
                ),
            );
        }
        // A user who isn't root keeps sudo.
        let user = linux(Some(UBUNTU), "lib-xi").with("id -u", "1000\n");
        let help = doctor_checks(&user).checks[8].help.clone().unwrap();
        assert!(
            help.starts_with("`sudo apt-get install -y libxi-dev`;"),
            "{help}"
        );
    }

    #[test]
    fn the_distribution_picks_the_package_manager() {
        let help = |os_release: Option<&str>| {
            let output = doctor_checks(&linux(os_release, "lib-gl"));
            let check = output.checks.iter().find(|c| c.name == "lib-gl").unwrap();
            let help = check.help.clone().unwrap();
            help.strip_suffix(SCRIPT_HINT).unwrap().to_owned()
        };
        assert_eq!(
            help(Some("NAME=\"Fedora Linux\"\nID=fedora\n")),
            "`sudo dnf install -y mesa-libGL-devel`"
        );
        assert_eq!(
            help(Some("ID=\"rhel\"\n")),
            "`sudo dnf install -y mesa-libGL-devel`"
        );
        assert_eq!(
            help(Some("ID=almalinux\nID_LIKE='rhel centos fedora'\n")),
            "`sudo dnf install -y mesa-libGL-devel`"
        );
        assert_eq!(
            help(Some("NAME=\"Arch Linux\"\nID=arch\n")),
            "`sudo pacman -S --needed libglvnd`"
        );
        assert_eq!(
            help(Some("ID=pop\nID_LIKE=\"ubuntu debian\"\n")),
            "`sudo apt-get install -y libgl1-mesa-dev`"
        );
        assert_eq!(
            help(Some("ID=debian\n")),
            "`sudo apt-get install -y libgl1-mesa-dev`"
        );
        let all = "`sudo apt-get install -y libgl1-mesa-dev`, `sudo dnf install -y \
                   mesa-libGL-devel` or `sudo pacman -S --needed libglvnd`";
        assert_eq!(help(Some("ID=gentoo\n")), all);
        assert_eq!(help(Some("ID_LIKEWISE=ubuntu\nVARIANT_ID=arch\n")), all);
        assert_eq!(help(None), all);
    }

    #[test]
    fn a_soname_is_found_through_ldconfig_or_in_a_library_folder() {
        let ldconfig = "1234 libs found in cache `/etc/ld.so.cache'\n\
                        \tlibXi.so.6.1 (libc6,x86-64) => /lib/x86_64-linux-gnu/libXi.so.6.1\n\
                        \tlibXi.so.6 (libc6,x86-64) => /lib/x86_64-linux-gnu/libXi.so.6\n";
        let listed = linux(Some(UBUNTU), "lib-xi").with("ldconfig -p", ldconfig);
        let output = doctor_checks(&listed);
        let xi = output.checks.iter().find(|c| c.name == "lib-xi").unwrap();
        assert!(xi.ok, "{xi:?}");
        assert_eq!(
            xi.found.as_deref(),
            Some("/lib/x86_64-linux-gnu/libXi.so.6")
        );
        // Without ldconfig, in /usr/lib64.
        let mut fedora = linux(Some("ID=fedora\n"), "lib-x11");
        assert!(!doctor_checks(&fedora).checks[7].ok);
        fedora.files.push(PathBuf::from("/usr/lib64/libX11.so.6"));
        let x11 = &doctor_checks(&fedora).checks[7];
        assert_eq!(
            (x11.name.as_str(), x11.found.as_deref()),
            ("lib-x11", Some("/usr/lib64/libX11.so.6"))
        );
        for dir in LIB_DIRS {
            let mut machine = linux(None, "lib-gl");
            machine
                .files
                .push(PathBuf::from(format!("{dir}/libGL.so.1")));
            assert!(doctor_checks(&machine).checks[10].ok, "{dir}");
        }
    }

    #[test]
    fn other_hosts_get_no_linux_checks() {
        for host in [
            "x86_64-pc-windows-gnu",
            "x86_64-pc-windows-msvc",
            "aarch64-apple-darwin",
        ] {
            let output = doctor_checks(
                &linux(Some(UBUNTU), "")
                    .with("rustc -vV", &format!("rustc 1.98.1\nhost: {host}\n")),
            );
            assert!(!output.checks.iter().any(|c| c.name == "cc"), "{host}");
        }
        let mut unknown = linux(Some(UBUNTU), "");
        unknown.programs.remove("rustc -vV");
        let output = doctor_checks(&unknown);
        assert_eq!(
            names(&output.checks),
            ["rust", "cargo", "rustup", "wasm-target"]
        );
        // Any Linux host gets them.
        let arm = linux(Some(UBUNTU), "").with(
            "rustc -vV",
            "rustc 1.98.1\nhost: aarch64-unknown-linux-musl\n",
        );
        assert_eq!(doctor_checks(&arm).checks.len(), 12);
    }

    #[test]
    fn text_prints_a_failed_linux_checks_help() {
        let output = doctor_checks(&linux(Some("ID=arch\n"), "pkg-config"));
        let said = text(&output);
        assert!(
            said.contains(
                "  FAIL pkg-config\n       help: `sudo pacman -S --needed pkgconf`; in a game made \
                 with `shuyi new`, `bash scripts/setup-linux.sh` installs them all\n"
            ),
            "{said}"
        );
        assert!(said.contains("  ok   lib-alsa: lib-alsa 1.0\n"), "{said}");
    }
}
