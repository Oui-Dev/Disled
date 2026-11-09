/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Domain types shared by the core modules. Explicit values: several of them
 * travel on the wire unchanged (see protocol/README.md).
 */
#ifndef DISLED_TYPES_H
#define DISLED_TYPES_H

#include <stdint.h>

/* The four built-in colors. With custom colors, a color is a palette slot:
 * these four, then the custom ones up to LAMP_SLOT_COUNT - 1. */
typedef enum {
    LAMP_COLOR_GREEN = 0,
    LAMP_COLOR_ORANGE = 1,
    LAMP_COLOR_RED = 2,
    LAMP_COLOR_WARM_WHITE = 3,
} lamp_color_t;

#define LAMP_COLOR_COUNT 4u

#define LAMP_SLOT_COUNT 16u

/* What the touch does. Integrations drive the color in power-only mode. */
typedef enum {
    TOUCH_MODE_NORMAL = 0,     /* short press: next color, long press: power */
    TOUCH_MODE_POWER_ONLY = 1, /* long press: power, short press ignored */
} touch_mode_t;

#define TOUCH_MODE_COUNT 2u

typedef enum {
    BRIGHTNESS_MODE_AUTO = 0,
    BRIGHTNESS_MODE_MANUAL = 1,
} brightness_mode_t;

/* Manual brightness range, in percent. */
#define BRIGHTNESS_MANUAL_MIN 1u
#define BRIGHTNESS_MANUAL_MAX 100u

/* USB link as seen by the power budget and the lamp. */
typedef enum {
    USB_LINK_UNCONFIGURED = 0, /* power-up, charger, hub without host */
    USB_LINK_CONFIGURED = 1,   /* enumerated by a host */
    USB_LINK_SUSPENDED = 2,    /* host asleep */
} usb_link_t;

/* What caused the latest state change. */
typedef enum {
    CHANGE_SOURCE_BOOT = 0,
    CHANGE_SOURCE_TOUCH = 1,
    CHANGE_SOURCE_HOST = 2,
    CHANGE_SOURCE_AUTO_BRIGHTNESS = 3,
    CHANGE_SOURCE_USB = 4,
    CHANGE_SOURCE_HOST_SILENCE = 5, /* touch mode back to normal: no command for too long */
} change_source_t;

/* Why the chip last reset. */
typedef enum {
    RESET_CAUSE_UNKNOWN = 0,
    RESET_CAUSE_POWER_ON = 1,
    RESET_CAUSE_PIN = 2,
    RESET_CAUSE_SOFTWARE = 3,
    RESET_CAUSE_WATCHDOG = 4,
    RESET_CAUSE_WINDOW_WATCHDOG = 5,
    RESET_CAUSE_LOW_POWER = 6,
} reset_cause_t;

/* LED channels, in PWM channel order. */
enum {
    LED_CHANNEL_RED = 0,
    LED_CHANNEL_GREEN = 1,
    LED_CHANNEL_BLUE = 2,
};

#define LED_CHANNEL_COUNT 3u

/* Linear light intensity per channel, 0 to 65535. */
typedef struct {
    uint16_t r;
    uint16_t g;
    uint16_t b;
} rgb16_t;

#endif /* DISLED_TYPES_H */
