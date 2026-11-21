/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Palette: fixed built-in colors, custom colors normalized to full scale,
 * and the LED drive of each slot.
 */
#include <unity.h>

#include "palette.h"

static const rgb16_t fixed[LAMP_COLOR_COUNT] = {
    { 0, 65535, 0 },
    { 65535, 29000, 0 },
    { 65535, 0, 0 },
    { 65535, 62887, 3435 },
};

static const rgb16_t black = { 0, 0, 0 };
static const rgb16_t violet = { 12000, 0, 65535 };

static palette_t palette;

void setUp(void)
{
    palette_init(&palette, fixed);
}

static void assert_color(rgb16_t expected, rgb16_t actual)
{
    TEST_ASSERT_EQUAL_UINT16(expected.r, actual.r);
    TEST_ASSERT_EQUAL_UINT16(expected.g, actual.g);
    TEST_ASSERT_EQUAL_UINT16(expected.b, actual.b);
}

static void test_starts_with_the_built_in_colors_only(void)
{
    for (uint32_t slot = 0; slot < LAMP_COLOR_COUNT; slot++) {
        TEST_ASSERT_TRUE(palette_defined(&palette, slot));
        assert_color(fixed[slot], palette_color(&palette, slot));
    }
    for (uint32_t slot = LAMP_COLOR_COUNT; slot < LAMP_SLOT_COUNT + 2u; slot++) {
        TEST_ASSERT_FALSE(palette_defined(&palette, slot));
        assert_color(black, palette_color(&palette, slot));
    }
    TEST_ASSERT_EQUAL_HEX16(PALETTE_FIXED_MASK, palette.defined);
}

static void test_built_in_colors_never_change(void)
{
    for (uint32_t slot = 0; slot < LAMP_COLOR_COUNT; slot++) {
        TEST_ASSERT_FALSE(palette_set(&palette, slot, violet));
        TEST_ASSERT_FALSE(palette_set(&palette, slot, black));
        TEST_ASSERT_TRUE(palette_defined(&palette, slot));
        assert_color(fixed[slot], palette_color(&palette, slot));
    }
}

static void test_custom_colors_are_normalized_to_full_scale(void)
{
    TEST_ASSERT_TRUE(palette_color_is_valid((rgb16_t){ 65535, 1, 2 }));
    TEST_ASSERT_TRUE(palette_color_is_valid((rgb16_t){ 1, 65535, 0 }));
    TEST_ASSERT_TRUE(palette_color_is_valid((rgb16_t){ 0, 0, 65535 }));
    TEST_ASSERT_TRUE(palette_color_is_valid(black)); /* empties a slot */
    TEST_ASSERT_FALSE(palette_color_is_valid((rgb16_t){ 65534, 0, 0 }));
    TEST_ASSERT_FALSE(palette_color_is_valid((rgb16_t){ 32768, 32768, 0 }));
    TEST_ASSERT_FALSE(palette_color_is_valid((rgb16_t){ 0, 0, 1 }));

    TEST_ASSERT_FALSE(palette_set(&palette, 4, (rgb16_t){ 40000, 0, 0 }));
    TEST_ASSERT_FALSE(palette_defined(&palette, 4));
}

static void test_set_redefine_and_empty_a_custom_slot(void)
{
    TEST_ASSERT_TRUE(palette_set(&palette, 9, violet));
    TEST_ASSERT_TRUE(palette_defined(&palette, 9));
    assert_color(violet, palette_color(&palette, 9));
    TEST_ASSERT_EQUAL_HEX16(PALETTE_FIXED_MASK | (1u << 9), palette.defined);

    const rgb16_t teal = { 0, 65535, 50000 };
    TEST_ASSERT_TRUE(palette_set(&palette, 9, teal));
    assert_color(teal, palette_color(&palette, 9));

    TEST_ASSERT_TRUE(palette_set(&palette, 9, black));
    TEST_ASSERT_FALSE(palette_defined(&palette, 9));
    assert_color(black, palette_color(&palette, 9));
    TEST_ASSERT_EQUAL_HEX16(PALETTE_FIXED_MASK, palette.defined);
}

/* An LED whose blue gives twice the light of its red and green. */
static const led_color_cal_t cal = {
    .matrix = { { 32767, 0, 0 }, { 0, 32767, 0 }, { 0, 0, 16384 } },
};

static void test_light_calibrates_the_custom_colors_only(void)
{
    /* The built-in colors are LED drives already. */
    for (uint32_t slot = 0; slot < LAMP_COLOR_COUNT; slot++) {
        assert_color(fixed[slot], palette_light(&palette, slot, &cal));
    }
    /* A custom color is sRGB: its white takes half the blue. */
    const rgb16_t white = { 65535, 65535, 65535 };
    TEST_ASSERT_TRUE(palette_set(&palette, 4, white));
    const rgb16_t drive = palette_light(&palette, 4, &cal);
    assert_color(led_cal_drive(&cal, white), drive);
    TEST_ASSERT_EQUAL_UINT16(65535, drive.r);
    TEST_ASSERT_EQUAL_UINT16(65535, drive.g);
    TEST_ASSERT_UINT16_WITHIN(2, 32768, drive.b);
    TEST_ASSERT_TRUE(palette_set(&palette, 9, violet));
    assert_color(led_cal_drive(&cal, violet), palette_light(&palette, 9, &cal));
}

static void test_light_of_an_empty_or_unknown_slot_is_black(void)
{
    assert_color(black, palette_light(&palette, 5, &cal));
    assert_color(black, palette_light(&palette, LAMP_SLOT_COUNT, &cal));
    assert_color(black, palette_light(&palette, 255, &cal));
    TEST_ASSERT_TRUE(palette_set(&palette, 9, violet));
    TEST_ASSERT_TRUE(palette_set(&palette, 9, black));
    assert_color(black, palette_light(&palette, 9, &cal));
}

static void test_every_custom_slot_and_no_other(void)
{
    for (uint32_t slot = LAMP_COLOR_COUNT; slot < LAMP_SLOT_COUNT; slot++) {
        TEST_ASSERT_TRUE(palette_set(&palette, slot, violet));
    }
    TEST_ASSERT_EQUAL_HEX16(0xFFFF, palette.defined);
    TEST_ASSERT_FALSE(palette_set(&palette, LAMP_SLOT_COUNT, violet));
    TEST_ASSERT_FALSE(palette_set(&palette, 255, violet));
}

void tearDown(void)
{
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_starts_with_the_built_in_colors_only);
    RUN_TEST(test_built_in_colors_never_change);
    RUN_TEST(test_custom_colors_are_normalized_to_full_scale);
    RUN_TEST(test_set_redefine_and_empty_a_custom_slot);
    RUN_TEST(test_light_calibrates_the_custom_colors_only);
    RUN_TEST(test_light_of_an_empty_or_unknown_slot_is_black);
    RUN_TEST(test_every_custom_slot_and_no_other);
    return UNITY_END();
}
