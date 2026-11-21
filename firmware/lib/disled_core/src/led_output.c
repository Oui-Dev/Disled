/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "led_output.h"

#include <stddef.h>

#include "alert.h"
#include "color.h"
#include "disled_assert.h"
#include "mul_div.h"
#include "time_utils.h"

static void copy(uint32_t to[LED_CHANNEL_COUNT], const uint32_t from[LED_CHANNEL_COUNT])
{
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        to[c] = from[c];
    }
}

static bool same(const uint32_t a[LED_CHANNEL_COUNT], const uint32_t b[LED_CHANNEL_COUNT])
{
    return a[0] == b[0] && a[1] == b[1] && a[2] == b[2];
}

static uint16_t full_duty(const led_output_config_t *config)
{
    return (uint16_t)(config->pwm_period * PWM_DITHER_SUBTICKS);
}

void led_output_init(led_output_t *out, const led_output_config_t *config)
{
    *out = (led_output_t){ .config = config };
    if (config->light == NULL) {
        for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
            out->excess[c] = 1; /* the rounding of linear_duty */
        }
        return;
    }
    /* full less the lowest offset must fit the 16 bits of mul_div
     * (led_cal_duty), and the table must end within the period. */
    const uint32_t full = (uint32_t)config->pwm_period * PWM_DITHER_SUBTICKS;
    DISLED_ASSERT(full + (uint32_t)(-LED_OFFSET_MIN) <= 0xFFFFu);
    DISLED_ASSERT(config->pwm_period >= LED_LIGHT_TICKS);
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        DISLED_ASSERT(led_cal_light_valid(&config->light[c]));
        out->excess[c] = led_cal_excess(&config->light[c], full_duty(config));
    }
}

/* Without light tables: the duty of a light, rounded, at most full. */
static uint16_t linear_duty(uint32_t light, uint16_t full)
{
    /* light <= LED_LIGHT_FULL: mul_div gives at most 256 * full. */
    return (uint16_t)((mul_div(light, full, 65535u) + 128u) / 256u);
}

uint32_t led_output_duty_ma_x10(const power_config_t *power, const uint16_t duty[LED_CHANNEL_COUNT],
                                uint16_t full)
{
    /* duty <= 65535 and every current <= POWER_MAX_MA: 3 * 65535 * 1000 * 10
     * < 2^32. */
    uint32_t sum = 0;
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        sum += (uint32_t)duty[c] * power->full_scale_ma[c] * 10u;
    }
    return (sum + full / 2u) / full;
}

/* The light the lamp settings ask for, before the fade. */
static void target_for(const led_output_inputs_t *in, uint32_t target[LED_CHANNEL_COUNT])
{
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        target[c] = 0;
    }
    if (in->link == USB_LINK_SUSPENDED) {
        return;
    }
    if (in->show_override) {
        const uint16_t d[LED_CHANNEL_COUNT] = { in->override_drive.r, in->override_drive.g,
                                                in->override_drive.b };
        for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
            target[c] = (uint32_t)d[c] << 8; /* 65535 is LED_LIGHT_FULL */
        }
        return;
    }
    if (!in->on) {
        return;
    }
    const uint32_t y = color_perceived_to_linear(in->level_permille);
    const uint16_t d[LED_CHANNEL_COUNT] = { in->drive.r, in->drive.g, in->drive.b };
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        target[c] = mul_div((uint32_t)d[c] << 8, y, 65535u); /* <= LED_LIGHT_FULL */
    }
}

/* Linear light from `a` to `b`, t from 0 to 65535: never past `b`. */
static uint32_t mix(uint32_t a, uint32_t b, uint32_t t)
{
    return (b >= a) ? a + mul_div(b - a, t, 65535u) : a - mul_div(a - b, t, 65535u);
}

/* The fade result with the alert breath: the lamp level moved along the breath
 * in perceived brightness, the hue kept. shown x linear(breath level) /
 * linear(level) is the lamp color at the breath level. */
