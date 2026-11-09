/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * The single path from the lamp settings to the PWM duties, through the fade
 * and the power limit. Nothing else may drive the LED, so the limit always
 * applies.
 */
#ifndef DISLED_LED_OUTPUT_H
#define DISLED_LED_OUTPUT_H

#include <stdbool.h>
#include <stdint.h>

#include "color.h"
#include "disled_types.h"
#include "power_limiter.h"
#include "pwm_dither.h"

/* Duties in sub-ticks, 1/16 of a tick: at low levels, whole ticks make visible steps. */
#define LED_DUTY_SUBTICKS PWM_DITHER_SUBTICKS

typedef struct {
    const power_config_t *power;
    uint16_t fade_ms;
    uint16_t pwm_period; /* timer ticks for 100 % duty, <= PWM_DITHER_PERIOD_MAX */
    /* Sub-ticks of each pulse lost to the driver edges, added to any lit
     * channel, or the few ticks of blue in a dim warm white give no light. */
    uint16_t pulse_loss[LED_CHANNEL_COUNT];
    /* Perceived swing of an alert breath (alert.h), per mille. */
    uint16_t alert_depth_permille;
} led_output_config_t;

typedef struct {
    bool on;
    rgb16_t color; /* the lamp color at full brightness, from the palette */
    uint16_t level_permille;
    usb_link_t link;
    /* Debug override of the current cap, 0 for the cap of the link. */
    uint16_t cap_override_ma;
    /* Self-test: this linear color instead of the lamp state, through the fade and the limit. */
    bool show_override;
    rgb16_t override_rgb;
    /* Alert breath amplitude, 0 (none) to ALERT_AMPLITUDE_MAX, applied after the fade. */
    uint16_t breath;
} led_output_inputs_t;

typedef struct {
    const led_output_config_t *config;
    rgb16_t from;
    rgb16_t target;
    rgb16_t shown;   /* before the power limit */
    rgb16_t visible; /* shown with the alert breath, before the power limit */
    uint32_t fade_start_ms;
    uint16_t cap_ma;
    uint32_t estimate_ma_x10;
} led_output_t;

/* Starts dark. */
void led_output_init(led_output_t *out, const led_output_config_t *config);

/* Writes the PWM duties for `now_ms`, in sub-ticks, red, green and blue. A
 * steady rate, 100 Hz for instance, keeps the fades smooth. */
void led_output_update(led_output_t *out, const led_output_inputs_t *in, uint32_t now_ms,
                       uint16_t duty[LED_CHANNEL_COUNT]);

/* An alert stopped in the middle of a breath: the next updates fade from the
 * light it left, instead of jumping back to the lamp level. */
void led_output_settle(led_output_t *out, uint32_t now_ms);

/* Estimated LED current after the limit, in tenths of mA. */
uint32_t led_output_estimate_ma_x10(const led_output_t *out);

/* Cap applied by the last update, in mA. */
uint16_t led_output_cap_ma(const led_output_t *out);

#endif /* DISLED_LED_OUTPUT_H */
