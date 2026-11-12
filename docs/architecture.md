# Architecture

Disled has three parts that share one protocol:

```
 ┌────────────── lamp (CH32V203) ──────────────┐        ┌──────── desktop app (Tauri) ────────┐
 │ touch ─┐                                    │  USB   │ Rust: USB worker, codec, control,   │
 │ light ─┼─> lamp state ─> LED output (PWM)   │<──────>│ plugins, microphone, updates        │
 │ host ──┘        └─> notifications           │  HID   │ Vue: popup and windows              │
 └─────────────────────────────────────────────┘        └─────────────────────────────────────┘
                      protocol/README.md + protocol/vectors/v2.json
```

The lamp owns its state. The touch changes it locally, the host reads and
changes it, and the lamp notifies every change. [behavior.md](behavior.md)
specifies the behavior itself. The lamp keeps nothing across a restart:
the desktop app sends it the custom colors, the touch cycle and its settings
at every connection. While the lamp follows an integration, and during a
call, the app chooses its color.

## Firmware

### Layers

| Layer | Path | Depends on | Role |
|---|---|---|---|
| Application | `src/app/` | everything below | wiring, scheduling, host protocol, debug commands, tunables |
| Diagnostics | `src/diag/` | usb, core | logs and line console over CDC (debug builds) |
| USB | `src/usb/` | bsp, TinyUSB, core | descriptors, link state, HID queues, CDC stream |
| Drivers | `src/drivers/` | bsp, SDK | LED PWM, TouchKey, I2C, VEML7700 |
| Board support | `src/bsp/` | SDK | pin map, clocks, time base, watchdog, unique ID, faults |
| Core | `lib/disled_core/` | C library only | all decisions: state machine, palette and touch cycle, touch detection and its gate during an alert, alert breathing, brightness, color, power cap, PWM dithering, self-test, protocol codec, scheduler, formatter, assertions |

Only `src/bsp/ch32.h` includes the vendor device header. Includes are
relative to `src/` (`#include "drivers/led_pwm.h"`), so every include shows
the layer it reaches into. The core never includes an SDK header, so it
builds natively and runs under Unity in CI.

### Data flow

```
TIM3 CC4 interrupt (1 kHz, fixed PWM phase) ─> TouchKey sample ─> queue
                                                                    │
main loop ── touch task ─> touch_gate (closed during an alert) ─> touch_detector
                             ─> short/long press ─> lamp_fsm: touch cycle, touch mode
          ── ALS task (100 ms) ─> VEML7700 count at one range, every 400 ms
                                     ─> lux ─> brightness
          ── output task (10 ms) ─> led_output: palette, level, fade, alert
                                     breath, power cap, pulse loss ─> duties in
                                     1/16 tick ─> led_pwm
          ── usb task ─> TinyUSB, link state (unconfigured/configured/suspended)
          ── host task ─> commands, STATE notifications, telemetry, end of an
                          alert, normal touch again after 10 s without a command
          ── console task (debug) ─> text commands, watch
          └─ watchdog feed, then the next round at once (no WFI: it hangs on this chip)

TIM3 update interrupt (20 kHz, every PWM period) ─> dithers the duties
                                     into whole ticks for the next period
```

### Concurrency

The firmware runs bare metal, in one cooperative loop driven by a
table-based scheduler (`lib/disled_core/scheduler`). Interrupt handlers
keep their work short:

| Interrupt | Work |
|---|---|
| SysTick, 1 kHz | millisecond counter, arms the next TouchKey sample |
| TIM3 update, 20 kHz | dithers the three LED duties into whole ticks for the next period |
| TIM3 channel 4 | one TouchKey measurement (about 5 µs), pushed to a lock-free queue |
| USB (three vectors, one handler) | TinyUSB event capture; during suspend, first restores the full clock (EXTI line 18, USB wake-up) |
| HardFault, NMI | saves `mcause`, `mepc`, `mtval` in backup registers, resets |

All interrupts share one priority, so the QingKe hardware stack never
nests.

### Safety rules

