/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Fixtures and checks that several test suites share. PlatformIO puts test/
 * on the include path of every suite.
 */
#ifndef DISLED_TEST_SUPPORT_H
#define DISLED_TEST_SUPPORT_H

#include <stdint.h>
#include <stdio.h>
#include <unity.h>

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

/* What the sensitivity correction keeps on `cfg`, at every value a host can
 * send. A retune that breaks it fails here, whatever the exact thresholds. */
static inline void check_every_correction(const touch_config_t *cfg)
{
    const int32_t press = cfg->press_permille;
    const int32_t release = cfg->release_permille;
    const uint32_t release_floor = cfg->proximity_permille + cfg->proximity_permille / 2u;
    char step[24];
    for (int32_t asked = INT8_MIN; asked <= INT8_MAX; asked++) {
        touch_detector_t detector;
        touch_detector_init(&detector, cfg);
        touch_detector_set_sensitivity(&detector, (int8_t)asked);
        const int32_t percent = touch_detector_sensitivity(&detector);
        const touch_thresholds_t in_force = touch_detector_thresholds(&detector);
        snprintf(step, sizeof step, "asked %+d %%", (int)asked);

        /* Within the limits, on the side asked for, never further. */
        TEST_ASSERT_TRUE_MESSAGE(
            percent >= TOUCH_SENSITIVITY_MIN && percent <= TOUCH_SENSITIVITY_MAX, step);
        TEST_ASSERT_TRUE_MESSAGE(asked >= 0 ? (percent >= 0 && percent <= asked)
                                            : (percent < 0 && percent >= asked),
                                 step);
        /* The release stays under the press, at the configured ratio give or
         * take the rounding of each, and half a band above the proximity band. */
        TEST_ASSERT_TRUE_MESSAGE(in_force.release < in_force.press, step);
        const int32_t skew = (int32_t)in_force.release * press - (int32_t)in_force.press * release;
        TEST_ASSERT_TRUE_MESSAGE(2 * (skew < 0 ? -skew : skew) <= press + release, step);
        TEST_ASSERT_TRUE_MESSAGE(in_force.release >= release_floor, step);
        /* A touch never needs more than one and a half times the drop. */
        TEST_ASSERT_TRUE_MESSAGE(2 * (int32_t)in_force.press <= 3 * press + 1, step);
    }
}

#endif /* DISLED_TEST_SUPPORT_H */
