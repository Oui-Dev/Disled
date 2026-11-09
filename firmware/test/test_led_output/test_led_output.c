/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include <unity.h>

#include "alert.h"
#include "led_output.h"

#define PERIOD 3600u
/* 100 % in sub-ticks. */
#define FULL (PERIOD * LED_DUTY_SUBTICKS)

static const color_config_t colors = {
    .palette =
        {
            [LAMP_COLOR_GREEN] = {0, 65535, 0},
            [LAMP_COLOR_ORANGE] = {65535, 16384, 0},
            [LAMP_COLOR_RED] = {65535, 0, 0},
            [LAMP_COLOR_WARM_WHITE] = {65535, 65535, 65535},
        },
};

static const power_config_t power = {
    .full_scale_ma = { 86, 63, 125 },
    .unconfigured_cap_ma = 70,
    .configured_cap_ma = 125,
};

static const led_output_config_t config = {
    .power = &power,
    .fade_ms = 300,
    .pwm_period = PERIOD,
    .alert_depth_permille = 300,
};

static led_output_t out;
static uint16_t duty[LED_CHANNEL_COUNT];
static led_output_inputs_t in;

/* With custom colors, the lamp color reaches led_output as its RGB. */
static void use_color(lamp_color_t color)
{
    in.color = colors.palette[color];
}

void setUp(void)
{
    led_output_init(&out, &config);
    in.on = true;
    use_color(LAMP_COLOR_GREEN);
    in.level_permille = 1000;
    in.link = USB_LINK_CONFIGURED;
    in.cap_override_ma = 0;
    in.show_override = false;
    in.breath = 0;
}

void tearDown(void)
{
}

static void test_pulse_loss_is_added_to_lit_channels_only(void)
{
    static led_output_config_t lossy;
    lossy = config;
    lossy.fade_ms = 0;
    lossy.pulse_loss[LED_CHANNEL_GREEN] = 80;
    lossy.pulse_loss[LED_CHANNEL_BLUE] = 80;
    led_output_init(&out, &lossy);

    /* Green at a low level gains the loss; blue stays dark. */
    in.level_permille = 100;
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 10, duty);
    const uint16_t low = duty[LED_CHANNEL_GREEN];
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_BLUE]);
    led_output_init(&out, &config);
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 400, duty);
    TEST_ASSERT_EQUAL_UINT16(duty[LED_CHANNEL_GREEN] + 80u, low);

    /* Full scale never exceeds the period. */
    led_output_init(&out, &lossy);
    in.level_permille = 1000;
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 10, duty);
    TEST_ASSERT_EQUAL_UINT16(FULL, duty[LED_CHANNEL_GREEN]);
}

static void test_fade_in_from_dark(void)
{
    led_output_update(&out, &in, 1000, duty);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_GREEN]);

    led_output_update(&out, &in, 1150, duty);
    TEST_ASSERT_UINT16_WITHIN(30u * LED_DUTY_SUBTICKS, FULL / 2u, duty[LED_CHANNEL_GREEN]);

    led_output_update(&out, &in, 1300, duty);
    TEST_ASSERT_EQUAL_UINT16(FULL, duty[LED_CHANNEL_GREEN]);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_RED]);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_BLUE]);
}

static void test_turning_off_fades_out(void)
{
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 400, duty);
    in.on = false;
    led_output_update(&out, &in, 1000, duty);
    TEST_ASSERT_EQUAL_UINT16(FULL, duty[LED_CHANNEL_GREEN]);
    led_output_update(&out, &in, 1300, duty);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_GREEN]);
}

static void test_retarget_during_a_fade_does_not_jump(void)
{
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 150, duty);
    const uint16_t halfway = duty[LED_CHANNEL_GREEN];

    use_color(LAMP_COLOR_RED);
    led_output_update(&out, &in, 151, duty);
    TEST_ASSERT_UINT16_WITHIN(60u * LED_DUTY_SUBTICKS, halfway, duty[LED_CHANNEL_GREEN]);
    led_output_update(&out, &in, 451, duty);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_GREEN]);
    TEST_ASSERT_EQUAL_UINT16(FULL, duty[LED_CHANNEL_RED]);
}

static void test_suspend_turns_off_immediately(void)
{
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 400, duty);
    in.link = USB_LINK_SUSPENDED;
    led_output_update(&out, &in, 401, duty);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_GREEN]);
    TEST_ASSERT_EQUAL_UINT16(0, led_output_cap_ma(&out));
}

static void test_override_shows_a_raw_color_even_when_off(void)
{
    in.on = false;
    in.show_override = true;
    in.override_rgb = (rgb16_t){ 0, 0, 32768 };
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 400, duty);
    TEST_ASSERT_UINT16_WITHIN(LED_DUTY_SUBTICKS, FULL / 2u, duty[LED_CHANNEL_BLUE]);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_RED]);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_GREEN]);

    /* Suspend still wins. */
    in.link = USB_LINK_SUSPENDED;
    led_output_update(&out, &in, 401, duty);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_BLUE]);
}

