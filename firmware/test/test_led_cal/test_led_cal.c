/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * LED calibration: a chromaticity to LED drive with its gamut mapping,
 * against an exact 64-bit reference, and light to duty on synthetic light
 * tables shaped like the measured ones: no light from the shortest pulses, a
 * steep start, long pulses that gain light or lose it.
 */
#include <stdio.h>
#include <unity.h>

#include "led_cal.h"
#include "test_support.h"

#define N    LED_LIGHT_TICKS
#define FULL 57600u /* 100 % duty at 3600 ticks, in sub-ticks */

static char message[120];

/* For the loops over thousands of values: says which one failed. */
#define FAIL_AT(...)                                    \
    do {                                                \
        snprintf(message, sizeof message, __VA_ARGS__); \
        TEST_FAIL_MESSAGE(message);                     \
    } while (0)

/* --- Chromaticity to drive -------------------------------------------- */

/* An LED with the sRGB primaries: the XYZ to linear sRGB matrix of IEC
 * 61966-2-1, its red row at the 32767 that the products allow. A color of
 * the sRGB triangle shows as its linear sRGB, normalized, and one outside
 * needs white. */
static const led_color_cal_t cal = {
    .matrix =
        {
            { 20125, -9546, -3096 },
            { -6020, 11651, 258 },
            { 346, -1267, 6566 },
        },
};

/* led_cal_drive keeps 15 significant bits of the matrix product and 14 of
 * the white. Over 300 000 random chromaticities with this matrix, that put
 * every channel within 4 of the exact drive inside the gamut, and within 5
 * outside, where the white adds its own error. */
#define DRIVE_TOLERANCE_INSIDE  5u
#define DRIVE_TOLERANCE_OUTSIDE 8u

typedef struct {
    uint16_t drive[LED_CHANNEL_COUNT];
    /* The channel that the white brings to 0, LED_CHANNEL_COUNT inside the gamut. */
    size_t zero;
} reference_t;

/* led_cal_drive in exact integers: d, the matrix times x, y and 1 - x - y;
 * outside the gamut, white[m] * (d + k * white), the white being d for D65
 * and k = -d[m] / white[m] the largest over the negative channels; then the
 * largest channel to 65535. |d| and the white stay below 2^31, so their
 * products fit 64 bits, and the last ratio needs no more than a double. */
static reference_t reference_drive(const led_color_cal_t *c, xy16_t xy)
{
    const int64_t in[3] = { xy.x, xy.y, (int64_t)XY_ONE - xy.x - xy.y };
    const int64_t at_white[3] = { LED_WHITE_X, LED_WHITE_Y,
                                  (int64_t)XY_ONE - LED_WHITE_X - LED_WHITE_Y };
    int64_t d[LED_CHANNEL_COUNT];
    int64_t white[LED_CHANNEL_COUNT];
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        d[i] = 0;
        white[i] = 0;
        for (size_t j = 0; j < 3u; j++) {
            d[i] += c->matrix[i][j] * in[j];
            white[i] += c->matrix[i][j] * at_white[j];
        }
    }
    size_t m = LED_CHANNEL_COUNT;
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        if (d[i] < 0 && (m == LED_CHANNEL_COUNT || -d[i] * white[m] > -d[m] * white[i])) {
            m = i;
        }
    }
    int64_t e[LED_CHANNEL_COUNT];
    int64_t top = 0;
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        e[i] = (m == LED_CHANNEL_COUNT) ? d[i] : d[i] * white[m] - d[m] * white[i];
        if (e[i] > top) {
            top = e[i];
        }
    }
    reference_t ref = { .zero = m };
    for (size_t i = 0; i < LED_CHANNEL_COUNT && top != 0; i++) {
        ref.drive[i] = (uint16_t)((double)e[i] / (double)top * 65535.0 + 0.5);
    }
    return ref;
}

static void channels(rgb16_t color, uint16_t out[LED_CHANNEL_COUNT])
{
    out[LED_CHANNEL_RED] = color.r;
    out[LED_CHANNEL_GREEN] = color.g;
    out[LED_CHANNEL_BLUE] = color.b;
}

static uint16_t largest(rgb16_t color)
{
    uint16_t c[LED_CHANNEL_COUNT];
    channels(color, c);
    const uint16_t rg = (c[0] > c[1]) ? c[0] : c[1];
    return (rg > c[2]) ? rg : c[2];
}

