/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * The configuration the lamp ships with, src/app/app_config.c: its LED
 * calibration, built-in colors and touch settings, on the build machine. The
 * core suites test the modules on synthetic settings; this one, the measured
 * ones.
 */
#include <stdio.h>
#include <unity.h>

#include "app/app_config.h"
#include "color.h"
#include "led_cal.h"
#include "led_output.h"
#include "test_support.h"
#include "touch_detector.h"

static char message[80];

void setUp(void)
{
}

void tearDown(void)
{
}

/* --- LED -------------------------------------------------------------- */

/* As app_init() sets it up, at the period of the light tables. */
static const led_output_config_t output_config = {
    .power = &app_power_config,
    .light = app_led_light_cal,
    .fade_ms = APP_LED_FADE_MS,
    .pwm_period = APP_LED_CAL_PERIOD,
    .alert_depth_permille = APP_ALERT_DEPTH_PERMILLE,
};

static led_output_t out;

/* The duties of a built-in color at a level, once its fade is over. */
static void show(size_t slot, uint16_t level_permille, usb_link_t link,
                 uint16_t duty[LED_CHANNEL_COUNT])
{
    led_output_init(&out, &output_config);
    const led_output_inputs_t in = {
        .on = true,
        .drive = app_builtin_colors[slot],
        .level_permille = level_permille,
        .link = link,
    };
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, APP_LED_FADE_MS, duty);
}

/* What app_init() asserts: a failure traps, which fails the suite. */
static void test_the_calibration_passes_the_boot_checks(void)
{
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        TEST_ASSERT_TRUE(led_cal_light_valid(&app_led_light_cal[c]));
    }
    TEST_ASSERT_TRUE(led_cal_color_valid(&app_led_color_cal));
    led_output_init(&out, &output_config);
}

/* At level 1 %, each channel that a built-in color uses still gets a pulse,
 * so that the color keeps its hue. */
static void test_the_dimmest_level_lights_every_channel_of_a_built_in_color(void)
{
    const uint16_t dimmest = color_level_shown(10, APP_LED_FLOOR_PERMILLE);
    for (size_t slot = 0; slot < LAMP_COLOR_COUNT; slot++) {
        const rgb16_t c = app_builtin_colors[slot];
        const uint16_t drive[LED_CHANNEL_COUNT] = { c.r, c.g, c.b };
        uint16_t duty[LED_CHANNEL_COUNT];
        show(slot, dimmest, USB_LINK_CONFIGURED, duty);
        for (size_t channel = 0; channel < LED_CHANNEL_COUNT; channel++) {
            snprintf(message, sizeof message, "color %u, channel %u", (unsigned)slot,
                     (unsigned)channel);
            TEST_ASSERT_TRUE_MESSAGE((drive[channel] == 0u) == (duty[channel] == 0u), message);
        }
    }
}

static void test_the_built_in_colors_keep_to_the_cap(void)
{
    const usb_link_t links[] = { USB_LINK_UNCONFIGURED, USB_LINK_CONFIGURED };
    for (size_t l = 0; l < sizeof links / sizeof links[0]; l++) {
        for (size_t slot = 0; slot < LAMP_COLOR_COUNT; slot++) {
            uint16_t duty[LED_CHANNEL_COUNT];
            show(slot, 1000, links[l], duty);
            snprintf(message, sizeof message, "color %u, link %u", (unsigned)slot,
                     (unsigned)links[l]);
            TEST_ASSERT_TRUE_MESSAGE(led_output_estimate_ma_x10(&out) <=
                                         (uint32_t)power_cap_ma(&app_power_config, links[l], 0) *
                                             10u,
                                     message);
        }
    }
}

/* The warm white is the 2700 K white of the app's swatch, #FFAD59, whose
 * linear light led_of() of colors.rs gives, through the color calibration. */
static void test_the_warm_white_is_its_swatch_through_the_calibration(void)
{
    const rgb16_t drive = led_cal_drive(&app_led_color_cal, (rgb16_t){ 65535, 27386, 6547 });
    const rgb16_t warm_white = app_builtin_colors[LAMP_COLOR_WARM_WHITE];
    TEST_ASSERT_UINT16_WITHIN(1, drive.r, warm_white.r);
    TEST_ASSERT_UINT16_WITHIN(1, drive.g, warm_white.g);
    TEST_ASSERT_UINT16_WITHIN(1, drive.b, warm_white.b);
}

