/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "palette.h"

#include "color.h"

_Static_assert(LAMP_SLOT_COUNT <= 16u, "the defined slots fit 16 bits");

void palette_init(palette_t *palette, const rgb16_t fixed[LAMP_COLOR_COUNT])
{
    *palette = (palette_t){ .fixed = fixed, .defined = PALETTE_FIXED_MASK };
}

bool palette_defined(const palette_t *palette, uint32_t slot)
{
    return slot < LAMP_SLOT_COUNT && (palette->defined & (1u << slot)) != 0u;
}

rgb16_t palette_color(const palette_t *palette, uint32_t slot)
{
    if (!palette_defined(palette, slot)) {
        const rgb16_t black = { 0, 0, 0 };
        return black;
    }
    if (slot < LAMP_COLOR_COUNT) {
        return palette->fixed[slot];
    }
    return palette->custom[slot - LAMP_COLOR_COUNT];
}

rgb16_t palette_light(const palette_t *palette, uint32_t slot, const led_color_cal_t *cal)
{
    const rgb16_t color = palette_color(palette, slot);
    return (slot < LAMP_COLOR_COUNT) ? color : led_cal_drive(cal, color);
}

bool palette_color_is_valid(rgb16_t color)
{
    const bool black = color.r == 0u && color.g == 0u && color.b == 0u;
    const bool normalized =
        color.r == COLOR_LINEAR_MAX || color.g == COLOR_LINEAR_MAX || color.b == COLOR_LINEAR_MAX;
    return black || normalized;
}

bool palette_set(palette_t *palette, uint32_t slot, rgb16_t color)
{
    if (slot < LAMP_COLOR_COUNT || slot >= LAMP_SLOT_COUNT || !palette_color_is_valid(color)) {
        return false;
    }
    const uint16_t bit = (uint16_t)(1u << slot);
    palette->custom[slot - LAMP_COLOR_COUNT] = color;
    if (color.r == 0u && color.g == 0u && color.b == 0u) {
        palette->defined = (uint16_t)(palette->defined & ~bit);
    } else {
        palette->defined = (uint16_t)(palette->defined | bit);
    }
    return true;
}
