/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * LED output: the fade in light, the overrides, the alert breath and the
 * current cap, through light tables shaped like the measured ones.
 */
#include <stdio.h>
#include <unity.h>

#include "alert.h"
#include "led_output.h"

void setUp(void);
void tearDown(void);

#define PERIOD 3600u
/* 100 % in sub-ticks. */
#define FULL (PERIOD * LED_DUTY_SUBTICKS)

static const power_config_t power = {
    .full_scale_ma = { 86, 63, 125 },
    .unconfigured_cap_ma = 70,
    .configured_cap_ma = 125,
};

static led_light_cal_t tables[LED_CHANNEL_COUNT];

static const led_output_config_t config = {
    .power = &power,
    .light = tables,
    .fade_ms = 300,
    .pwm_period = PERIOD,
    .alert_depth_permille = 300,
};

/* Tables whose long pulses lose light instead: the duties then exceed their
 * share of the light, which the cap must allow for. */
static led_light_cal_t losing_tables[LED_CHANNEL_COUNT];

static const led_output_config_t losing_config = {
    .power = &power,
    .light = losing_tables,
    .fade_ms = 300,
    .pwm_period = PERIOD,
    .alert_depth_permille = 300,
};

static const rgb16_t green = { 0, 65535, 0 };
static const rgb16_t red = { 65535, 0, 0 };
static const rgb16_t white = { 65535, 65535, 65535 };

static led_output_t out;
static uint16_t duty[LED_CHANNEL_COUNT];
static led_output_inputs_t in;

/* A light table: no light up to `dark` ticks, then a straight rise to the
 * line of the long pulses, (16 n - offset) * 16, met at `meet` ticks, and
 * that line beyond (test_led_cal). */
static led_light_cal_t make_table(uint32_t dark, uint32_t meet, int16_t offset)
{
    led_light_cal_t t = { .offset = offset };
    const int32_t met = (16 * (int32_t)meet - offset) * 16;
    const int32_t rise_ticks = (int32_t)(meet - dark);
    for (uint32_t n = dark + 1u; n <= LED_LIGHT_TICKS; n++) {
        const int32_t line = (16 * (int32_t)n - offset) * 16;
        const int32_t rising = (met * (int32_t)(n - dark) + rise_ticks / 2) / rise_ticks;
        t.light[n] = (uint16_t)((n >= meet) ? line : rising);
    }
    return t;
}

void setUp(void)
{
    /* As measured: no light up to 4 ticks, a steep start, and long pulses
     * that gain 12.5 to 14.4 ticks of light. */
    tables[LED_CHANNEL_RED] = make_table(4, 16, -203);
    tables[LED_CHANNEL_GREEN] = make_table(4, 16, -200);
    tables[LED_CHANNEL_BLUE] = make_table(4, 18, -230);
    losing_tables[LED_CHANNEL_RED] = make_table(18, 19, 300);
    losing_tables[LED_CHANNEL_GREEN] = make_table(10, 12, 150);
    losing_tables[LED_CHANNEL_BLUE] = make_table(18, 19, 300);
    led_output_init(&out, &config);
    in = (led_output_inputs_t){
        .on = true,
        .drive = green,
        .level_permille = 1000,
        .link = USB_LINK_CONFIGURED,
    };
}

void tearDown(void)
{
}

/* The light in LU that a channel shows at a duty. */
static uint32_t light_at(size_t channel, uint32_t d)
{
    return led_cal_light(&tables[channel], (uint16_t)d);
}

/* The duty that a channel needs for a light, 0 to LED_LIGHT_FULL. */
static uint16_t duty_for(size_t channel, uint32_t light)
{
    return led_cal_duty(&tables[channel], light, FULL);
}

/* The light of one more sub-tick at a duty: how close a duty can get. */
static uint32_t subtick_light(size_t channel, uint32_t d)
{
    return light_at(channel, d + 1u) - light_at(channel, d);
}

