/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Invariant checks. A failed check calls disled_assert_failed(), which in the
 * firmware records a fault for the next boot and resets the chip, and in the
 * native tests aborts.
 */
#ifndef DISLED_ASSERT_H
#define DISLED_ASSERT_H

#include <stdint.h>

#define DISLED_ASSERT(condition)                      \
    do {                                              \
        if (!(condition)) {                           \
            disled_assert_failed((uint32_t)__LINE__); \
        }                                             \
    } while (0)

__attribute__((noreturn)) void disled_assert_failed(uint32_t line);

#endif /* DISLED_ASSERT_H */
