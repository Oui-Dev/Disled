/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "app_host.h"

#include <stdbool.h>

#include "app_config.h"
#include "app_context.h"
#include "bsp/board.h"
#include "bsp/bsp.h"
#include "bsp/bsp_uid.h"
#include "host_notify.h"
#include "protocol.h"
#include "time_utils.h"
#include "usb/usb_device.h"

static host_notify_t s_notify; /* STATE notifications and the telemetry */

static bool s_reset_pending;
static uint32_t s_reset_at_ms;

static uint32_t s_host_seen_ms; /* last command, for the touch mode fallback */

static uint16_t subticks_to_ticks(uint16_t subticks)
{
    return (uint16_t)((subticks + PWM_DITHER_SUBTICKS / 2u) / PWM_DITHER_SUBTICKS);
}

static host_notify_state_t observe(void)
{
    const host_notify_state_t content = {
        .lamp = app.lamp,
        .applied_percent = brightness_level_percent(&app.brightness, app.lamp.brightness_mode,
                                                    app.lamp.manual_level),
        .link = app.link,
        .rgb = app_lamp_rgb(),
    };
    return content;
}

/* Power-only touch needs a host that drives the lamp (APP_HOST_SILENCE_MS). */
static void check_touch_mode(uint32_t now_ms)
{
    change_source_t cause;
    if (lamp_fsm_touch_fallback(&app.lamp, app.link, time_elapsed_ms(now_ms, s_host_seen_ms),
                                APP_HOST_SILENCE_MS, &cause)) {
        app.source = cause;
    }
}

static void observe_changes(void)
{
    const host_notify_state_t now = observe();
    if (now.link == USB_LINK_UNCONFIGURED) {
        /* A suspended host still waits for the result; the next one never asked. */
        app.self_test_report = false;
        app.alert_report = false;
    }
    host_notify_observe(&s_notify, &now, app.source);
}

/* Sends the current state; `seq` is the request it answers (0: none). */
static void send_state(uint8_t seq, uint32_t now_ms)
{
    const proto_state_t state = host_notify_state(&s_notify);
    uint8_t payload[PROTO_REPORT_SIZE];
    proto_encode_state(payload, seq, &state);
    if (usb_hid_send(payload)) {
        host_notify_state_sent(&s_notify, now_ms);
    }
}

static uint32_t capabilities(void)
{
    uint32_t caps = PROTO_CAP_TELEMETRY | PROTO_CAP_AUTO_BIAS | PROTO_CAP_DIAGNOSTICS |
                    PROTO_CAP_SELF_TEST | PROTO_CAP_CUSTOM_COLORS | PROTO_CAP_TOUCH_CYCLE |
                    PROTO_CAP_TOUCH_MODE | PROTO_CAP_ALERT | PROTO_CAP_TOUCH_SENSITIVITY |
                    PROTO_CAP_PRESSES;
#if DISLED_DEBUG
    caps |= PROTO_CAP_DEBUG_CONSOLE;
#endif
    return caps;
}

static void send_info(uint8_t seq)
{
    proto_info_t info = {
        .fw_major = DISLED_FW_VERSION_MAJOR,
        .fw_minor = DISLED_FW_VERSION_MINOR,
        .fw_patch = DISLED_FW_VERSION_PATCH,
        .board_revision = BOARD_REVISION,
        .capabilities = capabilities(),
        .reset_cause = bsp_reset_cause(),
        .flags = (uint8_t)(app.fault.valid ? PROTO_INFO_FLAG_FAULT : 0u),
        .fault_mcause = app.fault.mcause,
        .fault_mepc = app.fault.mepc,
        .fw_version = DISLED_FW_VERSION_STRING,
    };
#if DISLED_DEBUG
    info.flags |= PROTO_INFO_FLAG_DEBUG_BUILD;
#endif
    bsp_uid_read(info.uid);
    uint8_t payload[PROTO_REPORT_SIZE];
    proto_encode_info(payload, seq, &info);
    (void)usb_hid_send(payload);
}

static void send_ack(uint8_t seq, uint8_t command_type)
{
    uint8_t payload[PROTO_REPORT_SIZE];
    proto_encode_ack(payload, seq, command_type);
    (void)usb_hid_send(payload);
}

static void send_error(uint8_t seq, uint8_t command_type, proto_status_t error)
{
    uint8_t payload[PROTO_REPORT_SIZE];
    proto_encode_error(payload, seq, command_type, error);
    (void)usb_hid_send(payload);
}

