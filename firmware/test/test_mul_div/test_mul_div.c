/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Scaling in 32-bit operations: mul_div against 64-bit arithmetic.
 */
#include <unity.h>

#include "mul_div.h"
#include "test_support.h"

void setUp(void)
{
}

void tearDown(void)
{
}

/* a * num / den rounded to the nearest, halves up, saturated: in 64 bits. */
static uint32_t reference(uint32_t a, uint32_t num, uint32_t den)
{
    const uint64_t v = ((uint64_t)a * num + den / 2u) / den;
    return (v > UINT32_MAX) ? UINT32_MAX : (uint32_t)v;
}

static void test_rounds_to_the_nearest(void)
{
    TEST_ASSERT_EQUAL_UINT32(1, mul_div(4, 1, 3));
    TEST_ASSERT_EQUAL_UINT32(2, mul_div(5, 1, 3));
    TEST_ASSERT_EQUAL_UINT32(3, mul_div(5, 1, 2)); /* a half goes up */
    TEST_ASSERT_EQUAL_UINT32(11, mul_div(7, 3, 2));
    TEST_ASSERT_EQUAL_UINT32(0, mul_div(UINT32_MAX, 0, 7));
    /* led_cal_duty: the full light, 65535 * 256, to the LU of a span of
     * 57800 sub-ticks. */
    TEST_ASSERT_EQUAL_UINT32(57800u * 256u, mul_div(65535u * 256u, 57800, 65535));
}

static void test_saturates_beyond_32_bits(void)
{
    /* Up to the top exactly. */
    TEST_ASSERT_EQUAL_UINT32(UINT32_MAX, mul_div(UINT32_MAX, 0xFFFF, 0xFFFF));
    TEST_ASSERT_EQUAL_UINT32(UINT32_MAX - 1u, mul_div(UINT32_MAX - 1u, 0xFFFF, 0xFFFF));
    TEST_ASSERT_EQUAL_UINT32(UINT32_MAX - 0xFFFFu, mul_div(65536, 0xFFFF, 1));
    TEST_ASSERT_EQUAL_UINT32(UINT32_MAX, mul_div(65537, 0xFFFF, 1));
    /* Beyond it, through each part of the sum: the high half of the
     * quotient times num, its low half, then the rounded remainder. */
    TEST_ASSERT_EQUAL_UINT32(UINT32_MAX, mul_div(0x20000, 0xFFFF, 1));
    TEST_ASSERT_EQUAL_UINT32(UINT32_MAX, mul_div(0x1FFFF, 0xFFFF, 1));
    TEST_ASSERT_EQUAL_UINT32(UINT32_MAX, mul_div(131075, 0xFFFF, 2));
}

static void test_matches_64_bit_arithmetic(void)
{
    uint32_t state = 1;
    for (uint32_t i = 0; i < 1000000u; i++) {
        /* Any a, often small; num and den within the 16 bits allowed. */
        const uint32_t shift = next_random(&state) % 32u;
        const uint32_t a = next_random(&state) >> shift;
        const uint32_t num = next_random(&state) % 0x10000u;
        const uint32_t den = 1u + next_random(&state) % 0xFFFFu;
        TEST_ASSERT_EQUAL_UINT32(reference(a, num, den), mul_div(a, num, den));
    }
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_rounds_to_the_nearest);
    RUN_TEST(test_saturates_beyond_32_bits);
    RUN_TEST(test_matches_64_bit_arithmetic);
    return UNITY_END();
}
