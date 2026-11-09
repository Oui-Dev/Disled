/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Touch cycle: the default of version 1, custom cycles, their validation,
 * and a color that disappears.
 */
#include <unity.h>

#include "touch_cycle.h"

void setUp(void);
void tearDown(void);

/* Built-in colors and custom slots 4, 9 and 12. */
#define DEFINED (0x000Fu | (1u << 4) | (1u << 9) | (1u << 12))

static touch_cycle_t cycle;

void setUp(void)
{
    touch_cycle_init(&cycle);
}

static void assert_cycle(const uint8_t *expected, uint32_t count)
{
    TEST_ASSERT_EQUAL_UINT8(count, cycle.count);
    TEST_ASSERT_EQUAL_UINT8_ARRAY(expected, cycle.slots, count);
    for (uint32_t i = count; i < TOUCH_CYCLE_MAX; i++) {
        TEST_ASSERT_EQUAL_UINT8(0, cycle.slots[i]);
    }
}

static void test_default_is_green_orange_red(void)
{
    const uint8_t expected[] = { LAMP_COLOR_GREEN, LAMP_COLOR_ORANGE, LAMP_COLOR_RED };
    assert_cycle(expected, 3);
    TEST_ASSERT_EQUAL(LAMP_COLOR_ORANGE, touch_cycle_next(&cycle, LAMP_COLOR_GREEN));
    TEST_ASSERT_EQUAL(LAMP_COLOR_RED, touch_cycle_next(&cycle, LAMP_COLOR_ORANGE));
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, touch_cycle_next(&cycle, LAMP_COLOR_RED));
}

static void test_a_color_outside_the_cycle_goes_to_the_first(void)
{
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, touch_cycle_next(&cycle, LAMP_COLOR_WARM_WHITE));
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, touch_cycle_next(&cycle, (lamp_color_t)9));
}

static void test_custom_cycle_in_its_order(void)
{
    const uint8_t slots[] = { 12, LAMP_COLOR_RED, 4, LAMP_COLOR_GREEN };
    TEST_ASSERT_TRUE(touch_cycle_set(&cycle, slots, 4, DEFINED));
    assert_cycle(slots, 4);
    TEST_ASSERT_EQUAL(LAMP_COLOR_RED, touch_cycle_next(&cycle, (lamp_color_t)12));
    TEST_ASSERT_EQUAL(4, touch_cycle_next(&cycle, LAMP_COLOR_RED));
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, touch_cycle_next(&cycle, (lamp_color_t)4));
    TEST_ASSERT_EQUAL(12, touch_cycle_next(&cycle, LAMP_COLOR_GREEN));
    TEST_ASSERT_EQUAL(12, touch_cycle_next(&cycle, LAMP_COLOR_ORANGE)); /* outside */
}

static void test_one_to_nine_colors(void)
{
    const uint8_t ten[] = { 0, 1, 2, 3, 4, 9, 12, 5, 6, 7 };
    TEST_ASSERT_FALSE(touch_cycle_is_valid(ten, 0, DEFINED));
    TEST_ASSERT_TRUE(touch_cycle_is_valid(ten, 1, DEFINED));
    TEST_ASSERT_TRUE(touch_cycle_is_valid(ten, 7, DEFINED));
    TEST_ASSERT_FALSE(touch_cycle_is_valid(ten, 8, DEFINED)); /* slot 5 holds no color */
    TEST_ASSERT_TRUE(touch_cycle_is_valid(ten, 8, DEFINED | (1u << 5)));
    TEST_ASSERT_TRUE(touch_cycle_is_valid(ten, 9, DEFINED | (1u << 5) | (1u << 6)));
    TEST_ASSERT_FALSE(touch_cycle_is_valid(ten, 10, 0xFFFFu));
}

/* The 4 built-in colors and the 5 custom ones the app allows, all in the
 * cycle. */
