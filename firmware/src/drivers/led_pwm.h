/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * RGB LED PWM on TIM3, channels 1 to 3, with duties in sub-ticks that the update
 * interrupt dithers. Channel 4, an internal compare that never drives the touch
 * pin PB1, gives a sample point away from the LED edges (N1 in hardware/README.md).
 */
#ifndef DISLED_LED_PWM_H
#define DISLED_LED_PWM_H

#include <stdint.h>

#include "disled_types.h"

/* Starts the PWM at `frequency_hz` with all channels at 0 %. */
void led_pwm_init(uint32_t frequency_hz);

/* Stops the timer with every output low (USB suspend), and restarts it at
 * 0 %. The touch sample point needs the timer: stop the touch first. */
void led_pwm_stop(void);
void led_pwm_start(void);

/* Timer ticks for 100 % duty. */
uint16_t led_pwm_period(void);

/* New duties in sub-ticks (red, green, blue), applied from the next
 * period. */
void led_pwm_set(const uint16_t duty[LED_CHANNEL_COUNT]);

/* Sample point: `callback` runs in interrupt context once per arming, in
 * the middle of the PWM period. */
typedef void (*led_pwm_sample_callback_t)(void);
void led_pwm_set_sample_callback(led_pwm_sample_callback_t callback);
void led_pwm_arm_sample(void);

#endif /* DISLED_LED_PWM_H */
