/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Touch detection on synthetic TouchKey signals sampled at 1 kHz.
 */
#include <unity.h>

#include "touch_detector.h"

#define BASELINE 2000u
/* A touch lowers the reading by 5 % (50 per mille). */
#define TOUCHED 1900u

static const touch_config_t config = {
    .window_samples = 20,
    .calibration_windows = 8,
    .baseline_shift = 6,
    .press_permille = 40,
    .release_permille = 25,
    .press_rise_permille = 18,
    .proximity_permille = 10,
    .debounce_ms = 60,
    .rearm_ms = 100,
    .long_press_ms = 2000,
    .object_ms = 10000,
    .inverted_permille = 30,
    .inverted_ms = 1000,
};

static touch_detector_t td;
static uint32_t now_ms;
static unsigned short_presses;
static unsigned long_presses;
static uint32_t last_event_ms;

static void reset_counts(void)
{
    short_presses = 0;
    long_presses = 0;
    last_event_ms = 0;
}

static void record(touch_event_t event)
{
    if (event == TOUCH_EVENT_SHORT_PRESS) {
        short_presses++;
        last_event_ms = now_ms;
    } else if (event == TOUCH_EVENT_LONG_PRESS) {
        long_presses++;
        last_event_ms = now_ms;
    }
}

/* One sample per millisecond. */
static void hold(uint16_t raw, uint32_t duration_ms)
{
    for (uint32_t i = 0; i < duration_ms; i++) {
        now_ms++;
        record(touch_detector_feed(&td, raw, now_ms));
    }
}

/* 50 Hz hum of the given amplitude (counts) on top of `raw`. */
static void hold_with_hum(uint16_t raw, int16_t amplitude, uint32_t duration_ms)
{
    /* sin(2*pi*k/20) * 1000, one 50 Hz period at 1 kHz. */
    static const int16_t sine[20] = { 0, 309,  588,  809,  951,  1000,  951,  809,  588,  309,
                                      0, -309, -588, -809, -951, -1000, -951, -809, -588, -309 };
    for (uint32_t i = 0; i < duration_ms; i++) {
        now_ms++;
        const int32_t hum = (amplitude * sine[now_ms % 20u]) / 1000;
        record(touch_detector_feed(&td, (uint16_t)((int32_t)raw + hum), now_ms));
    }
}

void setUp(void)
{
    now_ms = 0;
    reset_counts();
    touch_detector_init(&td, &config);
    hold(BASELINE, 200); /* 8 calibration windows take 160 ms */
}

void tearDown(void)
{
}

static void test_calibration_sets_the_baseline(void)
{
    TEST_ASSERT_EQUAL(TOUCH_STATE_IDLE, touch_detector_state(&td));
    TEST_ASSERT_EQUAL_UINT16(BASELINE, touch_detector_baseline(&td));
    TEST_ASSERT_EQUAL_INT16(0, touch_detector_delta_permille(&td));
}

static void test_recalibration_drops_the_old_baseline(void)
{
    /* As at the end of a USB suspend: the old baseline may no longer hold. */
    touch_detector_recalibrate(&td);
    TEST_ASSERT_EQUAL(TOUCH_STATE_CALIBRATING, touch_detector_state(&td));
    TEST_ASSERT_EQUAL_UINT16(0, touch_detector_baseline(&td));

    hold(BASELINE + 100u, 200);
    TEST_ASSERT_EQUAL(TOUCH_STATE_IDLE, touch_detector_state(&td));
    TEST_ASSERT_EQUAL_UINT16(BASELINE + 100u, touch_detector_baseline(&td));
}

static void test_flat_signal_gives_no_event(void)
{
    hold(BASELINE, 5000);
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);
    TEST_ASSERT_EQUAL_UINT16(0, touch_detector_recalibrations(&td));
}

static void test_short_press_fires_once_on_release(void)
{
    hold(TOUCHED, 300);
    TEST_ASSERT_EQUAL(TOUCH_STATE_PRESSED, touch_detector_state(&td));
    TEST_ASSERT_EQUAL_UINT(0, short_presses);

    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
    TEST_ASSERT_EQUAL_UINT(0, long_presses);
    TEST_ASSERT_EQUAL(TOUCH_STATE_IDLE, touch_detector_state(&td));
}

static void test_blip_shorter_than_debounce_is_ignored(void)
{
    hold(TOUCHED, 40);
    hold(BASELINE, 500);
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);
}

