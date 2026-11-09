/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * The touch during an alert: the gate between the touch detector and the lamp,
 * driven as task_touch() and task_output() drive it.
 */
#include <unity.h>

#include "alert.h"
#include "lamp_fsm.h"
#include "touch_cycle.h"
#include "touch_detector.h"
#include "touch_gate.h"

#define BASELINE 2000u
/* A touch lowers the reading by 5 % (50 per mille). */
#define TOUCHED 1900u

/* Three breaths, the alert the app sends. */
#define BREATHS   3u
#define PERIOD_MS 1300u

static const touch_config_t config = {
    .window_samples = 20,
    .calibration_windows = 8,
    .baseline_shift = 6,
    .press_permille = 40,
    .release_permille = 25,
    .press_rise_permille = 18,
    .proximity_permille = 10,
    .debounce_ms = 60,
    .rearm_ms = 100,
    .long_press_ms = 2000,
    .object_ms = 10000,
    .inverted_permille = 30,
    .inverted_ms = 1000,
};

static touch_detector_t td;
static touch_gate_t gate;
static alert_t alert;
static lamp_state_t lamp;
static touch_cycle_t cycle;
static uint32_t now_ms;
static unsigned gestures;   /* the gestures that reached the lamp */
static unsigned alert_ends; /* the ends that alert_update() reported */

/* One touch sample per millisecond, and the alert updated every 10 ms, as by
 * task_output(): the alert ends there, between two touch samples. */
static void hold(uint16_t raw, uint32_t duration_ms)
{
    for (uint32_t i = 0; i < duration_ms; i++) {
        now_ms++;
        if (now_ms % 10u == 0u) {
            uint16_t breath;
            if (alert_update(&alert, now_ms, &breath)) {
                alert_ends++;
            }
        }
        const touch_event_t event = touch_gate_feed(&gate, &td, raw, now_ms, alert_running(&alert));
        if (event == TOUCH_EVENT_SHORT_PRESS) {
            gestures++;
            (void)lamp_fsm_short_press(&lamp, &cycle);
        } else if (event == TOUCH_EVENT_LONG_PRESS) {
            gestures++;
            lamp_fsm_long_press(&lamp);
        }
    }
}

/* A press, then the finger away past the debounce and the rearm delay. */
static void press(uint32_t duration_ms)
{
    hold(TOUCHED, duration_ms);
    hold(BASELINE, 300);
}

static void start_alert(void)
{
    alert_start(&alert, BREATHS, PERIOD_MS, now_ms);
}

void setUp(void)
{
    now_ms = 0;
    gestures = 0;
    alert_ends = 0;
    touch_detector_init(&td, &config);
    touch_gate_init(&gate);
    alert_stop(&alert);
    lamp_fsm_init(&lamp);
    touch_cycle_init(&cycle);
    hold(BASELINE, 200); /* 8 calibration windows take 160 ms */
}

void tearDown(void)
{
}

static void test_without_an_alert_the_touch_acts(void)
{
    press(200);
    TEST_ASSERT_EQUAL_UINT(1, gestures);
    TEST_ASSERT_EQUAL(LAMP_COLOR_ORANGE, lamp.color);

    press(2500);
    TEST_ASSERT_EQUAL_UINT(2, gestures);
    TEST_ASSERT_FALSE(lamp.on);
}

static void test_no_gesture_acts_during_an_alert(void)
{
    start_alert();
    press(200);  /* a short press */
    press(2500); /* a long press */
    TEST_ASSERT_TRUE(alert_running(&alert));
    TEST_ASSERT_EQUAL_UINT(0, gestures);
    TEST_ASSERT_TRUE(lamp.on);
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, lamp.color);
}

static void test_no_gesture_acts_during_an_alert_in_power_only_mode(void)
{
    lamp.touch_mode = TOUCH_MODE_POWER_ONLY;
    start_alert();
    press(2500); /* a long press, which turns the lamp off in this mode */
    TEST_ASSERT_TRUE(alert_running(&alert));
    TEST_ASSERT_EQUAL_UINT(0, gestures);
    TEST_ASSERT_TRUE(lamp.on);
    TEST_ASSERT_EQUAL(TOUCH_MODE_POWER_ONLY, lamp.touch_mode);
}