static void assert_drive_within(uint16_t tolerance, rgb16_t expected, rgb16_t actual)
{
    TEST_ASSERT_UINT16_WITHIN(tolerance, expected.r, actual.r);
    TEST_ASSERT_UINT16_WITHIN(tolerance, expected.g, actual.g);
    TEST_ASSERT_UINT16_WITHIN(tolerance, expected.b, actual.b);
}

static void test_a_chromaticity_has_y_above_0_and_x_plus_y_at_most_1(void)
{
    TEST_ASSERT_FALSE(led_cal_xy_valid((xy16_t){ 0, 0 }));
    TEST_ASSERT_FALSE(led_cal_xy_valid((xy16_t){ 20000, 0 }));
    TEST_ASSERT_TRUE(led_cal_xy_valid((xy16_t){ 0, 1 }));
    TEST_ASSERT_TRUE(led_cal_xy_valid((xy16_t){ 65535, 1 }));
    TEST_ASSERT_TRUE(led_cal_xy_valid((xy16_t){ 30000, 35536 }));
    TEST_ASSERT_FALSE(led_cal_xy_valid((xy16_t){ 30000, 35537 }));
}

static void test_anything_but_a_chromaticity_gives_black(void)
{
    const xy16_t none[] = { { 0, 0 }, { 20000, 0 }, { 40000, 30000 } };
    for (size_t i = 0; i < sizeof none / sizeof none[0]; i++) {
        const rgb16_t black = led_cal_drive(&cal, none[i]);
        TEST_ASSERT_EQUAL_UINT16(0, black.r);
        TEST_ASSERT_EQUAL_UINT16(0, black.g);
        TEST_ASSERT_EQUAL_UINT16(0, black.b);
    }
}

static void test_the_white_lights_every_led_alike(void)
{
    /* D65 is the white of sRGB: equal channels, but for the rounding of the
     * matrix to integers, which leaves red and green 0.04 % below blue. */
    const xy16_t d65 = { LED_WHITE_X, LED_WHITE_Y };
    assert_drive_within(DRIVE_TOLERANCE_INSIDE, (rgb16_t){ 65507, 65507, 65535 },
                        led_cal_drive(&cal, d65));
}

static void test_a_primary_lights_its_led_alone(void)
{
    /* The sRGB primaries, the corners of the gamut of this LED. */
    static const struct {
        xy16_t xy;
        rgb16_t drive;
    } cases[] = {
        { { 41943, 21627 }, { 65535, 0, 0 } }, /* red, 0.64 0.33 */
        { { 19661, 39322 }, { 0, 65535, 0 } }, /* green, 0.30 0.60 */
        { { 9830, 3932 }, { 0, 0, 65535 } },   /* blue, 0.15 0.06 */
    };
    for (size_t i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        assert_drive_within(DRIVE_TOLERANCE_INSIDE, cases[i].drive,
                            led_cal_drive(&cal, cases[i].xy));
    }
}

static void test_a_color_outside_the_gamut_takes_the_least_white(void)
{
    static const struct {
        xy16_t xy;
        size_t zero;
    } cases[] = {
        /* A spectral green, 0.15 0.70: needs negative red. */
        { { 9830, 45875 }, LED_CHANNEL_RED },
        /* A deep red, 0.70 0.29: needs negative green. */
        { { 45875, 19005 }, LED_CHANNEL_GREEN },
        /* A yellow beyond sRGB, 0.50 0.49: needs negative blue. */
        { { 32768, 32113 }, LED_CHANNEL_BLUE },
    };
    for (size_t i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        const reference_t ref = reference_drive(&cal, cases[i].xy);
        TEST_ASSERT_EQUAL(cases[i].zero, ref.zero);
        uint16_t drive[LED_CHANNEL_COUNT];
        channels(led_cal_drive(&cal, cases[i].xy), drive);
        for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
            if (c == cases[i].zero) {
                TEST_ASSERT_EQUAL_UINT16(0, drive[c]);
            } else {
                /* The others keep their light, plus the white: none is 0. */
                TEST_ASSERT_UINT16_WITHIN(DRIVE_TOLERANCE_OUTSIDE, ref.drive[c], drive[c]);
                TEST_ASSERT_TRUE(drive[c] > DRIVE_TOLERANCE_OUTSIDE);
            }
        }
        TEST_ASSERT_EQUAL_UINT16(65535, largest(led_cal_drive(&cal, cases[i].xy)));
    }
}