static void send_diagnostics(uint8_t seq)
{
    proto_diagnostics_t diagnostics;
    app_diagnostics(&diagnostics);
    uint8_t payload[PROTO_REPORT_SIZE];
    proto_encode_diagnostics(payload, seq, &diagnostics);
    (void)usb_hid_send(payload);
}

static void send_self_test_result(void)
{
    const self_test_t *test = &app.self_test;
    const proto_self_test_result_t result = {
        .results = test->results,
        .lux_centi = test->lux_centi,
        .touch_baseline = test->touch_baseline,
        .touch_peak_permille = test->touch_peak_permille,
    };
    uint8_t payload[PROTO_REPORT_SIZE];
    proto_encode_self_test_result(payload, app.self_test_seq, &result);
    if (usb_hid_send(payload)) {
        app.self_test_report = false;
    }
}

static void send_alert_done(void)
{
    uint8_t payload[PROTO_REPORT_SIZE];
    proto_encode_alert_done(payload, app.alert_done_seq, app.alert_done_reason);
    if (usb_hid_send(payload)) {
        app.alert_report = false;
    }
}

static void send_touch(void)
{
    uint8_t payload[PROTO_REPORT_SIZE];
    proto_encode_touch(payload, PROTO_PRESS_SHORT);
    if (usb_hid_send(payload)) {
        app.touch_report = false;
    }
}

/* A short press goes out before the STATE of the color it changed. It waits
 * for the way to be clear, as STATE does, but only while it is fresh. */
static void report_touch(uint32_t now_ms)
{
    if (!app.touch_report) {
        return;
    }
    if (app.link != USB_LINK_CONFIGURED ||
        time_elapsed_ms(now_ms, app.touch_report_ms) >= APP_TOUCH_FRESH_MS) {
        app.touch_report = false;
    } else if (usb_hid_idle()) {
        send_touch();
    }
}

/* Commands that change what the lamp shows wait for the self-test. Version 2
 * adds a color redefined in place and the alert. */
static bool busy_with_self_test(uint8_t type)
{
    if (!self_test_running(&app.self_test)) {
        return false;
    }
    switch (type) {
        case PROTO_CMD_SET_STATE:
        case PROTO_CMD_SET_AUTO_BIAS:
        case PROTO_CMD_SELF_TEST:
        case PROTO_CMD_SET_COLOR:
        case PROTO_CMD_ALERT:
            return true;
        default:
            return false;
    }
}

/* ACK, or the error of a command the lamp refused. */
static void send_outcome(const proto_command_t *command, proto_status_t status)
{
    if (status == PROTO_OK) {
        send_ack(command->seq, command->type);
    } else {
        send_error(command->seq, command->type, status);
    }
}

