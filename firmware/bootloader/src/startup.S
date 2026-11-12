/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Reset entry of the bootloader, at address 0. Unlike the SDK startup, it
 * stays in machine mode with interrupts off: main() can then start the
 * application as a reset would, and the application's own startup sets its
 * CSRs, its trap vector (mtvec) and its stack.
 */
    .section .init, "ax", @progbits
    .globl _start
    .type _start, @function
_start:
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop
    la sp, _eusrstack
    /* Pipelining and instruction prediction, as the SDK startup sets them. */
    li t0, 0x1f
    csrw 0xbc0, t0
    /* Any exception resets the chip (unified entry, mtvec mode 0). */
    la t0, bl_trap
    csrw mtvec, t0
    /* Clear .bss. There is no initialized data (link.ld). */
    la a0, _sbss
    la a1, _ebss
1:
    bgeu a0, a1, 2f
    sw zero, 0(a0)
    addi a0, a0, 4
    j 1b
2:
    j main

    .align 2
bl_trap:
    j bl_system_reset
