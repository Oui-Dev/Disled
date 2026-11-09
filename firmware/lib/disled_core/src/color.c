/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
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

uint16_t color_scale(uint16_t value, uint16_t factor)
{
    return (uint16_t)(((uint32_t)value * factor + COLOR_LINEAR_MAX / 2u) / COLOR_LINEAR_MAX);
}

rgb16_t color_shade(rgb16_t base, uint16_t level_permille)
{
    const uint16_t intensity = color_perceived_to_linear(level_permille);
    const rgb16_t result = {
        .r = color_scale(base.r, intensity),
        .g = color_scale(base.g, intensity),
        .b = color_scale(base.b, intensity),
    };
    return result;
}

static uint16_t gain_channel(uint16_t value, uint16_t num, uint16_t den)
{
    /* 65535 * 65535 + 32767 still fits 32 bits. */
    const uint32_t scaled = ((uint32_t)value * num + den / 2u) / den;
    return (scaled > COLOR_LINEAR_MAX) ? (uint16_t)COLOR_LINEAR_MAX : (uint16_t)scaled;
}

rgb16_t color_gain(rgb16_t color, uint16_t num, uint16_t den)
{
    const rgb16_t result = {
        .r = gain_channel(color.r, num, den),
        .g = gain_channel(color.g, num, den),
        .b = gain_channel(color.b, num, den),
    };
    return result;
}

static uint16_t mix_channel(uint16_t from, uint16_t to, uint16_t t)
{
    /* t on 15 bits keeps span * t in 32 bits, with no 64-bit division from libgcc. */
    const int32_t span = (int32_t)to - (int32_t)from;
    const int32_t t15 = (int32_t)(t >> 1);
    const int32_t offset = (span * t15 + (span >= 0 ? 16383 : -16383)) / 32767;
    return (uint16_t)((int32_t)from + offset);
}

rgb16_t color_mix(rgb16_t from, rgb16_t to, uint16_t t)
{
    const rgb16_t result = {
        .r = mix_channel(from.r, to.r, t),
        .g = mix_channel(from.g, to.g, t),
        .b = mix_channel(from.b, to.b, t),
    };
    return result;
}
