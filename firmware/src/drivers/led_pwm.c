/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "led_pwm.h"

#include <stddef.h>

#include "bsp/board.h"
#include "bsp/bsp.h"
#include "bsp/bsp_gpio.h"
#include "bsp/ch32.h"
#include "pwm_dither.h"

static uint16_t s_period;
static volatile led_pwm_sample_callback_t s_sample_callback;

/* Duties in sub-ticks, written by the main loop and read by the update
 * interrupt (16-bit stores are atomic). */
static volatile uint16_t s_duty[LED_CHANNEL_COUNT];
static uint8_t s_dither[LED_CHANNEL_COUNT]; /* interrupt only */

/* Output compare modes, compares preloaded: `led` for red, green and blue,
 * and the timing mode for channel 4, an internal event only. */
static void set_modes(uint16_t led)
{
    BOARD_LED_TIMER->CHCTLR1 = (uint16_t)((led | TIM_OC1PE) | ((led | TIM_OC1PE) << 8));
    BOARD_LED_TIMER->CHCTLR2 = (uint16_t)(led | TIM_OC3PE | TIM_OC4PE);
}

void led_pwm_init(uint32_t frequency_hz)
{
    RCC_APB1PeriphClockCmd(RCC_APB1Periph_TIM3, ENABLE);

    /* Largest resolution within PWM_DITHER_PERIOD_MAX: prescaler only if
     * needed (72 MHz / 20 kHz = 3600 ticks needs none). */
    const uint32_t ticks = bsp_apb1_timer_clock_hz() / frequency_hz;
    const uint32_t prescaler = (ticks - 1u) / PWM_DITHER_PERIOD_MAX;
    s_period = (uint16_t)(ticks / (prescaler + 1u));

    BOARD_LED_TIMER->PSC = (uint16_t)prescaler;
    BOARD_LED_TIMER->ATRLR = (uint16_t)(s_period - 1u);
    BOARD_LED_TIMER->CH4CVR = (uint16_t)(s_period / 2u);
    /* Channel 4 drives no pin: PB1 is the touch input. */
    BOARD_LED_TIMER->CCER = TIM_CC1E | TIM_CC2E | TIM_CC3E;
    BOARD_LED_TIMER->CTLR1 = TIM_ARPE;
    led_pwm_start(); /* its update event also loads the prescaler */

    /* Hand the pins to the timer only now, with every duty at 0 %. The gate
     * resistors (100 Ω) set how fast the MOSFETs switch; 10 MHz edges still
     * give the shortest blue pulses a tenth more light than 2 MHz ones, with
     * the same touch noise. The pulse losses of app_config.c are measured
     * with them. */
    bsp_gpio_config(BOARD_LED_RED_PORT, BOARD_LED_RED_PIN, GPIO_Mode_AF_PP, GPIO_Speed_10MHz);
    bsp_gpio_config(BOARD_LED_GREEN_PORT, BOARD_LED_GREEN_PIN, GPIO_Mode_AF_PP, GPIO_Speed_10MHz);
    bsp_gpio_config(BOARD_LED_BLUE_PORT, BOARD_LED_BLUE_PIN, GPIO_Mode_AF_PP, GPIO_Speed_10MHz);

    NVIC_EnableIRQ(TIM3_IRQn);
}

void led_pwm_stop(void)
{
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        s_duty[i] = 0;
    }
    /* Force the outputs low now: stopping the counter in the middle of a
     * pulse would otherwise leave a channel on. */
    set_modes(TIM_ForcedAction_InActive);
    BOARD_LED_TIMER->DMAINTENR = 0;
    BOARD_LED_TIMER->CTLR1 &= (uint16_t)~TIM_CEN;
}

void led_pwm_start(void)
{
    /* The compares are preloaded: the update event loads the zeros now and
     * restarts the count, so that no pulse of the old duties comes out. */
    BOARD_LED_TIMER->CH1CVR = 0;
    BOARD_LED_TIMER->CH2CVR = 0;
    BOARD_LED_TIMER->CH3CVR = 0;
    BOARD_LED_TIMER->SWEVGR = TIM_UG;
    set_modes(TIM_OCMode_PWM1);
    BOARD_LED_TIMER->INTFR = (uint16_t)~TIM_IT_Update;
    BOARD_LED_TIMER->DMAINTENR = TIM_IT_Update;
    BOARD_LED_TIMER->CTLR1 |= TIM_CEN;
}

uint16_t led_pwm_period(void)
{
    return s_period;
}

void led_pwm_set(const uint16_t duty[LED_CHANNEL_COUNT])
{
    const uint16_t full = (uint16_t)(s_period * PWM_DITHER_SUBTICKS);
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        s_duty[i] = (duty[i] > full) ? full : duty[i];
    }
}

/* Start of a period: the compare registers are preloaded, so the values
 * written now apply to the next period. */
static void dither_period(void)
{
    BOARD_LED_TIMER->CH1CVR = pwm_dither_next(&s_dither[LED_CHANNEL_RED], s_duty[LED_CHANNEL_RED]);
    BOARD_LED_TIMER->CH2CVR =
        pwm_dither_next(&s_dither[LED_CHANNEL_GREEN], s_duty[LED_CHANNEL_GREEN]);
    BOARD_LED_TIMER->CH3CVR =
        pwm_dither_next(&s_dither[LED_CHANNEL_BLUE], s_duty[LED_CHANNEL_BLUE]);
}

void led_pwm_set_sample_callback(led_pwm_sample_callback_t callback)
{
    s_sample_callback = callback;
}

void led_pwm_arm_sample(void)
{
    BOARD_LED_TIMER->INTFR = (uint16_t)~TIM_IT_CC4;
    BOARD_LED_TIMER->DMAINTENR |= TIM_IT_CC4;
}

void TIM3_IRQHandler(void) BSP_ISR;
void TIM3_IRQHandler(void)
{
    const uint16_t flags = BOARD_LED_TIMER->INTFR;
    if ((flags & TIM_IT_Update) != 0u) {
        BOARD_LED_TIMER->INTFR = (uint16_t)~TIM_IT_Update;
        dither_period();
    }
    /* The CC4 flag rises every period: only an armed sample counts. */
    if ((BOARD_LED_TIMER->INTFR & BOARD_LED_TIMER->DMAINTENR & TIM_IT_CC4) != 0u) {
        /* One shot: disarm before running the callback. */
        BOARD_LED_TIMER->DMAINTENR &= (uint16_t)~TIM_IT_CC4;
        BOARD_LED_TIMER->INTFR = (uint16_t)~TIM_IT_CC4;
        const led_pwm_sample_callback_t callback = s_sample_callback;
        if (callback != NULL) {
            callback();
        }
    }
}