static void test_drive_matches_an_exact_reference(void)
{
    uint32_t state = 1;
    for (uint32_t i = 0; i < 200000u; i++) {
        /* Any chromaticity: y from 1, x up to 1 - y. */
        const uint32_t y = 1u + next_random(&state) % 65535u;
        const uint32_t x = next_random(&state) % (XY_ONE - y + 1u);
        const xy16_t xy = { (uint16_t)x, (uint16_t)y };
        const reference_t ref = reference_drive(&cal, xy);
        const uint16_t tolerance =
            (ref.zero == LED_CHANNEL_COUNT) ? DRIVE_TOLERANCE_INSIDE : DRIVE_TOLERANCE_OUTSIDE;
        const rgb16_t got = led_cal_drive(&cal, xy);
        uint16_t drive[LED_CHANNEL_COUNT];
        channels(got, drive);
        for (size_t c = 0; c < LED_CHANNEL_COUNT; c++) {
            const int32_t error = (int32_t)drive[c] - (int32_t)ref.drive[c];
            if (error > (int32_t)tolerance || error < -(int32_t)tolerance) {
                FAIL_AT("xy %u %u: channel %u is %u, the reference %u", xy.x, xy.y, (unsigned)c,
                        drive[c], ref.drive[c]);
            }
        }
        if (largest(got) != 65535u) {
            FAIL_AT("xy %u %u: largest channel %u", xy.x, xy.y, largest(got));
        }
    }
}

/* --- Light to duty ---------------------------------------------------- */

#define TABLE_COUNT 5u

static led_light_cal_t tables[TABLE_COUNT];
static const char *const table_names[TABLE_COUNT] = {
    "measured-like", "creeping", "losing", "gaining most", "gaining late",
};

void setUp(void)
{
    /* As measured: no light up to 4 ticks, the next ones rising 2.4 times
     * as fast as the line, long pulses gaining 12.5 ticks of light. */
    tables[0] = make_table(4, 16, -200);
    /* The same with a few LU from the first tick, as the red LED shows. */
    tables[1] = tables[0];
    tables[1].light[1] = 1;
    tables[1].light[2] = 2;
    tables[1].light[3] = 4;
    tables[1].light[4] = 8;
    /* Long pulses that lose 300 sub-ticks of light: no light up to 18 ticks. */
    tables[2] = make_table(18, 19, 300);
    /* The largest gain a table may hold. */
    tables[3] = make_table(4, 40, LED_OFFSET_MIN);
    /* The same gain, all of it in the last tick: below it, the duties
     * exceed their share of the full light by up to 16 N * |offset| / full,
     * the most that led_cal_excess allows for. */
    tables[4] = (led_light_cal_t){ .offset = LED_OFFSET_MIN };
    for (uint32_t n = 1; n < N; n++) {
        tables[4].light[n] = (uint16_t)(256u * n);
    }
    tables[4].light[N] = (uint16_t)((16u * N + 1024u) * 16u);
}

void tearDown(void)
{
}

static uint32_t span_of(const led_light_cal_t *t, uint16_t full)
{
    return (uint32_t)((int32_t)full - t->offset);
}

/* Lights run from 0 to LED_LIGHT_FULL, the light of 100 % duty, which is
 * (full - offset) * 256 LU: led_cal_duty reads light * span / 65535 LU,
 * rounded. */
static uint32_t lu_of_light(const led_light_cal_t *t, uint32_t light, uint16_t full)
{
    return (uint32_t)(((uint64_t)light * span_of(t, full) + 32767u) / 65535u);
}

/* A light that led_cal_duty reads as `lu` LU: there is one for every LU,
 * since the scale, span / 65535, is below 1. */
static uint32_t light_of_lu(const led_light_cal_t *t, uint32_t lu, uint16_t full)
{
    const uint64_t span = span_of(t, full);
    return (uint32_t)(((uint64_t)lu * 65535u + span / 2u) / span);
}

