/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "touch_cycle.h"

_Static_assert(LAMP_SLOT_COUNT <= 32u, "a slot mask fits 32 bits");

void touch_cycle_init(touch_cycle_t *cycle)
{
    *cycle = (touch_cycle_t){
        .count = 3,
        .slots = { LAMP_COLOR_GREEN, LAMP_COLOR_ORANGE, LAMP_COLOR_RED },
    };
}

bool touch_cycle_is_valid(const uint8_t *slots, uint32_t count, uint32_t defined)
{
    if (count == 0u || count > TOUCH_CYCLE_MAX) {
        return false;
    }
    uint32_t seen = 0;
    for (uint32_t i = 0; i < count; i++) {
        if (slots[i] >= LAMP_SLOT_COUNT) {
            return false;
        }
        const uint32_t bit = 1u << slots[i];
        if ((defined & bit) == 0u || (seen & bit) != 0u) {
            return false;
        }
        seen |= bit;
    }
    return true;
}

bool touch_cycle_set(touch_cycle_t *cycle, const uint8_t *slots, uint32_t count, uint32_t defined)
{
    if (!touch_cycle_is_valid(slots, count, defined)) {
        return false;
    }
    *cycle = (touch_cycle_t){ .count = (uint8_t)count };
    for (uint32_t i = 0; i < count; i++) {
        cycle->slots[i] = slots[i];
    }
    return true;
}

lamp_color_t touch_cycle_next(const touch_cycle_t *cycle, lamp_color_t color)
{
    for (uint32_t i = 0; i < cycle->count; i++) {
        if (cycle->slots[i] == (uint32_t)color) {
            const uint32_t next = (i + 1u < cycle->count) ? i + 1u : 0u;
            return (lamp_color_t)cycle->slots[next];
        }
    }
    return (lamp_color_t)cycle->slots[0];
}

void touch_cycle_remove(touch_cycle_t *cycle, uint32_t slot)
{
    uint32_t kept = 0;
    for (uint32_t i = 0; i < cycle->count; i++) {
        if (cycle->slots[i] != slot) {
            cycle->slots[kept++] = cycle->slots[i];
        }
    }
    if (kept == 0u) {
        touch_cycle_init(cycle);
        return;
    }
    for (uint32_t i = kept; i < TOUCH_CYCLE_MAX; i++) {
        cycle->slots[i] = 0;
    }
    cycle->count = (uint8_t)kept;
}
