/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Alert: the breath curve, its bounds, the end of the alert, and the level
 * rule (up from a dim lamp, down from a bright one).
 */
#include <unity.h>

#include "alert.h"

void setUp(void)
{
}

void tearDown(void)
{
}

static void test_breath_starts_and_ends_at_zero_and_peaks_at_mid_period(void)
{
    TEST_ASSERT_EQUAL_UINT16(0, alert_breath(0, 1300));
    TEST_ASSERT_EQUAL_UINT16(ALERT_AMPLITUDE_MAX, alert_breath(650, 1300));
    TEST_ASSERT_UINT16_WITHIN(2, 0, alert_breath(1299, 1300));
    TEST_ASSERT_EQUAL_UINT16(0, alert_breath(1300, 1300)); /* the next breath */
    TEST_ASSERT_EQUAL_UINT16(ALERT_AMPLITUDE_MAX, alert_breath(1300 + 650, 1300));
}

static void test_breath_stays_within_bounds_for_every_period(void)
{
    for (uint32_t period = ALERT_PERIOD_MIN_MS; period <= ALERT_PERIOD_MAX_MS; period += 97u) {
        for (uint32_t t = 0; t < 2u * period; t++) {
            TEST_ASSERT_TRUE(alert_breath(t, (uint16_t)period) <= ALERT_AMPLITUDE_MAX);
        }
    }
}

static void test_breath_rises_then_falls_without_steps(void)
{
    /* The steepest point of the smoothstep, mid-rise, moves 1.5 x 8192 /
     * period per ms: 12 at 1 s. */
    const uint16_t period = 1000;
    uint16_t previous = alert_breath(0, period);
    for (uint32_t t = 1; t <= period / 2u; t++) {
        const uint16_t a = alert_breath(t, period);
        TEST_ASSERT_TRUE(a >= previous);
        TEST_ASSERT_TRUE((uint32_t)a - previous <= 16u);
        previous = a;
    }
    for (uint32_t t = period / 2u + 1u; t < period; t++) {
        const uint16_t a = alert_breath(t, period);
        TEST_ASSERT_TRUE(a <= previous);
        TEST_ASSERT_TRUE((uint32_t)previous - a <= 16u);
        previous = a;
    }
}

static void test_breath_is_flat_at_both_ends_and_symmetric(void)
{
    /* Smoothstep: the first and last 5 % of a breath barely move. */
    TEST_ASSERT_TRUE(alert_breath(50, 1000) < ALERT_AMPLITUDE_MAX / 30u);
    TEST_ASSERT_TRUE(alert_breath(950, 1000) < ALERT_AMPLITUDE_MAX / 30u);
    for (uint32_t t = 1; t < 650u; t += 13u) {
        TEST_ASSERT_UINT16_WITHIN(4, alert_breath(t, 1300), alert_breath(1300u - t, 1300));
    }
}

static void test_alert_ends_after_its_breaths_and_says_so_once(void)
{
    alert_t alert = { 0 };
    uint16_t amplitude = 1;
    alert_start(&alert, 3, 1300, 1000);
    TEST_ASSERT_TRUE(alert_running(&alert));
    TEST_ASSERT_FALSE(alert_update(&alert, 1000, &amplitude));
    TEST_ASSERT_EQUAL_UINT16(0, amplitude);
    TEST_ASSERT_FALSE(alert_update(&alert, 1000 + 650, &amplitude));
    TEST_ASSERT_EQUAL_UINT16(ALERT_AMPLITUDE_MAX, amplitude);
    TEST_ASSERT_FALSE(alert_update(&alert, 1000 + 2 * 1300 + 650, &amplitude));
    TEST_ASSERT_EQUAL_UINT16(ALERT_AMPLITUDE_MAX, amplitude);
    TEST_ASSERT_FALSE(alert_update(&alert, 1000 + 3 * 1300 - 1, &amplitude));
    TEST_ASSERT_TRUE(alert_running(&alert));

    TEST_ASSERT_TRUE(alert_update(&alert, 1000 + 3 * 1300, &amplitude));
    TEST_ASSERT_EQUAL_UINT16(0, amplitude);
    TEST_ASSERT_FALSE(alert_running(&alert));
    TEST_ASSERT_FALSE(alert_update(&alert, 1000 + 3 * 1300 + 10, &amplitude));
    TEST_ASSERT_EQUAL_UINT16(0, amplitude);
}

static void test_three_default_breaths_last_between_3_and_5_seconds(void)
{
    alert_t alert = { 0 };
    uint16_t amplitude;
    alert_start(&alert, 3, 1300, 0);
    uint32_t end = 0;
    for (uint32_t t = 0; t < 10000u && end == 0u; t += 10u) {
        if (alert_update(&alert, t, &amplitude)) {
            end = t;
        }
    }
    TEST_ASSERT_TRUE(end >= 3000u && end <= 5000u);
}

