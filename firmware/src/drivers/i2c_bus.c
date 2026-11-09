/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "i2c_bus.h"

#include "bsp/board.h"
#include "bsp/bsp_gpio.h"
#include "bsp/bsp_time.h"
#include "bsp/ch32.h"
#include "time_utils.h"

/* A byte at 100 kHz takes 90 us; 3 ms covers any legal clock stretching. */
#define WAIT_TIMEOUT_MS 3u

static uint32_t s_speed_hz;

static uint16_t s_error_count;

static bool fail(void)
{
    if (s_error_count < UINT16_MAX) {
        s_error_count++;
    }
    return false;
}

/* Clocks SCL by hand: up to nine clocks let a slave finish the byte it is
 * sending and release SDA, then a STOP condition resets its state. */
static void release_bus(void)
{
    GPIO_SetBits(BOARD_I2C_PORT, BOARD_I2C_SCL_PIN | BOARD_I2C_SDA_PIN);
    bsp_gpio_config(BOARD_I2C_PORT, BOARD_I2C_SCL_PIN | BOARD_I2C_SDA_PIN, GPIO_Mode_Out_OD,
                    GPIO_Speed_10MHz);
    for (int i = 0; i < 9; i++) {
        if (GPIO_ReadInputDataBit(BOARD_I2C_PORT, BOARD_I2C_SDA_PIN) == Bit_SET) {
            break;
        }
        GPIO_ResetBits(BOARD_I2C_PORT, BOARD_I2C_SCL_PIN);
        bsp_time_delay_us(5);
        GPIO_SetBits(BOARD_I2C_PORT, BOARD_I2C_SCL_PIN);
        bsp_time_delay_us(5);
    }
    /* STOP: SDA rises while SCL is high. SDA must go low while SCL is low;
     * pulling it low with SCL high is a START, which leaves the sensor out of
     * step and makes the next transfer lose arbitration. */
    GPIO_ResetBits(BOARD_I2C_PORT, BOARD_I2C_SCL_PIN);
    bsp_time_delay_us(5);
    GPIO_ResetBits(BOARD_I2C_PORT, BOARD_I2C_SDA_PIN);
    bsp_time_delay_us(5);
    GPIO_SetBits(BOARD_I2C_PORT, BOARD_I2C_SCL_PIN);
    bsp_time_delay_us(5);
    GPIO_SetBits(BOARD_I2C_PORT, BOARD_I2C_SDA_PIN);
    bsp_time_delay_us(5);
}

/* Full reset of the peripheral, then hands it the pins. Resetting before
 * the pins switch to the alternate function avoids the spurious BUSY state
 * these controllers are known for, which keeps the bus busy for good after
 * the first transfer. */
static void configure_peripheral(void)
{
    I2C_DeInit(BOARD_I2C);
    I2C_SoftwareResetCmd(BOARD_I2C, ENABLE);
    I2C_SoftwareResetCmd(BOARD_I2C, DISABLE);

    bsp_gpio_config(BOARD_I2C_PORT, BOARD_I2C_SCL_PIN | BOARD_I2C_SDA_PIN, GPIO_Mode_AF_OD,
                    GPIO_Speed_10MHz);

    I2C_InitTypeDef init = {
        .I2C_ClockSpeed = s_speed_hz,
        .I2C_Mode = I2C_Mode_I2C,
        .I2C_DutyCycle = I2C_DutyCycle_2,
        .I2C_OwnAddress1 = 0,
        .I2C_Ack = I2C_Ack_Enable,
        .I2C_AcknowledgedAddress = I2C_AcknowledgedAddress_7bit,
    };
    I2C_Init(BOARD_I2C, &init); /* also enables the peripheral */
}

/* Releases the bus and resets the peripheral: at start-up, and after a
 * failed transfer so that the next one starts clean whatever went wrong. */
static void recover(void)
{
    release_bus();
    configure_peripheral();
}

void i2c_bus_init(uint32_t speed_hz)
{
    s_speed_hz = speed_hz;
    RCC_APB1PeriphClockCmd(RCC_APB1Periph_I2C1, ENABLE);
    recover();
}

/* Waits until all the flags of `event` are set. A NACK, a lost arbitration
 * or a bus error ends the wait, as the timeout does. */
