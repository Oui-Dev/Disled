/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Touch gestures from raw TouchKey readings: short and long presses, and an
 * object left on the lid, which the baseline absorbs. Drops below the
 * baseline are in per mille of it, so that the thresholds suit every board.
 */
#ifndef DISLED_TOUCH_DETECTOR_H
#define DISLED_TOUCH_DETECTOR_H

#include <stdbool.h>
#include <stdint.h>

/* A press must also rise above the floor, the lowest drop of the last 16 windows,
 * or the taps on a hand resting near the lid would merge into one long press. A
 * tap rises in 40 to 100 ms, a hand settling near the lid more slowly. */
#define TOUCH_FLOOR_WINDOWS 16u

typedef enum {
    TOUCH_EVENT_NONE = 0,
    TOUCH_EVENT_SHORT_PRESS,
    TOUCH_EVENT_LONG_PRESS,
} touch_event_t;

typedef enum {
    TOUCH_STATE_CALIBRATING = 0, /* building the first baseline */
    TOUCH_STATE_IDLE,
    TOUCH_STATE_PRESSED,
} touch_state_t;

/* Where the idle signal lies, for the baseline. */
typedef enum {
    TOUCH_ZONE_NONE = 0, /* the baseline follows the signal */
    TOUCH_ZONE_NEAR,     /* proximity band: baseline frozen up to object_ms */
    TOUCH_ZONE_INVERTED, /* above the baseline: frozen up to inverted_ms */
} touch_zone_t;

typedef struct {
    uint16_t window_samples;     /* raw samples averaged per window */
    uint8_t calibration_windows; /* windows averaged into the first baseline */
    uint8_t baseline_shift;      /* baseline time constant: 2^shift windows */
    uint16_t press_permille;     /* drop that starts a touch */
    uint16_t release_permille;   /* drop under which the touch ends */
    /* Rise above the floor that starts a touch; it ends at half of it. */
    uint16_t press_rise_permille;
    uint16_t proximity_permille; /* drop that freezes the baseline (hand near) */
    uint16_t debounce_ms;        /* a change must hold this long */
    uint16_t rearm_ms;           /* after a release, no new press before this */
    uint16_t long_press_ms;      /* contact time of a long press */
    uint16_t object_ms;          /* contact or proximity time that means "object" */
    uint16_t inverted_permille;  /* rise above baseline that means "recalibrate" */
    uint16_t inverted_ms;        /* how long the rise must hold */
} touch_config_t;

/* The thresholds that the sensitivity correction scales, per mille. */
typedef struct {
    uint16_t press;
    uint16_t release;
    uint16_t rise;
} touch_thresholds_t;

/* Limits of the sensitivity correction in percent, those of SET_TOUCH_SENSITIVITY. */
#define TOUCH_SENSITIVITY_MIN (-50)
#define TOUCH_SENSITIVITY_MAX 50

typedef struct {
    const touch_config_t *config;
    touch_state_t state;
    /* Window accumulator. */
    uint32_t window_sum;
    uint16_t window_count;
    uint16_t last_raw;
    uint16_t value; /* last window average */
    /* Baseline in 1/256 counts, for a smooth fixed-point filter. */
    uint32_t baseline_q8;
    uint32_t calibration_sum;
    uint8_t calibration_count;
    int16_t delta_permille; /* positive while touched */
    /* Debounce: the raw contact differs from the state since pending_since_ms. */
    bool pending;
    uint32_t pending_since_ms;
    uint32_t press_start_ms;
    /* Recent drops, for the floor; the floor is latched while touched. */
    int16_t recent[TOUCH_FLOOR_WINDOWS];
    uint8_t recent_next;
    int16_t press_floor;
    bool long_fired; /* also set by touch_detector_consume() */
    bool rearming;   /* a release happened less than rearm_ms ago */
    uint32_t release_ms;
    touch_zone_t zone;
    uint32_t zone_since_ms;
    uint16_t recalibrations; /* automatic re-baselines since init */
    /* Correction in percent: the one set, and the one in force. They differ
     * only until the press under way ends. */
    int8_t sensitivity;
    int8_t sensitivity_applied;
} touch_detector_t;

void touch_detector_init(touch_detector_t *td, const touch_config_t *config);

/* Feeds one raw sample taken at `now_ms`. Returns the gesture it completes: a
 * short press at its release, a long press while the finger is still down. */
touch_event_t touch_detector_feed(touch_detector_t *td, uint16_t raw, uint32_t now_ms);

/* Builds a new baseline from the next readings, firing no gesture until then.
 * The baseline reads 0 meanwhile, as before the first calibration. */
void touch_detector_recalibrate(touch_detector_t *td);

/* The current press (TOUCH_STATE_PRESSED) fires no gesture. */
void touch_detector_consume(touch_detector_t *td);

/* Sensitivity correction in percent, positive for a more sensitive lid: the
 * press, release and rise thresholds are multiplied by (100 - percent) / 100,
 * so +50 asks for half the drop and -50 for one and a half times it. 0 keeps
 * the configured thresholds, and the hysteresis keeps its ratio. The
 * proximity and inverted bands and the timings never change. The correction
 * is clamped to the TOUCH_SENSITIVITY_ limits, then brought toward 0 until
 * the release stays half a band above the proximity band, itself just above
 * the idle noise. It applies from the next window, or once the press under
 * way ends, and keeps the baseline. */
void touch_detector_set_sensitivity(touch_detector_t *td, int8_t percent);

/* The correction set, after the clamp and the release floor. */
int8_t touch_detector_sensitivity(const touch_detector_t *td);

/* The press, release and rise thresholds in force. */
touch_thresholds_t touch_detector_thresholds(const touch_detector_t *td);

touch_state_t touch_detector_state(const touch_detector_t *td);
uint16_t touch_detector_raw(const touch_detector_t *td);
uint16_t touch_detector_value(const touch_detector_t *td);
uint16_t touch_detector_baseline(const touch_detector_t *td);
int16_t touch_detector_delta_permille(const touch_detector_t *td);

/* Automatic re-baselines since init: finger at power-up, object on or near the lid. */
uint16_t touch_detector_recalibrations(const touch_detector_t *td);

#endif /* DISLED_TOUCH_DETECTOR_H */
