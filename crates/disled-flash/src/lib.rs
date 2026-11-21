// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Flashing a Disled lamp, shared by the desktop app and the production
//! tool: the files signed with tauri signer, the firmware files of the
//! releases among them, and the wchisp flasher.

pub mod firmware;
pub mod signature;
pub mod wchisp;
