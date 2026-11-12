/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "touch_gate.h"

void touch_gate_init(touch_gate_t *gate)
{
    gate->blocked = false;
}

touch_event_t touch_gate_feed(touch_gate_t *gate, touch_detector_t *detector, uint16_t raw,
                              uint32_t sample_ms, bool blocked)
{
    if (gate->blocked && !blocked) {
        /* Before the sample: touch_detector_consume() acts only on a press
         * under way, and this sample may be the one that confirms its release. */
        touch_detector_consume(detector);
    }
    gate->blocked = blocked;
    const touch_event_t event = touch_detector_feed(detector, raw, sample_ms);
    return blocked ? TOUCH_EVENT_NONE : event;
}