static void handle_command(const uint8_t payload[PROTO_REPORT_SIZE], uint32_t now_ms)
{
    s_host_seen_ms = now_ms; /* any command is a heartbeat */
    proto_command_t command;
    const proto_status_t status = proto_decode_command(payload, &command);
    if (status != PROTO_OK) {
        send_error(command.seq, command.type, status);
        return;
    }
    if (busy_with_self_test(command.type)) {
        send_error(command.seq, command.type, PROTO_ERR_BUSY);
        return;
    }

    switch (command.type) {
        case PROTO_CMD_GET_INFO:
            send_info(command.seq);
            break;
        case PROTO_CMD_GET_STATE:
            observe_changes();
            send_state(command.seq, now_ms);
            break;
        case PROTO_CMD_SET_STATE:
            if (!app_request_allowed(&command.body.set_state)) {
                send_error(command.seq, command.type, PROTO_ERR_BAD_VALUE);
                break;
            }
            app_apply_request(&command.body.set_state);
            /* The answer is the new state; it also serves as the
             * notification of the change. */
            observe_changes();
            send_state(command.seq, now_ms);
            break;
        case PROTO_CMD_SET_COLOR:
            send_outcome(&command,
                         app_set_color(command.body.set_color.slot, command.body.set_color.color)
                             ? PROTO_OK
                             : PROTO_ERR_BAD_VALUE);
            break;
        case PROTO_CMD_GET_COLOR: {
            uint8_t reply[PROTO_REPORT_SIZE];
            proto_encode_color(reply, command.seq, command.body.color_slot,
                               palette_color(&app.palette, command.body.color_slot));
            (void)usb_hid_send(reply);
            break;
        }
        case PROTO_CMD_SET_TOUCH_CYCLE:
            send_outcome(&command, app_set_touch_cycle(&command.body.touch_cycle)
                                       ? PROTO_OK
                                       : PROTO_ERR_BAD_VALUE);
            break;
        case PROTO_CMD_GET_TOUCH_CYCLE: {
            uint8_t reply[PROTO_REPORT_SIZE];
            proto_encode_touch_cycle(reply, command.seq, &app.cycle);
            (void)usb_hid_send(reply);
            break;
        }
        case PROTO_CMD_ALERT:
            send_outcome(&command, app_alert(command.seq, command.body.alert.count,
                                             command.body.alert.period_ms));
            break;
        case PROTO_CMD_SET_TELEMETRY:
            host_notify_set_telemetry(&s_notify, command.body.telemetry_period_ms, now_ms);
            send_ack(command.seq, command.type);
            break;
        case PROTO_CMD_RESET:
            send_ack(command.seq, command.type);
            s_reset_pending = true;
            s_reset_at_ms = now_ms + APP_RESET_DELAY_MS;
            break;
        case PROTO_CMD_SET_AUTO_BIAS:
            brightness_set_auto_bias(&app.brightness, command.body.auto_bias);
            send_ack(command.seq, command.type);
            break;
        case PROTO_CMD_SET_TOUCH_SENSITIVITY:
            /* Never busy: it changes nothing the lamp shows, and the
             * self-test then judges the touch as the user set it. */
            touch_detector_set_sensitivity(&app.touch, command.body.touch_sensitivity);
            send_ack(command.seq, command.type);
            break;
        case PROTO_CMD_GET_DIAGNOSTICS:
            send_diagnostics(command.seq);
            break;
        case PROTO_CMD_SELF_TEST:
            app_start_self_test(command.seq);
            send_ack(command.seq, command.type);
            break;
        default:
            /* Unreachable: every command the decoder accepts has its case. */
            send_error(command.seq, command.type, PROTO_ERR_UNKNOWN_COMMAND);
            break;
    }
}

static void send_telemetry(uint32_t now_ms)
{
    /* A sample due waits for the previous messages to leave. */
    if (!usb_hid_idle() || !host_notify_take_telemetry(&s_notify, now_ms)) {
        return;
    }
    const proto_telemetry_t telemetry = {
        .uptime_ms = now_ms,
        .touch_raw = touch_detector_raw(&app.touch),
        .touch_value = touch_detector_value(&app.touch),
        .touch_baseline = touch_detector_baseline(&app.touch),
        .touch_delta_permille = touch_detector_delta_permille(&app.touch),
        .touch_state = (uint8_t)touch_detector_state(&app.touch),
        .lux_centi = (app.als.lux_milli + 5u) / 10u,
        .als_raw = app.als.count,
        /* The protocol reports whole ticks. */
        .duty = { subticks_to_ticks(app.duty[LED_CHANNEL_RED]),
                  subticks_to_ticks(app.duty[LED_CHANNEL_GREEN]),
                  subticks_to_ticks(app.duty[LED_CHANNEL_BLUE]) },
        .led_current_ma_x10 = (uint16_t)led_output_estimate_ma_x10(&app.output),
        .cap_ma = led_output_cap_ma(&app.output),
        .link = app.link,
        .level_permille = app_level_permille(),
    };
    uint8_t payload[PROTO_REPORT_SIZE];
    proto_encode_telemetry(payload, &telemetry);
    (void)usb_hid_send(payload);
}

void app_host_init(void)
{
    const host_notify_state_t now = observe();
    host_notify_init(&s_notify, &now, APP_AUTO_LEVEL_NOTIFY_MS);
}

void app_host_task(uint32_t now_ms)
{
    uint8_t payload[PROTO_REPORT_SIZE];
    while (usb_hid_receive(payload)) {
        handle_command(payload, now_ms);
    }

    check_touch_mode(now_ms);
    observe_changes();
    report_touch(now_ms);
    /* Until the way is clear, the changes merge and go out as one STATE. */
    if (usb_hid_idle() && host_notify_state_due(&s_notify, now_ms)) {
        send_state(0, now_ms);
    }

    send_telemetry(now_ms);

    if (app.self_test_report && app.link == USB_LINK_CONFIGURED) {
        send_self_test_result();
    }
    if (app.alert_report && app.link == USB_LINK_CONFIGURED) {
        send_alert_done();
    }

    if (s_reset_pending && time_reached(now_ms, s_reset_at_ms)) {
        bsp_system_reset();
    }
}