static void test_fade_in_from_dark_reaches_the_target_at_the_fade_time(void)
{
    led_output_update(&out, &in, 1000, duty);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_GREEN]);

    /* Half way, half the light: since long pulses gain 200 sub-ticks of
     * light, 100 sub-ticks short of half the duty. */
    led_output_update(&out, &in, 1150, duty);
    TEST_ASSERT_UINT32_WITHIN(subtick_light(LED_CHANNEL_GREEN, duty[LED_CHANNEL_GREEN]),
                              light_at(LED_CHANNEL_GREEN, FULL) / 2u,
                              light_at(LED_CHANNEL_GREEN, duty[LED_CHANNEL_GREEN]));
    TEST_ASSERT_UINT16_WITHIN(1, (FULL - 200u) / 2u, duty[LED_CHANNEL_GREEN]);

    led_output_update(&out, &in, 1299, duty);
    TEST_ASSERT_TRUE(duty[LED_CHANNEL_GREEN] < FULL);
    led_output_update(&out, &in, 1300, duty);
    TEST_ASSERT_EQUAL_UINT16(FULL, duty[LED_CHANNEL_GREEN]);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_RED]);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_BLUE]);
}

static void test_a_dim_fade_passes_half_the_light_where_half_the_duty_is_dark(void)
{
    in.level_permille = 20; /* 2 %: a duty in the steep start of the table */
    led_output_update(&out, &in, 1000, duty);
    led_output_update(&out, &in, 1150, duty);
    const uint16_t middle = duty[LED_CHANNEL_GREEN];
    led_output_update(&out, &in, 1300, duty);
    const uint16_t end = duty[LED_CHANNEL_GREEN];

    TEST_ASSERT_UINT32_WITHIN(subtick_light(LED_CHANNEL_GREEN, middle),
                              light_at(LED_CHANNEL_GREEN, end) / 2u,
                              light_at(LED_CHANNEL_GREEN, middle));
    /* Half the duty would show nothing: the 4 dark ticks. */
    TEST_ASSERT_EQUAL_UINT32(0, light_at(LED_CHANNEL_GREEN, end / 2u));
    TEST_ASSERT_TRUE(light_at(LED_CHANNEL_GREEN, middle) > 0u);
}

static void test_turning_off_fades_out(void)
{
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 400, duty);
    in.on = false;
    led_output_update(&out, &in, 1000, duty);
    TEST_ASSERT_EQUAL_UINT16(FULL, duty[LED_CHANNEL_GREEN]);
    led_output_update(&out, &in, 1150, duty);
    TEST_ASSERT_UINT32_WITHIN(subtick_light(LED_CHANNEL_GREEN, duty[LED_CHANNEL_GREEN]),
                              light_at(LED_CHANNEL_GREEN, FULL) / 2u,
                              light_at(LED_CHANNEL_GREEN, duty[LED_CHANNEL_GREEN]));
    led_output_update(&out, &in, 1300, duty);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_GREEN]);
}

static void test_a_new_target_during_a_fade_starts_from_what_is_shown(void)
{
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 150, duty);
    const uint16_t halfway = duty[LED_CHANNEL_GREEN];

    in.drive = red;
    led_output_update(&out, &in, 150, duty);
    TEST_ASSERT_EQUAL_UINT16(halfway, duty[LED_CHANNEL_GREEN]);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_RED]);

    /* Half way again: a quarter of the green light, half the red one. */
    led_output_update(&out, &in, 300, duty);
    TEST_ASSERT_UINT32_WITHIN(subtick_light(LED_CHANNEL_GREEN, duty[LED_CHANNEL_GREEN]),
                              light_at(LED_CHANNEL_GREEN, FULL) / 4u,
                              light_at(LED_CHANNEL_GREEN, duty[LED_CHANNEL_GREEN]));
    TEST_ASSERT_UINT32_WITHIN(subtick_light(LED_CHANNEL_RED, duty[LED_CHANNEL_RED]),
                              light_at(LED_CHANNEL_RED, FULL) / 2u,
                              light_at(LED_CHANNEL_RED, duty[LED_CHANNEL_RED]));

    led_output_update(&out, &in, 450, duty);
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
    TEST_ASSERT_EQUAL_UINT32(0, led_output_estimate_ma_x10(&out));
}

