/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include <unity.h>

#include "brightness.h"

#define LUX(x) ((uint32_t)(x) * 1000u)

static const brightness_point_t curve[] = {
    { LUX(1), 20 }, { LUX(10), 80 }, { LUX(100), 300 }, { LUX(1000), 800 }, { LUX(3000), 1000 },
};

static const brightness_config_t config = {
    .curve = curve,
    .curve_points = sizeof curve / sizeof curve[0],
    .tick_ms = 100,
    .slow_divisor = 8,
    .fast_divisor = 3,
    .large_q8 = 256,
    .confirm_ms = 400,
    .settled_q8 = 32,
    .hold_q8 = 8,
    .fallback_permille = 500,
};

static brightness_t b;
static uint32_t now_ms;

void setUp(void)
{
    brightness_init(&b, &config);
    now_ms = 1000;
}

void tearDown(void)
{
}

/* Readings of a steady light, `period_ms` apart. */
static void read_lux(uint32_t lux_milli, int count, uint32_t period_ms)
{
    for (int i = 0; i < count; i++) {
        now_ms += period_ms;
        brightness_update_lux(&b, lux_milli, now_ms);
    }
}

static void test_curve_clamps_outside_its_range(void)
{
    TEST_ASSERT_EQUAL_UINT16(20, brightness_curve_permille(&config, 0));
    TEST_ASSERT_EQUAL_UINT16(20, brightness_curve_permille(&config, LUX(1)));
    TEST_ASSERT_EQUAL_UINT16(1000, brightness_curve_permille(&config, LUX(100000)));
}

static void test_curve_hits_its_points(void)
{
    TEST_ASSERT_EQUAL_UINT16(80, brightness_curve_permille(&config, LUX(10)));
    TEST_ASSERT_EQUAL_UINT16(300, brightness_curve_permille(&config, LUX(100)));
    TEST_ASSERT_EQUAL_UINT16(800, brightness_curve_permille(&config, LUX(1000)));
}

static void test_curve_interpolates_on_a_log_scale(void)
{
    /* 31.6 lx is halfway between 10 and 100 lx on a log scale. */
    TEST_ASSERT_UINT16_WITHIN(15, 190, brightness_curve_permille(&config, 31623u));
}

static void test_curve_is_monotonic(void)
{
    uint16_t previous = 0;
    for (uint32_t lux_milli = 1; lux_milli < LUX(5000); lux_milli = lux_milli * 11u / 10u + 1u) {
        const uint16_t level = brightness_curve_permille(&config, lux_milli);
        TEST_ASSERT_TRUE(level >= previous);
        previous = level;
    }
}

static void test_fallback_until_the_first_reading(void)
{
    TEST_ASSERT_EQUAL_UINT16(500, brightness_level_permille(&b));
    read_lux(LUX(100), 1, 100);
    TEST_ASSERT_EQUAL_UINT16(300, brightness_level_permille(&b));
}

static void test_a_lasting_large_change_is_followed_within_1_5_s(void)
{
    read_lux(LUX(1000), 1, 100); /* 800 per mille */
    read_lux(LUX(10), 4, 100);   /* target 80 */
    TEST_ASSERT_EQUAL_UINT16(800, brightness_level_permille(&b));
    read_lux(LUX(10), 7, 100); /* 1.1 s: most of the way */
    TEST_ASSERT_TRUE(brightness_level_permille(&b) < 120);
    read_lux(LUX(10), 4, 100); /* 1.5 s */
    TEST_ASSERT_UINT16_WITHIN(10, 80, brightness_level_permille(&b));
    read_lux(LUX(10), 30, 100);
    TEST_ASSERT_UINT16_WITHIN(3, 80, brightness_level_permille(&b));
}

static void test_a_brief_shadow_does_not_move_the_lamp(void)
{
    read_lux(LUX(100), 1, 100); /* 300 per mille */
    read_lux(LUX(10), 3, 100);  /* a hand over the sensor for 0.3 s */
    read_lux(LUX(100), 10, 100);
    TEST_ASSERT_EQUAL_UINT16(300, brightness_level_permille(&b));
}

