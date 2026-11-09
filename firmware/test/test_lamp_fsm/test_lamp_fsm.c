/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Lamp rules from docs/behavior.md.
 */
#include <unity.h>

#include "lamp_fsm.h"

static lamp_state_t lamp;
static touch_cycle_t cycle;

void setUp(void)
{
    lamp_fsm_init(&lamp);
    touch_cycle_init(&cycle);
}

static bool short_press(void)
{
    return lamp_fsm_short_press(&lamp, &cycle);
}

void tearDown(void)
{
}

static void test_power_up_is_on_green_auto(void)
{
    TEST_ASSERT_TRUE(lamp.on);
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, lamp.color);
    TEST_ASSERT_EQUAL(BRIGHTNESS_MODE_AUTO, lamp.brightness_mode);
    TEST_ASSERT_EQUAL_UINT8(LAMP_DEFAULT_MANUAL_LEVEL, lamp.manual_level);
}

static void test_short_press_cycles_green_orange_red(void)
{
    TEST_ASSERT_TRUE(short_press());
    TEST_ASSERT_EQUAL(LAMP_COLOR_ORANGE, lamp.color);
    TEST_ASSERT_TRUE(short_press());
    TEST_ASSERT_EQUAL(LAMP_COLOR_RED, lamp.color);
    TEST_ASSERT_TRUE(short_press());
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, lamp.color);
}

static void test_short_press_from_warm_white_goes_green(void)
{
    lamp.color = LAMP_COLOR_WARM_WHITE;
    TEST_ASSERT_TRUE(short_press());
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, lamp.color);
}

static void test_short_press_is_ignored_while_off(void)
{
    lamp.on = false;
    lamp.color = LAMP_COLOR_RED;
    TEST_ASSERT_FALSE(short_press());
    TEST_ASSERT_FALSE(lamp.on);
    TEST_ASSERT_EQUAL(LAMP_COLOR_RED, lamp.color);
}

static void test_long_press_turns_off_then_back_on_with_last_color(void)
{
    lamp.color = LAMP_COLOR_ORANGE;
    lamp_fsm_long_press(&lamp);
    TEST_ASSERT_FALSE(lamp.on);
    lamp_fsm_long_press(&lamp);
    TEST_ASSERT_TRUE(lamp.on);
    TEST_ASSERT_EQUAL(LAMP_COLOR_ORANGE, lamp.color);
}

static void test_request_applies_only_selected_fields(void)
{
    const lamp_request_t request = {
        .fields = LAMP_FIELD_COLOR,
        .on = false,
        .color = LAMP_COLOR_WARM_WHITE,
        .brightness_mode = BRIGHTNESS_MODE_MANUAL,
        .manual_level = 7,
    };
    TEST_ASSERT_TRUE(lamp_fsm_apply(&lamp, &request));
    TEST_ASSERT_TRUE(lamp.on);
    TEST_ASSERT_EQUAL(LAMP_COLOR_WARM_WHITE, lamp.color);
    TEST_ASSERT_EQUAL(BRIGHTNESS_MODE_AUTO, lamp.brightness_mode);
    TEST_ASSERT_EQUAL_UINT8(LAMP_DEFAULT_MANUAL_LEVEL, lamp.manual_level);
}

static void test_request_setting_the_current_values_reports_no_change(void)
{
    const lamp_request_t request = {
        .fields = LAMP_FIELD_POWER | LAMP_FIELD_COLOR,
        .on = true,
        .color = LAMP_COLOR_GREEN,
    };
    TEST_ASSERT_FALSE(lamp_fsm_apply(&lamp, &request));
}

static void test_color_set_while_off_stays_off(void)
{
    lamp.on = false;
    const lamp_request_t request = { .fields = LAMP_FIELD_COLOR, .color = LAMP_COLOR_RED };
    TEST_ASSERT_TRUE(lamp_fsm_apply(&lamp, &request));
    TEST_ASSERT_FALSE(lamp.on);
    TEST_ASSERT_EQUAL(LAMP_COLOR_RED, lamp.color);
}

static void test_invalid_request_changes_nothing(void)
{
    const lamp_state_t before = lamp;
    const lamp_request_t bad_level = {
        .fields = LAMP_FIELD_POWER | LAMP_FIELD_MANUAL_LEVEL,
        .on = false,
        .manual_level = 0,
    };
    TEST_ASSERT_FALSE(lamp_request_is_valid(&bad_level));
    TEST_ASSERT_FALSE(lamp_fsm_apply(&lamp, &bad_level));
    TEST_ASSERT_EQUAL(before.on, lamp.on);
    TEST_ASSERT_EQUAL(before.color, lamp.color);
    TEST_ASSERT_EQUAL(before.brightness_mode, lamp.brightness_mode);
    TEST_ASSERT_EQUAL_UINT8(before.manual_level, lamp.manual_level);
}

