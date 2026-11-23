/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Palette: fixed built-in colors, custom colors as chromaticities, and the
 * LED drive of each slot.
 */
#include <unity.h>

#include "palette.h"

static const palette_builtin_t fixed[LAMP_COLOR_COUNT] = {
    { .drive = { 0, 65535, 0 }, .xy = { 9748, 46140 } },
    { .drive = { 65535, 29000, 0 }, .xy = { 37825, 25696 } },
    { .drive = { 65535, 0, 0 }, .xy = { 45170, 20347 } },
    { .drive = { 65535, 62887, 3435 }, .xy = { 30097, 26924 } },
};

static const xy16_t none = { 0, 0 };
static const xy16_t white = { LED_WHITE_X, LED_WHITE_Y };
static const xy16_t violet = { 16384, 7864 };
static const rgb16_t black = { 0, 0, 0 };

static palette_t palette;

void setUp(void)
{
    palette_init(&palette, fixed);
}

static void assert_xy(xy16_t expected, xy16_t actual)
{
    TEST_ASSERT_EQUAL_UINT16(expected.x, actual.x);
    TEST_ASSERT_EQUAL_UINT16(expected.y, actual.y);
}

static void assert_drive(rgb16_t expected, rgb16_t actual)
{
    TEST_ASSERT_EQUAL_UINT16(expected.r, actual.r);
    TEST_ASSERT_EQUAL_UINT16(expected.g, actual.g);
    TEST_ASSERT_EQUAL_UINT16(expected.b, actual.b);
}

static void test_starts_with_the_built_in_colors_only(void)
{
    for (uint32_t slot = 0; slot < LAMP_COLOR_COUNT; slot++) {
        TEST_ASSERT_TRUE(palette_defined(&palette, slot));
        assert_xy(fixed[slot].xy, palette_color(&palette, slot));
    }
    for (uint32_t slot = LAMP_COLOR_COUNT; slot < LAMP_SLOT_COUNT + 2u; slot++) {
        TEST_ASSERT_FALSE(palette_defined(&palette, slot));
        assert_xy(none, palette_color(&palette, slot));
    }
    TEST_ASSERT_EQUAL_HEX16(PALETTE_FIXED_MASK, palette.defined);
}

static void test_built_in_colors_never_change(void)
{
    for (uint32_t slot = 0; slot < LAMP_COLOR_COUNT; slot++) {
        TEST_ASSERT_FALSE(palette_set(&palette, slot, violet));
        TEST_ASSERT_FALSE(palette_set(&palette, slot, none));
        TEST_ASSERT_TRUE(palette_defined(&palette, slot));
        assert_xy(fixed[slot].xy, palette_color(&palette, slot));
    }
}

static void test_custom_colors_are_chromaticities(void)
{
    TEST_ASSERT_TRUE(palette_color_is_valid(white));
    TEST_ASSERT_TRUE(palette_color_is_valid((xy16_t){ 0, 65535 }));
    TEST_ASSERT_TRUE(palette_color_is_valid((xy16_t){ 30000, 35536 })); /* x + y = 1 */
    TEST_ASSERT_TRUE(palette_color_is_valid(none));                     /* empties a slot */
    TEST_ASSERT_FALSE(palette_color_is_valid((xy16_t){ 20000, 0 }));
    TEST_ASSERT_FALSE(palette_color_is_valid((xy16_t){ 40000, 30000 }));

    TEST_ASSERT_FALSE(palette_set(&palette, 4, (xy16_t){ 40000, 30000 }));
    TEST_ASSERT_FALSE(palette_defined(&palette, 4));
}

static void test_set_redefine_and_empty_a_custom_slot(void)
{
    TEST_ASSERT_TRUE(palette_set(&palette, 9, violet));
    TEST_ASSERT_TRUE(palette_defined(&palette, 9));
    assert_xy(violet, palette_color(&palette, 9));
    TEST_ASSERT_EQUAL_HEX16(PALETTE_FIXED_MASK | (1u << 9), palette.defined);

    const xy16_t teal = { 11000, 26000 };
    TEST_ASSERT_TRUE(palette_set(&palette, 9, teal));
    assert_xy(teal, palette_color(&palette, 9));

    TEST_ASSERT_TRUE(palette_set(&palette, 9, none));
    TEST_ASSERT_FALSE(palette_defined(&palette, 9));
    assert_xy(none, palette_color(&palette, 9));
    TEST_ASSERT_EQUAL_HEX16(PALETTE_FIXED_MASK, palette.defined);
}

/* An LED with the sRGB primaries (test_led_cal), its blue twice as bright as
 * there: the white takes half the blue. */
static const led_color_cal_t cal = {
    .matrix = { { 20125, -9546, -3096 }, { -6020, 11651, 258 }, { 173, -634, 3283 } },
};

static void test_light_calibrates_the_custom_colors_only(void)
{
    /* The built-in colors are LED drives already. */
    for (uint32_t slot = 0; slot < LAMP_COLOR_COUNT; slot++) {
        assert_drive(fixed[slot].drive, palette_light(&palette, slot, &cal));
    }
    TEST_ASSERT_TRUE(palette_set(&palette, 4, white));
    const rgb16_t drive = palette_light(&palette, 4, &cal);
    assert_drive(led_cal_drive(&cal, white), drive);
    TEST_ASSERT_UINT16_WITHIN(40, 65535, drive.r);
    TEST_ASSERT_UINT16_WITHIN(40, 65535, drive.g);
    TEST_ASSERT_UINT16_WITHIN(40, 32768, drive.b);
    TEST_ASSERT_TRUE(palette_set(&palette, 9, violet));
    assert_drive(led_cal_drive(&cal, violet), palette_light(&palette, 9, &cal));
}

static void test_light_of_an_empty_or_unknown_slot_is_black(void)
{
    assert_drive(black, palette_light(&palette, 5, &cal));
    assert_drive(black, palette_light(&palette, LAMP_SLOT_COUNT, &cal));
    assert_drive(black, palette_light(&palette, 255, &cal));
    TEST_ASSERT_TRUE(palette_set(&palette, 9, violet));
    TEST_ASSERT_TRUE(palette_set(&palette, 9, none));
    assert_drive(black, palette_light(&palette, 9, &cal));
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
    RUN_TEST(test_custom_colors_are_chromaticities);
    RUN_TEST(test_set_redefine_and_empty_a_custom_slot);
    RUN_TEST(test_light_calibrates_the_custom_colors_only);
    RUN_TEST(test_light_of_an_empty_or_unknown_slot_is_black);
    RUN_TEST(test_every_custom_slot_and_no_other);
    return UNITY_END();
}