/* The light where the table ends and the line of the long pulses goes on. */
static uint32_t table_end_light(const led_light_cal_t *t, uint16_t full)
{
    return light_of_lu(t, (uint32_t)t->light[N] * 16u, full);
}

static void test_tables_of_the_tests_are_valid(void)
{
    for (size_t i = 0; i < TABLE_COUNT; i++) {
        TEST_ASSERT_TRUE_MESSAGE(led_cal_light_valid(&tables[i]), table_names[i]);
    }
}

static void test_light_mixes_the_ticks_and_follows_the_line_beyond(void)
{
    const led_light_cal_t *t = &tables[0];
    for (uint32_t n = 0; n < N; n++) {
        const uint32_t a = (uint32_t)t->light[n] * 16u;
        const uint32_t b = (uint32_t)t->light[n + 1u] * 16u;
        TEST_ASSERT_EQUAL_UINT32(a, led_cal_light(t, (uint16_t)(n * 16u)));
        /* Dithered: 5 periods in 16 one tick longer. */
        TEST_ASSERT_EQUAL_UINT32(a + (b - a) * 5u / 16u,
                                 led_cal_light(t, (uint16_t)(n * 16u + 5u)));
    }
    /* Beyond the table, 256 LU per sub-tick, the long pulses gaining 200. */
    TEST_ASSERT_EQUAL_UINT32((uint32_t)t->light[N] * 16u, led_cal_light(t, 16u * N));
    TEST_ASSERT_EQUAL_UINT32((16u * N + 1u + 200u) * 256u, led_cal_light(t, 16u * N + 1u));
    TEST_ASSERT_EQUAL_UINT32((FULL + 200u) * 256u, led_cal_light(t, FULL));
}

static void test_dark_and_full_light(void)
{
    for (size_t i = 0; i < TABLE_COUNT; i++) {
        TEST_ASSERT_EQUAL_UINT16_MESSAGE(0, led_cal_duty(&tables[i], 0, FULL), table_names[i]);
        TEST_ASSERT_EQUAL_UINT16_MESSAGE(FULL, led_cal_duty(&tables[i], LED_LIGHT_FULL, FULL),
                                         table_names[i]);
        TEST_ASSERT_EQUAL_UINT16_MESSAGE(
            FULL, led_cal_duty(&tables[i], LED_LIGHT_FULL + 1000u, FULL), table_names[i]);
    }
}

static void test_light_nearer_to_dark_than_to_the_first_light_gives_no_pulse(void)
{
    for (size_t i = 0; i < TABLE_COUNT; i++) {
        const led_light_cal_t *t = &tables[i];
        uint32_t first = 1; /* the shortest duty that gives light */
        while (led_cal_light(t, (uint16_t)first) == 0u) {
            first++;
        }
        const uint32_t first_lu = led_cal_light(t, (uint16_t)first);
        /* No pulse of the dead zone either, which would give no light. */
        const uint32_t dark = light_of_lu(t, (first_lu - 1u) / 2u, FULL);
        TEST_ASSERT_EQUAL_UINT16_MESSAGE(0, led_cal_duty(t, dark, FULL), table_names[i]);
        const uint32_t lit = light_of_lu(t, (first_lu + 1u) / 2u, FULL);
        TEST_ASSERT_EQUAL_UINT16_MESSAGE(first, led_cal_duty(t, lit, FULL), table_names[i]);
    }
    /* In the measured-like table, the first light is one sub-tick past 4
     * ticks, 608 LU: it shows from 304 LU on. */
    TEST_ASSERT_EQUAL_UINT16(4u * 16u + 1u,
                             led_cal_duty(&tables[0], light_of_lu(&tables[0], 304, FULL), FULL));
}

/* Every duty whose light rises above the one before comes back from its
 * light, in the table and beyond. */
static void assert_round_trip(const led_light_cal_t *t, const char *name, uint16_t full)
{
    uint32_t before = 0;
    for (uint32_t d = 0; d <= full; d++) {
        const uint32_t lu = led_cal_light(t, (uint16_t)d);
        if (d == 0u || lu > before) {
            const uint32_t light = light_of_lu(t, lu, full);
            const uint16_t back = led_cal_duty(t, light, full);
            if (lu_of_light(t, light, full) != lu || back != d) {
                FAIL_AT("%s: duty %u, light %u LU, back %u", name, (unsigned)d, (unsigned)lu, back);
            }
        }
        before = lu;
    }
}