static void test_self_test_shows_its_drive_without_the_level(void)
{
    in.on = false;
    in.level_permille = 0;
    in.show_override = true;
    in.override_drive = (rgb16_t){ 0, 0, 32768 };
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 400, duty);
    /* Half the light of the blue LED, whatever the level: a drive of 65535
     * is LED_LIGHT_FULL. */
    const uint16_t half_blue = duty_for(LED_CHANNEL_BLUE, 32768u * 256u);
    TEST_ASSERT_EQUAL_UINT16(half_blue, duty[LED_CHANNEL_BLUE]);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_RED]);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_GREEN]);
    in.level_permille = 1000;
    led_output_update(&out, &in, 401, duty);
    TEST_ASSERT_EQUAL_UINT16(half_blue, duty[LED_CHANNEL_BLUE]);

    /* Suspend still wins. */
    in.link = USB_LINK_SUSPENDED;
    led_output_update(&out, &in, 402, duty);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_BLUE]);
}

static void test_estimate_is_duty_times_full_scale(void)
{
    const uint16_t red_full[LED_CHANNEL_COUNT] = { FULL, 0, 0 };
    const uint16_t white_full[LED_CHANNEL_COUNT] = { FULL, FULL, FULL };
    const uint16_t half_blue[LED_CHANNEL_COUNT] = { 0, 0, FULL / 2u };
    TEST_ASSERT_EQUAL_UINT32(860, led_output_duty_ma_x10(&power, red_full, FULL));
    TEST_ASSERT_EQUAL_UINT32(2740, led_output_duty_ma_x10(&power, white_full, FULL));
    TEST_ASSERT_EQUAL_UINT32(625, led_output_duty_ma_x10(&power, half_blue, FULL));
}

/* Each channel's light as a share of its full light, in 1/65535. */
static uint32_t light_share(size_t channel)
{
    return (uint32_t)((uint64_t)light_at(channel, duty[channel]) * 65535u /
                      light_at(channel, FULL));
}

static void test_white_is_capped_by_the_usb_state(void)
{
    in.drive = white;
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 400, duty);
    /* About 1 mA under the cap: 0.4 mA kept for the duties that may exceed
     * the light (led_cal_excess), and 0.6 mA since long pulses that gain
     * light need less duty than their share of it. */
    TEST_ASSERT_EQUAL_UINT16(125, led_output_cap_ma(&out));
    TEST_ASSERT_TRUE(led_output_estimate_ma_x10(&out) <= 1250u);
    TEST_ASSERT_TRUE(led_output_estimate_ma_x10(&out) >= 1235u);
    /* The light of all channels scales alike, so the hue stays; scaling
     * the duties would not, the long pulses gaining different lights. */
    TEST_ASSERT_UINT32_WITHIN(2, light_share(LED_CHANNEL_RED), light_share(LED_CHANNEL_GREEN));
    TEST_ASSERT_UINT32_WITHIN(2, light_share(LED_CHANNEL_RED), light_share(LED_CHANNEL_BLUE));

    in.link = USB_LINK_UNCONFIGURED;
    led_output_update(&out, &in, 401, duty);
    TEST_ASSERT_EQUAL_UINT16(70, led_output_cap_ma(&out));
    TEST_ASSERT_TRUE(led_output_estimate_ma_x10(&out) <= 700u);
    TEST_ASSERT_TRUE(led_output_estimate_ma_x10(&out) >= 685u);
}

static void test_cap_override_for_the_thermal_test(void)
{
    in.drive = white;
    in.cap_override_ma = 300;
    led_output_update(&out, &in, 0, duty);
    led_output_update(&out, &in, 400, duty);
    TEST_ASSERT_EQUAL_UINT16(FULL, duty[LED_CHANNEL_RED]);
    TEST_ASSERT_EQUAL_UINT16(FULL, duty[LED_CHANNEL_GREEN]);
    TEST_ASSERT_EQUAL_UINT16(FULL, duty[LED_CHANNEL_BLUE]);
    TEST_ASSERT_EQUAL_UINT32(2740, led_output_estimate_ma_x10(&out));
}

