# Changelog

## [2.0.0](https://github.com/Oui-Dev/Disled/compare/fw-v1.0.0...fw-v2.0.0) (2026-11-12)


### ⚠ BREAKING CHANGES

* **firmware:** protocol 2.0 with custom colors, touch cycle, alerts and touch sensitivity

### 🚀 Enhancements

* **firmware:** alert breaths of 1.5 s, chosen on a closed lamp, and their depth in the debug console ([2d2905e](https://github.com/Oui-Dev/Disled/commit/2d2905e6a27daf2c786a42a0aeb390e81fc4a9c6))
* **firmware:** balance the custom colors so that white shows white ([70a8165](https://github.com/Oui-Dev/Disled/commit/70a81652c0be199d080b38866a521c4ed48b0edd))
* **firmware:** ignore the touch during an alert ([d710a16](https://github.com/Oui-Dev/Disled/commit/d710a164c09da7d24d9e0ef52bb827117ff33faa))
* **firmware:** name the lamp Disled in its USB strings ([5ddba1f](https://github.com/Oui-Dev/Disled/commit/5ddba1f30c19c2b124e6db7f385a2b20dba35f82))
* **firmware:** protocol 2.0 with custom colors, touch cycle, alerts and touch sensitivity ([b04ef57](https://github.com/Oui-Dev/Disled/commit/b04ef57326d5a5181580bfb8a9f959537659df02))


### 🩹 Fixes

* **deps:** update the dependencies of the app, the firmware tools and the CI ([1dcd7b1](https://github.com/Oui-Dev/Disled/commit/1dcd7b172665ce324599f7859e45112c9c32797d))
* **firmware:** a pulse loss that follows the length of the pulse ([eec17b3](https://github.com/Oui-Dev/Disled/commit/eec17b38deba01f713e04b1ad211326190b08360))
* **firmware:** a warmer white for the custom colors ([80bed8a](https://github.com/Oui-Dev/Disled/commit/80bed8a777bc2555ef8c8b4b9cc470da2d93125a))
* **firmware:** drive the LED gates with 10 MHz edges ([8124b9b](https://github.com/Oui-Dev/Disled/commit/8124b9b0c4e66c3e6b239429721d661adaba0d73))
* **firmware:** keep the red in dim colors ([c8e4cba](https://github.com/Oui-Dev/Disled/commit/c8e4cba10fe21a5bc7001e8e4781a8ffaf81259e))

## [1.0.0](https://github.com/Oui-Dev/DISLED/compare/fw-v0.3.2...fw-v1.0.0) (2026-10-27)


### 🩹 Fixes

* **firmware:** confirm a large light change again when it turns back ([4524281](https://github.com/Oui-Dev/DISLED/commit/452428173c7db2ada0f8c10bd69811f73090d256))
* **firmware:** count only a self-test touch made during the blue glow ([14975a4](https://github.com/Oui-Dev/DISLED/commit/14975a4ba84396f59aa199791dd3b39b72543d61))
* **firmware:** end a busy wait of a millisecond or more ([5dcd7be](https://github.com/Oui-Dev/DISLED/commit/5dcd7be22859fd0fbeb38c837c4661663a154106))
* **firmware:** keep room for the answers while the host reads nothing ([43a6664](https://github.com/Oui-Dev/DISLED/commit/43a66643dd13d95189937fdadf5daf6cad6bfc0f))
* **firmware:** keep the time from going back when the core clock changes ([a196a2f](https://github.com/Oui-Dev/DISLED/commit/a196a2ff6d553b46955cc280dd6179518827dfae))
* **firmware:** leave the configured state on a USB bus reset ([40921bf](https://github.com/Oui-Dev/DISLED/commit/40921bf604e7855143349b653c8426c67fdf9a11))
* **firmware:** pass the self-test touch on a press, not on a hand near the lid ([c758348](https://github.com/Oui-Dev/DISLED/commit/c75834896c51b173bac9e4fe89f032ba0c4a8a39))
* **firmware:** read a touch sample before freeing its slot to the interrupt ([0fdea55](https://github.com/Oui-Dev/DISLED/commit/0fdea5551252add732e620f6d2ee1747c891b704))
* **firmware:** report a self-test cut short by a USB suspend ([347ee82](https://github.com/Oui-Dev/DISLED/commit/347ee82f006ee419531d1439a4f3251ac5a93a8d))
* **firmware:** start the crystal at boot with bounded waits ([e910f08](https://github.com/Oui-Dev/DISLED/commit/e910f0887de2bffbb6d9829b658baefc8bb963d0))
* **firmware:** stop the telemetry when the host leaves, whatever else changed ([cd03fdc](https://github.com/Oui-Dev/DISLED/commit/cd03fdc7480588d9962cf66a763b4ea80baa0d58))
* **firmware:** suspend at 1 MHz again after the crystal fails on resume ([ff4b41f](https://github.com/Oui-Dev/DISLED/commit/ff4b41fe458e57b55ba76040b9320d9f520a0132))


### 🔥 Performance

* **firmware:** drive the LED timer through its registers ([1894b19](https://github.com/Oui-Dev/DISLED/commit/1894b1915a768756aa9829d8de97ecb81aea51ab))
* **firmware:** leave the console string out of release builds ([2ff6eaa](https://github.com/Oui-Dev/DISLED/commit/2ff6eaae6ba4fc3be2bef4ac96c985c9c3626200))
* **firmware:** power the touch ADC down during USB suspend ([98469c3](https://github.com/Oui-Dev/DISLED/commit/98469c3c2e7ef6a02779923cfa3966d3c160dec9))
* **firmware:** set the pin modes without the SDK's GPIO_Init ([3715390](https://github.com/Oui-Dev/DISLED/commit/3715390ff8ced025b1c16c06487e539d3e753d40))
* **firmware:** set the touch ADC up through its registers ([3f02c0e](https://github.com/Oui-Dev/DISLED/commit/3f02c0e40932911212d0b87a7268060e54c5d941))
* **firmware:** stop timing the tasks ([e68bcdd](https://github.com/Oui-Dev/DISLED/commit/e68bcdd63e801f9c4606d3cebb4878f7db6a6c6c))
* **firmware:** use known clock values and read the reset cause once ([48aaa06](https://github.com/Oui-Dev/DISLED/commit/48aaa0607107514d471672745a29d16d6eec6728))


### 💅 Refactors

* **firmware:** count touch timeouts as lost samples ([6de3832](https://github.com/Oui-Dev/DISLED/commit/6de3832b3a4bd4b497f3bd7cd8694f88cd97ad6e))
* **firmware:** drop a touch delta clamp that never applies ([4305d27](https://github.com/Oui-Dev/DISLED/commit/4305d2787e567f32e8a14a945aea5cef6c586eeb))
* **firmware:** drop the logs that repeat what the host receives ([5c724ab](https://github.com/Oui-Dev/DISLED/commit/5c724ab9a7c23790b629dfb807bc39e1dce8fff8))
* **firmware:** drop the self-test touch_ready input ([b5f5110](https://github.com/Oui-Dev/DISLED/commit/b5f511088cd2ffe3ea5e2e0c35e890db2cd306f1))
* **firmware:** drop the unused %i and %X formats ([843c1e0](https://github.com/Oui-Dev/DISLED/commit/843c1e0f1aa0abb98fcccb849f8b2c007652501b))
* **firmware:** keep only the console commands the app does not offer ([4e77d29](https://github.com/Oui-Dev/DISLED/commit/4e77d293fe6b8ed3caef2af5a6a4110278e8b4be))
* **firmware:** let lamp_fsm_long_press return nothing ([164bc4e](https://github.com/Oui-Dev/DISLED/commit/164bc4ee69ec8a58e2d218f5ff83ec2b2b4ebf1c))
* **firmware:** read the light sensor at a single range ([62480bb](https://github.com/Oui-Dev/DISLED/commit/62480bb8cf7b6c8290dbcff749572d5ba846df03))
* **firmware:** remove the unused TinyUSB millisecond hook ([884d29e](https://github.com/Oui-Dev/DISLED/commit/884d29ee8c4fddd836cd25ff362fb8205b1b02af))
* **firmware:** reset the I2C bus after any failed transfer ([f9c4522](https://github.com/Oui-Dev/DISLED/commit/f9c452201efcbd34f83819200f892e749797318c))
* **firmware:** streamline and harden core components ([b32bfb6](https://github.com/Oui-Dev/DISLED/commit/b32bfb6940878bbf04b1625f040eec808909efb0))
* **firmware:** track the touch baseline zones with one timer ([b45c960](https://github.com/Oui-Dev/DISLED/commit/b45c960d44455d57b06315803dcaf9581a246567))

## [0.3.2](https://github.com/Oui-Dev/DISLED/compare/fw-v0.3.1...fw-v0.3.2) (2026-10-26)


### 💅 Refactors

* **firmware:** remove dead code and derived state from the core ([ceb38db](https://github.com/Oui-Dev/DISLED/commit/ceb38dbf12b381dbd84caed289f5908b99d0dc47))
* **firmware:** remove the tuning commands and the bring-up switches ([9d7231e](https://github.com/Oui-Dev/DISLED/commit/9d7231e6985111e0e5ff685f646b4a036aa947f3))

## [0.3.1](https://github.com/Oui-Dev/DISLED/compare/fw-v0.3.0...fw-v0.3.1) (2026-10-25)


### 🩹 Fixes

* **firmware:** recalibrate touch so a hovering hand is ignored ([04ed44c](https://github.com/Oui-Dev/DISLED/commit/04ed44ca8c7e0974ba93d202bb0a9a5399bb492d))

## [0.3.0](https://github.com/Oui-Dev/DISLED/compare/fw-v0.2.1...fw-v0.3.0) (2026-10-21)


### Features

* **firmware:** automatic brightness that follows the room in about 1.5 s ([e38c88f](https://github.com/Oui-Dev/DISLED/commit/e38c88f5b6492a8302e4f0e016ae0fe0b0c375f4))
* **firmware:** read the light sensor over its whole range ([4d9f8fa](https://github.com/Oui-Dev/DISLED/commit/4d9f8fa44199fe1648507cecd7879120e8772e8c))

## [0.2.1](https://github.com/Oui-Dev/DISLED/compare/fw-v0.2.0...fw-v0.2.1) (2026-10-20)


### Bug Fixes

* **firmware:** build the Intel HEX file that releases and flashing use ([e1980d6](https://github.com/Oui-Dev/DISLED/commit/e1980d696f36383b9741bb9a866a15429e443b12))

## [0.2.0](https://github.com/Oui-Dev/DISLED/compare/fw-v0.1.0...fw-v0.2.0) (2026-10-20)


### Features

* **firmware:** self-test, diagnostics, auto brightness bias, low-power idle ([02056ef](https://github.com/Oui-Dev/DISLED/commit/02056ef723f4658370e2ca886f660d4d2743c07a))
* **protocol:** protocol 1.1 draft and a shared Rust codec ([fe8cd0b](https://github.com/Oui-Dev/DISLED/commit/fe8cd0befb2c218c26578ec54271b1fb7fae4e72))


### Bug Fixes

* **firmware:** no WFI in the main loop, wake-up line only in suspend ([ab86f4e](https://github.com/Oui-Dev/DISLED/commit/ab86f4e651cdbbe398194e74174ac5085f61c75a))
* **firmware:** the self-test touch no longer changes the lamp, warm white step ([75a4799](https://github.com/Oui-Dev/DISLED/commit/75a4799ee4ee53e60360f2864b4735862c512c96))

## 0.1.0 (2026-10-19)


### Features

* **core:** lamp logic, touch detection, light pipeline and protocol codec ([ab5d6a2](https://github.com/Oui-Dev/DISLED/commit/ab5d6a2c416b5f20001ff10c132f09bf87a37d36))
* **firmware:** board support and peripheral drivers ([ec87d6b](https://github.com/Oui-Dev/DISLED/commit/ec87d6bd25280fe8c8192b9a3b3e5e4938e03b24))
* **firmware:** compensate the light lost per PWM pulse ([86e26b4](https://github.com/Oui-Dev/DISLED/commit/86e26b46b96e7c043362f89538240d72b5360fde))
* **firmware:** dither the LED PWM to 1/16 of a tick ([f3b5c55](https://github.com/Oui-Dev/DISLED/commit/f3b5c55f0305aa39c63254cc96ca96abc63429a2))
* **firmware:** pulse loss on red and green too ([b3fba4a](https://github.com/Oui-Dev/DISLED/commit/b3fba4abb0ee7941597f61e99ebd4d86d06359f6))
* **firmware:** shorter touch debounce ([8381c23](https://github.com/Oui-Dev/DISLED/commit/8381c2369f28d7afece859580f13e89d730e0bce))
* **firmware:** touch detection with a hand resting near the lid ([6b30667](https://github.com/Oui-Dev/DISLED/commit/6b30667cef9d912a9816cbf256168ee998f5c623))
* **firmware:** touch thresholds and orange for the closed enclosure ([3543e6d](https://github.com/Oui-Dev/DISLED/commit/3543e6d27106444ef1217fa40010a0f9a675c6f3))
* **firmware:** TouchKey timing measured on the board ([28b6913](https://github.com/Oui-Dev/DISLED/commit/28b6913d52b204747a4f94f914fb52e209cc04c5))
* **firmware:** tune the orange on the board ([1f9afb0](https://github.com/Oui-Dev/DISLED/commit/1f9afb0082d0f6029feffe54511a73d710ed4f55))
* **firmware:** tune warm white and the low brightness end on the board ([aec044b](https://github.com/Oui-Dev/DISLED/commit/aec044bb25a1758446914f71cb0803f4258557c3))
* **firmware:** USB transport, debug console and application MVP ([5628c64](https://github.com/Oui-Dev/DISLED/commit/5628c642fad21e73b69e98479e223d68d45aa029))
* **protocol:** HID protocol v1 specification and shared test vectors ([caf803d](https://github.com/Oui-Dev/DISLED/commit/caf803dc52bf977ceebe19668b43337e814592f1))


### Bug Fixes

* **firmware:** bring-up fixes on the first v4 board ([6a0b991](https://github.com/Oui-Dev/DISLED/commit/6a0b99198ebd4eae93e42c657c0e4f872151ab26))
* **firmware:** one state_seq step per STATE message with new content ([32d6d66](https://github.com/Oui-Dev/DISLED/commit/32d6d66597fd330f584795b871944f1faf73a642))
* **firmware:** skip light readings until the sensor settles ([c63b2a1](https://github.com/Oui-Dev/DISLED/commit/c63b2a1c85f0be7f8c5db7f86234cd26ea9d50c6))
