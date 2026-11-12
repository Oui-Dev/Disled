/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "self_test.h"

#include "disled_assert.h"
#include "time_utils.h"

void self_test_init(self_test_t *t, const self_test_config_t *config)
{
    DISLED_ASSERT(config->color_ms > 0u);
    *t = (self_test_t){ .config = config, .phase = SELF_TEST_IDLE };
}

void self_test_start(self_test_t *t, uint32_t now_ms)
{
    *t = (self_test_t){ .config = t->config, .phase = SELF_TEST_COLORS, .phase_start_ms = now_ms };
}

bool self_test_running(const self_test_t *t)
{
    return t->phase != SELF_TEST_IDLE;
}

/* End of the color steps: the checks that need no person. */
static void check_idle(self_test_t *t, const self_test_inputs_t *in)
{
    const self_test_config_t *cfg = t->config;
    t->lux_centi = in->lux_centi;
    if (in->light_present) {
        t->results |= SELF_TEST_LIGHT_OK;
    }
    t->touch_baseline = in->touch_baseline;
    if (in->touch_baseline >= cfg->touch_idle_min && in->touch_baseline <= cfg->touch_idle_max) {
        t->results |= SELF_TEST_TOUCH_IDLE_OK;
    }
}

bool self_test_update(self_test_t *t, uint32_t now_ms, const self_test_inputs_t *in)
{
    const self_test_config_t *cfg = t->config;
    const uint32_t elapsed = time_elapsed_ms(now_ms, t->phase_start_ms);

    switch (t->phase) {
        case SELF_TEST_COLORS:
            if (elapsed >= (uint32_t)cfg->color_ms * SELF_TEST_COLOR_STEPS) {
                check_idle(t, in);
                t->phase = SELF_TEST_TOUCH;
                t->phase_start_ms = now_ms;
            }
            return false;

        case SELF_TEST_TOUCH:
            if (in->touch_delta_permille > t->touch_peak_permille) {
                t->touch_peak_permille = in->touch_delta_permille;
            }
            /* A finger already down when the blue glow starts must lift first:
             * only a press made during the wait answers it. */
            if (!in->touch_pressed) {
                t->touch_released = true;
            } else if (t->touch_released) {
                t->results |= SELF_TEST_TOUCH_PRESSED;
            }
            if ((t->results & SELF_TEST_TOUCH_PRESSED) != 0u || elapsed >= cfg->touch_wait_ms) {
                t->results |= SELF_TEST_COMPLETED;
                t->phase = SELF_TEST_IDLE;
                return true;
            }
            return false;

        case SELF_TEST_IDLE:
        default:
            return false;
    }
}

void self_test_abort(self_test_t *t)
{
    t->phase = SELF_TEST_IDLE;
}

rgb16_t self_test_color(const self_test_t *t, uint32_t now_ms)
{
    const self_test_config_t *cfg = t->config;
    const uint16_t on = cfg->color_linear;
    rgb16_t color = { 0, 0, 0 };

    if (t->phase == SELF_TEST_TOUCH) {
        color.b = cfg->wait_linear;
    } else if (t->phase == SELF_TEST_COLORS) {
        const uint32_t step = time_elapsed_ms(now_ms, t->phase_start_ms) / cfg->color_ms;
        switch (step) {
            case 0:
                color.r = on;
                break;
            case 1:
                color.g = on;
                break;
            case 2:
                color.b = on;
                break;
            default:
                color = cfg->white;
                break;
        }
    }
    return color;
}