static void test_states_differing_in_one_field_are_not_equal(void)
{
    const lamp_state_t base = lamp;
    lamp_state_t other = base;
    TEST_ASSERT_TRUE(lamp_state_equal(&base, &other));
    other.on = !base.on;
    TEST_ASSERT_FALSE(lamp_state_equal(&base, &other));
    other = base;
    other.color = LAMP_COLOR_RED;
    TEST_ASSERT_FALSE(lamp_state_equal(&base, &other));
    other = base;
    other.brightness_mode = BRIGHTNESS_MODE_MANUAL;
    TEST_ASSERT_FALSE(lamp_state_equal(&base, &other));
    other = base;
    other.manual_level = (uint8_t)(base.manual_level + 1u);
    TEST_ASSERT_FALSE(lamp_state_equal(&base, &other));
}

static void test_request_validation_limits(void)
{
    lamp_request_t request = { .fields = LAMP_FIELD_MANUAL_LEVEL, .manual_level = 101 };
    TEST_ASSERT_FALSE(lamp_request_is_valid(&request));
    request.manual_level = 100;
    TEST_ASSERT_TRUE(lamp_request_is_valid(&request));
    request.manual_level = 1;
    TEST_ASSERT_TRUE(lamp_request_is_valid(&request));

    request.fields = LAMP_FIELD_COLOR;
    request.color = (lamp_color_t)LAMP_SLOT_COUNT;
    TEST_ASSERT_FALSE(lamp_request_is_valid(&request));
    request.color = (lamp_color_t)(LAMP_SLOT_COUNT - 1u);
    TEST_ASSERT_TRUE(lamp_request_is_valid(&request));

    request.fields = LAMP_FIELD_BRIGHTNESS_MODE;
    request.brightness_mode = (brightness_mode_t)2;
    TEST_ASSERT_FALSE(lamp_request_is_valid(&request));

    request.fields = (uint8_t)(LAMP_FIELD_ALL + 1u); /* the first unknown bit */
    TEST_ASSERT_FALSE(lamp_request_is_valid(&request));
}

static void test_short_press_follows_a_custom_cycle(void)
{
    const uint8_t slots[] = { LAMP_COLOR_RED, 9, LAMP_COLOR_GREEN };
    TEST_ASSERT_TRUE(touch_cycle_set(&cycle, slots, 3, 0x020Fu));
    lamp.color = LAMP_COLOR_RED;
    TEST_ASSERT_TRUE(short_press());
    TEST_ASSERT_EQUAL(9, lamp.color);
    TEST_ASSERT_TRUE(short_press());
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, lamp.color);
    TEST_ASSERT_TRUE(short_press());
    TEST_ASSERT_EQUAL(LAMP_COLOR_RED, lamp.color);
}

static void test_short_press_outside_the_cycle_goes_to_its_first_color(void)
{
    const uint8_t slots[] = { LAMP_COLOR_ORANGE, LAMP_COLOR_RED };
    TEST_ASSERT_TRUE(touch_cycle_set(&cycle, slots, 2, 0x000Fu));
    lamp.color = LAMP_COLOR_GREEN;
    TEST_ASSERT_TRUE(short_press());
    TEST_ASSERT_EQUAL(LAMP_COLOR_ORANGE, lamp.color);
}

static void test_short_press_on_a_one_color_cycle_changes_nothing(void)
{
    const uint8_t slots[] = { LAMP_COLOR_RED };
    TEST_ASSERT_TRUE(touch_cycle_set(&cycle, slots, 1, 0x000Fu));
    lamp.color = LAMP_COLOR_RED;
    TEST_ASSERT_FALSE(short_press());
    TEST_ASSERT_EQUAL(LAMP_COLOR_RED, lamp.color);
}

static void test_power_only_ignores_the_short_press_not_the_long_one(void)
{
    const lamp_request_t request = {
        .fields = LAMP_FIELD_TOUCH_MODE,
        .touch_mode = TOUCH_MODE_POWER_ONLY,
    };
    TEST_ASSERT_TRUE(lamp_fsm_apply(&lamp, &request));
    TEST_ASSERT_FALSE(short_press());
    TEST_ASSERT_EQUAL(LAMP_COLOR_GREEN, lamp.color);
    lamp_fsm_long_press(&lamp);
    TEST_ASSERT_FALSE(lamp.on);
    lamp_fsm_long_press(&lamp);
    TEST_ASSERT_TRUE(lamp.on);
}

