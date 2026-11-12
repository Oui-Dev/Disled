/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Update protocol and boot decision of the bootloader, against a flash
 * model: 32 KB, erased pages read 0xE339E339 as on the CH32V20x, the first
 * 4 KB write-protected as on a protected lamp.
 */
#include <string.h>
#include <unity.h>

#include "bl_core.h"

#define ERASED_WORD 0xE339E339u

static uint8_t flash[BL_FLASH_SIZE];
static uint8_t image[BL_APP_MAX];
static uint8_t request[BL_REPORT_SIZE];
static uint8_t reply[BL_REPORT_SIZE];
static uint8_t seq;

/* Flash model: writes in order, protection of the first sector, failures. */
static uint32_t writes[256];
static uint32_t write_count;
static uint32_t fail_after; /* the next writes that succeed, then failures; 0: never fail */
static bool failing;

static void erase(uint32_t offset)
{
    for (uint32_t i = 0; i < BL_PAGE_SIZE; i += 4u) {
        const uint32_t word = ERASED_WORD;
        memcpy(&flash[offset + i], &word, sizeof word);
    }
}

bool bl_port_write_page(uint32_t offset, const uint32_t *data)
{
    TEST_ASSERT_EQUAL_UINT32_MESSAGE(0, offset % BL_PAGE_SIZE, "page aligned");
    TEST_ASSERT_TRUE_MESSAGE(offset + BL_PAGE_SIZE <= BL_FLASH_SIZE, "inside the flash");
    if (write_count < sizeof writes / sizeof writes[0]) {
        writes[write_count] = offset | (data == NULL ? 1u : 0u);
    }
    write_count++;
    if (offset < BL_APP_OFFSET) {
        return false; /* write-protected: nothing changes */
    }
    if (fail_after != 0u) {
        if (--fail_after == 0u) {
            failing = true;
        }
    } else if (failing) {
        return false; /* say, a flash cell that no longer programs */
    }
    erase(offset);
    if (data != NULL) {
        memcpy(&flash[offset], data, BL_PAGE_SIZE);
    }
    return true;
}

static void flash_erase_all(void)
{
    for (uint32_t offset = 0; offset < BL_FLASH_SIZE; offset += BL_PAGE_SIZE) {
        erase(offset);
    }
}

static uint32_t get32(const uint8_t *p)
{
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static void put32(uint8_t *p, uint32_t value)
{
    for (uint32_t i = 0; i < 4u; i++) {
        p[i] = (uint8_t)(value >> (8u * i));
    }
}

static uint8_t send(uint8_t command, uint32_t arg, const uint8_t *data, uint8_t length)
{
    memset(request, 0, sizeof request);
    seq++;
    request[0] = command;
    request[1] = seq;
    request[2] = length;
    put32(&request[4], arg);
    if (length > 0u) {
        memcpy(&request[BL_DATA_OFFSET], data, (length < BL_DATA_MAX) ? length : BL_DATA_MAX);
    }
    memset(reply, 0xAA, sizeof reply);
    const bool boot = bl_proto_handle(flash, request, reply);
    TEST_ASSERT_EQUAL_UINT8(command, reply[0]);
    TEST_ASSERT_EQUAL_UINT8(seq, reply[1]);
    TEST_ASSERT_EQUAL_UINT8(command == BL_CMD_BOOT, boot);
    return reply[2];
}

static void make_image(uint32_t size)
{
    for (uint32_t i = 0; i < size; i++) {
        image[i] = (uint8_t)((i * 7u) ^ (i >> 8));
    }
}

/* What the host does: whole pages, the last one padded with 0xFF. */
static void program_image(uint32_t size)
{
    uint8_t page[BL_PAGE_SIZE];
    for (uint32_t index = 0; index * BL_PAGE_SIZE < size; index++) {
        memset(page, 0xFF, sizeof page);
        const uint32_t start = index * BL_PAGE_SIZE;
        const uint32_t count = (size - start < BL_PAGE_SIZE) ? size - start : BL_PAGE_SIZE;
        memcpy(page, &image[start], count);
        for (uint32_t offset = 0; offset < BL_PAGE_SIZE; offset += BL_DATA_MAX) {
            const uint32_t chunk =
                (BL_PAGE_SIZE - offset < BL_DATA_MAX) ? BL_PAGE_SIZE - offset : BL_DATA_MAX;
            TEST_ASSERT_EQUAL_UINT8(BL_OK,
                                    send(BL_CMD_DATA, offset, &page[offset], (uint8_t)chunk));
        }
        TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_PROGRAM, index, NULL, 0));
    }
}

static uint8_t finish(uint32_t size, uint32_t crc, uint32_t tag)
{
    uint8_t data[8];
    put32(&data[0], crc);
    put32(&data[4], tag);
    return send(BL_CMD_FINISH, size, data, sizeof data);
}