/* --- Touch ------------------------------------------------------------ */

/* The drops below describe the board as docs/bringup.md#6-touch records it,
 * through the closed lid and without correction. */

static touch_detector_t td;
static uint32_t now_ms;
static unsigned short_presses;
static unsigned long_presses;

/* One sample per millisecond. */
static void hold(uint16_t raw, uint32_t duration_ms)
{
    for (uint32_t i = 0; i < duration_ms; i++) {
        now_ms++;
        const touch_event_t event = touch_detector_feed(&td, raw, now_ms);
        if (event == TOUCH_EVENT_SHORT_PRESS) {
            short_presses++;
        } else if (event == TOUCH_EVENT_LONG_PRESS) {
            long_presses++;
        }
    }
}

/* A new detector with the shipped settings and this correction, calibrated,
 * then 340 ms at rest. */
static void start(int8_t percent)
{
    now_ms = 0;
    short_presses = 0;
    long_presses = 0;
    touch_detector_init(&td, &app_touch_config);
    touch_detector_set_sensitivity(&td, percent);
    hold(BASELINE, 500);
}

/* The drop goes from `from` to `to` per mille in a straight line. */
static void ramp(uint32_t from, uint32_t to, uint32_t duration_ms)
{
    for (uint32_t i = 0; i < duration_ms; i++) {
        const uint32_t permille = (to >= from) ? from + ((to - from) * i) / duration_ms
                                               : from - ((from - to) * i) / duration_ms;
        hold(dropped(permille), 1);
    }
}

static void assert_thresholds(uint16_t press, uint16_t release, uint16_t rise)
{
    const touch_thresholds_t thresholds = touch_detector_thresholds(&td);
    TEST_ASSERT_EQUAL_UINT16(press, thresholds.press);
    TEST_ASSERT_EQUAL_UINT16(release, thresholds.release);
    TEST_ASSERT_EQUAL_UINT16(rise, thresholds.rise);
}

static uint32_t noise_seed;

/* -9 to 9 counts, the same sequence at every run. */
static int32_t noise_counts(void)
{
    noise_seed = noise_seed * 1103515245u + 12345u;
    return (int32_t)((noise_seed >> 16) % 19u) - 9;
}

/* Idle reading with the user at the desk: each 20 ms window off by a random
 * amount (standard deviation about 5 per mille), and one window in 150 by
 * 25 per mille, the largest single windows of the calibration. */
static void hold_desk_noise(uint32_t duration_ms)
{
    for (uint32_t window = 0; window < duration_ms / 20u; window++) {
        int32_t counts = noise_counts() + noise_counts() + noise_counts() + noise_counts();
        if (window % 150u == 75u) {
            counts = (int32_t)(BASELINE - dropped(25));
        }
        hold((uint16_t)((int32_t)BASELINE - counts), 20);
    }
}

static void test_a_correction_scales_press_release_and_rise(void)
{
    start(20);
    assert_thresholds(48, 27, 40);
    touch_detector_set_sensitivity(&td, -10);
    assert_thresholds(66, 37, 55);

    /* The ends: half the drop, or one and a half times. */
    touch_detector_set_sensitivity(&td, 50);
    assert_thresholds(30, 17, 25);
    touch_detector_set_sensitivity(&td, -50);
    assert_thresholds(90, 51, 75);

    /* Beyond them, the lamp clamps what the host sends. */
    touch_detector_set_sensitivity(&td, 100);
    TEST_ASSERT_EQUAL_INT8(TOUCH_SENSITIVITY_MAX, touch_detector_sensitivity(&td));
    assert_thresholds(30, 17, 25);
    touch_detector_set_sensitivity(&td, INT8_MIN);
    TEST_ASSERT_EQUAL_INT8(TOUCH_SENSITIVITY_MIN, touch_detector_sensitivity(&td));
    assert_thresholds(90, 51, 75);
}

