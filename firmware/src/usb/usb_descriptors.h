/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * USB identity, interfaces and endpoints of the device.
 */
#ifndef DISLED_USB_DESCRIPTORS_H
#define DISLED_USB_DESCRIPTORS_H

#include "tusb_config.h"

/*
 * pid.codes test PIDs (see LICENSING.md): release builds use 1209:0001 until
 * pid.codes grants 1209:D1ED, and debug builds, for private testing only,
 * keep 1209:0002. Debug builds expose an extra CDC interface; a separate PID
 * keeps hosts from mixing up the cached descriptors of the two builds.
 */
#define USB_VID 0x1209u
#if DISLED_DEBUG
#define USB_PID 0x0002u
#else
#define USB_PID 0x0001u
#endif

#define USB_MANUFACTURER_STRING "DISLED"
#define USB_PRODUCT_STRING      "DISLED"

/* Declared current draw: LED budget plus MCU, with margin (constraint N2
 * in hardware/README.md). */
#define USB_MAX_POWER_MA 300u

#define USB_HID_POLL_INTERVAL_MS 4u

enum {
    USB_ITF_HID = 0,
#if CFG_TUD_CDC
    USB_ITF_CDC_COMM,
    USB_ITF_CDC_DATA,
#endif
    USB_ITF_COUNT
};

enum {
    USB_EP_HID_OUT = 0x01,
    USB_EP_HID_IN = 0x81,
    USB_EP_CDC_NOTIF = 0x82,
    USB_EP_CDC_OUT = 0x03,
    USB_EP_CDC_IN = 0x83,
};

#endif /* DISLED_USB_DESCRIPTORS_H */
