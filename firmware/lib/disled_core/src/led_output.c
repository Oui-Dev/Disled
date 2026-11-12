/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "led_output.h"

#include <stddef.h>

#include "alert.h"
#include "disled_assert.h"
#include "time_utils.h"

static const rgb16_t black = { 0, 0, 0 };

static bool same_color(rgb16_t a, rgb16_t b)
{
    return a.r == b.r && a.g == b.g && a.b == b.b;
}

static bool curve_is_valid(const led_loss_curve_t *curve)
{
    for (size_t i = 0; i < LED_LOSS_POINTS; i++) {
        if (curve->loss[i] > LED_LOSS_MAX || (i > 0u && curve->signal[i] < curve->signal[i - 1u])) {
            return false;
        }
    }
    return true;
}

void led_output_init(led_output_t *out, const led_output_config_t *config)
{
    DISLED_ASSERT(config->pwm_period <= PWM_DITHER_PERIOD_MAX);
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        DISLED_ASSERT(curve_is_valid(&config->pulse_loss[i]));
    }
    *out = (led_output_t){ .config = config };
}

static rgb16_t target_for(const led_output_t *out, const led_output_inputs_t *in)
{
    if (in->link == USB_LINK_SUSPENDED) {
        return black;
    }
    if (in->show_override) {
        return in->override_rgb;
    }
    if (!in->on) {
        return black;
    }
    (void)out;
    return color_shade(in->color, in->level_permille);
}

/* The fade result with the alert breath: the lamp level moved along the breath
 * in perceived brightness, the hue kept. shown x linear(breath level) /
 * linear(level) is the lamp color at the breath level. */
static rgb16_t breathe(const led_output_t *out, const led_output_inputs_t *in)
{
    const uint16_t base = color_perceived_to_linear(in->level_permille);
    if (in->breath == 0u || base == 0u) {
        return out->shown;
    }
    const uint16_t level =
        alert_level_permille(in->level_permille, out->config->alert_depth_permille, in->breath);
    return color_gain(out->shown, color_perceived_to_linear(level), base);
}

void led_output_settle(led_output_t *out, uint32_t now_ms)
{
    out->from = out->visible;
    out->shown = out->visible;
    out->fade_start_ms = now_ms;
}

/* The loss of `curve` for a pulse of `signal` sub-ticks. */
static uint16_t loss_at(const led_loss_curve_t *curve, uint16_t signal)
{
    if (signal <= curve->signal[0]) {
        return curve->loss[0];
    }
    for (size_t i = 1u; i < LED_LOSS_POINTS; i++) {
        /* signal >= signal[i - 1] here, so the span of a point taken is never 0. */
        if (signal < curve->signal[i]) {
            const int32_t from = curve->loss[i - 1u];
            const int32_t to = curve->loss[i];
            const int32_t span = (int32_t)curve->signal[i] - (int32_t)curve->signal[i - 1u];
            const int32_t done = (int32_t)signal - (int32_t)curve->signal[i - 1u];
            return (uint16_t)(from + ((to - from) * done) / span);
        }
    }
    return curve->loss[LED_LOSS_POINTS - 1u];
}

/* period * 16 fits 16 bits, since init checks period <= PWM_DITHER_PERIOD_MAX. */
static uint16_t to_duty(uint16_t linear, uint16_t period, const led_loss_curve_t *loss)
{
    if (linear == 0u) {
        return 0;
    }
    const uint16_t full = (uint16_t)(period * LED_DUTY_SUBTICKS);
    const uint16_t signal = color_scale(linear, full);
    const uint32_t duty = (uint32_t)signal + loss_at(loss, signal);
    return (uint16_t)((duty > full) ? full : duty);
}

void led_output_update(led_output_t *out, const led_output_inputs_t *in, uint32_t now_ms,
                       uint16_t duty[LED_CHANNEL_COUNT])
{
    const led_output_config_t *cfg = out->config;
    const rgb16_t target = target_for(out, in);

    if (!same_color(target, out->target)) {
        out->from = out->shown;
        out->target = target;
        out->fade_start_ms = now_ms;
    }

    /* A fade ends once the target shows: color_mix never overshoots it. */
    if (!same_color(out->shown, out->target)) {
        const uint32_t elapsed = time_elapsed_ms(now_ms, out->fade_start_ms);
        if (cfg->fade_ms == 0u || elapsed >= cfg->fade_ms) {
            out->shown = out->target;
        } else {
            const uint16_t t = (uint16_t)((elapsed * COLOR_LINEAR_MAX) / cfg->fade_ms);
            out->shown = color_mix(out->from, out->target, t);
        }
    }

    out->cap_ma =
        (in->cap_override_ma != 0u) ? in->cap_override_ma : power_cap_ma(cfg->power, in->link);
    if (in->link == USB_LINK_SUSPENDED) {
        out->cap_ma = 0;
    }
    out->visible = breathe(out, in);
    const rgb16_t limited = power_limit(cfg->power, out->visible, out->cap_ma);
    out->estimate_ma_x10 = power_estimate_ma_x10(cfg->power, limited);

    duty[LED_CHANNEL_RED] = to_duty(limited.r, cfg->pwm_period, &cfg->pulse_loss[LED_CHANNEL_RED]);
    duty[LED_CHANNEL_GREEN] =
        to_duty(limited.g, cfg->pwm_period, &cfg->pulse_loss[LED_CHANNEL_GREEN]);
    duty[LED_CHANNEL_BLUE] =
        to_duty(limited.b, cfg->pwm_period, &cfg->pulse_loss[LED_CHANNEL_BLUE]);
}

uint32_t led_output_estimate_ma_x10(const led_output_t *out)
{
    return out->estimate_ma_x10;
}

uint16_t led_output_cap_ma(const led_output_t *out)
{
    return out->cap_ma;
}