static void test_duty_comes_back_from_its_light(void)
{
    for (size_t i = 0; i < TABLE_COUNT; i++) {
        assert_round_trip(&tables[i], table_names[i], FULL);
    }
}

/* Every light up to the end of the table, every 61st beyond, where the duty
 * follows a line. */
static uint32_t light_step(uint32_t light, uint32_t table_end)
{
    return (light < table_end) ? 1u : 61u;
}

static void test_duty_never_decreases_with_the_light(void)
{
    for (size_t i = 0; i < TABLE_COUNT; i++) {
        const led_light_cal_t *t = &tables[i];
        const uint32_t end = table_end_light(t, FULL);
        uint16_t before = 0;
        for (uint32_t light = 0; light <= LED_LIGHT_FULL; light += light_step(light, end)) {
            const uint16_t d = led_cal_duty(t, light, FULL);
            if (d < before) {
                FAIL_AT("%s: light %u gives %u, below %u", table_names[i], (unsigned)light, d,
                        before);
            }
            before = d;
        }
    }
}

static int64_t distance(uint32_t a, uint32_t b)
{
    return (a > b) ? (int64_t)a - (int64_t)b : (int64_t)b - (int64_t)a;
}

/* The light of the duty is the nearest that any duty gives: with the light
 * rising with the duty, the next lower and next higher lights are enough. */
static void assert_nearest(const led_light_cal_t *t, const char *name, uint32_t light,
                           uint16_t full)
{
    const uint32_t lu = lu_of_light(t, light, full);
    const uint16_t d = led_cal_duty(t, light, full);
    const uint32_t shown = led_cal_light(t, d);
    const int64_t miss = distance(shown, lu);
    uint32_t up = d + 1u;
    while (up <= full && led_cal_light(t, (uint16_t)up) == shown) {
        up++;
    }
    uint32_t down = d;
    while (down > 0u && led_cal_light(t, (uint16_t)(down - 1u)) == shown) {
        down--;
    }
    if ((up <= full && distance(led_cal_light(t, (uint16_t)up), lu) < miss) ||
        (down > 0u && distance(led_cal_light(t, (uint16_t)(down - 1u)), lu) < miss)) {
        FAIL_AT("%s: %u LU gives duty %u, %u LU", name, (unsigned)lu, d, (unsigned)shown);
    }
}

static void test_duty_gives_the_nearest_light(void)
{
    for (size_t i = 0; i < TABLE_COUNT; i++) {
        const led_light_cal_t *t = &tables[i];
        const uint32_t end = table_end_light(t, FULL);
        for (uint32_t light = 0; light <= LED_LIGHT_FULL; light += light_step(light, end)) {
            assert_nearest(t, table_names[i], light, FULL);
        }
    }
}

/* The duty exceeds light * full / LED_LIGHT_FULL by the excess at most, in
 * exact integers. The duty never decreases with the light, so each duty
 * exceeds the most at the smallest light that gives it: a binary search
 * finds it. */
static void assert_excess_bounds_the_duty(const led_light_cal_t *t, const char *name, uint16_t full)
{
    const uint64_t excess = led_cal_excess(t, full);
    for (uint32_t d = 1; d <= full; d++) {
        uint32_t low = 0;
        uint32_t high = LED_LIGHT_FULL; /* gives full */
        while (low < high) {
            const uint32_t middle = low + (high - low) / 2u;
            if (led_cal_duty(t, middle, full) >= d) {
                high = middle;
            } else {
                low = middle + 1u;
            }
        }
        const uint64_t duty = led_cal_duty(t, low, full);
        if (duty * LED_LIGHT_FULL > (uint64_t)low * full + excess * LED_LIGHT_FULL) {
            FAIL_AT("%s, full %u: light %u gives duty %u, over the excess %u", name, (unsigned)full,
                    (unsigned)low, (unsigned)duty, (unsigned)excess);
        }
    }
}

static void test_excess_bounds_the_duty_above_the_light(void)
{
    for (size_t i = 0; i < TABLE_COUNT; i++) {
        assert_excess_bounds_the_duty(&tables[i], table_names[i], FULL);
        /* The longest period allowed, 16 times it less LED_OFFSET_MIN on 16 bits. */
        assert_excess_bounds_the_duty(&tables[i], table_names[i], 4031u * 16u);
    }
}