static void test_long_press_fires_at_two_seconds_while_held(void)
{
    const uint32_t start = now_ms;
    hold(TOUCHED, 2100);
    TEST_ASSERT_EQUAL_UINT(1, long_presses);
    /* Timed from the first contact, within one window. */
    TEST_ASSERT_UINT32_WITHIN(25, start + 2000u, last_event_ms);

    hold(TOUCHED, 1000);
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(1, long_presses);
    TEST_ASSERT_EQUAL_UINT(0, short_presses);
}

static void test_object_on_lid_fires_once_then_is_absorbed(void)
{
    hold(TOUCHED, 15000);
    TEST_ASSERT_EQUAL_UINT(1, long_presses);
    TEST_ASSERT_EQUAL_UINT(0, short_presses);
    TEST_ASSERT_EQUAL(TOUCH_STATE_IDLE, touch_detector_state(&td));
    TEST_ASSERT_UINT16_WITHIN(2, TOUCHED, touch_detector_baseline(&td));

    /* Object removed: the reading rises, the baseline recalibrates and
     * nothing fires. */
    hold(BASELINE, 2000);
    TEST_ASSERT_EQUAL_UINT(1, long_presses);
    TEST_ASSERT_EQUAL_UINT(0, short_presses);
    TEST_ASSERT_UINT16_WITHIN(2, BASELINE, touch_detector_baseline(&td));

    /* The touch works again. */
    hold(TOUCHED, 300);
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
}

static void test_mains_hum_is_averaged_out(void)
{
    /* +-3 % hum, close to the press threshold, without any touch. */
    hold_with_hum(BASELINE, 60, 5000);
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);

    /* A touch with the same hum is still detected. */
    hold_with_hum(TOUCHED, 60, 300);
    hold_with_hum(BASELINE, 60, 300);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
}

static void test_baseline_follows_slow_drift(void)
{
    /* +5 % over 50 s: as large as a touch, but far too slow. */
    for (uint16_t step = 0; step <= 100; step++) {
        hold((uint16_t)(BASELINE + step), 500);
    }
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);
    TEST_ASSERT_UINT16_WITHIN(5, BASELINE + 100u, touch_detector_baseline(&td));
}

static void test_finger_at_power_up_recalibrates(void)
{
    touch_detector_init(&td, &config);
    hold(TOUCHED, 200);   /* calibrated with the finger on the lid */
    hold(BASELINE, 1500); /* finger lifted: reading above the baseline */
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);
    TEST_ASSERT_UINT16_WITHIN(2, BASELINE, touch_detector_baseline(&td));

    hold(TOUCHED, 300);
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
}

static void test_hysteresis_keeps_the_press_between_thresholds(void)
{
    hold(TOUCHED, 200);
    /* 3 % drop: below the press threshold but above the release one. */
    hold((uint16_t)(BASELINE - 60u), 500);
    TEST_ASSERT_EQUAL(TOUCH_STATE_PRESSED, touch_detector_state(&td));
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
}

static void test_hand_near_the_lid_freezes_the_baseline(void)
{
    /* 2 % drop for 5 s: a hand close to the lid, no touch. */
    hold((uint16_t)(BASELINE - 40u), 5000);
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);
    TEST_ASSERT_EQUAL_UINT16(BASELINE, touch_detector_baseline(&td));

    /* The baseline was not pulled down: the touch keeps its full drop. */
    hold(BASELINE, 300);
    hold(TOUCHED, 300);
    TEST_ASSERT_EQUAL_INT16(50, touch_detector_delta_permille(&td));
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
}

static void test_object_near_the_lid_is_absorbed(void)
{
    hold((uint16_t)(BASELINE - 40u), 12000);
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);
    TEST_ASSERT_UINT16_WITHIN(1, BASELINE - 40u, touch_detector_baseline(&td));
    TEST_ASSERT_EQUAL_UINT16(1, touch_detector_recalibrations(&td));
}

static void test_a_rise_after_a_hand_near_the_lid_waits_its_own_delay(void)
{
    /* The hand leaves 1 s short of an object, then the reading rises above
     * the baseline: the rise counts its own second, not the hand's time. */
    hold((uint16_t)(BASELINE - 40u), 9000);
    hold(BASELINE + 80u, 900);
    TEST_ASSERT_EQUAL_UINT16(0, touch_detector_recalibrations(&td));
    TEST_ASSERT_EQUAL_UINT16(BASELINE, touch_detector_baseline(&td));

    hold(BASELINE + 80u, 200);
    TEST_ASSERT_EQUAL_UINT16(1, touch_detector_recalibrations(&td));
    TEST_ASSERT_EQUAL_UINT16(BASELINE + 80u, touch_detector_baseline(&td));
}

