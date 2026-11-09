/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * LED current budget (constraint N2 in hardware/README.md). The average
 * current is duty x full-scale current, and a color over the cap is scaled
 * down on all channels alike, which keeps its hue.
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

/* LED current allowed in a USB state; zero while suspended. */
uint16_t power_cap_ma(const power_config_t *config, usb_link_t link);

/* Estimated LED current of a linear light output, in tenths of mA. */
uint32_t power_estimate_ma_x10(const power_config_t *config, rgb16_t light);

/* Scales `light` down, all channels alike, so that its estimated current
 * does not exceed `cap_ma`. */
rgb16_t power_limit(const power_config_t *config, rgb16_t light, uint16_t cap_ma);

#endif /* DISLED_POWER_LIMITER_H */
