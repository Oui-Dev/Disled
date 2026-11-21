/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "power_limiter.h"

#include <stddef.h>

uint16_t power_cap_ma(const power_config_t *config, usb_link_t link, uint16_t override_ma)
{
    if (link == USB_LINK_SUSPENDED) {
        return 0;
    }
    if (override_ma != 0u) {
        return override_ma;
    }
    switch (link) {
        case USB_LINK_CONFIGURED:
            return config->configured_cap_ma;
        case USB_LINK_UNCONFIGURED:
            return config->unconfigured_cap_ma;
        default:
            return 0;
    }
}

uint32_t power_duty_ma_x10(const power_config_t *config, const uint16_t duty[LED_CHANNEL_COUNT],
                           uint16_t full)
{
    /* duty <= 65535 and every current <= POWER_MAX_MA: 3 * 65535 * 1000 * 10
     * < 2^32. */
    uint32_t sum = 0;
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        sum += (uint32_t)duty[c] * config->full_scale_ma[c] * 10u;
    }
    return (sum + full / 2u) / full;
}