static void test_white_is_capped_by_the_usb_state(void)
{
    use_color(LAMP_COLOR_WARM_WHITE); /* full white in this palette */
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 400, duty);
    TEST_ASSERT_TRUE(led_output_estimate_ma_x10(&out) <= 1250u);

    in.link = USB_LINK_UNCONFIGURED;
    led_output_update(&out, &in, 401, duty);
    TEST_ASSERT_TRUE(led_output_estimate_ma_x10(&out) <= 700u);
    TEST_ASSERT_EQUAL_UINT16(70, led_output_cap_ma(&out));
}

static void test_cap_override_for_the_thermal_test(void)
{
    use_color(LAMP_COLOR_WARM_WHITE);
    in.cap_override_ma = 300;
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 400, duty);
    TEST_ASSERT_EQUAL_UINT16(FULL, duty[LED_CHANNEL_BLUE]);
    TEST_ASSERT_UINT32_WITHIN(1, 2740, led_output_estimate_ma_x10(&out));
}

/* Duties once a fade to the inputs has ended. */
static void settle_on_inputs(void)
{
    led_output_init(&out, &config);
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 400, duty);
}

static void test_breath_raises_a_dim_lamp_to_the_alert_level(void)
{
    in.level_permille = 500; /* 200 + the depth of 300 */
    settle_on_inputs();
    const uint16_t expected = duty[LED_CHANNEL_GREEN];

    in.level_permille = 200;
    in.breath = ALERT_AMPLITUDE_MAX;
    settle_on_inputs();
    TEST_ASSERT_EQUAL_UINT16(expected, duty[LED_CHANNEL_GREEN]);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_RED]);
}

static void test_breath_dims_a_bright_lamp(void)
{
    use_color(LAMP_COLOR_RED);
    in.level_permille = 600; /* 900 - 300 */
    settle_on_inputs();
    const uint16_t expected = duty[LED_CHANNEL_RED];

    in.level_permille = 900;
    in.breath = ALERT_AMPLITUDE_MAX;
    settle_on_inputs();
    TEST_ASSERT_EQUAL_UINT16(expected, duty[LED_CHANNEL_RED]);
}

static void test_breath_at_zero_changes_nothing(void)
{
    in.level_permille = 300;
    settle_on_inputs();
    const uint16_t still = duty[LED_CHANNEL_GREEN];
    in.breath = 0;
    led_output_update(&out, &in, 500, duty);
    TEST_ASSERT_EQUAL_UINT16(still, duty[LED_CHANNEL_GREEN]);
}

static void test_breath_stays_within_the_current_cap(void)
{
    /* Full white at 60 % rises to 90 %, far above the 70 mA without a host. */
    use_color(LAMP_COLOR_WARM_WHITE);
    in.link = USB_LINK_UNCONFIGURED;
    in.level_permille = 600;
    in.breath = ALERT_AMPLITUDE_MAX;
    settle_on_inputs();
    TEST_ASSERT_TRUE(led_output_estimate_ma_x10(&out) <= 700u);
}

static void test_settle_fades_back_from_the_breath_without_a_jump(void)
{
    in.level_permille = 200;
    in.breath = ALERT_AMPLITUDE_MAX;
    settle_on_inputs();
    const uint16_t peak = duty[LED_CHANNEL_GREEN];

    /* The alert stops at its peak: the light leaves from there. */
    led_output_settle(&out, 500);
    in.breath = 0;
    led_output_update(&out, &in, 500, duty);
    TEST_ASSERT_EQUAL_UINT16(peak, duty[LED_CHANNEL_GREEN]);
    led_output_update(&out, &in, 650, duty);
    TEST_ASSERT_TRUE(duty[LED_CHANNEL_GREEN] < peak);

    in.level_permille = 200;
    led_output_update(&out, &in, 800, duty);
    const uint16_t settled = duty[LED_CHANNEL_GREEN];
    settle_on_inputs();
    TEST_ASSERT_EQUAL_UINT16(duty[LED_CHANNEL_GREEN], settled);
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_pulse_loss_is_added_to_lit_channels_only);
    RUN_TEST(test_fade_in_from_dark);
    RUN_TEST(test_turning_off_fades_out);
    RUN_TEST(test_retarget_during_a_fade_does_not_jump);
    RUN_TEST(test_suspend_turns_off_immediately);
    RUN_TEST(test_override_shows_a_raw_color_even_when_off);
    RUN_TEST(test_white_is_capped_by_the_usb_state);
    RUN_TEST(test_cap_override_for_the_thermal_test);
    RUN_TEST(test_breath_raises_a_dim_lamp_to_the_alert_level);
    RUN_TEST(test_breath_dims_a_bright_lamp);
    RUN_TEST(test_breath_at_zero_changes_nothing);
    RUN_TEST(test_breath_stays_within_the_current_cap);
    RUN_TEST(test_settle_fades_back_from_the_breath_without_a_jump);
    return UNITY_END();
}
