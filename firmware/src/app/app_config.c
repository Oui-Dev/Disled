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

/* Colors at full brightness, in linear light. The orange and the warm white
 * are tuned by eye through the PLA; with less green, the orange is hard to
 * tell from the red. */
/* clang-format off: keep the table readable */
const color_config_t app_color_config = {
    .palette = {
        [LAMP_COLOR_GREEN]      = {     0, 65535,    0 },
        [LAMP_COLOR_ORANGE]     = { 65535, 29000,    0 },
        [LAMP_COLOR_RED]        = { 65535,     0,    0 },
        [LAMP_COLOR_WARM_WHITE] = { 63000, 58000, 1600 },
    },
};
/* clang-format on */

/* Light lost per pulse by the driver edges, in ns; a tick lasts 14 ns at
 * 72 MHz. Measured with a camera on two whites, yellow, cyan and magenta,
 * from 1 to 30 %: with 28 ns of red, every color with red lost it at the
 * bottom, a warm white at 3 % about a fifth, and turned yellow-green. The
 * green and the blue keep their hue together: the 1-tick blue of a warm
 * white at 10 % gives light only with its 58 ns. */
const uint16_t app_led_pulse_loss_ns[LED_CHANNEL_COUNT] = { 63, 42, 58 };

/* Worst-case LED currents at 100 % duty, at 5.25 V and the lowest forward
 * voltage (constraint N2 in hardware/README.md). 70 mA plus about 25 mA for
 * the MCU stays within the 100 mA of an unconfigured USB device. 125 mA lets
 * any single color reach 100 % and holds warm white, about 140 mA, near 90 %
 * of its power; the heat test of docs/bringup.md tells whether it may rise. */
const power_config_t app_power_config = {
    .full_scale_ma = { 86, 63, 125 },
    .unconfigured_cap_ma = 70,
    .configured_cap_ma = 125,
};

/* Self-test: colors at 50 % and the blue glow at 20 % of perceived
 * brightness, in linear light (the CIE table of color.c). The white step is
 * the warm white of the palette at 50 % (63000/58000/1600 times
 * 12071/65535), since equal channels look pink. The idle TouchKey reading is
 * about 2100 counts, and 4095 when saturated. */
const self_test_config_t app_self_test_config = {
    .color_ms = 1500,
    .touch_wait_ms = 10000,
    .color_linear = 12071,
    .white = { 11604, 10683, 295 },
    .wait_linear = 1959,
    .touch_idle_min = 1000,
    .touch_idle_max = 3800,
};
