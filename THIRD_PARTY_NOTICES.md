# Third-party notices

Pinplay launches the following separately installed programs. They are not
linked into or bundled with the Pinplay binary.

## mpv

- Project: <https://mpv.io/>
- Source and license information: <https://github.com/mpv-player/mpv>
- mpv is commonly distributed under GPL-2.0-or-later; exact terms can depend on
  how a distributor builds it.

## yt-dlp

- Project: <https://github.com/yt-dlp/yt-dlp>
- License information: <https://github.com/yt-dlp/yt-dlp#license>
- The source project is released into the public domain under The Unlicense.
  Prebuilt executables can include components under additional licenses.
- Pinplay does not install, bundle, locate, or configure a JavaScript runtime.
  Optional runtime and EJS components used for site-specific extraction remain
  part of the user's yt-dlp environment and retain their own license terms.

## Rust crates

The Pinplay binary also contains Rust crates declared in `Cargo.toml` and pinned
in `Cargo.lock`, including clap, serde, serde_json, thiserror, and their
transitive dependencies. These projects are primarily available under MIT
and/or Apache-2.0 terms; `Cargo.lock` is the authoritative version inventory.
Anyone distributing prebuilt Pinplay binaries should generate and bundle a
complete license inventory for the exact release artifact.

All product names and trademarks belong to their respective owners.
