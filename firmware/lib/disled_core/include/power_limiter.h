/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * LED current budget (constraint N2 in hardware/README.md): the cap in force,
 * and the current of PWM duties, a channel drawing its duty times its
 * full-scale current. led_output scales a color over the cap down on all
 * channels alike, which keeps its hue.
 */
#ifndef DISLED_POWER_LIMITER_H
#define DISLED_POWER_LIMITER_H

#include <stdint.h>

#include "disled_types.h"

/* Upper bound of every current in this module (keeps the math 32-bit). */
#define POWER_MAX_MA 1000u

typedef struct {
    uint16_t full_scale_ma[LED_CHANNEL_COUNT]; /* current at 100 % duty, <= POWER_MAX_MA */
    uint16_t unconfigured_cap_ma;              /* no host: USB allows 100 mA in total */
    uint16_t configured_cap_ma;                /* thermal limit of the closed cube */
} power_config_t;

/* The LED current allowed, in mA: none while suspended, else `override_ma`
 * unless it is 0 (the heat test of the debug console), else the cap of the
 * USB state. */
uint16_t power_cap_ma(const power_config_t *config, usb_link_t link, uint16_t override_ma);

/* The LED current of duties in sub-ticks, in tenths of mA, for a 100 % duty
 * of `full` sub-ticks. */
uint32_t power_duty_ma_x10(const power_config_t *config, const uint16_t duty[LED_CHANNEL_COUNT],
                           uint16_t full);

#endif /* DISLED_POWER_LIMITER_H */
