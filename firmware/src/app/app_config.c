/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "app_config.h"

#include "bsp/ch32.h"

#define LUX(x)  ((uint32_t)(x) * 1000u)
#define MLUX(x) ((uint32_t)(x))

/* TouchKey timing. Charge 3 and an ADC sample of 1.5 cycles keep the idle
 * reading near mid-scale, about 2100 of 4095. The WCH example values
 * (charge 16, 7.5 cycles) saturate it.
 *
 * Thresholds, in per mille below the baseline, through the closed lid, from
 * the measurements of 2026-10-31 on a PC on an earthed monitor hub: a firm
 * finger on the ring reads 182 to 198, a light tap in the center 69 to 105, a
 * hand held 1 cm above 80 to 89, the idle noise about 6 (peaks under 15). A
 * finger lands faster than a hand nears, so the rise above the floor (50)
 * tells them apart. A 60 ms debounce misses light taps over the ring's hole.
 * The touch sensitivity setting of the desktop app scales press, release and
 * rise from 50 to 150 %, and never takes the release under 1.5 times the
 * proximity band. */
const tkey_config_t app_tkey_config = {
    .charge_time = 3,
    .discharge_time = 8,
    .adc_sample_time = ADC_SampleTime_1Cycles5,
};

const touch_config_t app_touch_config = {
    .window_samples = 20,     /* 20 ms at 1 kHz: one 50 Hz mains period */
    .calibration_windows = 8, /* 160 ms */
    .baseline_shift = 8,      /* about 5 s time constant */
    .press_permille = 60,
    .release_permille = 34,
    .press_rise_permille = 50,
    .proximity_permille = 10, /* rest noise and the LED on/off shift stay below */
    .debounce_ms = 40,        /* the noise stays far below */
    .rearm_ms = 100,          /* fast taps come 260 ms apart */
    .long_press_ms = 2000,
    .object_ms = 10000,
    .inverted_permille = 30,
    .inverted_ms = 1000,
};

/* Ambient light to brightness, tuned by eye. The lux are those the sensor
 * reads behind its window in the side wall, a small fraction of the light of
 * the room: a dim room reads under 0.01 lx there. The floor of 1.5 % is the
 * level chosen for the night. */
/* clang-format off */
static const brightness_point_t app_brightness_curve[] = {
    { MLUX(100),   15 },
    { MLUX(500),   95 },
    {    LUX(2),  285 },
    {    LUX(8),  550 },
    {   LUX(25),  750 },
    {  LUX(100), 1000 },
};
/* clang-format on */

/* The lamp follows a light switched on or off in about 1.5 s: 0.4 s to
 * confirm the change, then a time constant of 0.25 s. */
const brightness_config_t app_brightness_config = {
    .curve = app_brightness_curve,
    .curve_points = sizeof app_brightness_curve / sizeof app_brightness_curve[0],
    .tick_ms = 100,    /* four filter steps per reading of the sensor */
    .slow_divisor = 8, /* small changes: time constant about 0.75 s */
    .fast_divisor = 3, /* large ones: about 0.25 s */
    .large_q8 = 256,   /* large: a factor of 2 */
    .confirm_ms = 400, /* a hand or a shadow passes faster */
    .settled_q8 = 32,  /* the fast filter ends within 9 % of the light */
    .hold_q8 = 8,      /* about 2 %: the sensor noise never moves the lamp */
    .fallback_permille = 500,
};

/* The LED, measured on 2026-11-20 on a closed lamp (pcb 1.0.0, 100 Ω gate
 * resistors, white PLA) with the procedure of docs/bringup.md.
 *
 * Built-in colors, as LED drives: green, orange and red are the LED's own
 * primaries, more saturated than any sRGB color and kept so; their app
 * swatches are the nearest sRGB colors (#00FFA2, #FF6F00, #FF001C). The warm
 * white is 2700 K (#FFAD59) through the color calibration below. */
/* clang-format off */
const rgb16_t app_builtin_colors[LAMP_COLOR_COUNT] = {
    [LAMP_COLOR_GREEN] = {     0, 65535,     0 }, /* the green LED alone */
    [LAMP_COLOR_ORANGE] = { 65535, 29000,     0 }, /* red and green LEDs */
    [LAMP_COLOR_RED] = { 65535,     0,     0 }, /* the red LED alone */
    [LAMP_COLOR_WARM_WHITE] = { 65535, 62887,  3435 }, /* 2700 K, #FFAD59 */
};

/* Linear sRGB to LED drive, with the D65 white of #FFFFFF. The primaries
 * were matched by eye, by A/B choices, against the screen of a Galaxy S23 in
 * its Natural (sRGB) mode, the only calibrated reference at hand: no camera
 * sees narrow LED primaries as the eye does. Their dominant wavelengths are
 * 619, 521 and 458 nm as the matches see them, and the white needs drives
 * 0.564, 1 and 0.214. */
const led_color_cal_t app_led_color_cal = {
    .matrix = {
        {  11785,   6713,     -3 },
        {   1466,  29585,   1714 },
        {     95,    -66,   6982 },
    },
    .white = { 36993, 65535, 14023 },
};

/* Light of a pulse of each length in whole ticks, measured with a camera
 * through the cube: a pulse of 4 ticks or less gives almost no light, the
 * slow gate edges (note N4) give the next ticks up to 1.4 times the light of
 * those of a long pulse, and a long pulse gives about 13 ticks more light than
 * its duty, its edges outlasting it. */
