/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Production self-test, the SELF_TEST command: red, green, blue and warm white
 * for a person to check, then the light sensor, the idle touch reading and a
 * touch on the lid. The application feeds the readings and shows the color.
 */
#ifndef DISLED_SELF_TEST_H
#define DISLED_SELF_TEST_H

#include <stdbool.h>
#include <stdint.h>

#include "disled_types.h"

/* Results, same bits as the SELF_TEST_RESULT message. */
#define SELF_TEST_LIGHT_OK      (1u << 0)
#define SELF_TEST_TOUCH_IDLE_OK (1u << 1)
#define SELF_TEST_TOUCH_PRESSED (1u << 2)
#define SELF_TEST_COMPLETED     (1u << 3)

#define SELF_TEST_COLOR_STEPS 4u /* red, green, blue, white */

typedef struct {
    uint16_t color_ms;       /* each color step */
    uint16_t touch_wait_ms;  /* time allowed for the touch */
    uint16_t color_linear;   /* light of the red, green and blue steps, linear */
    rgb16_t white;           /* last step: the lamp's warm white (linear) */
    uint16_t wait_linear;    /* light of the blue glow while waiting */
    uint16_t touch_idle_min; /* plausible idle TouchKey baseline, counts */
    uint16_t touch_idle_max;
} self_test_config_t;

typedef struct {
    bool light_present; /* the sensor answers */
    uint32_t lux_centi;
    uint16_t touch_baseline; /* 0 while the detector calibrates: not plausible */
    int16_t touch_delta_permille;
    /* The detector sees a press. A drop alone is no touch: a hand above
     * the lid reaches the press threshold too. */
    bool touch_pressed;
} self_test_inputs_t;

typedef enum {
    SELF_TEST_IDLE = 0,
    SELF_TEST_COLORS,
    SELF_TEST_TOUCH,
} self_test_phase_t;

typedef struct {
    const self_test_config_t *config;
    self_test_phase_t phase;
    uint32_t phase_start_ms;
    uint8_t results; /* SELF_TEST_* */
    uint32_t lux_centi;
    uint16_t touch_baseline;
    int16_t touch_peak_permille;
    bool touch_released; /* seen with no press since the blue glow started */
} self_test_t;

void self_test_init(self_test_t *t, const self_test_config_t *config);

void self_test_start(self_test_t *t, uint32_t now_ms);

bool self_test_running(const self_test_t *t);

/* Returns true once, when the test ends; the results are then in `t`. */
bool self_test_update(self_test_t *t, uint32_t now_ms, const self_test_inputs_t *in);

/* Ends the test early, on USB suspend: no SELF_TEST_COMPLETED in the results. */
void self_test_abort(self_test_t *t);

/* Linear color to show while the test runs. */
rgb16_t self_test_color(const self_test_t *t, uint32_t now_ms);

#endif /* DISLED_SELF_TEST_H */
