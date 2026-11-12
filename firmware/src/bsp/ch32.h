/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Single entry point to the WCH SDK header, whose warnings are silenced here
 * only. The SDK startup leaves main() in user mode (mstatus 0x88): a write to
 * mtvec or mstatus, or a jump to the ROM bootloader, needs a trap handler.
 */
#ifndef DISLED_BSP_CH32_H
#define DISLED_BSP_CH32_H

#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wstrict-prototypes"
#pragma GCC diagnostic ignored "-Wmissing-prototypes"
#pragma GCC diagnostic ignored "-Wshadow"
#include "ch32v20x.h"
#pragma GCC diagnostic pop

/* The startup enables the QingKe hardware prologue (INTSYSCR = 0x3), which saves
 * the caller-saved registers: the WCH "fast" attribute relies on it and emits only
 * the mret. All interrupts share one priority: none nests and overflows the hardware stack. */
#define BSP_ISR __attribute__((interrupt("WCH-Interrupt-fast")))

#endif /* DISLED_BSP_CH32_H */
