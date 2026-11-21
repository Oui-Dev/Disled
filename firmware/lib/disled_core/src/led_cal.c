/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "led_cal.h"

#include <stddef.h>

#include "mul_div.h"

#define N          LED_LIGHT_TICKS
#define LU_SUBTICK 256u /* LU per sub-tick in the linear part */
#define TABLE_UNIT 16u  /* LU per unit of a table entry */

static const rgb16_t black = { 0, 0, 0 };

static uint32_t magnitude(int32_t x)
{
    return (x < 0) ? (uint32_t)(-x) : (uint32_t)x;
}

/* Right shifts that leave `x` with `bits` bits at most. */
static uint32_t shift_to_bits(uint32_t x, uint32_t bits)
{
    uint32_t shift = 0;
    while ((x >> shift) >> bits != 0u) {
        shift++;
    }
    return shift;
}

/* x / 2^shift rounded to the nearest, without shifting a negative value
 * (implementation defined in C). */
static int32_t shift_signed(int32_t x, uint32_t shift)
{
    const uint32_t half = (shift == 0u) ? 0u : (1u << (shift - 1u));
    const uint32_t m = (((uint32_t)((x < 0) ? -x : x)) + half) >> shift;
    return (x < 0) ? -(int32_t)m : (int32_t)m;
}

rgb16_t led_cal_drive(const led_color_cal_t *cal, rgb16_t srgb)
{
    const int32_t in[3] = { srgb.r, srgb.g, srgb.b };
    int32_t d[LED_CHANNEL_COUNT];
    /* The drive of the sRGB white, the matrix times (1, 1, 1), unscaled: 1 to
     * 32767 (led_cal_color_valid). */
    int32_t white[LED_CHANNEL_COUNT];
    uint32_t largest = 0;
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        /* The magnitudes of a row add up to 32767 at most, and in <= 65535:
         * |sum| < 2^31. */
        int32_t sum = 0;
        white[i] = 0;
        for (size_t j = 0; j < 3u; j++) {
            sum += (int32_t)cal->matrix[i][j] * in[j];
            white[i] += cal->matrix[i][j];
        }
        d[i] = sum;
        if (magnitude(sum) > largest) {
            largest = magnitude(sum);
        }
    }
    if (largest == 0u) {
        return black;
    }
    /* 15 significant bits, rounded, so that |d| <= 32768 in the products
     * below: a relative resolution of 2^-15. */
    const uint32_t shift = shift_to_bits(largest, 15u);
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        d[i] = shift_signed(d[i], shift);
    }

    /* Outside the gamut, add k white, k = max(-d / white) over the negative
     * channels: the channel m that sets k reads 0. Only the direction of the
     * white counts, since its scale cancels out below. The cross products
     * stay at most 32768 * 32767 < 2^30. */
    size_t m = LED_CHANNEL_COUNT;
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        if (d[i] < 0 && (m == LED_CHANNEL_COUNT || -d[i] * white[m] > -d[m] * white[i])) {
            m = i;
        }
    }
    uint32_t e[LED_CHANNEL_COUNT];
    uint32_t top = 0;
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        if (m == LED_CHANNEL_COUNT) {
            e[i] = (uint32_t)d[i]; /* no channel is negative */
        } else {
            /* white[m] * (d[i] + k * white[i]) = d[i] * white[m] - d[m] *
             * white[i], from 0 to 2 * 32768 * 32767 < 2^31. */
            e[i] = (uint32_t)(d[i] * white[m] - d[m] * white[i]);
        }
        if (e[i] > top) {
            top = e[i];
        }
    }
    if (top == 0u) {
        return black;
    }

    /* The largest channel to 65535, both sides on 16 bits: (e >> s) * 65535
     * <= 65535^2 < 2^32. */
    const uint32_t s = shift_to_bits(top, 16u);
    const uint32_t den = top >> s;
    uint16_t out[LED_CHANNEL_COUNT];
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        out[i] = (uint16_t)(((e[i] >> s) * 65535u + den / 2u) / den);
    }
    const rgb16_t result = { out[LED_CHANNEL_RED], out[LED_CHANNEL_GREEN], out[LED_CHANNEL_BLUE] };
    return result;
}

