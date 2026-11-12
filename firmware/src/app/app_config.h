/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Every tunable of the firmware, in one place, with the reason for its
 * value. docs/bringup.md checks them on a new board.
 */
#ifndef DISLED_APP_CONFIG_H
#define DISLED_APP_CONFIG_H

#include "brightness.h"
#include "color.h"
#include "drivers/tkey.h"
#include "led_output.h"
#include "power_limiter.h"
#include "self_test.h"
#include "touch_detector.h"

/* LED PWM: above 20 kHz no audible capacitor whine. TIM3 runs at 72 MHz
 * (bsp.c), so a period has 3600 ticks; the dithering splits each into
 * sixteenths for the very dim evening levels. */
#define APP_PWM_FREQUENCY_HZ 20000u
#define APP_LED_FADE_MS      300u
#define APP_OUTPUT_PERIOD_MS 10u

/* Light sensor: the task polls, and reads once per integration
 * (drivers/veml7700.h). */
#define APP_ALS_PERIOD_MS      100u
#define APP_ALS_MAX_ERRORS     5u    /* consecutive failures before giving up */
#define APP_ALS_RETRY_MS       5000u /* then retry the sensor this often */
#define APP_ALS_FIRST_RETRY_MS 200u  /* sooner at start-up, in case it was still booting */
#define APP_I2C_SPEED_HZ       100000u

/* Host notifications caused by automatic brightness: at most this often. */
#define APP_AUTO_LEVEL_NOTIFY_MS 500u

/* Delay between the ACK of a RESET command and the reset itself. */
#define APP_RESET_DELAY_MS 50u

#define APP_WATCHDOG_MS 1000u

/* Alert breath: perceived swing, per mille of L*, up from a lamp below 70 %
 * and down from a brighter one. Chosen by eye on a closed lamp. Deeper, the
 * light sensor sees the breaths of a dim lamp in a dark room, and the
 * automatic level climbs during the alert: 3 points at 400, 8 at 500. */
#define APP_ALERT_DEPTH_PERMILLE 300u
/* The console's alert, the same as the app's: 3 breaths of 1.5 s, chosen by
 * eye on a closed lamp. */
#define APP_ALERT_DEFAULT_COUNT     3u
#define APP_ALERT_DEFAULT_PERIOD_MS 1500u

/* Power-only touch falls back to normal after this long without a command.
 * In that mode, the app sends a SET_STATE of the touch mode alone every 3 s. */
#define APP_HOST_SILENCE_MS 10000u

extern const tkey_config_t app_tkey_config;
extern const touch_config_t app_touch_config;
extern const brightness_config_t app_brightness_config;
extern const color_config_t app_color_config;
extern const power_config_t app_power_config;
/* Light lost per PWM pulse, per channel, in ns: points of the pulse that the
 * light needs and of the loss added to it (led_loss_curve_t). */
typedef struct {
    uint16_t signal_ns[LED_LOSS_POINTS];
    uint16_t loss_ns[LED_LOSS_POINTS];
} app_loss_curve_t;
extern const app_loss_curve_t app_led_pulse_loss[LED_CHANNEL_COUNT];
/* Gains of the custom colors, fractions of 65535 (palette_light). */
extern const rgb16_t app_led_white_balance;
extern const self_test_config_t app_self_test_config;

#endif /* DISLED_APP_CONFIG_H */