/* The release floor never cuts the most sensitive correction short. */
static void test_the_shipped_thresholds_keep_the_whole_range(void)
{
    start(50);
    TEST_ASSERT_EQUAL_INT8(50, touch_detector_sensitivity(&td));
}

static void test_a_weak_tap_counts_from_plus_30(void)
{
    /* 45 per mille, under the press threshold of 60: a light tap on the
     * center of the lid through a weaker setup than the measured one. */
    for (int32_t percent = -50; percent <= 50; percent += 10) {
        start((int8_t)percent);
        hold(dropped(45), 150);
        hold(BASELINE, 500);
        TEST_ASSERT_EQUAL_UINT_MESSAGE(percent >= 30 ? 1u : 0u, short_presses, "short presses");
        TEST_ASSERT_EQUAL_UINT(0, long_presses);
    }
}

static void test_a_hand_above_the_lid_may_count_from_plus_25(void)
{
    /* A hand settles 1 cm above the lid in 0.7 s, 80 to 89 per mille, stays
     * 3 s, then leaves: too slow a rise for a finger, until the correction
     * brings the rise threshold down to what the hand gives. It then turns
     * the lamp off at 2 s: the nearest hand from +25 %, every one from +35 %,
     * in the 5 % steps of the app. */
    for (uint32_t drop = 80; drop <= 89; drop++) {
        for (int32_t percent = -50; percent <= 50; percent += 5) {
            start((int8_t)percent);
            ramp(0, drop, 700);
            hold(dropped(drop), 3000);
            ramp(drop, 0, 700);
            hold(BASELINE, 1000);
            snprintf(message, sizeof message, "a hand %u per mille down at %+d %%", (unsigned)drop,
                     (int)percent);
            TEST_ASSERT_EQUAL_UINT_MESSAGE(0, short_presses, message);
            if (percent < 25) {
                TEST_ASSERT_EQUAL_UINT_MESSAGE(0, long_presses, message);
            } else if (percent >= 35) {
                TEST_ASSERT_EQUAL_UINT_MESSAGE(1, long_presses, message);
            } else {
                TEST_ASSERT_TRUE_MESSAGE(long_presses <= 1u, message);
            }
        }
    }
}

static void test_the_ends_of_the_range_on_the_measured_board(void)
{
    /* Most sensitive: a minute of idle noise at the desk fires nothing, and
     * the lightest tap of the record still ends, with the noise back, as a
     * short press. */
    noise_seed = 1u;
    start(TOUCH_SENSITIVITY_MAX);
    hold_desk_noise(60000);
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);
    TEST_ASSERT_EQUAL_UINT16(0, touch_detector_recalibrations(&td));
    hold(dropped(69), 150);
    hold_desk_noise(3000);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
    TEST_ASSERT_EQUAL_UINT(0, long_presses);

    /* Least sensitive: a firm tap and a finger held still work. */
    start(TOUCH_SENSITIVITY_MIN);
    hold(dropped(130), 150);
    hold(BASELINE, 500);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
    hold(dropped(170), 2100);
    TEST_ASSERT_EQUAL_UINT(1, long_presses);
    hold(BASELINE, 500);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
}

static void test_a_correction_while_idle_applies_at_once_and_keeps_the_baseline(void)
{
    start(0);
    hold(dropped(5), 2000); /* a baseline that moved a little */
    const uint16_t baseline = touch_detector_baseline(&td);
    touch_detector_set_sensitivity(&td, 30);
    assert_thresholds(42, 24, 35);
    TEST_ASSERT_EQUAL_UINT16(baseline, touch_detector_baseline(&td));
    TEST_ASSERT_EQUAL(TOUCH_STATE_IDLE, touch_detector_state(&td));
    TEST_ASSERT_EQUAL_UINT16(0, touch_detector_recalibrations(&td));

    /* A tap of about 48 per mille below that baseline, under 60, counts at once. */
    hold(dropped(5), 100);
    hold(dropped(50), 150);
    hold(dropped(5), 500);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);

    /* A recalibration, as after a USB suspend, keeps the correction. */
    touch_detector_recalibrate(&td);
    TEST_ASSERT_EQUAL_INT8(30, touch_detector_sensitivity(&td));
    assert_thresholds(42, 24, 35);
}

