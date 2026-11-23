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
#include "led_cal.h"
#include "led_output.h"
#include "palette.h"
#include "power_limiter.h"
#include "self_test.h"
#include "touch_detector.h"

/* LED PWM: above 20 kHz no audible capacitor whine. TIM3 runs at 72 MHz
 * (bsp.c), so a period has 3600 ticks; the dithering splits each into
 * sixteenths for the very dim evening levels. */
#define APP_PWM_FREQUENCY_HZ 20000u
#define APP_LED_FADE_MS      300u
#define APP_OUTPUT_PERIOD_MS 10u
/* Timer ticks per PWM period that the light tables were measured at. */
#define APP_LED_CAL_PERIOD 3600u
/* Level 1 % shows 3 % of perceived brightness, and the levels above spread up
 * to 100 % (color_level_shown). Lower, the pulses of a channel fall to the
 * few ticks where the light tables are least sure and differ from board to
 * board (note N4 in hardware/README.md). */
#define APP_LED_FLOOR_PERMILLE 30u

/* Light sensor: the task polls, and reads once per integration
 * (drivers/veml7700.h). */
#define APP_ALS_PERIOD_MS      100u
#define APP_ALS_MAX_ERRORS     5u    /* consecutive failures before giving up */
#define APP_ALS_RETRY_MS       5000u /* then retry the sensor this often */
#define APP_ALS_FIRST_RETRY_MS 200u  /* sooner at start-up, in case it was still booting */
#define APP_I2C_SPEED_HZ       100000u

/* Host notifications caused by automatic brightness: at most this often. */
#define APP_AUTO_LEVEL_NOTIFY_MS 500u

/* A TOUCH that cannot leave within this delay, as when no host reads the
 * lamp, is dropped: a press is news only while it is fresh. */
#define APP_TOUCH_FRESH_MS 500u

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
/* Each built-in color as LED drive (led_cal.h), its largest channel at
 * COLOR_LINEAR_MAX, with the chromaticity of that drive. */
extern const palette_builtin_t app_builtin_colors[LAMP_COLOR_COUNT];
extern const power_config_t app_power_config;
/* The LED calibration (led_cal.h): chromaticity to LED light, then light to
 * duty. */
extern const led_color_cal_t app_led_color_cal;
extern const led_light_cal_t app_led_light_cal[LED_CHANNEL_COUNT];
/* The chromaticities of the red, green and blue LEDs: the gamut of the
 * calibration, which GAMUT tells the host. */
extern const xy16_t app_led_gamut[LED_CHANNEL_COUNT];
extern const self_test_config_t app_self_test_config;

#endif /* DISLED_APP_CONFIG_H */
