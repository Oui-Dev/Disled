/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Application wiring: the drivers feed the core logic, which drives the
 * outputs, all run by the cooperative scheduler (data flow in
 * docs/architecture.md).
 */
#include "app.h"

#include <stddef.h>

#include "app_config.h"
#include "app_console.h"
#include "app_context.h"
#include "app_host.h"
#include "bsp/bsp.h"
#include "bsp/bsp_power.h"
#include "bsp/bsp_time.h"
#include "bsp/bsp_watchdog.h"
#include "diag/log.h"
#include "drivers/i2c_bus.h"
#include "drivers/led_pwm.h"
#include "drivers/tkey.h"
#include "drivers/veml7700.h"
#include "scheduler.h"
#include "time_utils.h"
#include "usb/usb_device.h"

app_context_t app;

void app_apply_request(const lamp_request_t *request)
{
    if (lamp_fsm_apply(&app.lamp, request)) {
        app.source = CHANGE_SOURCE_HOST;
    }
}

rgb16_t app_lamp_rgb(void)
{
    return palette_color(&app.palette, (uint32_t)app.lamp.color);
}

/* --- Colors and touch cycle (version 2) ------------------------------ */

static uint32_t defined_slots(void)
{
    return app.palette.defined;
}

bool app_set_touch_cycle(const touch_cycle_t *cycle)
{
    return touch_cycle_set(&app.cycle, cycle->slots, cycle->count, defined_slots());
}

bool app_set_color(uint32_t slot, rgb16_t color)
{
    if (!palette_set(&app.palette, slot, color)) {
        return false;
    }
    if (palette_defined(&app.palette, slot)) {
        return true; /* a lamp showing it fades to the new color */
    }
    touch_cycle_remove(&app.cycle, slot);
    const lamp_color_t fallback = (lamp_color_t)app.cycle.slots[0];
    if ((uint32_t)app.lamp.color == slot) {
        app.lamp.color = fallback;
        app.source = CHANGE_SOURCE_HOST;
    }
    return true;
}

/* --- Alert (version 2) ----------------------------------------------- */

static void report_alert_end(uint8_t reason)
{
    app.alert_done_seq = app.alert_seq;
    app.alert_done_reason = reason;
    app.alert_report = true;
}

/* Cut short: the light fades back from where the breath left it. */
static void end_alert(uint8_t reason, uint32_t now_ms)
{
    if (!alert_running(&app.alert)) {
        return;
    }
    alert_stop(&app.alert);
    led_output_settle(&app.output, now_ms);
    report_alert_end(reason);
}

proto_status_t app_alert(uint8_t seq, uint8_t count, uint16_t period_ms)
{
    const uint32_t now_ms = bsp_time_ms();
    if (count == 0u) {
        end_alert(PROTO_ALERT_CANCELLED, now_ms);
        return PROTO_OK;
    }
    if (!app.lamp.on) {
        return PROTO_ERR_LAMP_OFF;
    }
    if (alert_running(&app.alert)) {
        /* Replaced without an ALERT_DONE: the ACK of the new one says it. */
        led_output_settle(&app.output, now_ms);
    }
    app.alert_seq = seq;
    alert_start(&app.alert, count, period_ms, now_ms);
    return PROTO_OK;
}

/* The touch acts on nothing during an alert (task_touch()). An alert ends
 * with the lamp turned off by the host, at a USB suspend, when the self-test
 * takes the LED, and after its last breath. Returns the breath to show now. */
static uint16_t update_alert(uint32_t now_ms, bool testing)
{
    if (app.link == USB_LINK_SUSPENDED) {
        end_alert(PROTO_ALERT_SUSPENDED, now_ms);
    } else if (testing) {
        end_alert(PROTO_ALERT_SELF_TEST, now_ms);
    } else if (!app.lamp.on) {
        end_alert(PROTO_ALERT_LAMP_OFF, now_ms);
    }
    uint16_t breath;
    if (alert_update(&app.alert, now_ms, &breath)) {
        report_alert_end(PROTO_ALERT_COMPLETED);
    }
    return breath;
}

/* --- Tasks ----------------------------------------------------------- */

static void task_output(uint32_t now_ms);
static void als_start(app_als_t *als, uint32_t now_ms, uint32_t retry_delay_ms);

/* Host asleep: the LED goes dark, the touch and the light sensor stop, and
 * the core drops to the slow clock until the bus wakes up. */
static void enter_suspend(uint32_t now_ms)
{
    if (self_test_running(&app.self_test)) {
        /* Reported at resume, without SELF_TEST_COMPLETED: the host waits for the result. */
        self_test_abort(&app.self_test);
        app.self_test_report = true;
    }
    tkey_stop();
    task_output(now_ms); /* app.link is suspended: duties drop to 0 */
    led_pwm_stop();
    if (app.als.present) {
        (void)veml7700_shutdown();
    }
    if (app.usb_suspends < UINT16_MAX) {
        app.usb_suspends++;
    }
    usb_device_wake_irq(true);
    bsp_power_slow_clock();
}