const led_light_cal_t app_led_light_cal[LED_CHANNEL_COUNT] = {
    [LED_CHANNEL_RED] = {
        .offset = -203,
        .light = {
                0,     1,     5,     6,    51,   430,   892,  1443,  2041,
             2577,  3060,  3528,  4010,  4467,  4896,  5303,  5687,  6036,
             6382,  6725,  7060,  7388,  7748,  8099,  8445,  8776,  9084,
             9384,  9672,  9919, 10167, 10417, 10673, 10951, 11230, 11515,
            11797, 12077, 12354, 12625, 12891, 13152, 13416, 13683, 13954,
            14232, 14512, 14795, 15081, 15364, 15639, 15906, 16164, 16417,
            16667, 16914, 17157, 17401, 17649, 17903, 18160, 18421, 18679,
            18937, 19193, 19444, 19690, 19931, 20168, 20405, 20648, 20896,
            21152, 21413, 21681, 21955, 22236, 22517, 22796, 23072, 23347,
            23619, 23888, 24156, 24421, 24686, 24951, 25216, 25481, 25747,
            26012, 26277, 26542, 26807, 27070, 27332, 27591, 27848, 28104,
            28357, 28608, 28860, 29111, 29362, 29614, 29876, 30139, 30402,
            30665, 30929, 31195, 31464, 31734, 32006, 32279, 32553, 32828,
            33101, 33372, 33643, 33912, 34179, 34446, 34711, 34974, 35237,
            35498, 35757, 36016,
        },
    },
    [LED_CHANNEL_GREEN] = {
        .offset = -200,
        .light = {
                0,     2,     5,    14,   154,  1019,  1781,  2451,  3051,
             3534,  3956,  4357,  4782,  5183,  5564,  5938,  6289,  6640,
             6988,  7332,  7670,  8001,  8316,  8628,  8917,  9192,  9460,
             9748, 10035, 10329, 10622, 10916, 11207, 11495, 11747, 11992,
            12247, 12509, 12765, 13019, 13271, 13521, 13778, 14026, 14265,
            14511, 14771, 15046, 15335, 15633, 15930, 16227, 16525, 16814,
            17088, 17346, 17589, 17825, 18056, 18281, 18500, 18722, 18947,
            19177, 19412, 19648, 19887, 20128, 20372, 20615, 20867, 21128,
            21397, 21674, 21961, 22256, 22561, 22865, 23162, 23450, 23730,
            24002, 24266, 24522, 24768, 25015, 25262, 25509, 25756, 26003,
            26250, 26496, 26743, 26990, 27242, 27498, 27760, 28026, 28297,
            28574, 28855, 29136, 29417, 29699, 29980, 30254, 30526, 30795,
            31062, 31328, 31588, 31843, 32094, 32341, 32585, 32826, 33065,
            33304, 33544, 33785, 34025, 34267, 34508, 34750, 34993, 35236,
            35480, 35724, 35968,
        },
    },
    [LED_CHANNEL_BLUE] = {
        .offset = -230,
        .light = {
                0,     0,     0,     0,    86,   598,  1134,  1763,  2367,
             2878,  3344,  3760,  4215,  4643,  5049,  5426,  5806,  6201,
             6576,  6935,  7280,  7609,  7919,  8206,  8460,  8714,  8975,
             9252,  9580,  9921, 10268, 10605, 10929, 11239, 11528, 11758,
            11982, 12202, 12446, 12719, 13022, 13354, 13692, 14035, 14384,
            14705, 14995, 15254, 15481, 15705, 15924, 16138, 16347, 16554,
            16773, 17003, 17244, 17498, 17763, 18039, 18326, 18611, 18899,
            19190, 19485, 19786, 20092, 20405, 20723, 21042, 21345, 21632,
            21903, 22157, 22395, 22617, 22821, 23026, 23238, 23459, 23687,
            23923, 24167, 24419, 24680, 24941, 25201, 25462, 25723, 25983,
            26244, 26504, 26765, 27026, 27288, 27550, 27815, 28080, 28346,
            28614, 28883, 29152, 29420, 29689, 29958, 30241, 30523, 30803,
            31083, 31362, 31637, 31910, 32180, 32448, 32715, 32979, 33243,
            33507, 33772, 34038, 34304, 34570, 34837, 35105, 35373, 35642,
            35912, 36182, 36448,
        },
    },
};
/* clang-format on */

/* Worst-case LED currents at 100 % duty, at 5.25 V and the lowest forward
 * voltage (constraint N2 in hardware/README.md). 70 mA plus about 25 mA for
 * the MCU stays within the 100 mA of an unconfigured USB device. 125 mA lets
 * any single color reach 100 % and holds warm white, about 150 mA, near 80 %
 * of its power; the heat test of docs/bringup.md tells whether it may rise. */
const power_config_t app_power_config = {
    .full_scale_ma = { 86, 63, 125 },
    .unconfigured_cap_ma = 70,
    .configured_cap_ma = 125,
};

/* Self-test: colors at 50 % and the blue glow at 20 % of perceived
 * brightness. The white step is the warm white, since equal channels look
 * violet. The idle TouchKey reading is about 2100 counts, and 4095 when
 * saturated. */
const self_test_config_t app_self_test_config = {
    .color_ms = 1500,
    .touch_wait_ms = 10000,
    .color_permille = 500,
    .white = &app_builtin_colors[LAMP_COLOR_WARM_WHITE],
    .wait_permille = 200,
    .touch_idle_min = 1000,
    .touch_idle_max = 3800,
};
