/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Brightness levels: the CIE L* table from perceived brightness to light.
 */
#include <unity.h>

#include "color.h"

void setUp(void)
{
}

void tearDown(void)
{
}

/* Y = ((L* + 16) / 116)^3 above L* = 8 and L* / 903.3 below, scaled to
 * 65535 and rounded, in exact integers: L* is level / 10. */
static uint32_t cie_linear(uint32_t level_permille)
{
    if (level_permille <= 80u) {
        return (uint32_t)((65535u * (uint64_t)level_permille + 9033u / 2u) / 9033u);
    }
    const uint64_t cube =
        (uint64_t)(level_permille + 160u) * (level_permille + 160u) * (level_permille + 160u);
    const uint64_t full = 1160u * 1160u * (uint64_t)1160u;
    return (uint32_t)((65535u * cube + full / 2u) / full);
}

static void test_perceived_above_1000_is_full_light(void)
{
    TEST_ASSERT_EQUAL_UINT16(65535, color_perceived_to_linear(1200));
}

static void test_perceived_scale_follows_the_cie_curve(void)
{
    /* Exact every 2.5 L*, where the table has its points. In between, the
     * straight line under the cubic stays within 23 of 65535 of it. */
    for (uint16_t level = 0; level <= 1000; level++) {
        const uint32_t exact = cie_linear(level);
        if (level % 25u == 0u) {
            TEST_ASSERT_EQUAL_UINT32(exact, color_perceived_to_linear(level));
        } else {
            TEST_ASSERT_UINT32_WITHIN(24, exact, color_perceived_to_linear(level));
        }
    }
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

static void test_level_shown_spreads_1_to_100_percent_over_the_floor_to_100(void)
{
    TEST_ASSERT_EQUAL_UINT16(30, color_level_shown(10, 30));
    TEST_ASSERT_EQUAL_UINT16(1000, color_level_shown(1000, 30));
    /* Half way between 1 % and 100 %, half way between 3 % and 100 %. */
    TEST_ASSERT_EQUAL_UINT16(515, color_level_shown(505, 30));
    /* Below 1 %, as an automatic curve may ask: the floor. */
    TEST_ASSERT_EQUAL_UINT16(30, color_level_shown(5, 30));
    uint16_t previous = color_level_shown(10, 30);
    for (uint16_t level = 11; level <= 1000; level++) {
        const uint16_t shown = color_level_shown(level, 30);
        TEST_ASSERT_TRUE(shown >= previous && shown >= level);
        previous = shown;
    }
}

static void test_a_floor_of_1_percent_changes_nothing(void)
{
    for (uint16_t level = 10; level <= 1000; level++) {
        TEST_ASSERT_EQUAL_UINT16(level, color_level_shown(level, 10));
    }
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_perceived_above_1000_is_full_light);
    RUN_TEST(test_perceived_scale_follows_the_cie_curve);
    RUN_TEST(test_perceived_scale_is_strictly_increasing);
    RUN_TEST(test_level_shown_spreads_1_to_100_percent_over_the_floor_to_100);
    RUN_TEST(test_a_floor_of_1_percent_changes_nothing);
    return UNITY_END();
}