static void test_light_valid_rejects_a_broken_table(void)
{
    /* Still never decreasing, the creeping table starting at 1. */
    led_light_cal_t t = tables[1];
    t.light[0] = 1;
    TEST_ASSERT_FALSE(led_cal_light_valid(&t));

    t = tables[0];
    t.light[40] = (uint16_t)(t.light[39] - 1u);
    TEST_ASSERT_FALSE(led_cal_light_valid(&t));

    /* The last entry meets the line beyond, to the unit. */
    t = tables[0];
    t.light[N]++;
    TEST_ASSERT_FALSE(led_cal_light_valid(&t));
    t.light[N] = (uint16_t)(t.light[N] - 2u);
    TEST_ASSERT_FALSE(led_cal_light_valid(&t));
}

static void test_light_valid_bounds_the_offset(void)
{
    led_light_cal_t t = make_table(4, 40, LED_OFFSET_MIN);
    TEST_ASSERT_TRUE(led_cal_light_valid(&t));
    t = make_table(4, 40, (int16_t)(LED_OFFSET_MIN - 1));
    TEST_ASSERT_FALSE(led_cal_light_valid(&t));

    /* Up to a single sub-tick of light at the end of the table. */
    t = make_table(127, 128, (int16_t)(16u * N - 1u));
    TEST_ASSERT_EQUAL_UINT16(16, t.light[N]);
    TEST_ASSERT_TRUE(led_cal_light_valid(&t));
    t = make_table(127, 128, (int16_t)(16u * N));
    TEST_ASSERT_FALSE(led_cal_light_valid(&t));
}

static void test_a_flat_stretch_gives_its_shortest_pulse(void)
{
    /* Ticks 29 to 31 give the same light: their light comes back as 29
     * ticks, the least current for it. */
    led_light_cal_t t = tables[0];
    t.light[30] = t.light[29];
    t.light[31] = t.light[29];
    TEST_ASSERT_TRUE(led_cal_light_valid(&t));
    TEST_ASSERT_EQUAL_UINT16(29u * 16u,
                             led_cal_duty(&t, light_of_lu(&t, t.light[29] * 16u, FULL), FULL));
}

static void test_color_valid_guards_the_products(void)
{
    TEST_ASSERT_TRUE(led_cal_color_valid(&cal));

    led_color_cal_t c = cal;
    c.matrix[0][1] = (int16_t)(c.matrix[0][1] - 1); /* the red row at 32768 */
    TEST_ASSERT_FALSE(led_cal_color_valid(&c));

    c = cal;
    c.matrix[2][2] = 800; /* the blue row below 0 for D65: the white outside the gamut */
    TEST_ASSERT_FALSE(led_cal_color_valid(&c));
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_a_chromaticity_has_y_above_0_and_x_plus_y_at_most_1);
    RUN_TEST(test_anything_but_a_chromaticity_gives_black);
    RUN_TEST(test_the_white_lights_every_led_alike);
    RUN_TEST(test_a_primary_lights_its_led_alone);
    RUN_TEST(test_a_color_outside_the_gamut_takes_the_least_white);
    RUN_TEST(test_drive_matches_an_exact_reference);
    RUN_TEST(test_tables_of_the_tests_are_valid);
    RUN_TEST(test_light_mixes_the_ticks_and_follows_the_line_beyond);
    RUN_TEST(test_dark_and_full_light);
    RUN_TEST(test_light_nearer_to_dark_than_to_the_first_light_gives_no_pulse);
    RUN_TEST(test_duty_comes_back_from_its_light);
    RUN_TEST(test_duty_never_decreases_with_the_light);
    RUN_TEST(test_duty_gives_the_nearest_light);
    RUN_TEST(test_excess_bounds_the_duty_above_the_light);
    RUN_TEST(test_light_valid_rejects_a_broken_table);
    RUN_TEST(test_light_valid_bounds_the_offset);
    RUN_TEST(test_a_flat_stretch_gives_its_shortest_pulse);
    RUN_TEST(test_color_valid_guards_the_products);
    return UNITY_END();
}