static void test_a_cycle_of_nine_colors(void)
{
    const uint8_t nine[] = { 0, 9, 1, 4, 2, 12, 15, 3, 5 };
    const uint32_t defined = DEFINED | (1u << 5) | (1u << 15);
    TEST_ASSERT_TRUE(touch_cycle_set(&cycle, nine, 9, defined));
    assert_cycle(nine, 9);
    TEST_ASSERT_EQUAL(LAMP_COLOR_WARM_WHITE, touch_cycle_next(&cycle, (lamp_color_t)15));
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, touch_cycle_next(&cycle, (lamp_color_t)5)); /* wraps */
    touch_cycle_remove(&cycle, LAMP_COLOR_GREEN);
    assert_cycle(&nine[1], 8);
}

static void test_only_defined_colors_none_twice_none_out_of_range(void)
{
    const uint8_t undefined[] = { LAMP_COLOR_GREEN, 7 };
    TEST_ASSERT_FALSE(touch_cycle_is_valid(undefined, 2, DEFINED));
    const uint8_t twice[] = { LAMP_COLOR_GREEN, LAMP_COLOR_RED, LAMP_COLOR_GREEN };
    TEST_ASSERT_FALSE(touch_cycle_is_valid(twice, 3, DEFINED));
    const uint8_t too_far[] = { LAMP_SLOT_COUNT };
    TEST_ASSERT_FALSE(touch_cycle_is_valid(too_far, 1, 0xFFFFFFFFu));
    const uint8_t far_beyond[] = { 255 };
    TEST_ASSERT_FALSE(touch_cycle_is_valid(far_beyond, 1, 0xFFFFFFFFu));
}

static void test_an_invalid_cycle_changes_nothing(void)
{
    const uint8_t twice[] = { LAMP_COLOR_RED, LAMP_COLOR_RED };
    TEST_ASSERT_FALSE(touch_cycle_set(&cycle, twice, 2, DEFINED));
    const uint8_t expected[] = { LAMP_COLOR_GREEN, LAMP_COLOR_ORANGE, LAMP_COLOR_RED };
    assert_cycle(expected, 3);
}

static void test_a_shorter_cycle_clears_the_old_tail(void)
{
    const uint8_t long_one[] = { 0, 1, 2, 3, 4, 9, 12 };
    TEST_ASSERT_TRUE(touch_cycle_set(&cycle, long_one, 7, DEFINED));
    const uint8_t short_one[] = { 9, 4 };
    TEST_ASSERT_TRUE(touch_cycle_set(&cycle, short_one, 2, DEFINED));
    assert_cycle(short_one, 2);
}

static void test_a_deleted_color_leaves_the_cycle(void)
{
    const uint8_t slots[] = { LAMP_COLOR_GREEN, 9, LAMP_COLOR_RED, 12 };
    TEST_ASSERT_TRUE(touch_cycle_set(&cycle, slots, 4, DEFINED));
    touch_cycle_remove(&cycle, 9);
    const uint8_t expected[] = { LAMP_COLOR_GREEN, LAMP_COLOR_RED, 12 };
    assert_cycle(expected, 3);
    touch_cycle_remove(&cycle, 4); /* not in the cycle */
    assert_cycle(expected, 3);
}

static void test_an_emptied_cycle_is_the_default_again(void)
{
    const uint8_t slots[] = { 9 };
    TEST_ASSERT_TRUE(touch_cycle_set(&cycle, slots, 1, DEFINED));
    TEST_ASSERT_EQUAL(9, touch_cycle_next(&cycle, (lamp_color_t)9));
    touch_cycle_remove(&cycle, 9);
    const uint8_t expected[] = { LAMP_COLOR_GREEN, LAMP_COLOR_ORANGE, LAMP_COLOR_RED };
    assert_cycle(expected, 3);
}

void tearDown(void)
{
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_default_is_green_orange_red);
    RUN_TEST(test_a_color_outside_the_cycle_goes_to_the_first);
    RUN_TEST(test_custom_cycle_in_its_order);
    RUN_TEST(test_one_to_nine_colors);
    RUN_TEST(test_a_cycle_of_nine_colors);
    RUN_TEST(test_only_defined_colors_none_twice_none_out_of_range);
    RUN_TEST(test_an_invalid_cycle_changes_nothing);
    RUN_TEST(test_a_shorter_cycle_clears_the_old_tail);
    RUN_TEST(test_a_deleted_color_leaves_the_cycle);
    RUN_TEST(test_an_emptied_cycle_is_the_default_again);
    return UNITY_END();
}
