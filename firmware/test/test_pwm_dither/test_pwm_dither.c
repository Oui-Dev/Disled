/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include <unity.h>

#include "pwm_dither.h"

void setUp(void)
{
}

void tearDown(void)
{
}

/* Sum of the ticks over `periods` periods. */
static uint32_t run(uint16_t subticks, uint32_t periods)
{
    uint8_t accumulator = 0;
    uint32_t total = 0;
    for (uint32_t i = 0; i < periods; i++) {
        total += pwm_dither_next(&accumulator, subticks);
    }
    return total;
}

static void test_whole_ticks_are_steady(void)
{
    uint8_t accumulator = 0;
    for (int i = 0; i < 40; i++) {
        TEST_ASSERT_EQUAL_UINT16(5, pwm_dither_next(&accumulator, 5u * PWM_DITHER_SUBTICKS));
    }
    TEST_ASSERT_EQUAL_UINT32(0, run(0, 100));
}

static void test_every_fraction_averages_out_over_16_periods(void)
{
    for (uint16_t subticks = 0; subticks < 200u; subticks++) {
        TEST_ASSERT_EQUAL_UINT32(subticks, run(subticks, PWM_DITHER_SUBTICKS));
    }
}

static void test_a_half_alternates(void)
{
    uint8_t accumulator = 0;
    const uint16_t half = 4u * PWM_DITHER_SUBTICKS + PWM_DITHER_SUBTICKS / 2u;
    TEST_ASSERT_EQUAL_UINT16(4, pwm_dither_next(&accumulator, half));
    TEST_ASSERT_EQUAL_UINT16(5, pwm_dither_next(&accumulator, half));
    TEST_ASSERT_EQUAL_UINT16(4, pwm_dither_next(&accumulator, half));
    TEST_ASSERT_EQUAL_UINT16(5, pwm_dither_next(&accumulator, half));
}

static void test_full_scale_is_never_exceeded(void)
{
    uint8_t accumulator = PWM_DITHER_SUBTICKS - 1u; /* left over from a lower duty */
    TEST_ASSERT_EQUAL_UINT16(3600, pwm_dither_next(&accumulator, 3600u * PWM_DITHER_SUBTICKS));
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_whole_ticks_are_steady);
    RUN_TEST(test_every_fraction_averages_out_over_16_periods);
    RUN_TEST(test_a_half_alternates);
    RUN_TEST(test_full_scale_is_never_exceeded);
    return UNITY_END();
}
