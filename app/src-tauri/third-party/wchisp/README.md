# wchisp

The Disled app updates the lamp firmware with wchisp 0.3.0, a flasher for
WCH microcontrollers by the ch32-rs project
(<https://github.com/ch32-rs/wchisp>). wchisp is a separate program,
installed next to the Disled app, which runs it; it is not linked with the
app.

wchisp is free software under the GNU General Public License, version 2:
see `LICENSE`, next to this file. It comes with no warranty.

**Source.** `wchisp-0.3.0-source.tar.gz`, attached to every Disled app
release (<https://github.com/Oui-Dev/Disled/releases>), is the crates.io
package of wchisp that the release workflow builds, unchanged. Its
`Cargo.lock` pins the libraries compiled into wchisp, published on
<https://crates.io>; among them libusb (in `libusb1-sys`), under the GNU
Lesser General Public License, version 2.1 or later.

**Build.** `cargo install wchisp@0.3.0 --locked`, with
`RUSTFLAGS="-C target-feature=+crt-static"` on Windows.