static void test_a_late_update_still_ends_the_alert(void)
{
    alert_t alert = { 0 };
    uint16_t amplitude = 1;
    alert_start(&alert, 2, 1000, 0);
    TEST_ASSERT_TRUE(alert_update(&alert, 60000, &amplitude));
    TEST_ASSERT_EQUAL_UINT16(0, amplitude);
}

static void test_alert_runs_across_the_millisecond_wrap(void)
{
    alert_t alert = { 0 };
    uint16_t amplitude;
    const uint32_t start = 0xFFFFFF00u;
    alert_start(&alert, 3, 1300, start);
    TEST_ASSERT_FALSE(alert_update(&alert, start + 650u, &amplitude));
    TEST_ASSERT_EQUAL_UINT16(ALERT_AMPLITUDE_MAX, amplitude);
    TEST_ASSERT_FALSE(alert_update(&alert, start + 3899u, &amplitude));
    TEST_ASSERT_TRUE(alert_update(&alert, start + 3900u, &amplitude));
}

static void test_stop_ends_silently(void)
{
    alert_t alert = { 0 };
    uint16_t amplitude = 1;
    alert_start(&alert, 3, 1300, 0);
    alert_stop(&alert);
    TEST_ASSERT_FALSE(alert_running(&alert));
    TEST_ASSERT_FALSE(alert_update(&alert, 650, &amplitude));
    TEST_ASSERT_EQUAL_UINT16(0, amplitude);
}

static void test_a_new_alert_replaces_the_running_one(void)
{
    alert_t alert = { 0 };
    uint16_t amplitude;
    alert_start(&alert, 3, 1300, 0);
    alert_start(&alert, 1, 600, 1000);
    TEST_ASSERT_FALSE(alert_update(&alert, 1300, &amplitude));
    TEST_ASSERT_EQUAL_UINT16(ALERT_AMPLITUDE_MAX, amplitude);
    TEST_ASSERT_TRUE(alert_update(&alert, 1600, &amplitude));
}

static void test_level_rises_from_a_dim_lamp_and_dips_from_a_bright_one(void)
{
    TEST_ASSERT_EQUAL_UINT16(315, alert_level_permille(15, 300, ALERT_AMPLITUDE_MAX));
    TEST_ASSERT_EQUAL_UINT16(165, alert_level_permille(15, 300, ALERT_AMPLITUDE_MAX / 2u));
    TEST_ASSERT_EQUAL_UINT16(15, alert_level_permille(15, 300, 0));
    /* 700 still has room for the whole swing above it, 701 no longer. */
    TEST_ASSERT_EQUAL_UINT16(1000, alert_level_permille(700, 300, ALERT_AMPLITUDE_MAX));
    TEST_ASSERT_EQUAL_UINT16(401, alert_level_permille(701, 300, ALERT_AMPLITUDE_MAX));
    TEST_ASSERT_EQUAL_UINT16(700, alert_level_permille(1000, 300, ALERT_AMPLITUDE_MAX));
}

static void test_level_stays_in_range_for_any_input(void)
{
    for (uint32_t level = 0; level <= 1100u; level += 7u) {
        for (uint32_t depth = 0; depth <= 700u; depth += 50u) {
            for (uint32_t a = 0; a <= ALERT_AMPLITUDE_MAX + 512u; a += 256u) {
                const uint16_t result =
                    alert_level_permille((uint16_t)level, (uint16_t)depth, (uint16_t)a);
                TEST_ASSERT_TRUE(result <= 1000u);
                const uint32_t clamped = (level > 1000u) ? 1000u : level;
                const uint32_t swing = (result > clamped) ? result - clamped : clamped - result;
                TEST_ASSERT_TRUE(swing <= ALERT_DEPTH_MAX_PERMILLE);
                TEST_ASSERT_TRUE(swing <= depth);
            }
        }
    }
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_breath_starts_and_ends_at_zero_and_peaks_at_mid_period);
    RUN_TEST(test_breath_stays_within_bounds_for_every_period);
    RUN_TEST(test_breath_rises_then_falls_without_steps);
    RUN_TEST(test_breath_is_flat_at_both_ends_and_symmetric);
    RUN_TEST(test_alert_ends_after_its_breaths_and_says_so_once);
    RUN_TEST(test_three_default_breaths_last_between_3_and_5_seconds);
    RUN_TEST(test_a_late_update_still_ends_the_alert);
    RUN_TEST(test_alert_runs_across_the_millisecond_wrap);
    RUN_TEST(test_stop_ends_silently);
    RUN_TEST(test_a_new_alert_replaces_the_running_one);
    RUN_TEST(test_level_rises_from_a_dim_lamp_and_dips_from_a_bright_one);
    RUN_TEST(test_level_stays_in_range_for_any_input);
    return UNITY_END();
}
