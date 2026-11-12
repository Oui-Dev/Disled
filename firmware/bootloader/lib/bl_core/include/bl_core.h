/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Hardware-independent part of the bootloader: flash layout, validity
 * record, boot decision and update protocol (README.md of the bootloader).
 * It includes no SDK header, so it also builds natively for the tests.
 */
#ifndef BL_CORE_H
#define BL_CORE_H

#include <stdbool.h>
#include <stdint.h>

/* Flash layout, in bytes from the start of the 32 KB flash. The bootloader
 * fills the first 4 KB write-protect sector; the validity record takes the
 * last 256-byte page, so that it is never in a protected sector. */
#define BL_FLASH_SIZE    0x8000u
#define BL_PAGE_SIZE     256u /* unit of the fast erase and program */
#define BL_APP_OFFSET    0x1000u
#define BL_RECORD_OFFSET (BL_FLASH_SIZE - BL_PAGE_SIZE)
#define BL_APP_MAX       (BL_RECORD_OFFSET - BL_APP_OFFSET) /* 28416 bytes */
#define BL_APP_PAGES     (BL_APP_MAX / BL_PAGE_SIZE)        /* 111 */

/* Validity record, at the start of the record page, written last by an
 * update and erased first by the next one. The image is valid when the
 * magic is there and the CRC-32 of its `size` bytes matches. Erased flash
 * reads 0xE339E339 on this chip, never the magic. */
#define BL_RECORD_MAGIC 0x50414C44u /* "DLAP" */
typedef struct {
    uint32_t magic;
    uint32_t size; /* bytes from BL_APP_OFFSET */
    uint32_t crc;  /* CRC-32 (zlib) of those bytes */
    uint32_t tag;  /* host value, kept for INFO: the firmware version */
} bl_record_t;

/* Backup register DATAR8: the application writes this value, then resets,
 * to ask for update mode. Power-on clears it. */
#define BL_ENTER_MAGIC 0xB007u

/* Why the bootloader stays in update mode (0: start the application). */
#define BL_REASON_REQUESTED   (1u << 0) /* BL_ENTER_MAGIC in DATAR8 */
#define BL_REASON_APP_INVALID (1u << 1) /* no record, or a CRC mismatch */

/* Update protocol: 64-byte HID reports without report ID, one reply for
 * each request. Request: command, seq, data length, 0, u32 argument, data.
 * Reply: command, seq, status, 0, then the INFO body or zeros. */
#define BL_PROTO_MAJOR  1u
#define BL_PROTO_MINOR  0u
#define BL_REPORT_SIZE  64u
#define BL_DATA_OFFSET  8u
#define BL_DATA_MAX     (BL_REPORT_SIZE - BL_DATA_OFFSET) /* 56 */
#define BL_REPLY_OFFSET 4u

enum {
    BL_CMD_INFO = 0x01,    /* reply: the INFO body */
    BL_CMD_BEGIN = 0x02,   /* erases the record: the application is invalid */
    BL_CMD_DATA = 0x03,    /* argument: offset in the page buffer */
    BL_CMD_PROGRAM = 0x04, /* argument: page index from BL_APP_OFFSET */
    BL_CMD_FINISH = 0x05,  /* argument: image size; data: CRC-32, tag */
    BL_CMD_BOOT = 0x06,    /* reply, then a reset into the application */
};

enum {
    BL_OK = 0,
    BL_ERR_COMMAND = 1,  /* unknown command */
    BL_ERR_ARGUMENT = 2, /* out of range */
    BL_ERR_SEQUENCE = 3, /* PROGRAM or FINISH without BEGIN */
    BL_ERR_FLASH = 4,    /* the flash does not read back as written */
    BL_ERR_CRC = 5,      /* the image in flash does not match the CRC */
};

/* CRC-32 of zlib (reflected polynomial 0xEDB88320, initial value and final
 * XOR 0xFFFFFFFF). */
uint32_t bl_crc32(const uint8_t *data, uint32_t size);

/* `flash`: the 32 KB of flash, as the CPU reads it. */
bool bl_app_valid(const uint8_t *flash);

/* Boot decision, made once after reset: the BL_REASON_ bits, remembered for
 * INFO. `requested`: BL_ENTER_MAGIC was in DATAR8. No update runs after it. */
uint8_t bl_boot_reasons(const uint8_t *flash, bool requested);

/* Handles one request and fills its reply. Returns true for BOOT: the
 * caller resets once the reply is sent. */
bool bl_proto_handle(const uint8_t *flash, const uint8_t request[BL_REPORT_SIZE],
                     uint8_t reply[BL_REPORT_SIZE]);

/* An update runs: between BEGIN and FINISH. */
bool bl_proto_updating(void);

/* Provided by the port (src/flash.c on the chip, the tests on the build
 * machine): erases the page at `offset` and, unless `data` is NULL,
 * programs its 256 bytes. True when the page reads back as expected. */
bool bl_port_write_page(uint32_t offset, const uint32_t *data);

#endif /* BL_CORE_H */
