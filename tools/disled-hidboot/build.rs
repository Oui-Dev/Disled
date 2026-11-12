// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Compiles the hardware-independent core of the bootloader (C), which the
//! tests of src/protocol.rs drive: the host side and the bootloader logic
//! run together, without a board. The tool itself does not link it.

fn main() {
    let core = "../../firmware/bootloader/lib/bl_core";
    println!("cargo:rerun-if-changed={core}");
    cc::Build::new()
        .std("c11")
        .include(format!("{core}/include"))
        .file(format!("{core}/src/bl_proto.c"))
        .file(format!("{core}/src/bl_crc32.c"))
        .compile("bl_core");
}
