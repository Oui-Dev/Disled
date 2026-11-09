/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "brightness.h"

#include <stddef.h>

#include "time_utils.h"

/* Readings far apart, after a restart of the sensor, catch up this many ticks at most. */
#define MAX_TICKS 16u

/* log2(x) in Q8 for x >= 1, Mitchell's approximation: at most 6 % off in lux,
 * which the brightness curve does not show. */
static int32_t log2_q8(uint32_t x)
{
    if (x == 0u) {
        return 0;
    }
    uint32_t msb = 0;
    while ((x >> (msb + 1u)) != 0u) {
        msb++;
    }
    const uint32_t fraction =
        (msb >= 8u) ? ((x >> (msb - 8u)) & 0xFFu) : ((x << (8u - msb)) & 0xFFu);
    return (int32_t)((msb << 8) | fraction);
}

/* Curve level at a light given as log2_q8(lux x 1000). */
static uint16_t curve_at(const brightness_config_t *config, int32_t light_q8)
{
    const brightness_point_t *curve = config->curve;
    int32_t x0 = log2_q8(curve[0].lux_milli);
    if (light_q8 <= x0) {
        return curve[0].level_permille;
    }
    for (size_t i = 1; i < config->curve_points; i++) {
        const int32_t x1 = log2_q8(curve[i].lux_milli);
        if (light_q8 <= x1) {
            const int32_t y0 = curve[i - 1u].level_permille;
            const int32_t y1 = curve[i].level_permille;
            if (x1 <= x0) {
                return (uint16_t)y1;
            }
            return (uint16_t)(y0 + ((y1 - y0) * (light_q8 - x0)) / (x1 - x0));
        }
        x0 = x1;
    }
    return curve[config->curve_points - 1u].level_permille;
}

uint16_t brightness_curve_permille(const brightness_config_t *config, uint32_t lux_milli)
{
    return curve_at(config, log2_q8(lux_milli));
}

void brightness_init(brightness_t *b, const brightness_config_t *config)
{
    *b = (brightness_t){
        .config = config,
        .mode = BRIGHTNESS_MODE_AUTO,
        .manual_percent = BRIGHTNESS_MANUAL_MAX,
        .auto_permille = config->fallback_permille,
    };
}

/* Automatic level with the bias applied, 1 to 1000. */
static uint16_t biased(const brightness_t *b, uint32_t level_permille)
{
    const uint32_t factor = (uint32_t)(100 + (int32_t)b->bias_percent);
    uint32_t level = (level_permille * factor + 50u) / 100u;
    if (level > BRIGHTNESS_PERMILLE_MAX) {
        level = BRIGHTNESS_PERMILLE_MAX;
    }
    return (uint16_t)((level == 0u) ? 1u : level);
}

void brightness_set_mode(brightness_t *b, brightness_mode_t mode, uint8_t manual_percent)
{
    b->mode = mode;
    b->manual_percent = manual_percent;
}

void brightness_set_auto_bias(brightness_t *b, int8_t bias_percent)
{
    if (bias_percent < BRIGHTNESS_BIAS_MIN) {
        bias_percent = BRIGHTNESS_BIAS_MIN;
    } else if (bias_percent > BRIGHTNESS_BIAS_MAX) {
        bias_percent = BRIGHTNESS_BIAS_MAX;
    }
    b->bias_percent = bias_percent;
}

int8_t brightness_auto_bias(const brightness_t *b)
{
    return b->bias_percent;
}

/* Moves the filtered light toward a reading. */
static void follow(brightness_t *b, int32_t light_q8, uint32_t now_ms)
{
    const brightness_config_t *cfg = b->config;
    const int32_t difference = light_q8 - b->light_q8;
    const int32_t size = (difference < 0) ? -difference : difference;
    const int8_t sign = (int8_t)((difference < 0) ? -1 : 1);

    if (size < (int32_t)cfg->large_q8) {
        /* Kept while followed: a second change the same way needs no new wait. */
        if (!b->fast) {
            b->large_sign = 0;
        }
    } else if (b->large_sign != sign) {
        /* Wait until the change lasts: a hand or a shadow on the sensor must not move the
         * lamp, even while it follows a light just switched on or off. */
        b->fast = false;
        b->large_sign = sign;
        b->large_since_ms = now_ms;
        return;
    } else if (!b->fast) {
        if (time_elapsed_ms(now_ms, b->large_since_ms) < cfg->confirm_ms) {
            return;
        }
        b->fast = true;
    }
    if (size < (int32_t)cfg->settled_q8) {
        b->fast = false;
        b->large_sign = 0;
    }
    if (size < (int32_t)cfg->hold_q8) {
        return;
    }

    uint32_t ticks = (time_elapsed_ms(now_ms, b->last_ms) + cfg->tick_ms / 2u) / cfg->tick_ms;
    if (ticks < 1u) {
        ticks = 1u;
    } else if (ticks > MAX_TICKS) {
        ticks = MAX_TICKS;
    }
    const int32_t divisor = b->fast ? cfg->fast_divisor : cfg->slow_divisor;
    for (uint32_t i = 0; i < ticks; i++) {
        /* Divided, not shifted: shifting a negative value is implementation-defined.
         * The last steps are one unit each. */
        const int32_t error = light_q8 - b->light_q8;
        int32_t step = error / divisor;
        if (step == 0) {
            step = (error > 0) ? 1 : ((error < 0) ? -1 : 0);
        }
        b->light_q8 += step;
    }
}

void brightness_update_lux(brightness_t *b, uint32_t lux_milli, uint32_t now_ms)
{
    const int32_t light_q8 = log2_q8(lux_milli);
    if (b->have_lux) {
        follow(b, light_q8, now_ms);
    } else {
        /* First reading: start at the target instead of crawling to it. */
        b->have_lux = true;
        b->light_q8 = light_q8;
    }
    b->last_ms = now_ms;
    b->auto_permille = curve_at(b->config, b->light_q8);
}

void brightness_lux_lost(brightness_t *b)
{
    b->have_lux = false;
    b->fast = false;
    b->large_sign = 0;
}

uint16_t brightness_level_permille(const brightness_t *b)
{
    if (b->mode == BRIGHTNESS_MODE_MANUAL) {
        return (uint16_t)(b->manual_percent * 10u);
    }
    return biased(b, b->have_lux ? b->auto_permille : b->config->fallback_permille);
}

uint8_t brightness_level_percent(const brightness_t *b)
{
    return (uint8_t)((brightness_level_permille(b) + 5u) / 10u);
}
