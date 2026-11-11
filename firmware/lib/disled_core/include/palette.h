/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Lamp colors by slot: the four built-in colors, which never change, then
 * custom colors set by the host. Colors are linear light at full brightness;
 * the brightness level scales them (color.h).
 */
#ifndef DISLED_PALETTE_H
#define DISLED_PALETTE_H

#include <stdbool.h>
#include <stdint.h>

#include "disled_types.h"

#define PALETTE_SLOTS        LAMP_SLOT_COUNT
#define PALETTE_FIXED_SLOTS  LAMP_COLOR_COUNT /* green, orange, red, warm white */
#define PALETTE_CUSTOM_SLOTS (PALETTE_SLOTS - PALETTE_FIXED_SLOTS)

/* Bit n set: slot n holds a color. */
#define PALETTE_FIXED_MASK ((uint16_t)((1u << PALETTE_FIXED_SLOTS) - 1u))

typedef struct {
    const rgb16_t *fixed; /* PALETTE_FIXED_SLOTS colors */
    rgb16_t custom[PALETTE_CUSTOM_SLOTS];
    uint16_t defined; /* bit per slot; the fixed bits are always set */
} palette_t;

/* The built-in colors only, every custom slot empty. */
void palette_init(palette_t *palette, const rgb16_t fixed[PALETTE_FIXED_SLOTS]);

bool palette_defined(const palette_t *palette, uint32_t slot);

/* The color of a slot, black for an empty or unknown one. */
rgb16_t palette_color(const palette_t *palette, uint32_t slot);

/* The light a slot gives. A built-in color is tuned by eye through the PLA
 * already; a custom color, which the host gives in linear light, takes the
 * gains of `balance` (color_balance), so that its white shows white. */
rgb16_t palette_light(const palette_t *palette, uint32_t slot, rgb16_t balance);

/* A custom color has its largest channel at COLOR_LINEAR_MAX, so that the
 * brightness level alone sets how bright it is. Black empties a slot. */
bool palette_color_is_valid(rgb16_t color);

/* Sets a custom slot, or empties it with black. Returns false and changes
 * nothing for a built-in or unknown slot, or an invalid color. */
bool palette_set(palette_t *palette, uint32_t slot, rgb16_t color);

#endif /* DISLED_PALETTE_H */