static void breathe(led_output_t *out, const led_output_inputs_t *in)
{
    const uint16_t base = color_perceived_to_linear(in->level_permille);
    if (in->breath == 0u || base == 0u) {
        copy(out->visible, out->shown);
        return;
    }
    const uint16_t level =
        alert_level_permille(in->level_permille, out->config->alert_depth_permille, in->breath);
    const uint32_t y = color_perceived_to_linear(level);
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        const uint32_t v = mul_div(out->shown[c], y, base);
        out->visible[c] = (v > LED_LIGHT_FULL) ? LED_LIGHT_FULL : v;
    }
}

void led_output_settle(led_output_t *out, uint32_t now_ms)
{
    copy(out->from, out->visible);
    copy(out->shown, out->visible);
    out->fade_start_ms = now_ms;
}

/* Scales `light` down, all channels alike so that the hue stays, until the
 * current of the duties it will give cannot exceed the cap: the duty of a lit
 * channel is at most light * full / LED_LIGHT_FULL plus its excess, and a
 * dark channel gets no pulse. */
static void cap_light(const led_output_t *out, uint32_t light[LED_CHANNEL_COUNT], uint16_t full)
{
    const power_config_t *power = out->config->power;
    uint32_t fixed = 0; /* the current the excesses may add, tenths of mA times full */
    uint32_t var = 0;   /* the current of the light, tenths of mA */
    uint32_t bound = 0; /* both per channel, which no duty above full can pass */
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        if (light[c] == 0u) {
            continue;
        }
        const uint32_t ma_x10 = (uint32_t)power->full_scale_ma[c] * 10u; /* <= 10000 */
        const uint32_t excess = (uint32_t)out->excess[c] * ma_x10;
        /* light * ma_x10 / LED_LIGHT_FULL, <= 10000: mul_div rounds to the
         * nearest before the division rounds up, which the half sub-tick of
         * slack in every excess covers. */
        const uint32_t own = (mul_div(light[c], ma_x10, 65535u) + 255u) / 256u;
        const uint32_t channel = own + (excess + full - 1u) / full;
        bound += (channel < ma_x10) ? channel : ma_x10;
        fixed += excess;
        var += own;
    }
    const uint32_t cap = (uint32_t)out->cap_ma * 10u;
    if (bound <= cap) {
        return; /* a pure color at 100 % keeps its full current */
    }
    fixed = (fixed + full - 1u) / full;
    if (fixed + var <= cap) {
        return;
    }
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        /* cap - fixed < var <= 30000: within the 16 bits of mul_div. */
        light[c] = (cap <= fixed) ? 0u : mul_div(light[c], cap - fixed, var);
    }
}

void led_output_update(led_output_t *out, const led_output_inputs_t *in, uint32_t now_ms,
                       uint16_t duty[LED_CHANNEL_COUNT])
{
    const led_output_config_t *cfg = out->config;
    const uint16_t full = full_duty(cfg);
    uint32_t target[LED_CHANNEL_COUNT];
    target_for(in, target);

    if (!same(target, out->target)) {
        copy(out->from, out->shown);
        copy(out->target, target);
        out->fade_start_ms = now_ms;
    }

    /* A fade ends once the target shows: mix never overshoots it. */
    if (!same(out->shown, out->target)) {
        const uint32_t elapsed = time_elapsed_ms(now_ms, out->fade_start_ms);
        if (cfg->fade_ms == 0u || elapsed >= cfg->fade_ms) {
            copy(out->shown, out->target);
        } else {
            const uint32_t t = (elapsed * 65535u) / cfg->fade_ms;
            for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
                out->shown[c] = mix(out->from[c], out->target[c], t);
            }
        }
    }

    out->cap_ma =
        (in->cap_override_ma != 0u) ? in->cap_override_ma : power_cap_ma(cfg->power, in->link);
    if (in->link == USB_LINK_SUSPENDED) {
        out->cap_ma = 0;
    }
    breathe(out, in);
    uint32_t light[LED_CHANNEL_COUNT];
    copy(light, out->visible);
    cap_light(out, light, full);
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        duty[c] = (cfg->light != NULL) ? led_cal_duty(&cfg->light[c], light[c], full)
                                       : linear_duty(light[c], full);
    }
    out->estimate_ma_x10 = led_output_duty_ma_x10(cfg->power, duty, full);
}

uint32_t led_output_estimate_ma_x10(const led_output_t *out)
{
    return out->estimate_ma_x10;
}

uint16_t led_output_cap_ma(const led_output_t *out)
{
    return out->cap_ma;
}
