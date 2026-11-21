/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * The calibration of the LED, measured on a closed lamp (docs/bringup.md):
 * - color: linear sRGB to the light of each LED channel, through a 3x3
 *   matrix built from the primaries of the LED;
 * - light: per channel, the light of a pulse of each length in whole ticks,
 *   since short pulses lose light to the slow gate edges (note N4 in
 *   hardware/README.md), to turn a light into a PWM duty.
 *
 * Units:
 * - drive: the light of a channel at full brightness, 65535 being the
 *   light of that channel at 100 % duty (rgb16_t);
 * - light: the same with 8 more bits, LED_LIGHT_FULL at 100 %, for the
 *   dimmest levels;
 * - LU: 1/256 of the light that one more sub-tick gives a long pulse;
 * - duty: sub-ticks, 1/16 of a timer tick (pwm_dither.h).
 */
#ifndef DISLED_LED_CAL_H
#define DISLED_LED_CAL_H

#include <stdbool.h>
#include <stdint.h>

#include "disled_types.h"
#include "pwm_dither.h"

#define LED_LIGHT_FULL  (65535u * 256u)
#define LED_LIGHT_TICKS 128u /* whole ticks in a light table */
#define LED_OFFSET_MIN  (-1024)

typedef struct {
    /* Linear sRGB to drive: one row per LED channel, one column per input
     * channel r, g, b. The magnitudes of each row add up to 32767 at most,
     * so that no product overflows 32 bits. */
    int16_t matrix[LED_CHANNEL_COUNT][3];
    /* The drive of the sRGB white, the matrix times (1, 1, 1), its largest
     * entry at 65535. No entry is 0: the white lies inside the LED gamut. */
    uint16_t white[LED_CHANNEL_COUNT];
} led_color_cal_t;

typedef struct {
    /* Beyond the table, a duty of d sub-ticks gives (d - offset) * 256 LU:
     * a long pulse loses `offset` sub-ticks of light to its edges, or gains
     * them when the offset is negative. From LED_OFFSET_MIN to below
     * 16 * LED_LIGHT_TICKS. */
    int16_t offset;
    /* Light of a pulse of n whole ticks, n from 0 to LED_LIGHT_TICKS, in
     * units of 16 LU: light[0] is 0, no entry is below the one before, and
     * the last one meets the linear part, (16 * LED_LIGHT_TICKS - offset) *
     * 16. A dithered duty between n and n + 1 ticks gives the linear mix of
     * their lights. */
    uint16_t light[LED_LIGHT_TICKS + 1u];
} led_light_cal_t;

/* A linear sRGB color, as SET_COLOR carries it, to the drive that shows it,
 * its largest channel at 65535. A color outside the LED gamut first takes
 * the least white that brings every channel to 0 or above: the hue stays,
 * the color is less saturated. Black stays black. */
rgb16_t led_cal_drive(const led_color_cal_t *cal, rgb16_t srgb);

/* The duty, in sub-ticks, nearest to `light` (0 to LED_LIGHT_FULL) on a
 * channel whose 100 % duty is `full` sub-ticks. Nearer to dark than to the
 * first light of the table, no pulse at all. */
uint16_t led_cal_duty(const led_light_cal_t *cal, uint32_t light, uint16_t full);

/* The light in LU of a duty in sub-ticks, as the dithering shows it. */
uint32_t led_cal_light(const led_light_cal_t *cal, uint16_t duty);

/* A bound, in sub-ticks, on what a duty from led_cal_duty can exceed light *
 * full / LED_LIGHT_FULL: what the current cap must allow for above the
 * light. */
uint16_t led_cal_excess(const led_light_cal_t *cal, uint16_t full);

/* The table starts at 0, never decreases and meets its linear part, and the
 * offset is in range. */
bool led_cal_light_valid(const led_light_cal_t *cal);

/* The magnitudes of each row of the matrix add up to 32767 at most, and no
 * entry of the white is 0: what led_cal_drive needs. */
bool led_cal_color_valid(const led_color_cal_t *cal);

#endif /* DISLED_LED_CAL_H */
