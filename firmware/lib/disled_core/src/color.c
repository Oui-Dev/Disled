/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "color.h"

/* L* to relative luminance every 2.5 L*, scaled to 65535: Y = ((L* + 16) / 116)^3
 * above L* = 8, L* / 903.3 below. */
#define LUT_STEP_PERMILLE 25u
static const uint16_t perceived_lut[] = {
    0,     181,   363,   544,   738,   972,   1251,  1578,  1959,  2396,  2894,
    3456,  4087,  4790,  5569,  6429,  7373,  8406,  9530,  10750, 12071, 13495,
    15027, 16671, 18431, 20310, 22313, 24443, 26705, 29102, 31639, 34319, 37146,
    40124, 43258, 46550, 50005, 53628, 57421, 61388, 65535,
};

_Static_assert(sizeof(perceived_lut) / sizeof(perceived_lut[0]) == 1000u / LUT_STEP_PERMILLE + 1u,
               "one LUT entry per step from 0 to 1000 per mille");

uint16_t color_perceived_to_linear(uint16_t level_permille)
{
    if (level_permille >= 1000u) {
        return COLOR_LINEAR_MAX;
    }
    const uint32_t index = level_permille / LUT_STEP_PERMILLE;
    const uint32_t remainder = level_permille % LUT_STEP_PERMILLE;
    const uint32_t y0 = perceived_lut[index];
    const uint32_t y1 = perceived_lut[index + 1u];
    return (uint16_t)(y0 + ((y1 - y0) * remainder + LUT_STEP_PERMILLE / 2u) / LUT_STEP_PERMILLE);
}

uint16_t color_level_shown(uint16_t level_permille, uint16_t floor_permille)
{
    if (level_permille <= 10u) {
        return floor_permille;
    }
    if (level_permille >= 1000u) {
        return 1000u;
    }
    /* (level - 10) * (1000 - floor) < 990 * 1000: no overflow. */
    const uint32_t span = 1000u - floor_permille;
    return (uint16_t)(floor_permille + ((uint32_t)(level_permille - 10u) * span + 495u) / 990u);
}