static void update(uint32_t size, uint32_t tag)
{
    make_image(size);
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_BEGIN, 0, NULL, 0));
    program_image(size);
    TEST_ASSERT_EQUAL_UINT8(BL_OK, finish(size, bl_crc32(image, size), tag));
}

void setUp(void)
{
    flash_erase_all();
    memset(writes, 0, sizeof writes);
    write_count = 0;
    fail_after = 0;
    failing = false;
    seq = 0;
    (void)bl_boot_reasons(flash, false); /* as after a reset */
}

void tearDown(void)
{
}

static void test_crc32_vectors(void)
{
    TEST_ASSERT_EQUAL_HEX32(0x00000000u, bl_crc32((const uint8_t *)"", 0));
    TEST_ASSERT_EQUAL_HEX32(0xE8B7BE43u, bl_crc32((const uint8_t *)"a", 1));
    TEST_ASSERT_EQUAL_HEX32(0xCBF43926u, bl_crc32((const uint8_t *)"123456789", 9));
    TEST_ASSERT_EQUAL_HEX32(
        0x414FA339u, bl_crc32((const uint8_t *)"The quick brown fox jumps over the lazy dog", 43));
}

static void test_crc32_matches_the_bit_loop(void)
{
    make_image(BL_APP_MAX);
    uint32_t crc = 0xFFFFFFFFu;
    for (uint32_t i = 0; i < BL_APP_MAX; i++) {
        crc ^= image[i];
        for (uint32_t bit = 0; bit < 8u; bit++) {
            crc = (crc >> 1) ^ (0xEDB88320u & (0u - (crc & 1u)));
        }
    }
    TEST_ASSERT_EQUAL_HEX32(~crc, bl_crc32(image, BL_APP_MAX));
}

static void test_layout(void)
{
    TEST_ASSERT_EQUAL_UINT32(0x1000u, BL_APP_OFFSET);
    TEST_ASSERT_EQUAL_UINT32(0x7F00u, BL_RECORD_OFFSET);
    TEST_ASSERT_EQUAL_UINT32(28416u, BL_APP_MAX);
    TEST_ASSERT_EQUAL_UINT32(111u, BL_APP_PAGES);
    TEST_ASSERT_TRUE(BL_RECORD_MAGIC != ERASED_WORD);
    TEST_ASSERT_TRUE(BL_RECORD_MAGIC != 0xFFFFFFFFu && BL_RECORD_MAGIC != 0u);
}

static void test_erased_flash_stays_in_update_mode(void)
{
    TEST_ASSERT_EQUAL_UINT8(BL_REASON_APP_INVALID, bl_boot_reasons(flash, false));
    TEST_ASSERT_EQUAL_UINT8(BL_REASON_APP_INVALID | BL_REASON_REQUESTED,
                            bl_boot_reasons(flash, true));
    TEST_ASSERT_FALSE(bl_app_valid(flash));
}

static void test_valid_image_starts_unless_requested(void)
{
    update(5000, 0x010000u);
    TEST_ASSERT_TRUE(bl_app_valid(flash));
    TEST_ASSERT_EQUAL_UINT8(0, bl_boot_reasons(flash, false));
    TEST_ASSERT_EQUAL_UINT8(BL_REASON_REQUESTED, bl_boot_reasons(flash, true));
}

static void test_update_writes_the_record_last(void)
{
    const uint32_t size = 3u * BL_PAGE_SIZE + 10u;
    update(size, 0x00020001u);

    /* Record erased first, the four pages, then the record. */
    TEST_ASSERT_EQUAL_UINT32(6, write_count);
    TEST_ASSERT_EQUAL_HEX32(BL_RECORD_OFFSET | 1u, writes[0]);
    for (uint32_t i = 0; i < 4u; i++) {
        TEST_ASSERT_EQUAL_HEX32(BL_APP_OFFSET + (i * BL_PAGE_SIZE), writes[1u + i]);
    }
    TEST_ASSERT_EQUAL_HEX32(BL_RECORD_OFFSET, writes[5]);

    TEST_ASSERT_EQUAL_MEMORY(image, &flash[BL_APP_OFFSET], size);
    TEST_ASSERT_EQUAL_HEX8(0xFF, flash[BL_APP_OFFSET + size]); /* padding of the last page */
    const uint8_t *record = &flash[BL_RECORD_OFFSET];
    TEST_ASSERT_EQUAL_HEX32(BL_RECORD_MAGIC, get32(&record[0]));
    TEST_ASSERT_EQUAL_UINT32(size, get32(&record[4]));
    TEST_ASSERT_EQUAL_HEX32(bl_crc32(image, size), get32(&record[8]));
    TEST_ASSERT_EQUAL_HEX32(0x00020001u, get32(&record[12]));
    TEST_ASSERT_EQUAL_HEX8(0xFF, record[16]);
    TEST_ASSERT_FALSE(bl_proto_updating());
}