static void test_a_press_restarts_the_hand_near_the_lid_delay(void)
{
    /* Without the restart, the hand would pass for an object 1 s after the tap. */
    hold((uint16_t)(BASELINE - 40u), 9000);
    hold(TOUCHED, 300);
    hold((uint16_t)(BASELINE - 40u), 2000);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
    TEST_ASSERT_EQUAL_UINT16(0, touch_detector_recalibrations(&td));
    TEST_ASSERT_EQUAL_UINT16(BASELINE, touch_detector_baseline(&td));
}

static void test_taps_with_the_hand_resting_near_the_lid(void)
{
    /* The resting hand (30 per mille) sits above the release threshold:
     * without the floor, the taps would merge into one long press. */
    hold((uint16_t)(BASELINE - 60u), 500);
    for (int i = 0; i < 5; i++) {
        hold((uint16_t)(BASELINE - 110u), 150); /* tap: 55 per mille */
        hold((uint16_t)(BASELINE - 60u), 400);
    }
    TEST_ASSERT_EQUAL_UINT(5, short_presses);
    TEST_ASSERT_EQUAL_UINT(0, long_presses);
}

static void test_long_press_with_the_hand_resting_near_the_lid(void)
{
    hold((uint16_t)(BASELINE - 60u), 500);
    hold((uint16_t)(BASELINE - 110u), 2100);
    TEST_ASSERT_EQUAL_UINT(1, long_presses);
    hold((uint16_t)(BASELINE - 60u), 400);
    TEST_ASSERT_EQUAL_UINT(0, short_presses);
}

static void test_blip_right_after_a_release_is_ignored(void)
{
    hold(TOUCHED, 300);
    hold(BASELINE, 80); /* released after the debounce */
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
    hold(TOUCHED, 80); /* within the re-arm delay */
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
}

static void test_a_consumed_touch_fires_nothing(void)
{
    /* Consumed once pressed. */
    hold(TOUCHED, 300);
    touch_detector_consume(&td);
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);

    /* The next touch works again. */
    hold(TOUCHED, 300);
    hold(BASELINE, 300);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
}

/* app_touch_config (src/app/app_config.c), the thresholds that the board
 * measurements describe: a light tap 77 to 188 per mille, a firm one 120 to
 * 200, a finger held about 170, a hand held 1 cm above the lid 60 to 80, the
 * idle noise at the desk a standard deviation of about 5. */
static const touch_config_t board_config = {
    .window_samples = 20,
    .calibration_windows = 8,
    .baseline_shift = 8,
    .press_permille = 60,
    .release_permille = 34,
    .press_rise_permille = 50,
    .proximity_permille = 10,
    .debounce_ms = 40,
    .rearm_ms = 100,
    .long_press_ms = 2000,
    .object_ms = 10000,
    .inverted_permille = 30,
    .inverted_ms = 1000,
};

/* The reading `permille` below BASELINE. */
static uint16_t dropped(uint32_t permille)
{
    return (uint16_t)(BASELINE - (BASELINE * permille) / 1000u);
}

