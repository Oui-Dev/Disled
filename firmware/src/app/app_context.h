/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * State shared by the application modules (app.c, app_host.c,
 * app_console.c). Nothing outside src/app includes this file.
 */
#ifndef DISLED_APP_CONTEXT_H
#define DISLED_APP_CONTEXT_H

#include <stdbool.h>
#include <stdint.h>

#include "alert.h"
#include "brightness.h"
#include "bsp/bsp_fault.h"
#include "disled_types.h"
#include "lamp_fsm.h"
#include "led_output.h"
#include "palette.h"
#include "protocol.h"
#include "self_test.h"
#include "touch_cycle.h"
#include "touch_detector.h"

typedef struct {
    bool present;
    uint8_t errors;    /* consecutive failures */
    uint32_t retry_ms; /* next detection attempt while absent */
    uint32_t read_ms;  /* next reading: once per integration */
    uint16_t count;    /* last count */
    uint32_t lux_milli;
    uint16_t probes; /* detection attempts since boot (diagnostics) */
} app_als_t;

typedef struct {
    lamp_state_t lamp;
    change_source_t source; /* cause of the latest lamp change */
    brightness_t brightness;
    touch_detector_t touch;
    led_output_t output;
    uint16_t duty[LED_CHANNEL_COUNT];
    usb_link_t link;
    app_als_t als;
    uint16_t cap_override_ma; /* debug: lifts the cap for the heat test, 0 = off */
    reset_cause_t reset_cause;
    fault_record_t fault;
    bool crystal_ok;
    uint16_t usb_suspends; /* since boot (diagnostics) */
    self_test_t self_test;
    uint8_t self_test_seq; /* seq of the SELF_TEST command, for the result */
    bool self_test_report; /* a result is waiting to be sent */
    palette_t palette;
    touch_cycle_t cycle;
    uint32_t host_seen_ms; /* last command, for the touch mode fallback */
    alert_t alert;
    uint8_t alert_seq;      /* seq of the ALERT command running */
    uint8_t alert_done_seq; /* the ALERT_DONE waiting to be sent: seq and reason */
    uint8_t alert_done_reason;
    bool alert_report;
} app_context_t;

/* The single instance, owned by app.c. */
extern app_context_t app;

/* Applies a lamp request coming from the host or the console, valid and
 * allowed. */
void app_apply_request(const lamp_request_t *request);

/* A valid request may still name a color slot that holds no color. */
static inline bool app_request_allowed(const lamp_request_t *request)
{
    return (request->fields & LAMP_FIELD_COLOR) == 0u ||
           palette_defined(&app.palette, (uint32_t)request->color);
}

/* The lamp color at full brightness, linear (STATE). */
rgb16_t app_lamp_rgb(void);

/* Sets or empties (black) a custom slot. An emptied color leaves the touch
 * cycle, and the lamp showing it goes to the first color of the cycle. */
bool app_set_color(uint32_t slot, rgb16_t color);

/* False if a slot of the cycle holds no color. */
bool app_set_touch_cycle(const touch_cycle_t *cycle);

/* Starts `count` breaths, or replaces the running alert; 0 stops it. No
 * alert while the lamp is off. ALERT_DONE then carries `seq`. */
proto_status_t app_alert(uint8_t seq, uint8_t count, uint16_t period_ms);

/* Starts the self-test; its result is reported to the host with `seq`.
 * Never while one runs: the host gets BUSY instead. */
void app_start_self_test(uint8_t seq);

/* Snapshot of the DIAGNOSTICS message. */
void app_diagnostics(proto_diagnostics_t *diagnostics);

#endif /* DISLED_APP_CONTEXT_H */