static void test_a_shadow_while_following_a_change_does_not_move_the_lamp(void)
{
    read_lux(LUX(10), 1, 100);   /* 80 per mille */
    read_lux(LUX(1000), 5, 100); /* light on: confirmed, the fast filter takes its first step */
    const uint16_t level = brightness_level_permille(&b);
    TEST_ASSERT_TRUE(level > 80);
    read_lux(LUX(10), 3, 100); /* a hand over the sensor for 0.3 s */
    TEST_ASSERT_EQUAL_UINT16(level, brightness_level_permille(&b));
    read_lux(LUX(1000), 30, 100);
    TEST_ASSERT_UINT16_WITHIN(10, 800, brightness_level_permille(&b));
}

static void test_a_second_change_the_same_way_is_followed_at_once(void)
{
    read_lux(LUX(10), 1, 100);    /* 80 per mille */
    read_lux(LUX(1000), 10, 100); /* light on: followed, not settled yet */
    const uint16_t level = brightness_level_permille(&b);
    read_lux(LUX(3000), 1, 100); /* a second light while the first one is followed */
    TEST_ASSERT_TRUE(brightness_level_permille(&b) > level);
}

static void test_small_changes_are_followed_gently(void)
{
    read_lux(LUX(100), 1, 100); /* 300 per mille */
    const uint16_t target = brightness_curve_permille(&config, LUX(150));
    read_lux(LUX(150), 1, 100);
    const uint16_t first = brightness_level_permille(&b);
    TEST_ASSERT_TRUE(first > 300);
    TEST_ASSERT_TRUE(first < 300 + (target - 300) / 4);
    read_lux(LUX(150), 30, 100);
    TEST_ASSERT_UINT16_WITHIN(6, target, brightness_level_permille(&b));
}

static void test_noise_within_the_hold_band_does_not_move_the_lamp(void)
{
    read_lux(LUX(100), 1, 100);
    const uint16_t level = brightness_level_permille(&b);
    for (int i = 0; i < 100; i++) {
        read_lux((i % 2) ? LUX(101) : LUX(99), 1, 100);
    }
    TEST_ASSERT_EQUAL_UINT16(level, brightness_level_permille(&b));
}

static void test_slow_readings_catch_up_with_the_time_elapsed(void)
{
    read_lux(LUX(10), 1, 100);
    read_lux(LUX(15), 4, 100);
    const uint16_t fast_readings = brightness_level_permille(&b);

    brightness_init(&b, &config);
    read_lux(LUX(10), 1, 100);
    read_lux(LUX(15), 1, 400); /* one 400 ms integration in the dark */
    TEST_ASSERT_EQUAL_UINT16(fast_readings, brightness_level_permille(&b));
}

static void test_lights_on_in_the_dark_with_slow_readings(void)
{
    read_lux(LUX(1), 1, 400); /* 20 per mille */
    read_lux(LUX(100), 1, 400);
    TEST_ASSERT_EQUAL_UINT16(20, brightness_level_permille(&b)); /* not confirmed yet */
    read_lux(LUX(100), 2, 400);
    TEST_ASSERT_UINT16_WITHIN(20, 300, brightness_level_permille(&b));
}

static void test_manual_mode_uses_the_host_level(void)
{
    read_lux(LUX(100), 1, 100);
    brightness_set_mode(&b, BRIGHTNESS_MODE_MANUAL, 37);
    TEST_ASSERT_EQUAL_UINT16(370, brightness_level_permille(&b));
    TEST_ASSERT_EQUAL_UINT8(37, brightness_level_percent(&b));
}

static void test_back_to_auto_takes_the_automatic_level(void)
{
    read_lux(LUX(1000), 1, 100);
    brightness_set_mode(&b, BRIGHTNESS_MODE_MANUAL, 10);
    TEST_ASSERT_EQUAL_UINT16(100, brightness_level_permille(&b));
    brightness_set_mode(&b, BRIGHTNESS_MODE_AUTO, 10);
    TEST_ASSERT_EQUAL_UINT16(800, brightness_level_permille(&b));
}

