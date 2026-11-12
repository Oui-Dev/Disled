/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "disled_assert.h"

/* Unit tests: crash. The firmware overrides it in bsp_fault.c. */
__attribute__((weak)) void disled_assert_failed(uint32_t line)
{
    (void)line;
    __builtin_trap();
}
