/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "power_limiter.h"

#include "color.h"

/* Every product below fits 32 bits as long as currents stay within
 * POWER_MAX_MA (3 * 65535 * 1000 * 10 < 2^32). */
_Static_assert(3ull * COLOR_LINEAR_MAX * POWER_MAX_MA * 10ull < (1ull << 32), "32-bit sums");

uint16_t power_cap_ma(const power_config_t *config, usb_link_t link)
{
    switch (link) {
        case USB_LINK_CONFIGURED:
            return config->configured_cap_ma;
        case USB_LINK_UNCONFIGURED:
            return config->unconfigured_cap_ma;
        case USB_LINK_SUSPENDED:
        default:
            return 0;
    }
}

uint32_t power_estimate_ma_x10(const power_config_t *config, rgb16_t light)
{
    const uint32_t sum = (uint32_t)light.r * config->full_scale_ma[LED_CHANNEL_RED] +
                         (uint32_t)light.g * config->full_scale_ma[LED_CHANNEL_GREEN] +
                         (uint32_t)light.b * config->full_scale_ma[LED_CHANNEL_BLUE];
    return (sum * 10u + COLOR_LINEAR_MAX / 2u) / COLOR_LINEAR_MAX;
}

rgb16_t power_limit(const power_config_t *config, rgb16_t light, uint16_t cap_ma)
{
    const uint32_t estimate = power_estimate_ma_x10(config, light);
    const uint32_t cap = (uint32_t)((cap_ma > POWER_MAX_MA) ? POWER_MAX_MA : cap_ma) * 10u;

    if (estimate <= cap) {
        return light;
    }
    /* Rounded down, so that the result never exceeds the cap. */
    const uint16_t factor = (uint16_t)((cap * COLOR_LINEAR_MAX) / estimate);
    const rgb16_t limited = {
        .r = (uint16_t)(((uint32_t)light.r * factor) / COLOR_LINEAR_MAX),
        .g = (uint16_t)(((uint32_t)light.g * factor) / COLOR_LINEAR_MAX),
        .b = (uint16_t)(((uint32_t)light.b * factor) / COLOR_LINEAR_MAX),
    };
    return limited;
}
