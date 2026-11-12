/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * TinyUSB configuration. The MCU and the USB port (USBD/FSDEV) come from
 * platformio.ini: CFG_TUSB_MCU=OPT_MCU_CH32V20X, CFG_TUD_WCH_USBIP_FSDEV=1.
 */
#ifndef DISLED_TUSB_CONFIG_H
#define DISLED_TUSB_CONFIG_H

#ifndef CFG_TUSB_MCU
#error "CFG_TUSB_MCU must be defined by the build"
#endif

#ifndef DISLED_DEBUG
#error "DISLED_DEBUG must be defined by the build (0 or 1)"
#endif

#define CFG_TUSB_OS    OPT_OS_NONE
#define CFG_TUSB_DEBUG 0
#define CFG_TUSB_MEM_SECTION
#define CFG_TUSB_MEM_ALIGN __attribute__((aligned(4)))

/* Device stack on root port 0, full speed. */
#define CFG_TUD_ENABLED        1
#define CFG_TUD_MAX_SPEED      OPT_MODE_FULL_SPEED
#define CFG_TUD_ENDPOINT0_SIZE 64

/* Classes: vendor HID always, CDC console in debug builds only. */
#define CFG_TUD_HID    1
#define CFG_TUD_CDC    DISLED_DEBUG
#define CFG_TUD_MSC    0
#define CFG_TUD_MIDI   0
#define CFG_TUD_VENDOR 0

/* One HID report = 1 report ID byte + 63 payload bytes. */
#define CFG_TUD_HID_EP_BUFSIZE 64

#define CFG_TUD_CDC_RX_BUFSIZE 64
#define CFG_TUD_CDC_TX_BUFSIZE 512
#define CFG_TUD_CDC_EP_BUFSIZE 64

#endif /* DISLED_TUSB_CONFIG_H */