static void test_largest_image(void)
{
    update(BL_APP_MAX, 1);
    TEST_ASSERT_TRUE(bl_app_valid(flash));
    TEST_ASSERT_EQUAL_UINT32(BL_APP_PAGES + 2u, write_count);
}

static void test_info(void)
{
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_INFO, 0, NULL, 0));
    const uint8_t *body = &reply[BL_REPLY_OFFSET];
    TEST_ASSERT_EQUAL_UINT8(0, reply[3]);
    TEST_ASSERT_EQUAL_UINT8(BL_PROTO_MAJOR, body[0]);
    TEST_ASSERT_EQUAL_UINT8(BL_PROTO_MINOR, body[1]);
    TEST_ASSERT_EQUAL_UINT8(BL_REASON_APP_INVALID, body[2]);
    TEST_ASSERT_EQUAL_UINT32(BL_APP_OFFSET, get32(&body[4]));
    TEST_ASSERT_EQUAL_UINT32(BL_APP_MAX, get32(&body[8]));
    TEST_ASSERT_EQUAL_UINT8(0x00, body[12]); /* 256, little-endian */
    TEST_ASSERT_EQUAL_UINT8(0x01, body[13]);
    TEST_ASSERT_EQUAL_UINT8(BL_DATA_MAX, body[14]);
    TEST_ASSERT_EQUAL_UINT32(0, get32(&body[16])); /* no record */
    for (uint32_t i = BL_REPLY_OFFSET + 28u; i < BL_REPORT_SIZE; i++) {
        TEST_ASSERT_EQUAL_UINT8(0, reply[i]);
    }

    update(1234, 0x00010203u);
    (void)bl_boot_reasons(flash, true);
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_INFO, 0, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_REASON_REQUESTED, body[2]);
    TEST_ASSERT_EQUAL_UINT32(1234, get32(&body[16]));
    TEST_ASSERT_EQUAL_HEX32(bl_crc32(image, 1234), get32(&body[20]));
    TEST_ASSERT_EQUAL_HEX32(0x00010203u, get32(&body[24]));
}

static void test_program_and_finish_need_begin(void)
{
    make_image(BL_PAGE_SIZE);
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_DATA, 0, image, BL_DATA_MAX));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_SEQUENCE, send(BL_CMD_PROGRAM, 0, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_SEQUENCE, finish(BL_PAGE_SIZE, 0, 0));
    TEST_ASSERT_EQUAL_UINT32(0, write_count);

    /* A reset ends the update. */
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_BEGIN, 0, NULL, 0));
    TEST_ASSERT_TRUE(bl_proto_updating());
    (void)bl_boot_reasons(flash, false);
    TEST_ASSERT_FALSE(bl_proto_updating());
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_SEQUENCE, send(BL_CMD_PROGRAM, 0, NULL, 0));
}

static void test_arguments_out_of_range(void)
{
    uint8_t data[BL_DATA_MAX] = { 0 };
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_BEGIN, 0, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_DATA, BL_PAGE_SIZE - 8u, data, 8));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_ARGUMENT, send(BL_CMD_DATA, BL_PAGE_SIZE - 7u, data, 8));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_ARGUMENT, send(BL_CMD_DATA, 0xFFFFFFFFu, data, 8));
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_DATA, BL_PAGE_SIZE, data, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_ARGUMENT, send(BL_CMD_DATA, 0, data, BL_DATA_MAX + 1u));
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_PROGRAM, BL_APP_PAGES - 1u, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_ARGUMENT, send(BL_CMD_PROGRAM, BL_APP_PAGES, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_ARGUMENT, send(BL_CMD_PROGRAM, 0x01000000u, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_ARGUMENT, finish(0, 0, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_ARGUMENT, finish(BL_APP_MAX + 1u, 0, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_COMMAND, send(0x00, 0, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_COMMAND, send(0x81, 0, NULL, 0));
}

static void test_never_writes_the_bootloader(void)
{
    update(BL_APP_MAX, 0);
    for (uint32_t i = 0; i < write_count; i++) {
        TEST_ASSERT_TRUE((writes[i] & ~1u) >= BL_APP_OFFSET);
    }
}

static void test_bad_crc_leaves_the_image_invalid(void)
{
    const uint32_t size = 2000;
    make_image(size);
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_BEGIN, 0, NULL, 0));
    program_image(size);
    const uint32_t crc = bl_crc32(image, size);
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_CRC, finish(size, crc ^ 1u, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_CRC, finish(size - 1u, crc, 0));
    TEST_ASSERT_FALSE(bl_app_valid(flash));
    TEST_ASSERT_TRUE(bl_proto_updating());
    /* The host can still finish with the right values. */
    TEST_ASSERT_EQUAL_UINT8(BL_OK, finish(size, crc, 0));
    TEST_ASSERT_TRUE(bl_app_valid(flash));
}

static void test_missing_page_fails_the_crc(void)
{
    const uint32_t size = 4u * BL_PAGE_SIZE;
    update(size, 0);
    make_image(size);
    image[BL_PAGE_SIZE] ^= 0x55u; /* a new image that differs in its second page */
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_BEGIN, 0, NULL, 0));
    uint8_t page[BL_PAGE_SIZE];
    for (uint32_t index = 0; index < 4u; index++) {
        if (index == 1u) {
            continue; /* never sent: the old page stays */
        }
        memcpy(page, &image[index * BL_PAGE_SIZE], BL_PAGE_SIZE);
        for (uint32_t offset = 0; offset < BL_PAGE_SIZE; offset += 32u) {
            TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_DATA, offset, &page[offset], 32));
        }
        TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_PROGRAM, index, NULL, 0));
    }
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_CRC, finish(size, bl_crc32(image, size), 0));
    TEST_ASSERT_FALSE(bl_app_valid(flash));
}

