# rpam\_motd

[![Build Status](https://travis-ci.com/lucab/rpam_motd.svg?branch=master)](https://travis-ci.com/lucab/rpam_motd)

A small Rust implementation of `pam_motd`, with support for overlays and dropins.

This shows how to write a simple pure-Rust PAM module to display a
"message of the day" (`motd`).
At the same time, this enhances `pam_motd` to support looking up
paths with different priorities and merging multiple snippets.

By default, the following paths are sourced in this order:

 * `/etc/motd` (file)
 * `/run/motd` (file)
 * `/usr/lib/motd` (file)
 * `/etc/motd.d/` (snippets)
 * `/run/motd.d/` (snippets)
 * `/usr/lib/motd.d/` (snippets)

This project follows the systemd-style approach of overlaying
files and dropin snippets from multiple hierarchies (i.e. `/usr/lib`,
`/run`, and `/etc`).

It does not have any additional non-Rust runtime dependency, that is
it does not link against `libpam.so` on the target host.

# Demo

This crate produces a shared object (`librpam_motd.so`) which can be
directly installed as a PAM module, as follows:

```
$ cargo build --release
$ sudo install -o root -g root -m 0644 -D -s -Z target/release/librpam_motd.so /lib/x86_64-linux-gnu/security/rpam_motd.so
```

Please note that PAM modules path differs across distribution, so the
`/lib/x86_64-linux-gnu/security/` above may need to be adjusted to
match host setup.

Default configuration for this PAM module is available under
[config/rpam-sample](config/rpam-sample).

A live-action demo of that is in the following asciinema recording:

[![asciicast](https://asciinema.org/a/204664.png)](https://asciinema.org/a/204664)

# Disclaimer

This project is an early proof-of-concept, and it may expose some raw
edges or unexpected behavior.
