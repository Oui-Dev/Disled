/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "app_console.h"

#if DISLED_DEBUG

#include <stdbool.h>
#include <string.h>

#include "app_config.h"
#include "app_context.h"
#include "diag/console.h"
#include "drivers/led_pwm.h"
#include "time_utils.h"

static uint32_t s_watch_period_ms;
static uint32_t s_watch_next_ms;

static const char *const color_names[] = { "green", "orange", "red", "warm-white" };
static const char *const link_names[] = { "unconfigured", "configured", "suspended" };

static const char *color_name(lamp_color_t color)
{
    if ((uint32_t)color >= LAMP_COLOR_COUNT) {
        return "custom";
    }
    return color_names[color];
}

/* Parses a decimal number; false if `text` is not one. */
static bool parse_u32(const char *text, uint32_t *value)
{
    uint32_t result = 0;
    if (*text == '\0') {
        return false;
    }
    for (; *text != '\0'; text++) {
        if (*text < '0' || *text > '9' || result > 100000000u) {
            return false;
        }
        result = result * 10u + (uint32_t)(*text - '0');
    }
    *value = result;
    return true;
}

/* Prints sub-ticks as ticks with two decimals. */
static void print_subticks(uint16_t subticks)
{
    console_printf("%u.%02u", (unsigned)(subticks / LED_DUTY_SUBTICKS),
                   (unsigned)(((subticks % LED_DUTY_SUBTICKS) * 100u) / LED_DUTY_SUBTICKS));
}

static void cmd_state(int argc, char **argv)
{
    (void)argc;
    (void)argv;
    console_printf("lamp %s, %s, %s, manual %u%%, applied %u per mille\r\n",
                   app.lamp.on ? "on" : "off", color_name(app.lamp.color),
                   app.lamp.brightness_mode == BRIGHTNESS_MODE_AUTO ? "auto" : "manual",
                   (unsigned)app.lamp.manual_level,
                   (unsigned)brightness_level_permille(&app.brightness));
    const rgb16_t rgb = app_lamp_rgb();
    console_printf("color slot %u = %u %u %u", (unsigned)app.lamp.color, (unsigned)rgb.r,
                   (unsigned)rgb.g, (unsigned)rgb.b);
    console_printf(", touch %s",
                   app.lamp.touch_mode == TOUCH_MODE_NORMAL ? "normal" : "power only");
    console_printf(", alert %s", alert_running(&app.alert) ? "running" : "off");
    console_printf("\r\n");
    const uint32_t estimate = led_output_estimate_ma_x10(&app.output);
    console_printf("usb %s, LED %lu.%lu mA of %u mA, duty ", link_names[app.link],
                   (unsigned long)(estimate / 10u), (unsigned long)(estimate % 10u),
                   (unsigned)led_output_cap_ma(&app.output));
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        print_subticks(app.duty[i]);
        console_printf(i + 1u < LED_CHANNEL_COUNT ? "/" : "");
    }
    console_printf(" of %u\r\n", (unsigned)led_pwm_period());
}

static void print_touch_sensitivity(void)
{
    const touch_thresholds_t thresholds = touch_detector_thresholds(&app.touch);
    console_printf("touch sensitivity %d%%: press %u, release %u, rise %u per mille\r\n",
                   (int)touch_detector_sensitivity(&app.touch), (unsigned)thresholds.press,
                   (unsigned)thresholds.release, (unsigned)thresholds.rise);
}

static void cmd_set(int argc, char **argv)
{
    lamp_request_t request = { 0 };
    uint32_t value = 0;
    if (argc == 2 && strcmp(argv[1], "on") == 0) {
        request.fields = LAMP_FIELD_POWER;
        request.on = true;
    } else if (argc == 2 && strcmp(argv[1], "off") == 0) {
        request.fields = LAMP_FIELD_POWER;
        request.on = false;
    } else if (argc == 2 && strcmp(argv[1], "auto") == 0) {
        request.fields = LAMP_FIELD_BRIGHTNESS_MODE;
        request.brightness_mode = BRIGHTNESS_MODE_AUTO;
    } else if (argc == 3 && strcmp(argv[1], "color") == 0 && parse_u32(argv[2], &value)) {
        request.fields = LAMP_FIELD_COLOR;
        request.color = (lamp_color_t)value;
    } else if (argc == 3 && strcmp(argv[1], "level") == 0 && parse_u32(argv[2], &value)) {
        request.fields = LAMP_FIELD_BRIGHTNESS_MODE | LAMP_FIELD_MANUAL_LEVEL;
        request.brightness_mode = BRIGHTNESS_MODE_MANUAL;
        request.manual_level = (uint8_t)((value > 255u) ? 255u : value);
    } else if (argc == 3 && strcmp(argv[1], "bias") == 0 &&
               parse_u32(argv[2] + (argv[2][0] == '-' ? 1 : 0), &value) &&
               value <= (uint32_t)BRIGHTNESS_BIAS_MAX) {
        const int32_t bias = (argv[2][0] == '-') ? -(int32_t)value : (int32_t)value;
        brightness_set_auto_bias(&app.brightness, (int8_t)bias);
        cmd_state(0, NULL);
        return;
    } else if (argc == 3 && strcmp(argv[1], "sensitivity") == 0 &&
               parse_u32(argv[2] + (argv[2][0] == '-' ? 1 : 0), &value) &&
               value <= (uint32_t)TOUCH_SENSITIVITY_MAX) {
        const int32_t percent = (argv[2][0] == '-') ? -(int32_t)value : (int32_t)value;
        touch_detector_set_sensitivity(&app.touch, (int8_t)percent);
        print_touch_sensitivity();
        return;
    } else {
        console_printf("usage: set on|off|auto|color <0-15>|level <1-100>|bias <-50..50>"
                       "|sensitivity <-50..50>\r\n");
        return;
    }
    if (!lamp_request_is_valid(&request) || !app_request_allowed(&request)) {
        console_printf("invalid value\r\n");
        return;
    }
    app_apply_request(&request);
    cmd_state(0, NULL);
}

