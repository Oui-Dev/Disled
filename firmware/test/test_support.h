/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Fixtures that several test suites share. PlatformIO puts test/ on the
 * include path of every suite.
 */
#ifndef DISLED_TEST_SUPPORT_H
#define DISLED_TEST_SUPPORT_H

#include <stdint.h>

#include "led_cal.h"
#include "touch_detector.h"

/* xorshift32: the same sequence on every run. */
static inline uint32_t next_random(uint32_t *state)
{
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    return *state;
}

/* A light table: no light up to `dark` ticks, then a straight rise to the
 * line of the long pulses, (16 n - offset) * 16, met at `meet` ticks, and
 * that line beyond. A rise above the 256 per tick of the line is a steep
 * start, as the slow gate edges give. */
static inline led_light_cal_t make_table(uint32_t dark, uint32_t meet, int16_t offset)
{
    led_light_cal_t t = { .offset = offset };
    const int32_t met = (16 * (int32_t)meet - offset) * 16;
    const int32_t rise_ticks = (int32_t)(meet - dark);
    for (uint32_t n = dark + 1u; n <= LED_LIGHT_TICKS; n++) {
        const int32_t line = (16 * (int32_t)n - offset) * 16;
        const int32_t rising = (met * (int32_t)(n - dark) + rise_ticks / 2) / rise_ticks;
        t.light[n] = (uint16_t)((n >= meet) ? line : rising);
    }
    return t;
}

/* A synthetic TouchKey signal, one sample per millisecond: a touch lowers the
 * reading by 5 % (50 per mille). */
#define BASELINE 2000u
#define TOUCHED  1900u

/* The reading `permille` below BASELINE. */
static inline uint16_t dropped(uint32_t permille)
{
    return (uint16_t)(BASELINE - (BASELINE * permille) / 1000u);
}

/* Detector settings for that signal. */
static const touch_config_t touch_config = {
    .window_samples = 20,
    .calibration_windows = 8,
    .baseline_shift = 6,
    .press_permille = 40,
    .release_permille = 25,
    .press_rise_permille = 18,
    .proximity_permille = 10,
    .debounce_ms = 60,
    .rearm_ms = 100,
    .long_press_ms = 2000,
    .object_ms = 10000,
    .inverted_permille = 30,
    .inverted_ms = 1000,
};

#endif /* DISLED_TEST_SUPPORT_H */