static void test_lost_sensor_falls_back(void)
{
    read_lux(LUX(1), 1, 100);
    brightness_lux_lost(&b);
    TEST_ASSERT_EQUAL_UINT16(500, brightness_level_permille(&b));
    read_lux(LUX(100), 1, 100); /* found again: no crawl from the fallback */
    TEST_ASSERT_EQUAL_UINT16(300, brightness_level_permille(&b));
}

static void test_bias_scales_the_automatic_level(void)
{
    read_lux(LUX(100), 1, 100); /* 300 per mille */
    brightness_set_auto_bias(&b, 20);
    TEST_ASSERT_EQUAL_UINT16(360, brightness_level_permille(&b));
    brightness_set_auto_bias(&b, -50);
    TEST_ASSERT_EQUAL_UINT16(150, brightness_level_permille(&b));
    TEST_ASSERT_EQUAL_INT8(-50, brightness_auto_bias(&b));
}

static void test_bias_is_clamped_and_kept_in_range(void)
{
    read_lux(LUX(3000), 1, 100); /* 1000 per mille */
    brightness_set_auto_bias(&b, 90);
    TEST_ASSERT_EQUAL_INT8(BRIGHTNESS_BIAS_MAX, brightness_auto_bias(&b));
    TEST_ASSERT_EQUAL_UINT16(1000, brightness_level_permille(&b));
    brightness_set_auto_bias(&b, -100);
    TEST_ASSERT_EQUAL_INT8(BRIGHTNESS_BIAS_MIN, brightness_auto_bias(&b));
}

static void test_bias_does_not_touch_manual_mode(void)
{
    brightness_set_auto_bias(&b, 50);
    brightness_set_mode(&b, BRIGHTNESS_MODE_MANUAL, 40);
    TEST_ASSERT_EQUAL_UINT16(400, brightness_level_permille(&b));
}

static void test_percent_is_rounded(void)
{
    brightness_set_mode(&b, BRIGHTNESS_MODE_MANUAL, 1);
    TEST_ASSERT_EQUAL_UINT8(1, brightness_level_percent(&b));
    brightness_set_mode(&b, BRIGHTNESS_MODE_MANUAL, 100);
    TEST_ASSERT_EQUAL_UINT8(100, brightness_level_percent(&b));
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_curve_clamps_outside_its_range);
    RUN_TEST(test_curve_hits_its_points);
    RUN_TEST(test_curve_interpolates_on_a_log_scale);
    RUN_TEST(test_curve_is_monotonic);
    RUN_TEST(test_fallback_until_the_first_reading);
    RUN_TEST(test_a_lasting_large_change_is_followed_within_1_5_s);
    RUN_TEST(test_a_brief_shadow_does_not_move_the_lamp);
    RUN_TEST(test_a_shadow_while_following_a_change_does_not_move_the_lamp);
    RUN_TEST(test_a_second_change_the_same_way_is_followed_at_once);
    RUN_TEST(test_small_changes_are_followed_gently);
    RUN_TEST(test_noise_within_the_hold_band_does_not_move_the_lamp);
    RUN_TEST(test_slow_readings_catch_up_with_the_time_elapsed);
    RUN_TEST(test_lights_on_in_the_dark_with_slow_readings);
    RUN_TEST(test_manual_mode_uses_the_host_level);
    RUN_TEST(test_back_to_auto_takes_the_automatic_level);
    RUN_TEST(test_lost_sensor_falls_back);
    RUN_TEST(test_bias_scales_the_automatic_level);
    RUN_TEST(test_bias_is_clamped_and_kept_in_range);
    RUN_TEST(test_bias_does_not_touch_manual_mode);
    RUN_TEST(test_percent_is_rounded);
    return UNITY_END();
}