static void leave_suspend(uint32_t now_ms)
{
    /* The USB wake-up interrupt normally restarted the crystal already; if
     * it failed, this tries again, since USB has no clock without it. A
     * second failure goes unreported: no message could leave without USB. */
    (void)bsp_power_full_clock();
    usb_device_wake_irq(false);
    led_pwm_start();
    if (app.als.present) {
        als_start(&app.als, now_ms, APP_ALS_FIRST_RETRY_MS); /* wakes the sensor */
    }
    touch_detector_recalibrate(&app.touch);
    tkey_start();
}

static void task_usb(uint32_t now_ms)
{
    usb_device_task();

    const usb_link_t link = usb_device_link();
    if (link == app.link) {
        return;
    }
    const usb_link_t previous = app.link;
    app.link = link;
    if (link == USB_LINK_SUSPENDED) {
        enter_suspend(now_ms);
    } else if (previous == USB_LINK_SUSPENDED) {
        leave_suspend(now_ms);
    }
}

static void task_touch(uint32_t now_ms)
{
    (void)now_ms;
    uint16_t raw;
    uint32_t sample_ms;
    while (tkey_read(&raw, &sample_ms)) {
        /* No gesture acts during an alert, nor at the release of a press it
         * covered. The alert ends in task_output(), between two samples. */
        const touch_event_t event =
            touch_gate_feed(&app.touch_gate, &app.touch, raw, sample_ms, alert_running(&app.alert));
        bool changed = false;
        if (self_test_running(&app.self_test)) {
            /* The self-test watches the touch signal itself. */
        } else if (event == TOUCH_EVENT_SHORT_PRESS) {
            changed = lamp_fsm_short_press(&app.lamp, &app.cycle);
        } else if (event == TOUCH_EVENT_LONG_PRESS) {
            lamp_fsm_long_press(&app.lamp);
            changed = true;
        }
        if (changed) {
            app.source = CHANGE_SOURCE_TOUCH;
            LOG_INFO("touch: %s press, lamp %s, color %u",
                     event == TOUCH_EVENT_SHORT_PRESS ? "short" : "long",
                     app.lamp.on ? "on" : "off", (unsigned)app.lamp.color);
        }
    }
}

/* Starts (or wakes) the sensor. The first readings after power-on are not
 * valid until an integration completes, so they are skipped. */
static void als_start(app_als_t *als, uint32_t now_ms, uint32_t retry_delay_ms)
{
    als->present = veml7700_init();
    als->errors = 0;
    als->retry_ms = now_ms + retry_delay_ms;
    als->read_ms = now_ms + 2u * VEML7700_INTEGRATION_MS;
}

/* A detection attempt (start-up, sensor lost): counted for the diagnostics. */
static void als_probe(app_als_t *als, uint32_t now_ms, uint32_t retry_delay_ms)
{
    if (als->probes < UINT16_MAX) {
        als->probes++;
    }
    als_start(als, now_ms, retry_delay_ms);
}

static void task_als(uint32_t now_ms)
{
    app_als_t *als = &app.als;

    if (app.link == USB_LINK_SUSPENDED) {
        return; /* sensor asleep, I2C idle on the slow clock */
    }
    if (!als->present) {
        if (time_reached(now_ms, als->retry_ms)) {
            als_probe(als, now_ms, APP_ALS_RETRY_MS);
        }
        return;
    }
    if (!time_reached(now_ms, als->read_ms)) {
        return;
    }

    uint16_t count;
    if (veml7700_read(&count)) {
        als->errors = 0;
        als->count = count;
        als->lux_milli = veml7700_lux_milli(count);
        brightness_update_lux(&app.brightness, als->lux_milli, now_ms);
        als->read_ms = now_ms + VEML7700_INTEGRATION_MS;
        return;
    }
    if (++als->errors >= APP_ALS_MAX_ERRORS) {
        /* Fixed brightness until the sensor answers again. */
        als->present = false;
        als->retry_ms = now_ms + APP_ALS_RETRY_MS;
        brightness_lux_lost(&app.brightness);
    }
}

/* Feeds the running self-test; returns the color it wants shown. */
static rgb16_t run_self_test(uint32_t now_ms)
{
    const self_test_inputs_t measurements = {
        .light_present = app.als.present,
        .lux_centi = (app.als.lux_milli + 5u) / 10u,
        .touch_baseline = touch_detector_baseline(&app.touch),
        .touch_delta_permille = touch_detector_delta_permille(&app.touch),
        .touch_pressed = touch_detector_state(&app.touch) == TOUCH_STATE_PRESSED,
    };
    if (self_test_update(&app.self_test, now_ms, &measurements)) {
        if ((app.self_test.results & SELF_TEST_TOUCH_PRESSED) != 0u) {
            touch_detector_consume(&app.touch); /* the finger is still down */
        }
        app.self_test_report = true;
    }
    return self_test_color(&app.self_test, now_ms);
}

void app_start_self_test(uint8_t seq)
{
    app.self_test_seq = seq;
    app.self_test_report = false;
    self_test_start(&app.self_test, bsp_time_ms());
}

