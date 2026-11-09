/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Wrap-safe arithmetic on 32-bit millisecond timestamps.
 */
#ifndef DISLED_TIME_UTILS_H
#define DISLED_TIME_UTILS_H

#include <stdbool.h>
#include <stdint.h>

/* Milliseconds from `since` to `now`, correct across counter wrap-around. */
static inline uint32_t time_elapsed_ms(uint32_t now, uint32_t since)
{
    return now - since;
}

/* True once `now` has reached or passed `deadline` (within 24.8 days). */
static inline bool time_reached(uint32_t now, uint32_t deadline)
{
    return (int32_t)(now - deadline) >= 0;
}

#endif /* DISLED_TIME_UTILS_H */