static void test_touch_mode_falls_back_when_the_host_leaves(void)
{
    change_source_t source = CHANGE_SOURCE_BOOT;
    lamp.touch_mode = TOUCH_MODE_POWER_ONLY;
    TEST_ASSERT_FALSE(lamp_fsm_touch_fallback(&lamp, USB_LINK_CONFIGURED, 0, 10000, &source));
    TEST_ASSERT_EQUAL(TOUCH_MODE_POWER_ONLY, lamp.touch_mode);

    TEST_ASSERT_TRUE(lamp_fsm_touch_fallback(&lamp, USB_LINK_SUSPENDED, 0, 10000, &source));
    TEST_ASSERT_EQUAL(TOUCH_MODE_NORMAL, lamp.touch_mode);
    TEST_ASSERT_EQUAL(CHANGE_SOURCE_USB, source);

    lamp.touch_mode = TOUCH_MODE_POWER_ONLY;
    TEST_ASSERT_TRUE(lamp_fsm_touch_fallback(&lamp, USB_LINK_UNCONFIGURED, 0, 10000, &source));
    TEST_ASSERT_EQUAL(TOUCH_MODE_NORMAL, lamp.touch_mode);
    TEST_ASSERT_TRUE(short_press()); /* the lid works again */
}

static void test_touch_mode_falls_back_after_host_silence(void)
{
    change_source_t source = CHANGE_SOURCE_BOOT;
    lamp.touch_mode = TOUCH_MODE_POWER_ONLY;
    TEST_ASSERT_FALSE(lamp_fsm_touch_fallback(&lamp, USB_LINK_CONFIGURED, 9999, 10000, &source));
    TEST_ASSERT_EQUAL(CHANGE_SOURCE_BOOT, source);
    TEST_ASSERT_TRUE(lamp_fsm_touch_fallback(&lamp, USB_LINK_CONFIGURED, 10000, 10000, &source));
    TEST_ASSERT_EQUAL(TOUCH_MODE_NORMAL, lamp.touch_mode);
    TEST_ASSERT_EQUAL(CHANGE_SOURCE_HOST_SILENCE, source);

    /* Normal mode has nothing to fall back from. */
    TEST_ASSERT_FALSE(lamp_fsm_touch_fallback(&lamp, USB_LINK_SUSPENDED, 99999, 10000, &source));
}

static void test_states_differing_in_touch_mode_are_not_equal(void)
{
    lamp_state_t other = lamp;
    other.touch_mode = TOUCH_MODE_POWER_ONLY;
    TEST_ASSERT_FALSE(lamp_state_equal(&lamp, &other));

    lamp_request_t request = { .fields = LAMP_FIELD_TOUCH_MODE, .touch_mode = (touch_mode_t)2 };
    TEST_ASSERT_FALSE(lamp_request_is_valid(&request));
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_power_up_is_on_green_auto);
    RUN_TEST(test_short_press_cycles_green_orange_red);
    RUN_TEST(test_short_press_from_warm_white_goes_green);
    RUN_TEST(test_short_press_is_ignored_while_off);
    RUN_TEST(test_long_press_turns_off_then_back_on_with_last_color);
    RUN_TEST(test_request_applies_only_selected_fields);
    RUN_TEST(test_request_setting_the_current_values_reports_no_change);
    RUN_TEST(test_color_set_while_off_stays_off);
    RUN_TEST(test_invalid_request_changes_nothing);
    RUN_TEST(test_states_differing_in_one_field_are_not_equal);
    RUN_TEST(test_request_validation_limits);
    RUN_TEST(test_short_press_follows_a_custom_cycle);
    RUN_TEST(test_short_press_outside_the_cycle_goes_to_its_first_color);
    RUN_TEST(test_short_press_on_a_one_color_cycle_changes_nothing);
    RUN_TEST(test_power_only_ignores_the_short_press_not_the_long_one);
    RUN_TEST(test_touch_mode_falls_back_when_the_host_leaves);
    RUN_TEST(test_touch_mode_falls_back_after_host_silence);
    RUN_TEST(test_states_differing_in_touch_mode_are_not_equal);
    return UNITY_END();
}
