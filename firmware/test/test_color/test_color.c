/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include <unity.h>

#include "color.h"

static const color_config_t config = {
    .palette =
        {
            [LAMP_COLOR_GREEN] = {0, 65535, 0},
            [LAMP_COLOR_ORANGE] = {65535, 16384, 0},
            [LAMP_COLOR_RED] = {65535, 0, 0},
            [LAMP_COLOR_WARM_WHITE] = {65535, 36000, 9000},
        },
};

void setUp(void)
{
}

void tearDown(void)
{
}

static void test_perceived_scale_ends(void)
{
    TEST_ASSERT_EQUAL_UINT16(0, color_perceived_to_linear(0));
    TEST_ASSERT_EQUAL_UINT16(65535, color_perceived_to_linear(1000));
    TEST_ASSERT_EQUAL_UINT16(65535, color_perceived_to_linear(1200));
}

static void test_perceived_mid_grey_is_18_percent(void)
{
    /* L* = 50 is a relative luminance of 18.4 %. */
    TEST_ASSERT_UINT16_WITHIN(20, 12071, color_perceived_to_linear(500));
}

static void test_perceived_scale_is_strictly_increasing(void)
{
    uint16_t previous = color_perceived_to_linear(0);
    for (uint16_t level = 1; level <= 1000; level++) {
        const uint16_t value = color_perceived_to_linear(level);
        TEST_ASSERT_TRUE(value > previous);
        previous = value;
    }
}

static void test_dimmest_manual_level_stays_visible(void)
{
    /* 1 % must still give a few PWM ticks out of 3600 (14 ns each), which
     * the dithering then splits into sixteenths. */
    const uint32_t ticks = (uint32_t)color_perceived_to_linear(10) * 3600u / 65535u;
    TEST_ASSERT_TRUE(ticks >= 3);
}

static void test_shade_applies_the_level(void)
{
    const rgb16_t full = color_shade(config.palette[LAMP_COLOR_GREEN], 1000);
    TEST_ASSERT_EQUAL_UINT16(0, full.r);
    TEST_ASSERT_EQUAL_UINT16(65535, full.g);
    TEST_ASSERT_EQUAL_UINT16(color_perceived_to_linear(500),
                             color_shade(config.palette[LAMP_COLOR_GREEN], 500).g);
    TEST_ASSERT_EQUAL_UINT16(0, full.b);

    const rgb16_t dark = color_shade(config.palette[LAMP_COLOR_RED], 0);
    TEST_ASSERT_EQUAL_UINT16(0, dark.r);
}

static void test_shade_keeps_the_hue_when_dimming(void)
{
    const rgb16_t bright = color_shade(config.palette[LAMP_COLOR_WARM_WHITE], 1000);
    const rgb16_t dim = color_shade(config.palette[LAMP_COLOR_WARM_WHITE], 500);
    /* Same red/blue ratio within rounding. */
    const uint32_t lhs = (uint32_t)bright.r * dim.b;
    const uint32_t rhs = (uint32_t)dim.r * bright.b;
    TEST_ASSERT_UINT32_WITHIN(lhs / 500u + 65535u, lhs, rhs);
}

static void test_mix_ends_and_middle(void)
{
    const rgb16_t a = { 0, 65535, 1000 };
    const rgb16_t b = { 65535, 0, 3000 };
    const rgb16_t start = color_mix(a, b, 0);
    const rgb16_t end = color_mix(a, b, 65535);
    const rgb16_t mid = color_mix(a, b, 32768);
    TEST_ASSERT_EQUAL_MEMORY(&a, &start, sizeof a);
    TEST_ASSERT_EQUAL_MEMORY(&b, &end, sizeof b);
    TEST_ASSERT_UINT16_WITHIN(1, 32768, mid.r);
    TEST_ASSERT_UINT16_WITHIN(1, 32767, mid.g);
    TEST_ASSERT_UINT16_WITHIN(1, 2000, mid.b);
}

static void test_scale(void)
{
    TEST_ASSERT_EQUAL_UINT16(65535, color_scale(65535, 65535));
    TEST_ASSERT_EQUAL_UINT16(0, color_scale(65535, 0));
    TEST_ASSERT_UINT16_WITHIN(1, 16384, color_scale(32768, 32768));
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_perceived_scale_ends);
    RUN_TEST(test_perceived_mid_grey_is_18_percent);
    RUN_TEST(test_perceived_scale_is_strictly_increasing);
    RUN_TEST(test_dimmest_manual_level_stays_visible);
    RUN_TEST(test_shade_applies_the_level);
    RUN_TEST(test_shade_keeps_the_hue_when_dimming);
    RUN_TEST(test_mix_ends_and_middle);
    RUN_TEST(test_scale);
    return UNITY_END();
}