static void test_a_correction_during_a_press_waits_for_its_release(void)
{
    /* A light press at 65 per mille that relaxes to 45, the finger still down. */
    start(0);
    hold(dropped(65), 300);
    hold(dropped(45), 200);
    TEST_ASSERT_EQUAL(TOUCH_STATE_PRESSED, touch_detector_state(&td));

    /* Applied now, a release of 51 would end the press, finger down, with a
     * short press. It waits. */
    touch_detector_set_sensitivity(&td, -50);
    TEST_ASSERT_EQUAL_INT8(-50, touch_detector_sensitivity(&td));
    assert_thresholds(60, 34, 50);
    hold(dropped(45), 500);
    TEST_ASSERT_EQUAL(TOUCH_STATE_PRESSED, touch_detector_state(&td));
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);

    /* The finger leaves: one short press, then the new thresholds. */
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
    assert_thresholds(90, 51, 75);
    hold(dropped(65), 300);
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);

    /* A long press under way: nothing more until the release, and nothing at it. */
    start(0);
    hold(dropped(100), 2100);
    TEST_ASSERT_EQUAL_UINT(1, long_presses);
    touch_detector_set_sensitivity(&td, 50);
    hold(dropped(100), 500);
    assert_thresholds(60, 34, 50);
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(1, long_presses);
    TEST_ASSERT_EQUAL_UINT(0, short_presses);
    assert_thresholds(30, 17, 25);
}

static void test_a_correction_during_a_press_waits_with_the_hand_near_the_lid(void)
{
    /* The hand rests 30 per mille down: a press asks for 80, the floor and
     * the rise, and its release for 55, the floor and half the rise. */
    start(0);
    hold(dropped(30), 500);
    hold(dropped(100), 300);
    hold(dropped(65), 200); /* the finger relaxes, still down */
    TEST_ASSERT_EQUAL(TOUCH_STATE_PRESSED, touch_detector_state(&td));

    /* Applied now, -50 would take the release to 67 through the rise (75),
     * above the finger: a short press, finger down. It waits. */
    touch_detector_set_sensitivity(&td, -50);
    hold(dropped(65), 500);
    TEST_ASSERT_EQUAL(TOUCH_STATE_PRESSED, touch_detector_state(&td));
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);

    /* The finger leaves, the hand stays: one short press. */
    hold(dropped(30), 400);
    TEST_ASSERT_EQUAL(TOUCH_STATE_IDLE, touch_detector_state(&td));
    TEST_ASSERT_EQUAL_UINT(1, short_presses);

    /* Then the new thresholds, over the same hand: a press asks for 105, so a
     * tap of 100 per mille no longer counts, and one of 115 does. */
    hold(dropped(100), 150);
    hold(dropped(30), 400);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
    hold(dropped(115), 150);
    hold(dropped(30), 400);
    TEST_ASSERT_EQUAL_UINT(2, short_presses);
    TEST_ASSERT_EQUAL_UINT(0, long_presses);
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_the_calibration_passes_the_boot_checks);
    RUN_TEST(test_the_dimmest_level_lights_every_channel_of_a_built_in_color);
    RUN_TEST(test_the_built_in_colors_keep_to_the_cap);
    RUN_TEST(test_the_warm_white_is_its_swatch_through_the_calibration);
    RUN_TEST(test_a_correction_scales_press_release_and_rise);
    RUN_TEST(test_the_shipped_thresholds_keep_the_whole_range);
    RUN_TEST(test_a_weak_tap_counts_from_plus_30);
    RUN_TEST(test_a_hand_above_the_lid_may_count_from_plus_25);
    RUN_TEST(test_the_ends_of_the_range_on_the_measured_board);
    RUN_TEST(test_a_correction_while_idle_applies_at_once_and_keeps_the_baseline);
    RUN_TEST(test_a_correction_during_a_press_waits_for_its_release);
    RUN_TEST(test_a_correction_during_a_press_waits_with_the_hand_near_the_lid);
    return UNITY_END();
}