static void test_interrupted_update_stays_in_update_mode(void)
{
    update(6000, 0);
    TEST_ASSERT_EQUAL_UINT8(0, bl_boot_reasons(flash, false));

    /* A new update loses power after BEGIN and a few pages. */
    make_image(6000);
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_BEGIN, 0, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_PROGRAM, 0, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_PROGRAM, 1, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_REASON_APP_INVALID, bl_boot_reasons(flash, false));

    /* And the next update completes. */
    update(6000, 0);
    TEST_ASSERT_EQUAL_UINT8(0, bl_boot_reasons(flash, false));
}

static void test_record_damage_is_caught(void)
{
    update(3000, 0);
    uint8_t *record = &flash[BL_RECORD_OFFSET];

    record[8] ^= 0x01u; /* CRC */
    TEST_ASSERT_FALSE(bl_app_valid(flash));
    record[8] ^= 0x01u;
    TEST_ASSERT_TRUE(bl_app_valid(flash));

    flash[BL_APP_OFFSET + 2999u] ^= 0x80u; /* the image itself */
    TEST_ASSERT_FALSE(bl_app_valid(flash));
    flash[BL_APP_OFFSET + 2999u] ^= 0x80u;

    put32(&record[4], 0); /* size */
    TEST_ASSERT_FALSE(bl_app_valid(flash));
    put32(&record[4], BL_APP_MAX + 1u);
    TEST_ASSERT_FALSE(bl_app_valid(flash));
}

static void test_flash_errors_are_reported(void)
{
    make_image(BL_PAGE_SIZE);
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_BEGIN, 0, NULL, 0));
    fail_after = 1; /* the next write works, then the cell dies */
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_PROGRAM, 0, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_FLASH, send(BL_CMD_PROGRAM, 1, NULL, 0));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_FLASH,
                            finish(BL_PAGE_SIZE, bl_crc32(&flash[BL_APP_OFFSET], BL_PAGE_SIZE), 0));
    TEST_ASSERT_FALSE(bl_app_valid(flash));
    TEST_ASSERT_EQUAL_UINT8(BL_ERR_FLASH, send(BL_CMD_BEGIN, 0, NULL, 0));
    TEST_ASSERT_FALSE(bl_proto_updating());
}

static void test_boot_replies_first(void)
{
    TEST_ASSERT_EQUAL_UINT8(BL_OK, send(BL_CMD_BOOT, 0, NULL, 0));
    for (uint32_t i = 3; i < BL_REPORT_SIZE; i++) {
        TEST_ASSERT_EQUAL_UINT8(0, reply[i]);
    }
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_crc32_vectors);
    RUN_TEST(test_crc32_matches_the_bit_loop);
    RUN_TEST(test_layout);
    RUN_TEST(test_erased_flash_stays_in_update_mode);
    RUN_TEST(test_valid_image_starts_unless_requested);
    RUN_TEST(test_update_writes_the_record_last);
    RUN_TEST(test_largest_image);
    RUN_TEST(test_info);
    RUN_TEST(test_program_and_finish_need_begin);
    RUN_TEST(test_arguments_out_of_range);
    RUN_TEST(test_never_writes_the_bootloader);
    RUN_TEST(test_bad_crc_leaves_the_image_invalid);
    RUN_TEST(test_missing_page_fails_the_crc);
    RUN_TEST(test_interrupted_update_stays_in_update_mode);
    RUN_TEST(test_record_damage_is_caught);
    RUN_TEST(test_flash_errors_are_reported);
    RUN_TEST(test_boot_replies_first);
    return UNITY_END();
}