static void test_a_pure_color_at_full_brightness_keeps_its_full_duty(void)
{
    /* Blue alone draws its 125 mA, the whole cap: the excess the cap keeps
     * for longer duties cannot apply to a duty already at 100 %. */
    const rgb16_t pure[LED_CHANNEL_COUNT] = { { 65535, 0, 0 }, { 0, 65535, 0 }, { 0, 0, 65535 } };
    for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
        led_output_init(&out, &config);
        in.drive = pure[c];
        led_output_update(&out, &in, 0, duty);
        led_output_update(&out, &in, 1000, duty);
        TEST_ASSERT_EQUAL_UINT16(FULL, duty[c]);
        TEST_ASSERT_LESS_OR_EQUAL_UINT32(1250, led_output_estimate_ma_x10(&out));
    }
}

static void test_without_tables_the_duty_follows_the_light(void)
{
    /* The timer period of a failed crystal: no table holds, so the duty is
     * proportional to the light, with the cap as before. */
    static led_output_config_t linear;
    linear = config;
    linear.light = NULL;
    linear.fade_ms = 0;
    led_output_init(&out, &linear);
    in.show_override = true;
    in.override_drive = (rgb16_t){ 0, 32768, 0 };
    led_output_update(&out, &in, 0, duty);
    TEST_ASSERT_UINT16_WITHIN(1, FULL / 2u, duty[LED_CHANNEL_GREEN]);
    TEST_ASSERT_EQUAL_UINT16(0, duty[LED_CHANNEL_RED]);
    in.override_drive = (rgb16_t){ 0, 0, 65535 };
    led_output_update(&out, &in, 10, duty);
    TEST_ASSERT_EQUAL_UINT16(FULL, duty[LED_CHANNEL_BLUE]);
    in.override_drive = white;
    led_output_update(&out, &in, 20, duty);
    TEST_ASSERT_LESS_OR_EQUAL_UINT32(1250, led_output_estimate_ma_x10(&out));
}

static uint32_t next_random(uint32_t *state)
{
    /* xorshift32: the same sequence on every run. */
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    return *state;
}

static uint16_t random_below(uint32_t *state, uint32_t limit)
{
    return (uint16_t)(next_random(state) % limit);
}

static char message[120];

/* Random lamp settings: any drive, often white or with a channel at full
 * scale as the palette gives, mostly bright, any link and cap override,
 * sometimes an alert or the self-test. */
static void random_inputs(uint32_t *state)
{
    in.drive.r = random_below(state, 65536);
    in.drive.g = random_below(state, 65536);
    in.drive.b = random_below(state, 65536);
    const uint32_t kind = random_below(state, 4);
    if (kind == 0u) {
        in.drive = white;
    } else if (kind == 1u) {
        in.drive.g = 65535;
    }
    in.on = random_below(state, 8) != 0u;
    in.level_permille = (random_below(state, 4) != 0u) ? (uint16_t)(600u + random_below(state, 401))
                                                       : random_below(state, 1001);
    in.link = (usb_link_t)random_below(state, 3);
    in.cap_override_ma = (random_below(state, 4) == 0u) ? random_below(state, 250) : 0u;
    in.breath = (random_below(state, 3) == 0u) ? random_below(state, ALERT_AMPLITUDE_MAX + 1u) : 0u;
    in.show_override = random_below(state, 10) == 0u;
    in.override_drive = in.drive;
}

#define RANDOM_UPDATES 40000u

/* Random updates through `cfg`, each checked against the cap; returns how
 * many the cap had to limit. */
