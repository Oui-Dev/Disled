/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Brightness level of the lamp, in per mille of perceived brightness. The
 * automatic mode follows the ambient light on a log scale, as the eye
 * adapts, and the manual mode takes the level of the host.
 */
#ifndef DISLED_BRIGHTNESS_H
#define DISLED_BRIGHTNESS_H

#include <stdbool.h>
#include <stdint.h>

#include "disled_types.h"

#define BRIGHTNESS_PERMILLE_MAX 1000u

typedef struct {
    uint32_t lux_milli;      /* ambient light, lux x 1000 */
    uint16_t level_permille; /* perceived brightness, 0 to 1000 */
} brightness_point_t;

/* Light differences are in log2 of the lux, Q8: 256 is a factor of 2. */
typedef struct {
    const brightness_point_t *curve; /* sorted by lux, at least 2 points */
    uint8_t curve_points;
    uint16_t tick_ms;           /* the filter takes one step per tick of elapsed time */
    uint8_t slow_divisor;       /* each step covers 1/divisor of the difference */
    uint8_t fast_divisor;       /* the same while following a large change */
    uint16_t large_q8;          /* a difference this large... */
    uint16_t confirm_ms;        /* ...lasting this long is followed fast */
    uint16_t settled_q8;        /* the fast filter ends this close to the light */
    uint16_t hold_q8;           /* smaller differences are ignored */
    uint16_t fallback_permille; /* automatic level without a lux reading */
} brightness_config_t;

/* Limits of the automatic brightness bias in percent, those of SET_AUTO_BIAS. */
#define BRIGHTNESS_BIAS_MIN (-50)
#define BRIGHTNESS_BIAS_MAX 50

typedef struct {
    const brightness_config_t *config;
    brightness_mode_t mode;
    uint8_t manual_percent;
    int8_t bias_percent;
    bool have_lux;
    bool fast;               /* following a large change */
    int8_t large_sign;       /* sign of a large difference confirmed or followed, else 0 */
    uint32_t large_since_ms; /* when that difference appeared */
    uint32_t last_ms;        /* time of the previous reading */
    int32_t light_q8;        /* filtered light, log2(lux x 1000) in Q8 */
    uint16_t auto_permille;  /* curve level of the filtered light, before the bias */
} brightness_t;

void brightness_init(brightness_t *b, const brightness_config_t *config);

/* `manual_percent` is 1 to 100. Back in automatic mode, the automatic level
 * applies at once, and the LED output fades to it. */
void brightness_set_mode(brightness_t *b, brightness_mode_t mode, uint8_t manual_percent);

/* The automatic level times (100 + bias) / 100, within 1 to 1000. The bias is
 * clamped to the BRIGHTNESS_BIAS_ limits, and manual mode ignores it. */
void brightness_set_auto_bias(brightness_t *b, int8_t bias_percent);
int8_t brightness_auto_bias(const brightness_t *b);

/* New ambient light reading, in lux x 1000, taken at `now_ms`. */
void brightness_update_lux(brightness_t *b, uint32_t lux_milli, uint32_t now_ms);

/* The sensor stopped answering: automatic mode falls back to `fallback_permille`. */
void brightness_lux_lost(brightness_t *b);

/* Level to apply now, 0 to 1000. */
uint16_t brightness_level_permille(const brightness_t *b);

/* The same in percent, rounded. */
uint8_t brightness_level_percent(const brightness_t *b);

/* Curve lookup without filtering. Exposed for tests. */
uint16_t brightness_curve_permille(const brightness_config_t *config, uint32_t lux_milli);

#endif /* DISLED_BRIGHTNESS_H */
