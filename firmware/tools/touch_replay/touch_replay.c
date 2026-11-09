/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Replays a touch recording through touch_detector on the build machine,
 * to try detector settings before flashing them. See README.md.
 */
#include <stdio.h>
#include <string.h>

#include "touch_detector.h"

#define WINDOW_MS 20u

/* Defaults: a copy of app_touch_config in src/app/app_config.c, which
 * cannot build here (it includes the SDK). Check them before relying on a
 * result, or override them on the command line. */
static touch_config_t config = {
    .window_samples = WINDOW_MS,
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

/* Sensitivity correction in percent, as SET_TOUCH_SENSITIVITY sets it. */
static int sensitivity;

typedef struct {
    const char *name;
    uint16_t *field;
} option_t;

static const option_t options[] = {
    { "press", &config.press_permille },     { "release", &config.release_permille },
    { "rise", &config.press_rise_permille }, { "proximity", &config.proximity_permille },
    { "debounce", &config.debounce_ms },     { "rearm", &config.rearm_ms },
    { "long", &config.long_press_ms },       { "object", &config.object_ms },
};

static int parse_option(const char *arg)
{
    static const char sensitivity_name[] = "sensitivity=";
    int percent = 0;
    if (strncmp(arg, sensitivity_name, sizeof sensitivity_name - 1u) == 0 &&
        sscanf(arg + sizeof sensitivity_name - 1u, "%d", &percent) == 1 &&
        percent >= TOUCH_SENSITIVITY_MIN && percent <= TOUCH_SENSITIVITY_MAX) {
        sensitivity = percent;
        return 1;
    }
    for (size_t i = 0; i < sizeof options / sizeof options[0]; i++) {
        const size_t length = strlen(options[i].name);
        unsigned value = 0;
        if (strncmp(arg, options[i].name, length) == 0 && arg[length] == '=' &&
            sscanf(arg + length + 1, "%u", &value) == 1 && value <= 65535u) {
            *options[i].field = (uint16_t)value;
            return 1;
        }
    }
    return 0;
}

int main(int argc, char **argv)
{
    if (argc < 2) {
        fprintf(stderr,
                "usage: touch_replay <recording> [name=value ...]\n"
                "names: press release rise proximity debounce rearm long object (per mille, ms)\n"
                "       sensitivity (percent, -50 to 50)\n");
        return 2;
    }
    for (int i = 2; i < argc; i++) {
        if (!parse_option(argv[i])) {
            fprintf(stderr, "unknown option: %s\n", argv[i]);
            return 2;
        }
    }
    FILE *file = fopen(argv[1], "r");
    if (file == NULL) {
        perror(argv[1]);
        return 1;
    }

    touch_detector_t td;
    touch_detector_init(&td, &config);
    touch_detector_set_sensitivity(&td, (int8_t)sensitivity);
    const touch_thresholds_t thresholds = touch_detector_thresholds(&td);
    printf("sensitivity %+d%%: press %u, release %u, rise %u per mille\n",
           (int)touch_detector_sensitivity(&td), (unsigned)thresholds.press,
           (unsigned)thresholds.release, (unsigned)thresholds.rise);
    uint32_t now_ms = 0;
    unsigned windows = 0;
    unsigned recorded = 0;
    unsigned shorts = 0;
    unsigned longs = 0;
    char line[256];
    while (fgets(line, sizeof line, file) != NULL) {
        const char *p = line;
        while (*p == ' ' || *p == '>' || (unsigned char)*p >= 0x80u) {
            p++; /* prompt, or the UTF-8 byte order mark */
        }
        unsigned raw = 0;
        unsigned value = 0;
        unsigned baseline = 0;
        int delta = 0;
        int state = 0;
        if (sscanf(p, "%u %u %u %d %d |", &raw, &value, &baseline, &delta, &state) != 5) {
            if (strstr(line, "] I touch:") != NULL) {
                recorded++;
            }
            continue;
        }
        if (windows == 0u) {
            /* Calibrate on the baseline the firmware had. */
            for (uint32_t i = 0; i < config.calibration_windows * WINDOW_MS; i++) {
                (void)touch_detector_feed(&td, (uint16_t)baseline, ++now_ms);
            }
        }
        windows++;
        for (uint32_t i = 0; i < WINDOW_MS; i++) {
            const touch_event_t event = touch_detector_feed(&td, (uint16_t)value, ++now_ms);
            if (event == TOUCH_EVENT_NONE) {
                continue;
            }
            const int is_long = event == TOUCH_EVENT_LONG_PRESS;
            shorts += is_long ? 0u : 1u;
            longs += is_long ? 1u : 0u;
            printf("%7.2f s  %s press\n", windows * (WINDOW_MS / 1000.0),
                   is_long ? "long " : "short");
        }
    }
    fclose(file);
    printf("%u windows: %u short, %u long; the recording logged %u events\n", windows, shorts,
           longs, recorded);
    return 0;
}