static uint32_t check_random_updates(const led_output_config_t *cfg)
{
    led_output_init(&out, cfg);
    uint32_t state = 1;
    uint32_t now_ms = 0;
    uint32_t capped = 0;
    for (uint32_t i = 0; i < RANDOM_UPDATES; i++) {
        /* New settings every 4 updates, so that fades end as well as being
         * caught at any point; a breath moves at every update. */
        if (i % 4u == 0u) {
            random_inputs(&state);
        } else if (in.breath != 0u) {
            in.breath = (uint16_t)(1u + random_below(&state, ALERT_AMPLITUDE_MAX));
        }
        now_ms += random_below(&state, 200);
        led_output_update(&out, &in, now_ms, duty);

        uint16_t cap_ma =
            (in.cap_override_ma != 0u) ? in.cap_override_ma : power_cap_ma(&power, in.link);
        if (in.link == USB_LINK_SUSPENDED) {
            cap_ma = 0;
        }
        TEST_ASSERT_EQUAL_UINT16(cap_ma, led_output_cap_ma(&out));
        /* The current of the duties, in tenths of mA times FULL. */
        uint64_t current = 0;
        uint64_t uncapped = 0; /* what the light would draw without the cap */
        for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
            current += (uint64_t)duty[c] * power.full_scale_ma[c] * 10u;
            /* Without tables, the duty is the light's share of FULL. */
            const uint32_t light_duty =
                (cfg->light != NULL)
                    ? led_cal_duty(&cfg->light[c], out.visible[c], FULL)
                    : (uint32_t)(((uint64_t)out.visible[c] * FULL) / LED_LIGHT_FULL);
            uncapped += (uint64_t)light_duty * power.full_scale_ma[c] * 10u;
        }
        if (current > (uint64_t)cap_ma * 10u * FULL) {
            snprintf(message, sizeof message,
                     "update %u: %u/%u tenths of mA, over the cap of %u mA", (unsigned)i,
                     (unsigned)current, (unsigned)FULL, cap_ma);
            TEST_FAIL_MESSAGE(message);
        }
        TEST_ASSERT_EQUAL_UINT32((uint32_t)((current + FULL / 2u) / FULL),
                                 led_output_estimate_ma_x10(&out));
        if (uncapped > (uint64_t)cap_ma * 10u * FULL) {
            capped++;
        }
    }
    return capped;
}

static void test_the_current_never_exceeds_the_cap(void)
{
    /* The cap had work to do in a good share of the updates. */
    TEST_ASSERT_TRUE(check_random_updates(&config) > RANDOM_UPDATES / 5u);
    TEST_ASSERT_TRUE(check_random_updates(&losing_config) > RANDOM_UPDATES / 5u);
    static led_output_config_t linear;
    linear = config;
    linear.light = NULL;
    TEST_ASSERT_TRUE(check_random_updates(&linear) > RANDOM_UPDATES / 5u);
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
    in.drive = red;
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
    in.drive = white;
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

    led_output_update(&out, &in, 800, duty);
    const uint16_t settled = duty[LED_CHANNEL_GREEN];
    settle_on_inputs();
    TEST_ASSERT_EQUAL_UINT16(duty[LED_CHANNEL_GREEN], settled);
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_fade_in_from_dark_reaches_the_target_at_the_fade_time);
    RUN_TEST(test_a_dim_fade_passes_half_the_light_where_half_the_duty_is_dark);
    RUN_TEST(test_turning_off_fades_out);
    RUN_TEST(test_a_new_target_during_a_fade_starts_from_what_is_shown);
    RUN_TEST(test_suspend_turns_off_immediately);
    RUN_TEST(test_self_test_shows_its_drive_without_the_level);
    RUN_TEST(test_estimate_is_duty_times_full_scale);
    RUN_TEST(test_white_is_capped_by_the_usb_state);
    RUN_TEST(test_cap_override_for_the_thermal_test);
    RUN_TEST(test_a_pure_color_at_full_brightness_keeps_its_full_duty);
    RUN_TEST(test_without_tables_the_duty_follows_the_light);
    RUN_TEST(test_the_current_never_exceeds_the_cap);
    RUN_TEST(test_breath_raises_a_dim_lamp_to_the_alert_level);
    RUN_TEST(test_breath_dims_a_bright_lamp);
    RUN_TEST(test_breath_at_zero_changes_nothing);
    RUN_TEST(test_breath_stays_within_the_current_cap);
    RUN_TEST(test_settle_fades_back_from_the_breath_without_a_jump);
    return UNITY_END();
}
