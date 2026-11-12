/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Linker script of the bootloader: the first 4 KB of the flash, one write-
 * protect sector. The application starts at 0x1000 (lib/bl_core/include/
 * bl_core.h). No initialized data: the startup only clears .bss.
 */
OUTPUT_ARCH("riscv")
ENTRY(_start)

MEMORY
{
    FLASH (rx)  : ORIGIN = 0x00000000, LENGTH = 4K
    RAM   (xrw) : ORIGIN = 0x20000000, LENGTH = 10K
}

__stack_size = 512;

SECTIONS
{
    .init :
    {
        KEEP(*(.init))
    } >FLASH

    .text :
    {
        *(.text .text.*)
        *(.rodata .rodata.* .srodata .srodata.*)
        . = ALIGN(4);
    } >FLASH

    .data :
    {
        *(.data .data.* .sdata .sdata.*)
    } >RAM AT>FLASH

    .bss (NOLOAD) :
    {
        . = ALIGN(4);
        _sbss = .;
        *(.sbss .sbss.* .bss .bss.* COMMON)
        . = ALIGN(4);
        _ebss = .;
    } >RAM

    /* Every variable within reach of gp: one instruction per access. */
    __global_pointer$ = ORIGIN(RAM) + 0x800;

    .stack ORIGIN(RAM) + LENGTH(RAM) - __stack_size (NOLOAD) :
    {
        _susrstack = .;
        . += __stack_size;
        _eusrstack = .;
    } >RAM
}

ASSERT(SIZEOF(.data) == 0, "the bootloader has no initialized data: the startup does not copy it")
ASSERT(_ebss <= ORIGIN(RAM) + 0x1000, "variables out of reach of gp")
