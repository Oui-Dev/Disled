/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * The single path from the lamp settings to the PWM duties, through the fade
 * and the power limit. Nothing else drives the LED, so the limit always
 * applies; only the calibration commands of the debug console replace the
 * duties, under the same cap. It works on the light of each channel
 * (led_cal.h), so that a fade mixes light and the light tables turn it into
 * duties.
 */
#ifndef DISLED_LED_OUTPUT_H
#define DISLED_LED_OUTPUT_H

#include <stdbool.h>
#include <stdint.h>

#include "disled_types.h"
#include "led_cal.h"
#include "power_limiter.h"
#include "pwm_dither.h"

typedef struct {
    const power_config_t *power;
    /* One light table per channel, LED_CHANNEL_COUNT of them, or NULL for a
     * duty proportional to the light: the tables hold only for the timer
     * period they were measured at. */
    const led_light_cal_t *light;
    uint16_t fade_ms;
    /* Timer ticks for 100 % duty, so that 16 times it less LED_OFFSET_MIN
     * fits 16 bits. */
    uint16_t pwm_period;
    /* Perceived swing of an alert breath (alert.h), per mille. */
    uint16_t alert_depth_permille;
} led_output_config_t;

typedef struct {
    bool on;
    /* The lamp color at full brightness, as LED drive (led_cal_drive). */
    rgb16_t drive;
    uint16_t level_permille;
    usb_link_t link;
    /* Debug override of the current cap, 0 for the cap of the link. */
    uint16_t cap_override_ma;
    /* Self-test: this drive, at the brightness it carries, instead of the
     * lamp state, through the fade and the limit. */
    bool show_override;
    rgb16_t override_drive;
    /* Alert breath amplitude, 0 (none) to ALERT_AMPLITUDE_MAX, applied after the fade. */
    uint16_t breath;
} led_output_inputs_t;

typedef struct {
    const led_output_config_t *config;
    /* Light per channel, 0 to LED_LIGHT_FULL. */
    uint32_t from[LED_CHANNEL_COUNT];
    uint32_t target[LED_CHANNEL_COUNT];
    uint32_t shown[LED_CHANNEL_COUNT];   /* before the power limit */
    uint32_t visible[LED_CHANNEL_COUNT]; /* shown with the alert breath, before the power limit */
    uint16_t excess[LED_CHANNEL_COUNT];  /* led_cal_excess of each channel */
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

/* LED current of the last duties, in tenths of mA. */
uint32_t led_output_estimate_ma_x10(const led_output_t *out);

/* Cap applied by the last update, in mA. */
uint16_t led_output_cap_ma(const led_output_t *out);

/* Estimated LED current of duties in sub-ticks, in tenths of mA, for a 100 %
 * duty of `full` sub-ticks. */
uint32_t led_output_duty_ma_x10(const power_config_t *power, const uint16_t duty[LED_CHANNEL_COUNT],
                                uint16_t full);

#endif /* DISLED_LED_OUTPUT_H */