static void cmd_alert(int argc, char **argv)
{
    uint32_t count = APP_ALERT_DEFAULT_COUNT;
    uint32_t period = APP_ALERT_DEFAULT_PERIOD_MS;
    const bool parsed = argc <= 3 && (argc < 2 || parse_u32(argv[1], &count)) &&
                        (argc < 3 || parse_u32(argv[2], &period));
    if (!parsed || count > ALERT_COUNT_MAX ||
        (count > 0u && (period < ALERT_PERIOD_MIN_MS || period > ALERT_PERIOD_MAX_MS))) {
        console_printf("usage: alert [breaths 0-10, 0 stops] [period 500-5000 ms]\r\n");
        return;
    }
    const proto_status_t status = app_alert(0, (uint8_t)count, (uint16_t)period);
    console_printf(status == PROTO_OK ? "ok\r\n" : "the lamp is off\r\n");
}

static void cmd_watch(int argc, char **argv)
{
    uint32_t period = 0;
    if (argc == 2 && strcmp(argv[1], "off") == 0) {
        s_watch_period_ms = 0;
        return;
    }
    if (argc != 2 || !parse_u32(argv[1], &period) || period < 20u) {
        console_printf("usage: watch <period ms, >= 20>|off\r\n");
        return;
    }
    s_watch_period_ms = period;
    s_watch_next_ms = 0;
    console_printf("raw value baseline delta(permille) state | lux count level | mA\r\n");
}

static void cmd_cap(int argc, char **argv)
{
    uint32_t cap = 0;
    if (argc == 2 && strcmp(argv[1], "off") == 0) {
        app.cap_override_ma = 0;
    } else if (argc == 2 && parse_u32(argv[1], &cap) && cap > 0u && cap <= 300u) {
        app.cap_override_ma = (uint16_t)cap;
        console_printf("WARNING: LED cap forced to %lu mA (thermal test only)\r\n",
                       (unsigned long)cap);
    } else {
        console_printf("usage: cap <1-300 mA>|off\r\n");
        return;
    }
    cmd_state(0, NULL);
}

static const console_command_t commands[] = {
    { "state", "", "lamp, USB and LED output", cmd_state },
    { "set", "<what> [value]", "change the lamp like the host does", cmd_set },
    { "watch", "<ms>|off", "print touch, light and current periodically", cmd_watch },
    { "cap", "<mA>|off", "override the LED current cap (thermal test only)", cmd_cap },
    { "alert", "[breaths] [period ms]", "breathe like an alert from the host", cmd_alert },
};

void app_console_init(void)
{
    console_init(commands, sizeof commands / sizeof commands[0]);
}

void app_console_task(uint32_t now_ms)
{
    console_task();
    if (s_watch_period_ms == 0u || !time_reached(now_ms, s_watch_next_ms)) {
        return;
    }
    s_watch_next_ms = now_ms + s_watch_period_ms;
    const uint32_t lux = app.als.lux_milli;
    const uint32_t estimate = led_output_estimate_ma_x10(&app.output);
    console_printf(
        "%u %u %u %d %u | %lu.%03lu %u %u | %lu.%lu\r\n", (unsigned)touch_detector_raw(&app.touch),
        (unsigned)touch_detector_value(&app.touch), (unsigned)touch_detector_baseline(&app.touch),
        (int)touch_detector_delta_permille(&app.touch), (unsigned)touch_detector_state(&app.touch),
        (unsigned long)(lux / 1000u), (unsigned long)(lux % 1000u), (unsigned)app.als.count,
        (unsigned)brightness_level_permille(&app.brightness), (unsigned long)(estimate / 10u),
        (unsigned long)(estimate % 10u));
}

#endif
