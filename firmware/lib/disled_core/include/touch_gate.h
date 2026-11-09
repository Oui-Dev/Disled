/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Touch gate: no gesture acts while it is blocked, as during an alert. It
 * feeds the touch detector itself, so that a press under way when the block
 * lifts fires nothing either, not even at its release.
 */
#ifndef DISLED_TOUCH_GATE_H
#define DISLED_TOUCH_GATE_H

#include <stdbool.h>
#include <stdint.h>

#include "touch_detector.h"

typedef struct {
    bool blocked; /* at the previous sample */
} touch_gate_t;

void touch_gate_init(touch_gate_t *gate);

/* Feeds one raw sample taken at `sample_ms` to the detector. Returns the
 * gesture to apply: none while `blocked`, nor from a press under way when the
 * block lifts. */
touch_event_t touch_gate_feed(touch_gate_t *gate, touch_detector_t *detector, uint16_t raw,
                              uint32_t sample_ms, bool blocked);

#endif /* DISLED_TOUCH_GATE_H */