/* A new detector with this correction, calibrated, then 340 ms at rest. */
static void start(const touch_config_t *cfg, int8_t percent)
{
    now_ms = 0;
    reset_counts();
    touch_detector_init(&td, cfg);
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

static void test_no_correction_keeps_the_configured_thresholds(void)
{
    const touch_config_t *const configs[] = { &config, &board_config };
    for (size_t i = 0; i < sizeof configs / sizeof configs[0]; i++) {
        const touch_config_t *cfg = configs[i];
        touch_detector_init(&td, cfg);
        TEST_ASSERT_EQUAL_INT8(0, touch_detector_sensitivity(&td));
        assert_thresholds(cfg->press_permille, cfg->release_permille, cfg->press_rise_permille);

        touch_detector_set_sensitivity(&td, 30);
        touch_detector_set_sensitivity(&td, 0);
        assert_thresholds(cfg->press_permille, cfg->release_permille, cfg->press_rise_permille);
    }
}

static void test_a_correction_scales_press_release_and_rise(void)
{
    start(&board_config, 20);
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

static void check_every_correction(const touch_config_t *cfg)
{
    const int32_t press = cfg->press_permille;
    const int32_t release = cfg->release_permille;
    for (int32_t asked = INT8_MIN; asked <= INT8_MAX; asked++) {
        touch_detector_init(&td, cfg);
        touch_detector_set_sensitivity(&td, (int8_t)asked);
        const int32_t percent = touch_detector_sensitivity(&td);
        const touch_thresholds_t in_force = touch_detector_thresholds(&td);

        /* Within the limits, on the side asked for, never further. */
        TEST_ASSERT_TRUE(percent >= TOUCH_SENSITIVITY_MIN && percent <= TOUCH_SENSITIVITY_MAX);
        TEST_ASSERT_TRUE(asked >= 0 ? (percent >= 0 && percent <= asked)
                                    : (percent < 0 && percent >= asked));
        /* The release stays under the press, at the configured ratio give or
         * take the rounding of each, and half a band above the proximity band. */
        TEST_ASSERT_TRUE(in_force.release < in_force.press);
        const int32_t skew = (int32_t)in_force.release * press - (int32_t)in_force.press * release;
        TEST_ASSERT_TRUE(2 * (skew < 0 ? -skew : skew) <= press + release);
        TEST_ASSERT_TRUE((uint32_t)in_force.release >=
                         (uint32_t)cfg->proximity_permille + cfg->proximity_permille / 2u);
        /* A touch never needs more than one and a half times the drop. */
        TEST_ASSERT_TRUE(2 * (int32_t)in_force.press <= 3 * press + 1);
    }
}

static void test_every_correction_keeps_the_hysteresis_above_the_noise(void)
{
    check_every_correction(&board_config);
    check_every_correction(&config);
}

static void test_the_release_floor_bounds_the_most_sensitive_corrections(void)
{
    /* Release 25 and proximity 10: from +43 % on, the release would fall
     * under 15, so +50 acts as +42. */
    start(&config, 50);
    TEST_ASSERT_EQUAL_INT8(42, touch_detector_sensitivity(&td));
    assert_thresholds(23, 15, 10);

    /* The board thresholds keep the whole range. */
    start(&board_config, 50);
    TEST_ASSERT_EQUAL_INT8(50, touch_detector_sensitivity(&td));
}

static void test_a_weak_tap_counts_from_plus_30(void)
{
    /* 45 per mille, under the press threshold of 60: a light tap on the
     * center of the lid through a weaker setup than the measured one. */
    for (int32_t percent = -50; percent <= 50; percent += 10) {
        start(&board_config, (int8_t)percent);
        hold(dropped(45), 150);
        hold(BASELINE, 500);
        TEST_ASSERT_EQUAL_UINT_MESSAGE(percent >= 30 ? 1u : 0u, short_presses, "short presses");
        TEST_ASSERT_EQUAL_UINT(0, long_presses);
    }
}

static void test_a_hand_above_the_lid_counts_from_plus_40(void)
{
    /* A hand settles 1 cm above the lid in 0.7 s (70 per mille), stays 3 s,
     * then leaves: too slow a rise for a finger, until the rise threshold
     * comes down to 30 per mille. It then turns the lamp off at 2 s. */
    for (int32_t percent = -50; percent <= 50; percent += 10) {
        start(&board_config, (int8_t)percent);
        ramp(0, 70, 700);
        hold(dropped(70), 3000);
        ramp(70, 0, 700);
        hold(BASELINE, 1000);
        TEST_ASSERT_EQUAL_UINT_MESSAGE(percent >= 40 ? 1u : 0u, long_presses, "long presses");
        TEST_ASSERT_EQUAL_UINT(0, short_presses);
    }
}

static void test_the_ends_of_the_range_on_the_measured_board(void)
{
    /* Most sensitive: a minute of idle noise at the desk fires nothing, and a
     * light tap still ends, with the noise back, as a short press. */
    noise_seed = 1u;
    start(&board_config, TOUCH_SENSITIVITY_MAX);
    hold_desk_noise(60000);
    TEST_ASSERT_EQUAL_UINT(0, short_presses + long_presses);
    TEST_ASSERT_EQUAL_UINT16(0, touch_detector_recalibrations(&td));
    hold(dropped(77), 150);
    hold_desk_noise(3000);
    TEST_ASSERT_EQUAL_UINT(1, short_presses);
    TEST_ASSERT_EQUAL_UINT(0, long_presses);

    /* Least sensitive: a firm tap and a finger held still work. */
    start(&board_config, TOUCH_SENSITIVITY_MIN);
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
    start(&board_config, 0);
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
    start(&board_config, 0);
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
    start(&board_config, 0);
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
    start(&board_config, 0);
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

static void test_back_to_no_correction_behaves_as_never_corrected(void)
{
    touch_detector_t fresh;
    touch_detector_init(&fresh, &board_config);
    touch_detector_init(&td, &board_config);
    touch_detector_set_sensitivity(&td, 50);
    touch_detector_set_sensitivity(&td, -50);
    touch_detector_set_sensitivity(&td, 0);

    /* Rest, taps with a hand resting near the lid, a long press, an object
     * absorbed then removed, a weak tap. */
    static const struct {
        uint16_t permille;
        uint16_t ms;
    } script[] = {
        { 0, 500 },    { 80, 150 }, { 0, 400 },     { 30, 600 }, { 85, 150 }, { 30, 400 },
        { 120, 2500 }, { 0, 600 },  { 150, 11000 }, { 0, 3000 }, { 45, 150 }, { 0, 500 },
    };
    uint32_t t = 0;
    for (size_t i = 0; i < sizeof script / sizeof script[0]; i++) {
        for (uint32_t k = 0; k < script[i].ms; k++) {
            t++;
            const uint16_t raw = dropped(script[i].permille);
            TEST_ASSERT_EQUAL(touch_detector_feed(&fresh, raw, t),
                              touch_detector_feed(&td, raw, t));
            TEST_ASSERT_EQUAL(touch_detector_state(&fresh), touch_detector_state(&td));
            TEST_ASSERT_EQUAL_INT16(touch_detector_delta_permille(&fresh),
                                    touch_detector_delta_permille(&td));
            TEST_ASSERT_EQUAL_UINT16(touch_detector_baseline(&fresh), touch_detector_baseline(&td));
        }
    }
    TEST_ASSERT_EQUAL_UINT16(touch_detector_recalibrations(&fresh),
                             touch_detector_recalibrations(&td));
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_calibration_sets_the_baseline);
    RUN_TEST(test_recalibration_drops_the_old_baseline);
    RUN_TEST(test_flat_signal_gives_no_event);
    RUN_TEST(test_short_press_fires_once_on_release);
    RUN_TEST(test_blip_shorter_than_debounce_is_ignored);
    RUN_TEST(test_long_press_fires_at_two_seconds_while_held);
    RUN_TEST(test_object_on_lid_fires_once_then_is_absorbed);
    RUN_TEST(test_mains_hum_is_averaged_out);
    RUN_TEST(test_baseline_follows_slow_drift);
    RUN_TEST(test_finger_at_power_up_recalibrates);
    RUN_TEST(test_hysteresis_keeps_the_press_between_thresholds);
    RUN_TEST(test_hand_near_the_lid_freezes_the_baseline);
    RUN_TEST(test_object_near_the_lid_is_absorbed);
    RUN_TEST(test_a_rise_after_a_hand_near_the_lid_waits_its_own_delay);
    RUN_TEST(test_a_press_restarts_the_hand_near_the_lid_delay);
    RUN_TEST(test_taps_with_the_hand_resting_near_the_lid);
    RUN_TEST(test_long_press_with_the_hand_resting_near_the_lid);
    RUN_TEST(test_blip_right_after_a_release_is_ignored);
    RUN_TEST(test_a_consumed_touch_fires_nothing);
    RUN_TEST(test_no_correction_keeps_the_configured_thresholds);
    RUN_TEST(test_a_correction_scales_press_release_and_rise);
    RUN_TEST(test_every_correction_keeps_the_hysteresis_above_the_noise);
    RUN_TEST(test_the_release_floor_bounds_the_most_sensitive_corrections);
    RUN_TEST(test_a_weak_tap_counts_from_plus_30);
    RUN_TEST(test_a_hand_above_the_lid_counts_from_plus_40);
    RUN_TEST(test_the_ends_of_the_range_on_the_measured_board);
    RUN_TEST(test_a_correction_while_idle_applies_at_once_and_keeps_the_baseline);
    RUN_TEST(test_a_correction_during_a_press_waits_for_its_release);
    RUN_TEST(test_a_correction_during_a_press_waits_with_the_hand_near_the_lid);
    RUN_TEST(test_back_to_no_correction_behaves_as_never_corrected);
    return UNITY_END();
}
