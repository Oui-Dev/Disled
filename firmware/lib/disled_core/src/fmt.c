/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "fmt.h"

#include <stdbool.h>

typedef struct {
    char *out;
    size_t size;
    size_t length; /* characters the full output would have */
} sink_t;

static void emit(sink_t *sink, char c)
{
    if (sink->length + 1u < sink->size) {
        sink->out[sink->length] = c;
    }
    sink->length++;
}

static void emit_padded(sink_t *sink, const char *text, size_t text_length, size_t width, bool left,
                        char pad)
{
    size_t padding = (width > text_length) ? width - text_length : 0u;
    if (!left) {
        for (; padding > 0u; padding--) {
            emit(sink, pad);
        }
    }
    for (size_t i = 0; i < text_length; i++) {
        emit(sink, text[i]);
    }
    for (; padding > 0u; padding--) {
        emit(sink, ' ');
    }
}

/* Writes `value` in decimal, ending at `buffer_end`. Returns its start. */
static char *format_unsigned(char *buffer_end, unsigned long value)
{
    char *p = buffer_end;
    do {
        *--p = (char)('0' + value % 10u);
        value /= 10u;
    } while (value != 0u);
    return p;
}

size_t fmt_vsnprintf(char *out, size_t size, const char *format, va_list args)
{
    sink_t sink = { out, size, 0 };

    for (const char *f = format; *f != '\0'; f++) {
        if (*f != '%') {
            emit(&sink, *f);
            continue;
        }
        f++;
        bool left = false;
        char pad = ' ';
        for (;; f++) {
            if (*f == '-') {
                left = true;
            } else if (*f == '0') {
                pad = '0';
            } else {
                break;
            }
        }
        size_t width = 0;
        while (*f >= '0' && *f <= '9') {
            width = width * 10u + (size_t)(*f - '0');
            f++;
        }
        bool is_long = false;
        if (*f == 'l') {
            is_long = true;
            f++;
        }

        char number[24];
        char *const end = number + sizeof number;
        switch (*f) {
            case 'd': {
                const long value = is_long ? va_arg(args, long) : (long)va_arg(args, int);
                const unsigned long magnitude =
                    (value < 0) ? (0ul - (unsigned long)value) : (unsigned long)value;
                char *start = format_unsigned(end, magnitude);
                if (value < 0 && pad == '0' && !left) {
                    /* The sign goes before the zero padding: -0042. */
                    emit(&sink, '-');
                    width = (width > 0u) ? width - 1u : 0u;
                } else if (value < 0) {
                    *--start = '-';
                }
                emit_padded(&sink, start, (size_t)(end - start), width, left, pad);
                break;
            }
            case 'u': {
                const unsigned long value =
                    is_long ? va_arg(args, unsigned long) : (unsigned long)va_arg(args, unsigned);
                const char *start = format_unsigned(end, value);
                emit_padded(&sink, start, (size_t)(end - start), width, left, pad);
                break;
            }
            case 's': {
                const char *text = va_arg(args, const char *);
                if (text == NULL) {
                    text = "(null)";
                }
                size_t text_length = 0;
                while (text[text_length] != '\0') {
                    text_length++;
                }
                emit_padded(&sink, text, text_length, width, left, ' ');
                break;
            }
            case '%':
                emit(&sink, '%');
                break;
            case '\0':
                f--; /* lone '%' at the end: stop at the terminator */
                break;
            default:
                emit(&sink, '%');
                emit(&sink, *f);
                break;
        }
    }

    if (size > 0u) {
        out[(sink.length < size) ? sink.length : size - 1u] = '\0';
    }
    return sink.length;
}

size_t fmt_snprintf(char *out, size_t size, const char *format, ...)
{
    va_list args;
    va_start(args, format);
    const size_t length = fmt_vsnprintf(out, size, format, args);
    va_end(args);
    return length;
}
