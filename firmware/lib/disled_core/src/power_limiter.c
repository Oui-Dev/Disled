/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "power_limiter.h"

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
