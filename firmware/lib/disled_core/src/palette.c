/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "palette.h"

_Static_assert(LAMP_SLOT_COUNT <= 16u, "the defined slots fit 16 bits");

static bool no_color(xy16_t color)
{
    return color.x == 0u && color.y == 0u;
}

void palette_init(palette_t *palette, const palette_builtin_t fixed[LAMP_COLOR_COUNT])
{
    *palette = (palette_t){ .fixed = fixed, .defined = PALETTE_FIXED_MASK };
}

bool palette_defined(const palette_t *palette, uint32_t slot)
{
    return slot < LAMP_SLOT_COUNT && (palette->defined & (1u << slot)) != 0u;
}

xy16_t palette_color(const palette_t *palette, uint32_t slot)
{
    if (!palette_defined(palette, slot)) {
        const xy16_t none = { 0, 0 };
        return none;
    }
    if (slot < LAMP_COLOR_COUNT) {
        return palette->fixed[slot].xy;
    }
    return palette->custom[slot - LAMP_COLOR_COUNT];
}

rgb16_t palette_light(const palette_t *palette, uint32_t slot, const led_color_cal_t *cal)
{
    if (slot < LAMP_COLOR_COUNT) {
        return palette->fixed[slot].drive;
    }
    /* An empty or unknown slot has no chromaticity: black. */
    return led_cal_drive(cal, palette_color(palette, slot));
}

bool palette_color_is_valid(xy16_t color)
{
    return no_color(color) || led_cal_xy_valid(color);
}

bool palette_set(palette_t *palette, uint32_t slot, xy16_t color)
{
    if (slot < LAMP_COLOR_COUNT || slot >= LAMP_SLOT_COUNT || !palette_color_is_valid(color)) {
        return false;
    }
    const uint16_t bit = (uint16_t)(1u << slot);
    palette->custom[slot - LAMP_COLOR_COUNT] = color;
    if (no_color(color)) {
        palette->defined = (uint16_t)(palette->defined & ~bit);
    } else {
        palette->defined = (uint16_t)(palette->defined | bit);
    }
    return true;
}