uint16_t led_cal_duty(const led_light_cal_t *cal, uint32_t light, uint16_t full)
{
    if (light == 0u) {
        return 0;
    }
    if (light > LED_LIGHT_FULL) {
        light = LED_LIGHT_FULL;
    }
    /* The full light is (full - offset) * 256 LU; full - offset fits the 16
     * bits mul_div takes (checked by the caller at init). */
    const uint32_t span = (uint32_t)((int32_t)full - cal->offset);
    const uint32_t lu = mul_div(light, span, 65535u);
    const uint32_t table_top = (uint32_t)cal->light[N] * TABLE_UNIT;
    if (lu >= table_top) {
        /* The linear part: lu / 256 >= 16 N - offset, so the duty is at
         * least 16 N. */
        const int32_t duty = cal->offset + (int32_t)((lu + LU_SUBTICK / 2u) / LU_SUBTICK);
        return (uint16_t)((duty > (int32_t)full) ? full : duty);
    }
    /* The tick where the light passes lu: light[low] <= lu < light[high].
     * Flat stretches, with no light yet, never satisfy it. */
    uint32_t low = 0;
    uint32_t high = N;
    while (high - low > 1u) {
        const uint32_t middle = (low + high) / 2u;
        if ((uint32_t)cal->light[middle] * TABLE_UNIT <= lu) {
            low = middle;
        } else {
            high = middle;
        }
    }
    const uint32_t base = (uint32_t)cal->light[low] * TABLE_UNIT;
    const uint32_t step = (uint32_t)cal->light[low + 1u] * TABLE_UNIT - base; /* > 0 */
    /* (lu - base) * 16 < step * 16 < 2^24. */
    const uint32_t sub = ((lu - base) * PWM_DITHER_SUBTICKS + step / 2u) / step; /* 0 to 16 */
    if (base == 0u && sub == 0u) {
        return 0; /* nearer to dark than to the first light */
    }
    if (sub == 0u) {
        /* On a flat stretch, the shortest pulse gives the same light. */
        while (low > 0u && cal->light[low - 1u] == cal->light[low]) {
            low--;
        }
    }
    return (uint16_t)(low * PWM_DITHER_SUBTICKS + sub);
}

uint32_t led_cal_light(const led_light_cal_t *cal, uint16_t duty)
{
    const uint32_t n = duty / PWM_DITHER_SUBTICKS;
    const uint32_t f = duty % PWM_DITHER_SUBTICKS;
    if (n >= N) {
        /* duty >= 16 N > offset */
        return (uint32_t)((int32_t)duty - cal->offset) * LU_SUBTICK;
    }
    /* The dithering shows f periods of n + 1 ticks in 16: the linear mix. */
    const uint32_t a = (uint32_t)cal->light[n] * TABLE_UNIT;
    const uint32_t b = (uint32_t)cal->light[n + 1u] * TABLE_UNIT;
    return a + ((b - a) * f) / PWM_DITHER_SUBTICKS;
}

uint16_t led_cal_excess(const led_light_cal_t *cal, uint16_t full)
{
    /* light * full / LED_LIGHT_FULL is lu / 256 * full / (full - offset).
     * The duty chosen exceeds it:
     * - by the duty less the light / 256 of a whole tick, the largest within
     *   a tick since both are linear in between, or by the offset in the
     *   linear part;
     * - by half a step of the table, the duty being the nearest one, at
     *   least half a sub-tick in the linear part;
     * - with a negative offset, by up to 16 N * |offset| / full in the table,
     *   where full / (full - offset) shrinks light / 256. Near the top of
     *   the table the shrink grows a little past that, but there the first
     *   term is negative (16 n - light / 256 < 16 n - 16 N <= 0), which
     *   covers it. */
    uint32_t worst = (cal->offset > 0) ? (uint32_t)cal->offset : 0u;
    /* The rise of a table entry over one tick is also the rise of the light,
     * in LU, per sub-tick within that tick; the linear part rises 256 LU per
     * sub-tick. */
    uint32_t rise_max = LU_SUBTICK;
    for (uint32_t n = 1; n <= N; n++) {
        /* light / 256, in sub-ticks */
        const uint32_t lit = (uint32_t)cal->light[n] / PWM_DITHER_SUBTICKS;
        const uint32_t duty = n * PWM_DITHER_SUBTICKS;
        if (duty > lit && duty - lit > worst) {
            worst = duty - lit;
        }
        const uint32_t rise = (uint32_t)cal->light[n] - cal->light[n - 1u];
        if (rise > rise_max) {
            rise_max = rise;
        }
    }
    /* The nearest duty overshoots the light by half a sub-tick's rise at
     * most: rise_max / 2 LU, that is rise_max / 512 sub-ticks, rounded up. */
    worst += (rise_max + 2u * LU_SUBTICK - 1u) / (2u * LU_SUBTICK);
    if (cal->offset < 0) {
        worst += (N * PWM_DITHER_SUBTICKS * (uint32_t)(-cal->offset) + full - 1u) / full;
    }
    return (uint16_t)worst;
}

bool led_cal_color_valid(const led_color_cal_t *cal)
{
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        uint32_t magnitudes = 0;
        int32_t white = 0; /* channel i of the drive of the sRGB white */
        for (size_t j = 0; j < 3u; j++) {
            magnitudes += magnitude(cal->matrix[i][j]);
            white += cal->matrix[i][j];
        }
        if (magnitudes > 32767u || white <= 0) {
            return false;
        }
    }
    return true;
}

bool led_cal_light_valid(const led_light_cal_t *cal)
{
    if (cal->light[0] != 0u || cal->offset < LED_OFFSET_MIN ||
        cal->offset >= (int32_t)(N * PWM_DITHER_SUBTICKS)) {
        return false;
    }
    for (uint32_t n = 1; n <= N; n++) {
        if (cal->light[n] < cal->light[n - 1u]) {
            return false;
        }
    }
    return (int32_t)cal->light[N] ==
           ((int32_t)(N * PWM_DITHER_SUBTICKS) - cal->offset) * (int32_t)(LU_SUBTICK / TABLE_UNIT);
}