static void task_output(uint32_t now_ms)
{
    brightness_set_mode(&app.brightness, app.lamp.brightness_mode, app.lamp.manual_level);
    const bool testing = self_test_running(&app.self_test);
    const uint16_t breath = update_alert(now_ms, testing);
    const led_output_inputs_t inputs = {
        .on = app.lamp.on,
        .color = palette_light(&app.palette, (uint32_t)app.lamp.color, app_led_white_balance),
        .level_permille = brightness_level_permille(&app.brightness),
        .link = app.link,
        .cap_override_ma = app.cap_override_ma,
        .show_override = testing,
        .override_rgb = testing ? run_self_test(now_ms) : (rgb16_t){ 0, 0, 0 },
        .breath = breath,
    };
    led_output_update(&app.output, &inputs, now_ms, app.duty);
    led_pwm_set(app.duty);
}

static uint16_t saturate_u16(uint32_t value)
{
    return (value > UINT16_MAX) ? (uint16_t)UINT16_MAX : (uint16_t)value;
}

void app_diagnostics(proto_diagnostics_t *diagnostics)
{
    const fault_stats_t stats = bsp_fault_stats();
    *diagnostics = (proto_diagnostics_t){
        .uptime_ms = bsp_time_ms(),
        .boots = stats.boots,
        .watchdog_resets = stats.watchdog_resets,
        .faults = stats.faults,
        .fault_mcause = stats.last.mcause,
        .fault_mepc = stats.last.mepc,
        .fault_mtval = stats.last.mtval,
        .stack_free_min = bsp_stack_free_min(),
        .i2c_errors = i2c_bus_error_count(),
        .light_sensor_probes = app.als.probes,
        .touch_samples_lost = saturate_u16(tkey_samples_lost()),
        .touch_recalibrations = touch_detector_recalibrations(&app.touch),
        .usb_suspends = app.usb_suspends,
        .auto_bias = brightness_auto_bias(&app.brightness),
        .flags = (uint8_t)((app.als.present ? PROTO_DIAG_FLAG_LIGHT_SENSOR : 0u) |
                           (app.crystal_ok ? PROTO_DIAG_FLAG_CRYSTAL : 0u)),
        .touch_sensitivity = touch_detector_sensitivity(&app.touch),
    };
}

static scheduler_task_t tasks[] = {
    { 0, task_usb, 0 },
    { 0, task_touch, 0 },
    { APP_ALS_PERIOD_MS, task_als, 0 },
    { APP_OUTPUT_PERIOD_MS, task_output, 0 },
    { 0, app_host_task, 0 },
#if DISLED_DEBUG
    { 0, app_console_task, 0 },
#endif
};

#define TASK_COUNT (sizeof tasks / sizeof tasks[0])

/* --- Start-up -------------------------------------------------------- */

static led_output_config_t output_config; /* led_output keeps a pointer to it */

uint16_t app_alert_depth(void)
{
    return output_config.alert_depth_permille;
}

void app_set_alert_depth(uint16_t depth_permille)
{
    output_config.alert_depth_permille = depth_permille;
}

void app_init(const fault_record_t *fault)
{
    app.fault = *fault;
    app.reset_cause = bsp_reset_cause();
    app.crystal_ok = bsp_crystal_ok();

    /* LED first, dark: from here on only led_output drives it. */
    led_pwm_init(APP_PWM_FREQUENCY_HZ);
    output_config = (led_output_config_t){
        .power = &app_power_config,
        .fade_ms = APP_LED_FADE_MS,
        .pwm_period = led_pwm_period(),
        .alert_depth_permille = APP_ALERT_DEPTH_PERMILLE,
    };
    const uint32_t timer_mhz = bsp_apb1_timer_clock_hz() / 1000000u;
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        output_config.pulse_loss[i] =
            (uint16_t)((app_led_pulse_loss_ns[i] * timer_mhz * LED_DUTY_SUBTICKS + 500u) / 1000u);
    }
    led_output_init(&app.output, &output_config);

    lamp_fsm_init(&app.lamp);
    app.source = CHANGE_SOURCE_BOOT;
    palette_init(&app.palette, app_color_config.palette);
    touch_cycle_init(&app.cycle);
    brightness_init(&app.brightness, &app_brightness_config);
    self_test_init(&app.self_test, &app_self_test_config);

    i2c_bus_init(APP_I2C_SPEED_HZ);
    als_probe(&app.als, bsp_time_ms(), APP_ALS_FIRST_RETRY_MS);

    touch_detector_init(&app.touch, &app_touch_config);
    touch_gate_init(&app.touch_gate);
    tkey_init(&app_tkey_config);
    tkey_start();

    usb_device_init(app.crystal_ok);
    app.link = usb_device_link();
    app_host_init();
#if DISLED_DEBUG
    app_console_init();
#endif

    bsp_watchdog_start(APP_WATCHDOG_MS);
}

void app_run(void)
{
    scheduler_start(tasks, TASK_COUNT, bsp_time_ms());
    for (;;) {
        scheduler_run(tasks, TASK_COUNT, bsp_time_ms());
        /* Every task ran in this pass: the loop is alive. */
        bsp_watchdog_feed();
        /* No WFI: in this loop the core never wakes up from it. */
    }
}