- **One path to the LED.** Only `led_output` computes duties, and it
  always applies the current cap of the USB state
  ([behavior.md](behavior.md#usb-states-and-led-power-limits)). The cap
  uses worst-case currents and scales the three channels together, which
  keeps the hue. Custom colors and alert breaths go through it like the
  built-in colors.
- **Shared pins.** The firmware never configures PA10, bonded to USB D−,
  nor PB8, bonded to BOOT0. A static assertion guards the unused-pin masks.
- **No dynamic allocation, no floating point, no 64-bit division.** The
  code uses fixed point everywhere, with static assertions on the
  intermediate ranges. The 64-bit division of libgcc alone would cost
  2 KB of flash.
- **Every wait is bounded.** I2C waits time out after 3 ms. A missing light
  sensor leaves a fixed brightness, and the firmware probes it again every
  5 s. A TouchKey conversion gives up after about 100 µs.
- **Watchdog.** The loop feeds the independent watchdog (1 s) only after
  every task has run. The longest task took 0.5 ms when measured during
  development, 1.7 ms on the suspend clock: well within that second.
- **Faults are reported.** A crash or a failed `DISLED_ASSERT` leaves its
  cause in the backup registers, then the chip resets. A failed assertion
  shows as `mcause` 0x0A55nnnn, nnnn being its source line. The next boot reports
  the fault in `INFO` and in `DIAGNOSTICS`. `DIAGNOSTICS`
  also counts boots, watchdog resets and faults since power-up, and gives
  the smallest stack margin (the boot fills the stack with a pattern) and
  error counters.
- **Crystal failure.** The firmware detects a failed crystal. It then keeps
  USB off, since the clock would be out of tolerance, and still runs the
  lamp locally.
- **The host may vanish.** HID does not tell the lamp when a host stops.
  A lamp left in power-only mode goes back to normal touch after 10 s
  without a command, so that the lid always changes the color again.

### Power

The core never sleeps, because WFI in the main loop hangs on this chip.
While the host sleeps (USB suspend), the firmware stops the LED timer, the
touch and the light sensor, and runs the core at 1 MHz from the internal
oscillator, with the crystal and the PLL off. The USB wake-up interrupt
restores the full clock before TinyUSB handles the resume.

### Memory

32 KB flash, 10 KB RAM. The firmware starts at 0x0 and may use the whole
flash: the ROM bootloader of WCH, which writes it, lives outside it.
`scripts/size_budget.py` prints the usage after each build, warns above
90 % and fails above 100 %. Link-time optimization is on.

| Build | Flash | RAM (2 KB stack included) |
|---|---|---|
| debug (HID + CDC console) | 26.2 KB | 4.9 KB |
| release (HID) | 20.4 KB | 4.2 KB |

### Configuration

Every tunable lives in `src/app/app_config.h` or `src/app/app_config.c`,
with the reason for its value. [bringup.md](bringup.md) checks them on a new board.

### LED output

The PWM runs at 20 kHz with 3600 ticks of 14 ns. Duties are in sixteenths
of a tick: at the bottom of the range a channel lasts only a few ticks, and
whole ticks would make dim fades step and the color drift. The TIM3 update
interrupt spreads the fraction over 16 periods (`pwm_dither`). Each lit
channel also gets a pulse loss added, the light that the driver edges take
from every pulse. The gates of the MOSFETs charge through 100 Ω and switch
in about 100 ns, so a pulse of a few ticks gives no light and the next
ticks give more light than those of a long pulse: the loss follows the
light the pulse should give, along a curve of a few points per channel
(`led_loss_curve_t`). Without it, the 1-tick blue of warm white at 10 %
gives no light and the white turns yellow.

The palette (`palette`) holds the four built-in colors and the custom ones
that the host sets, all in linear light at full brightness; the level then
scales them. The built-in colors are tuned by eye through the PLA. A custom
color comes as a screen shows it, so the lamp first weights its channels by
the white balance of the LED (`color_balance`), the duties of its white,
then brings its largest channel back to full scale. An alert (`alert`)
moves the perceived brightness of the lamp color along a smoothstep, after
the fade, up from a dim lamp and down from a bright one.

### Touch detection

`touch_detector` averages the TouchKey samples over 20 ms, one mains
period, and measures the drop below a slow baseline, in per mille. A press
must reach an absolute threshold **and** rise above the floor, the lowest
value of the last 320 ms. It ends halfway back down, or under the absolute
release threshold. A hand resting near the lid lifts the signal to the
level of a light touch, and only this rise separates the taps on top of it.
The baseline freezes while a hand is near, until it stays for the object
time, so that it cannot drift under the hand and blunt the next touch.

The sensitivity correction of the host multiplies the press, release and
rise thresholds by `(100 − percent) / 100`; a change during a press waits
for its release. During an alert, `touch_gate` keeps feeding the detector
but passes no gesture, nor the release of a press that outlived the alert.

### Testing

- `pio test -e native` runs the core on the build machine (Unity): lamp
  rules, palette, touch cycle and touch modes, touch detection on synthetic
  signals (mains hum, drift, objects, finger at power-up, hand resting near
  the lid, sensitivity), the touch gate, alerts, brightness, color, power
  cap, output fades and pulse loss, PWM dithering, self-test, scheduler,
  formatter and the protocol codec.
- The protocol tests check the codec byte for byte against
  `protocol/vectors/v2.json`, the same file the app tests use.
- `pio check` runs cppcheck on `src/` and the core at every severity.
- [bringup.md](bringup.md) lists the checks on a real board.

## Desktop app

- **Rust backend** (`app/src-tauri`). A worker thread owns the USB
  connection through `hidapi`. It finds the lamp, reconnects after an
  unplug, writes commands and decodes messages. The results reach the web
  view as Tauri events (`device-connection`, `device-message`). The codec
  lives in its own crate, `crates/disled-protocol`, shared with the
  production tool and tested against the shared vectors. After each
  connection the worker sends the settings, the custom colors and the touch
  cycle again ([protocol/README.md](../protocol/README.md#connection)). A
  version 1 lamp gets nothing after its `INFO`: the app locks everything
  that talks to it and offers its update.
- **Colors** (`colors.rs`). The names of the built-in colors, up to five
  custom colors, the touch cycle and the share codes. A deletion picks a
  replacement wherever the color was in use: on the lamp, as the color of
  the calls, in the states of a plugin.
- **Control** (`control.rs`). A pure core decides what the app sets on the
  lamp beyond the choices of the user: the source the lamp follows, the user
  or one integration; the power-only touch and its heartbeat, a command every
  3 s while an integration drives the lamp; the color of a call; the guards
  of the alerts; and the lamp handed back to its own colors, cycle and
  sensitivity when the app quits.
- **Plugins** (`plugins/`). The integrations come from plugins in the
  configuration folder of the app ([plugins.md](plugins.md)): their
  manifests, their approval by the fingerprint of their files, the rules
  that turn their events into states, the local server on
  `127.0.0.1:47831` with one token per plugin, and the programs some of them
  run.
- **Calls** (`calls/`). While "Microphone in use" is on, in manual mode, a
  thread reads whether an app captures the microphone: the capture sessions
  of WASAPI on Windows, the process objects of CoreAudio on macOS 14.2 and
  later, the capture streams of the sound server through libpulse, loaded at
  run time, on Linux. It only reads what the system shows and never opens a
  stream. A debounce turns the readings into calls, 2 s of capture to begin
  one and 5 s without to end it.
- **Tray and windows.** The app lives in the menu bar or the notification
  area, and `tray.rs` draws its icon at run time from the lamp state. Five
  windows share one page, chosen by the window label: the popup for daily
  use, the Diagnostics window, the lamp update window, the window that
  reinstalls a chosen firmware, and the integrations window. The Diagnostics
  window starts the telemetry stream at 250 ms when it opens, and the
  backend stops it when the window closes.
- **Settings.** The backend saves the settings with the store plugin and
  broadcasts them to every window (`settings-changed`). The USB worker
  applies those of the lamp (brightness mode, fixed level, corrections of the
  brightness and of the touch) as soon as the lamp answers `INFO`, so they
  hold across lamp restarts. The backend applies the color mode to the window
  frames and the language to the tray and the window titles (`i18n.rs`), and
  tells the windows the language in force (`locale-changed`).
- **Updates.** A thread checks the `latest.json` of the latest GitHub
  release at start-up and once a day (`updates.rs`, updater plugin). The
  popup shows the result (`update-status`) and asks the backend to install
  it. The first launch of a new version, as after an update, checks at once
  and opens the popup, so that a lamp update shows right after the app's.
- **Lamp updates.** The same thread lists the signed firmware releases
  (`firmware.rs`): every 2.x, newest first, then the last 1.x, which returns
  a lamp to Disled v1. The lamp update window installs the newest version;
  the reinstall window installs the one the user picks. An install runs on
  its own thread: download, signature check against the update key,
  `wchisp probe` every second until the ROM bootloader shows up, `wchisp
  erase`, which empties the whole flash, `wchisp flash`, then the `INFO` of
  the restarted lamp. wchisp is a separate program next to the app, driven
  through its log lines (`crates/disled-flash`, shared with the production
  tool). Both windows follow `firmware-status`.
- **Vue front end** (`app/src`, Nuxt UI). Each window runs its own Pinia
  stores (lamp, settings, colors, control, plugins, telemetry, diagnostics,
  updates, firmware, errors), fed by the backend events through a typed layer
  (`ipc/`). The components only render them and send intentions, such as
  "choose this color" or "follow this integration". ts-rs generates the
  types they exchange with the backend from the Rust types (`bindings/`),
  and the messages of both languages live in `locales/`, shared with the
  backend.
- USB access stays in Rust because the macOS web view has no WebHID, and
  because a connection owned by the backend keeps working while every
  window is hidden.

## Production tool

`tools/disled-factory` is a command-line program for batches of lamps
([production.md](production.md)). Board after board, it waits for the ROM
bootloader, flashes the signed release build with wchisp, checks the
`INFO` of the restarted lamp, runs `SELF_TEST` while the operator watches
the colors and touches the lid, and appends a row to a CSV log. It shares
two crates with the app: the protocol codec and the USB IDs
(`crates/disled-protocol`), and the flashing code, wchisp and the firmware
signatures (`crates/disled-flash`).