static bool wait_event(uint32_t event)
{
    const uint32_t start = bsp_time_ms();
    while (I2C_CheckEvent(BOARD_I2C, event) != READY) {
        if (I2C_GetFlagStatus(BOARD_I2C, I2C_FLAG_AF) != RESET ||
            I2C_GetFlagStatus(BOARD_I2C, I2C_FLAG_ARLO) != RESET ||
            I2C_GetFlagStatus(BOARD_I2C, I2C_FLAG_BERR) != RESET ||
            time_elapsed_ms(bsp_time_ms(), start) > WAIT_TIMEOUT_MS) {
            return fail();
        }
    }
    return true;
}

static bool wait_flag(uint32_t flag)
{
    const uint32_t start = bsp_time_ms();
    while (I2C_GetFlagStatus(BOARD_I2C, flag) == RESET) {
        if (I2C_GetFlagStatus(BOARD_I2C, I2C_FLAG_AF) != RESET ||
            time_elapsed_ms(bsp_time_ms(), start) > WAIT_TIMEOUT_MS) {
            return fail();
        }
    }
    return true;
}

static bool wait_bus_free(void)
{
    const uint32_t start = bsp_time_ms();
    while (I2C_GetFlagStatus(BOARD_I2C, I2C_FLAG_BUSY) != RESET) {
        if (time_elapsed_ms(bsp_time_ms(), start) > WAIT_TIMEOUT_MS) {
            return fail();
        }
    }
    return true;
}

static bool start_write(uint8_t address)
{
    if (!wait_bus_free()) {
        return false;
    }
    I2C_GenerateSTART(BOARD_I2C, ENABLE);
    if (!wait_event(I2C_EVENT_MASTER_MODE_SELECT)) {
        return false;
    }
    I2C_Send7bitAddress(BOARD_I2C, (uint8_t)(address << 1), I2C_Direction_Transmitter);
    return wait_event(I2C_EVENT_MASTER_TRANSMITTER_MODE_SELECTED);
}

bool i2c_bus_write(uint8_t address, const uint8_t *data, size_t length)
{
    bool ok = start_write(address);
    for (size_t i = 0; ok && i < length; i++) {
        I2C_SendData(BOARD_I2C, data[i]);
        ok = wait_event(I2C_EVENT_MASTER_BYTE_TRANSMITTED);
    }
    if (ok) {
        I2C_GenerateSTOP(BOARD_I2C, ENABLE);
    } else {
        recover();
    }
    return ok;
}

bool i2c_bus_read_word(uint8_t address, uint8_t reg, uint8_t out[2])
{
    bool ok = start_write(address);
    if (ok) {
        I2C_SendData(BOARD_I2C, reg);
        ok = wait_event(I2C_EVENT_MASTER_BYTE_TRANSMITTED);
    }
    if (ok) {
        I2C_GenerateSTART(BOARD_I2C, ENABLE); /* repeated start */
        ok = wait_event(I2C_EVENT_MASTER_MODE_SELECT);
    }
    if (!ok) {
        recover();
        return false;
    }

    /* Byte-by-byte reception, as in the WCH I2C examples: the first byte is
     * acknowledged; once it is read, ACK goes off and STOP is queued so the
     * second byte gets the NACK that ends the transfer. The STM32 two-byte
     * method, with POS and BTF, never completes on the CH32V203. */
    I2C_AcknowledgeConfig(BOARD_I2C, ENABLE);
    I2C_Send7bitAddress(BOARD_I2C, (uint8_t)(address << 1), I2C_Direction_Receiver);
    ok = wait_event(I2C_EVENT_MASTER_RECEIVER_MODE_SELECTED);
    for (size_t i = 0; ok && i < 2u; i++) {
        if (i == 1u) {
            I2C_AcknowledgeConfig(BOARD_I2C, DISABLE);
            I2C_GenerateSTOP(BOARD_I2C, ENABLE);
        }
        ok = wait_flag(I2C_FLAG_RXNE);
        if (ok) {
            out[i] = I2C_ReceiveData(BOARD_I2C);
        }
    }
    if (!ok) {
        recover();
    }
    return ok;
}

uint16_t i2c_bus_error_count(void)
{
    return s_error_count;
}
