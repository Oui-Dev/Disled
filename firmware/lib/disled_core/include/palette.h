/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Lamp colors by slot: the four built-in colors, which never change, then
 * custom colors set by the host. A color is a chromaticity, the brightness
 * set apart. The built-in colors are LED drives (led_cal.h), green and red
 * the LED's own primaries, given with the chromaticity of their drive. The
 * custom colors go through the LED calibration. The brightness level scales
 * the light.
 */
#ifndef DISLED_PALETTE_H
#define DISLED_PALETTE_H

#include <stdbool.h>
#include <stdint.h>

#include "disled_types.h"
#include "led_cal.h"

/* Bit n set: slot n holds a color. */
#define PALETTE_FIXED_MASK ((uint16_t)((1u << LAMP_COLOR_COUNT) - 1u))

/* A built-in color: its LED drive, its largest channel at 65535, and the
 * chromaticity of that drive. */
typedef struct {
    rgb16_t drive;
    xy16_t xy;
} palette_builtin_t;

typedef struct {
    const palette_builtin_t *fixed; /* LAMP_COLOR_COUNT colors */
    xy16_t custom[LAMP_SLOT_COUNT - LAMP_COLOR_COUNT];
    uint16_t defined; /* bit per slot; the fixed bits are always set */
} palette_t;

/* The built-in colors only, every custom slot empty. */
void palette_init(palette_t *palette, const palette_builtin_t fixed[LAMP_COLOR_COUNT]);

bool palette_defined(const palette_t *palette, uint32_t slot);

/* The chromaticity of a slot, 0, 0 for an empty or unknown one. */
xy16_t palette_color(const palette_t *palette, uint32_t slot);

/* The LED drive of a slot: that of a built-in color, a custom color through
 * the calibration (led_cal_drive). Black for an empty or unknown slot. */
rgb16_t palette_light(const palette_t *palette, uint32_t slot, const led_color_cal_t *cal);

/* A custom color is a chromaticity (led_cal_xy_valid); 0, 0 empties a slot. */
bool palette_color_is_valid(xy16_t color);

/* Sets a custom slot, or empties it with 0, 0. Returns false and changes
 * nothing for a built-in or unknown slot, or an invalid color. */
bool palette_set(palette_t *palette, uint32_t slot, xy16_t color);

#endif /* DISLED_PALETTE_H */