static void test_the_alert_ends_by_itself_once(void)
{
    start_alert();
    hold(BASELINE, BREATHS * PERIOD_MS - 10u);
    TEST_ASSERT_TRUE(alert_running(&alert));
    TEST_ASSERT_EQUAL_UINT(0, alert_ends);

    hold(BASELINE, 10);
    TEST_ASSERT_FALSE(alert_running(&alert));
    TEST_ASSERT_EQUAL_UINT(1, alert_ends);

    hold(BASELINE, 2000);
    TEST_ASSERT_EQUAL_UINT(1, alert_ends);
    TEST_ASSERT_TRUE(lamp.on);
}

static void test_a_press_from_before_the_alert_fires_nothing(void)
{
    /* Released during the alert. */
    hold(TOUCHED, 100);
    start_alert();
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(0, gestures);

    /* Long enough for a long press, which comes during the alert. */
    alert_stop(&alert);
    hold(TOUCHED, 1000);
    start_alert();
    hold(TOUCHED, 1500);
    hold(BASELINE, 300);
    TEST_ASSERT_TRUE(alert_running(&alert));
    TEST_ASSERT_EQUAL_UINT(0, gestures);
    TEST_ASSERT_TRUE(lamp.on);
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, lamp.color);
}

static void test_a_press_held_past_the_end_fires_nothing(void)
{
    start_alert();
    hold(BASELINE, 3000);
    hold(TOUCHED, 1000); /* down when the alert ends, at 3900 ms */
    TEST_ASSERT_FALSE(alert_running(&alert));
    TEST_ASSERT_EQUAL(TOUCH_STATE_PRESSED, touch_detector_state(&td));

    hold(TOUCHED, 1500); /* held 2500 ms in all: no long press either */
    hold(BASELINE, 300); /* nor anything at the release */
    TEST_ASSERT_EQUAL_UINT(0, gestures);
    TEST_ASSERT_TRUE(lamp.on);
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, lamp.color);
}

static void test_a_release_on_the_first_sample_after_the_block_fires_nothing(void)
{
    /* A short press, blocked from its start. */
    for (uint32_t i = 0; i < 100u; i++) {
        now_ms++;
        TEST_ASSERT_EQUAL(TOUCH_EVENT_NONE, touch_gate_feed(&gate, &td, TOUCHED, now_ms, true));
    }
    TEST_ASSERT_EQUAL(TOUCH_STATE_PRESSED, touch_detector_state(&td));

    /* The sample that confirms its release, found on a copy of the detector:
     * left alone, it fires a short press there. */
    touch_detector_t copy = td;
    uint32_t release_ms = now_ms;
    touch_event_t event = TOUCH_EVENT_NONE;
    while (event == TOUCH_EVENT_NONE && release_ms - now_ms < 1000u) {
        release_ms++;
        event = touch_detector_feed(&copy, BASELINE, release_ms);
    }
    TEST_ASSERT_EQUAL(TOUCH_EVENT_SHORT_PRESS, event);

    /* Blocked up to the sample before it, which lifts the block. */
    while (now_ms + 1u < release_ms) {
        now_ms++;
        TEST_ASSERT_EQUAL(TOUCH_EVENT_NONE, touch_gate_feed(&gate, &td, BASELINE, now_ms, true));
    }
    now_ms++;
    TEST_ASSERT_EQUAL(TOUCH_EVENT_NONE, touch_gate_feed(&gate, &td, BASELINE, now_ms, false));
    TEST_ASSERT_EQUAL(TOUCH_STATE_IDLE, touch_detector_state(&td));
}

static void test_after_the_alert_a_short_press_changes_the_color_again(void)
{
    start_alert();
    press(200);
    hold(BASELINE, BREATHS * PERIOD_MS);
    TEST_ASSERT_FALSE(alert_running(&alert));
    TEST_ASSERT_EQUAL_UINT(0, gestures);

    press(200);
    TEST_ASSERT_EQUAL_UINT(1, gestures);
    TEST_ASSERT_EQUAL(LAMP_COLOR_ORANGE, lamp.color);
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_without_an_alert_the_touch_acts);
    RUN_TEST(test_no_gesture_acts_during_an_alert);
    RUN_TEST(test_no_gesture_acts_during_an_alert_in_power_only_mode);
    RUN_TEST(test_the_alert_ends_by_itself_once);
    RUN_TEST(test_a_press_from_before_the_alert_fires_nothing);
    RUN_TEST(test_a_press_held_past_the_end_fires_nothing);
    RUN_TEST(test_a_release_on_the_first_sample_after_the_block_fires_nothing);
    RUN_TEST(test_after_the_alert_a_short_press_changes_the_color_again);
    return UNITY_END();
}
