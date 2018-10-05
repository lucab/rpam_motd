//! Simple PAM module example: MOTD handler.

extern crate openat;
#[macro_use]
extern crate pamsm;
extern crate syslog;

use pamsm::{Pam, PamError, PamFlag, PamServiceModule};
use std::{collections, fs, io, path};

// Local logger type.
type ModLogger = syslog::Logger<syslog::LoggerBackend, String, syslog::Formatter3164>;

// PAM module, declaration and entrypoint.
pam_module!(RPamMotd);
struct RPamMotd;
impl PamServiceModule for RPamMotd {
    fn open_session(_h: Pam, _flags: PamFlag, args: Vec<String>) -> PamError {
        // Syslog initializer.
        let formatter = syslog::Formatter3164 {
            facility: syslog::Facility::LOG_AUTH,
            hostname: None,
            process: "rpam_motd".to_owned(),
            pid: 0,
        };
        let mut logger = match syslog::unix::<String, _>(formatter) {
            Ok(l) => l,
            _ => return PamError::SERVICE_ERR,
        };

        // Main logic and error handling.
        match run_motd(&mut logger, args) {
            Ok(code) => code,
            Err(e) => {
                let _ = logger.crit(e);
                PamError::SERVICE_ERR
            }
        }
    }
}

// Parse, iterate and print motd files and snippets.
fn run_motd(logger: &mut ModLogger, args: Vec<String>) -> Result<PamError, String> {
    // Parse arguments into files and dirs buckets.
    let (file_paths, dir_paths) = parse_args(logger, args);

    // Check for file entries, print if one is found.
    let motd_file = locate_file(logger, file_paths);
    if let Some(mut fp) = motd_file {
        io::copy(&mut fp, &mut io::stdout()).map_err(|_| "failed to print motd file content")?;
    }

    // Iterate through all dirs and print content (with overrides).
    let motd_snippets = locate_snippets(logger, &dir_paths);
    for (_snip_name, mut snip_fp) in motd_snippets {
        io::copy(&mut snip_fp, &mut io::stdout()).map_err(|_| "failed to print snippet content")?;
    }

    Ok(PamError::SUCCESS)
}

fn parse_args(_logger: &mut ModLogger, args: Vec<String>) -> (Vec<String>, Vec<String>) {
    // When no arguments are supplied, use the default paths.
    let opts = match args.len() {
        0 => vec![
            "motd=/etc/motd".to_owned(),
            "motd=/run/motd".to_owned(),
            "motd=/usr/lib/motd".to_owned(),
            "motd_dir=/etc/motd.d/".to_owned(),
            "motd_dir=/run/motd.d/".to_owned(),
            "motd_dir=/usr/lib/motd.d/".to_owned(),
        ],
        _ => args,
    };

    // Parse each argument and put it in the relevant bucket.
    let mut motd_files = vec![];
    let mut motd_dirs = vec![];
    for p in opts {
        let kv: Vec<&str> = p.splitn(2, '=').collect();
        // A path, either to a file or a to dir.
        let fpath = match kv.get(1) {
            Some(p) => p,
            None => continue,
        };
        // A keyword, to select either a file or a dir type.
        match kv.get(0) {
            Some(&"motd_dir") => motd_dirs.push(fpath.to_string()),
            Some(&"motd") => motd_files.push(fpath.to_string()),
            _ => continue,
        };
    }

    (motd_files, motd_dirs)
}

fn locate_file(_logger: &mut ModLogger, fpaths: Vec<String>) -> Option<fs::File> {
    let nullpath = path::PathBuf::from("/dev/null");
    for p in fpaths {
        // Null-symlinks just win over any other lower-priority entries.
        if let Ok(target) = fs::read_link(&p) {
            if target == nullpath {
                return None;
            }
        };

        // Halt at the first available file.
        if let Ok(fp) = fs::File::open(&p) {
            return Some(fp);
        }
    }
    None
}

fn locate_snippets(
    logger: &mut ModLogger,
    dpaths: &[String],
) -> collections::BTreeMap<String, fs::File> {
    let nullpath = path::PathBuf::from("/dev/null");

    // Name to fd map, in lexicographic order.
    let mut snips = collections::BTreeMap::new();

    // Iterate *in reverse order* to accommodate higher priority overrides.
    for dir in dpaths.iter().rev() {
        // Visit each directory and try to list it.
        let dirfd = match openat::Dir::open(dir) {
            Ok(f) => f,
            _ => continue,
        };
        let dir_list = match dirfd.list_dir(".") {
            Ok(dl) => dl,
            _ => {
                let _ = logger.notice(format!("failed to list '{}'", dir));
                continue;
            }
        };

        // Iterate on each sub-file.
        for item in dir_list {
            let snip_name = match item {
                Ok(sn) => sn,
                _ => continue,
            };
            let snip_key = snip_name.file_name().to_string_lossy().into_owned();

            // Null-symlinks just remove lower-priority entries.
            if let Ok(target) = dirfd.read_link(&snip_name) {
                if target == nullpath {
                    snips.remove(&snip_key);
                    continue;
                }
            }

            // Existing files may override lower-priority entries.
            if let Ok(fp) = dirfd.open_file(&snip_name) {
                snips.insert(snip_key, fp);
            }
        }
    }
    snips
}
