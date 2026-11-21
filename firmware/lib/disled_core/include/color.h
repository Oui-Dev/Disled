/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Brightness levels are perceived brightness (CIE 1976 L*), so that equal
 * steps look equal; the light follows from them.
 */
#ifndef DISLED_COLOR_H
#define DISLED_COLOR_H

#include <stdint.h>

#include "disled_types.h"

#define COLOR_LINEAR_MAX 65535u

typedef struct {
    /* Each built-in color as LED drive (led_cal.h), its largest channel at
     * COLOR_LINEAR_MAX. */
    rgb16_t palette[LAMP_COLOR_COUNT];
} color_config_t;

/* Perceived brightness (per mille of L*) to linear light, 0 to 65535. */
uint16_t color_perceived_to_linear(uint16_t level_permille);

/* The perceived brightness a lamp level shows: the levels from 1 % (10 per
 * mille) to 100 % spread over `floor_permille` to 1000 per mille, so that
 * level 1 % gives `floor_permille`. A floor of 10 changes nothing. */
uint16_t color_level_shown(uint16_t level_permille, uint16_t floor_permille);

#endif /* DISLED_COLOR_H */
