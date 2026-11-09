/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include <unity.h>

#include "self_test.h"

static const self_test_config_t config = {
    .color_ms = 1500,
    .touch_wait_ms = 10000,
    .color_linear = 12000,
    .white = { 11000, 10000, 300 },
    .wait_linear = 1000,
    .touch_idle_min = 1000,
    .touch_idle_max = 3800,
};

static self_test_t t;
static self_test_inputs_t in;

void setUp(void)
{
    self_test_init(&t, &config);
    in = (self_test_inputs_t){
        .light_present = true,
        .lux_centi = 12345,
        .touch_baseline = 2620,
        .touch_delta_permille = 0,
        .touch_pressed = false,
    };
}

void tearDown(void)
{
}

/* Runs the test from `from_ms` to `to_ms` in 10 ms steps; returns the time
 * at which it ended, or 0. */
static uint32_t run(uint32_t from_ms, uint32_t to_ms)
{
    for (uint32_t now = from_ms; now <= to_ms; now += 10u) {
        if (self_test_update(&t, now, &in)) {
            return now;
        }
    }
    return 0;
}

static void test_idle_until_started(void)
{
    TEST_ASSERT_FALSE(self_test_running(&t));
    TEST_ASSERT_FALSE(self_test_update(&t, 100, &in));
}

static void test_shows_red_green_blue_white_then_waits_in_blue(void)
{
    self_test_start(&t, 1000);
    TEST_ASSERT_TRUE(self_test_running(&t));
    rgb16_t c = self_test_color(&t, 1000);
    TEST_ASSERT_TRUE(c.r == 12000 && c.g == 0 && c.b == 0);
    c = self_test_color(&t, 2600);
    TEST_ASSERT_TRUE(c.r == 0 && c.g == 12000 && c.b == 0);
    c = self_test_color(&t, 4100);
    TEST_ASSERT_TRUE(c.r == 0 && c.g == 0 && c.b == 12000);
    c = self_test_color(&t, 5600);
    TEST_ASSERT_TRUE(c.r == 11000 && c.g == 10000 && c.b == 300);

    TEST_ASSERT_EQUAL_UINT32(0, run(1000, 7000)); /* colors done at 7000 */
    c = self_test_color(&t, 7010);
    TEST_ASSERT_TRUE(c.r == 0 && c.g == 0 && c.b == 1000);
}

static void test_a_touch_passes_the_test(void)
{
    self_test_start(&t, 0);
    TEST_ASSERT_EQUAL_UINT32(0, run(0, 6990));
    TEST_ASSERT_EQUAL_UINT32(0, run(7000, 8000)); /* waiting */
    in.touch_delta_permille = 52;
    in.touch_pressed = true;
    TEST_ASSERT_EQUAL_UINT32(8010, run(8010, 8100));
    TEST_ASSERT_FALSE(self_test_running(&t));
    TEST_ASSERT_EQUAL_HEX8(SELF_TEST_LIGHT_OK | SELF_TEST_TOUCH_IDLE_OK | SELF_TEST_TOUCH_PRESSED |
                               SELF_TEST_COMPLETED,
                           t.results);
    TEST_ASSERT_EQUAL_INT16(52, t.touch_peak_permille);
    TEST_ASSERT_EQUAL_UINT16(2620, t.touch_baseline);
    TEST_ASSERT_EQUAL_UINT32(12345, t.lux_centi);
}

static void test_no_touch_ends_after_the_wait(void)
{
    self_test_start(&t, 0);
    /* A hand above the lid: as large a drop as a light tap, but no press. */
    in.touch_delta_permille = 70;
    const uint32_t ended = run(0, 20000);
    TEST_ASSERT_EQUAL_UINT32(16000, ended); /* 4 colors x 1.5 s, then 10 s */
    TEST_ASSERT_EQUAL_HEX8(SELF_TEST_LIGHT_OK | SELF_TEST_TOUCH_IDLE_OK | SELF_TEST_COMPLETED,
                           t.results);
    TEST_ASSERT_EQUAL_INT16(70, t.touch_peak_permille);
}

static void test_missing_sensor_and_bad_touch_baseline_are_reported(void)
{
    in.light_present = false;
    in.touch_baseline = 4090; /* saturated TouchKey */
    self_test_start(&t, 0);
    (void)run(0, 7500);
    in.touch_pressed = true;
    (void)run(7510, 8000);
    TEST_ASSERT_EQUAL_HEX8(SELF_TEST_TOUCH_PRESSED | SELF_TEST_COMPLETED, t.results);
}

static void test_a_finger_held_from_the_colors_must_lift_first(void)
{
    self_test_start(&t, 0);
    in.touch_pressed = true; /* down since the colors */
    TEST_ASSERT_EQUAL_UINT32(0, run(0, 9000));
    in.touch_pressed = false;
    TEST_ASSERT_EQUAL_UINT32(0, run(9010, 9100));
    in.touch_pressed = true;
    TEST_ASSERT_EQUAL_UINT32(9110, run(9110, 9200));
    TEST_ASSERT_EQUAL_HEX8(SELF_TEST_TOUCH_PRESSED, t.results & SELF_TEST_TOUCH_PRESSED);
}

static void test_abort_leaves_the_test_incomplete(void)
{
    self_test_start(&t, 0);
    (void)run(0, 8000);
    self_test_abort(&t);
    TEST_ASSERT_FALSE(self_test_running(&t));
    TEST_ASSERT_EQUAL_HEX8(0, t.results & SELF_TEST_COMPLETED);
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_idle_until_started);
    RUN_TEST(test_shows_red_green_blue_white_then_waits_in_blue);
    RUN_TEST(test_a_touch_passes_the_test);
    RUN_TEST(test_no_touch_ends_after_the_wait);
    RUN_TEST(test_missing_sensor_and_bad_touch_baseline_are_reported);
    RUN_TEST(test_a_finger_held_from_the_colors_must_lift_first);
    RUN_TEST(test_abort_leaves_the_test_incomplete);
    return UNITY_END();
}
