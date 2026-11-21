/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Scaling with 32-bit operations only: the chip has a 32-bit divider, and a
 * 64-bit division would pull about 2 KB of libgcc (docs/architecture.md).
 */
#ifndef DISLED_MUL_DIV_H
#define DISLED_MUL_DIV_H

#include <stdint.h>

/* a * num / den rounded to the nearest, saturated at UINT32_MAX. Any a;
 * num <= 0xFFFF; 0 < den <= 0xFFFF.
 *
 * With a = q * den + r and r < den: a * num / den = q * num + r * num / den.
 * r * num + den / 2 < 2^32, and q * num is split in 16-bit halves, each of
 * whose products fits 32 bits; the sums are checked for carries. */
static inline uint32_t mul_div(uint32_t a, uint32_t num, uint32_t den)
{
    const uint32_t q = a / den;
    const uint32_t r = a - q * den;
    const uint32_t low = (r * num + den / 2u) / den; /* <= num */
    const uint32_t high = (q >> 16) * num;
    if (high > 0xFFFFu) {
        return UINT32_MAX;
    }
    const uint32_t top = high << 16;
    const uint32_t middle = (q & 0xFFFFu) * num;
    if (middle > UINT32_MAX - top) {
        return UINT32_MAX;
    }
    const uint32_t sum = top + middle;
    return (low > UINT32_MAX - sum) ? UINT32_MAX : sum + low;
}

#endif /* DISLED_MUL_DIV_H */
