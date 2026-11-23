# Changelog

## [3.0.0](https://github.com/Oui-Dev/Disled/compare/app-v2.2.3...app-v3.0.0) (2026-11-23)


### ⚠ BREAKING CHANGES

* **app:** colors as CIE 1931 chromaticities, and the plugin API 2

### 🚀 Enhancements

* **app:** a plugin changes the lamp color 4 times a second at most ([97b71b4](https://github.com/Oui-Dev/Disled/commit/97b71b4cc347b953fa21769fad7c04a04a8c6ec6))
* **app:** colors as CIE 1931 chromaticities, and the plugin API 2 ([52bb42e](https://github.com/Oui-Dev/Disled/commit/52bb42e33c116366e15b60219c9ae78c25b7fdbf))


### 🩹 Fixes

* **app:** the lamp no longer stays on an old color of the plugin it follows ([b4b7b5f](https://github.com/Oui-Dev/Disled/commit/b4b7b5f172e7d570d9fbad56cf0e08c423b0ce50))


### 💅 Refactors

* **app:** one constant paces the lamp and the windows, and the color code drops what never runs ([0f4f318](https://github.com/Oui-Dev/Disled/commit/0f4f31882490350735da65300e930862d6f5d6ec))

## [2.2.3](https://github.com/Oui-Dev/Disled/compare/app-v2.2.2...app-v2.2.3) (2026-11-22)


### 🩹 Fixes

* **app:** on macOS, the popup that the launch shows sits right under the menu bar ([c85d7ca](https://github.com/Oui-Dev/Disled/commit/c85d7ca199b41ea3a96f676f0ad2d95045214bfa))

## [2.2.2](https://github.com/Oui-Dev/Disled/compare/app-v2.2.1...app-v2.2.2) (2026-11-22)


### 🩹 Fixes

* **app:** the launch shows the popup once its page has loaded, after macOS has launched the app ([25baed3](https://github.com/Oui-Dev/Disled/commit/25baed3dab41da2310ef4f42bbf5896944b55375))

## [2.2.1](https://github.com/Oui-Dev/Disled/compare/app-v2.2.0...app-v2.2.1) (2026-11-22)


### 🩹 Fixes

* **app:** on macOS, the launch shows the popup on the screen, so one click closes it ([df62f8b](https://github.com/Oui-Dev/Disled/commit/df62f8b57a2eb852103aa480585555978a7fc8f3))
* **app:** the popup that the launch shows sits above the taskbar ([0668dcf](https://github.com/Oui-Dev/Disled/commit/0668dcf634b2d71b046228e788923411108db43f))

## [2.2.0](https://github.com/Oui-Dev/Disled/compare/app-v2.1.0...app-v2.2.0) (2026-11-21)


### 🚀 Enhancements

* **app:** swatches that match the lamp, and a hue and vividness square instead of the color code ([bf5158c](https://github.com/Oui-Dev/Disled/commit/bf5158c635ed7b53a0512116abb0185068c8c4df))
* **app:** the reinstall window lists the firmware of its own major only ([2a2adae](https://github.com/Oui-Dev/Disled/commit/2a2adae4ac26b961c0e6e116fd58b0ee5f57cc48))


### 🩹 Fixes

* **app:** a file error of a plugin says so instead of 'No plugin.json file' ([bbe3a82](https://github.com/Oui-Dev/Disled/commit/bbe3a82555b54f31213754ab35df3b4925533621))
* **app:** a missing local test firmware stops the update before the erase ([423d7e7](https://github.com/Oui-Dev/Disled/commit/423d7e7438e51daec0c47b7d7f91c4286d63996d))
* **app:** a plugin whose files match its approval again turns back on ([7823895](https://github.com/Oui-Dev/Disled/commit/7823895d13f3373ddf1fe23f76caa52435d9a523))
* **app:** downloads bounded per read, so that a slow link still finishes ([c895f4e](https://github.com/Oui-Dev/Disled/commit/c895f4e80203e7196c58fd1060e912f825814ef2))
* **app:** opening Disled again on macOS shows its popup ([f332bc0](https://github.com/Oui-Dev/Disled/commit/f332bc0cc1092a32b6cd99524c6396c3304a75a5))
* **app:** reading a plugin's note no longer redraws the windows in a loop ([345c42d](https://github.com/Oui-Dev/Disled/commit/345c42d990b5e29f7e7c0ee8c52e337ee71ef34b))
* **app:** the built-in green swatch as saturated as the other colors ([e80ad1a](https://github.com/Oui-Dev/Disled/commit/e80ad1aee7a60694dde4251745b3211126e0e3cb))
* **app:** the default name of a recording in local time ([861744b](https://github.com/Oui-Dev/Disled/commit/861744b1fe21e087d90c443778a8625e5be21b71))
* **app:** the Diagnostics charts no longer pulse at each refresh ([f14690e](https://github.com/Oui-Dev/Disled/commit/f14690efdbbd685c3c27d5937b1fb07e796e00c2))
* **app:** the last error announced to screen readers in every window ([b3990f5](https://github.com/Oui-Dev/Disled/commit/b3990f5f49f86c9f214bf4ff2016c8f62f2bf835))
* **app:** the popup that the launch shows stays open ([3613887](https://github.com/Oui-Dev/Disled/commit/3613887367e8828e961eab7d83e83dc7868defc8))
* **app:** the scroll box of the versions no longer cuts the focus outline of a row ([3eb343e](https://github.com/Oui-Dev/Disled/commit/3eb343e745bf391976ad2830b287644bdccdcffc))


### 💅 Refactors

* **app:** a state_seq gap no longer asks the lamp again ([00e3759](https://github.com/Oui-Dev/Disled/commit/00e375946181e6ec69280c6de1b24edc634dc1f2))
* **app:** backend items without a second user ([3452f95](https://github.com/Oui-Dev/Disled/commit/3452f959e016afcc14990895b66d14904c286bd5))
* **app:** color tiles that take their hex from the store ([11846e6](https://github.com/Oui-Dev/Disled/commit/11846e6ac67cdc93166fc8e41d418e857020b8d8))
* **app:** Nuxt UI themes and components in place of copied classes ([169e4c8](https://github.com/Oui-Dev/Disled/commit/169e4c86ed62ecdad2be560eef6b5f8afb881c6d))
* **app:** one error helper and one event mirror for the stores ([fb60e2a](https://github.com/Oui-Dev/Disled/commit/fb60e2a2f9cc2dbe0f344a9fe75a565b6409ba73))
* **app:** one install command for the lamp firmware ([029d987](https://github.com/Oui-Dev/Disled/commit/029d987acddccfa72d2ed4979693d3fe55b7aa3b))
* **app:** one opener for the windows, and the popup built in Rust ([236cbaa](https://github.com/Oui-Dev/Disled/commit/236cbaa9d9936c2d83fbb4eed9f57f63dc1f74e2))
* **app:** one picture of the lamp in the device worker, which tells the windows its generation ([3a7bf1f](https://github.com/Oui-Dev/Disled/commit/3a7bf1fe0f3040570b15ee4a2059586bcfa284f3))
* **app:** one reader and one writer of the color codes ([5cd3052](https://github.com/Oui-Dev/Disled/commit/5cd30525098d8a3d13fc1d10cf3a4aa6ccaeaf6f))
* **app:** parking_lot locks, and the macOS version check through objc2 ([28365de](https://github.com/Oui-Dev/Disled/commit/28365de9579adbccde30abb3004fb64fef4b39d8))
* **app:** plugin helpers defined once, a catalog install from memory with its network error in full, and a host that copies less ([86eb561](https://github.com/Oui-Dev/Disled/commit/86eb561cbd6fd9d233c91327ba45ec2703e80df3))
* **app:** shared pieces for the sliders, the confirmations and the capability lists ([aced213](https://github.com/Oui-Dev/Disled/commit/aced2137b7120dbcda72b130503a5433995efa76))
* **app:** the check of signed files, the wait for the bootloader and the erase in disled-flash ([a54e6a5](https://github.com/Oui-Dev/Disled/commit/a54e6a57531c760db7ca679251db4ef1992794d9))
* **app:** the color square through Reka UI's color functions, with one update period for the controls that move ([81d526d](https://github.com/Oui-Dev/Disled/commit/81d526d791f41358ed75f97876e18bd279dae27f))
* **app:** the control's lamp commands through two helpers ([00f34b7](https://github.com/Oui-Dev/Disled/commit/00f34b7a38de729ee06e69f2975e2f03a25d4c2f))
* **app:** the integrations settings save the manual mode as null ([387e2ab](https://github.com/Oui-Dev/Disled/commit/387e2abff66ac4206c4bdec08828d686384820a8))
* **app:** the integrations window reads the plugin folder once as it opens ([f290023](https://github.com/Oui-Dev/Disled/commit/f2900237cd89d34e1ede537c9df07d645802db8c))
* **app:** the plugin host's rules testable without a running app ([626939f](https://github.com/Oui-Dev/Disled/commit/626939fe9fc7e441f8bc3d112c8ca6b3fd95f92e))
* **app:** the telemetry CSV columns in one table ([431ebb6](https://github.com/Oui-Dev/Disled/commit/431ebb6845aee7cf9bef41a46ef29dcefda2da5a))
* **app:** the update windows share their steps and footer ([b78d319](https://github.com/Oui-Dev/Disled/commit/b78d319e55795cf436eb835ef8afca9d1af8defe))
* **app:** the version picker as one list ([2ae368b](https://github.com/Oui-Dev/Disled/commit/2ae368b29262afb49a5347cac35ba3188c53ae01))
* **app:** the windows write the clipboard themselves ([aab0e8a](https://github.com/Oui-Dev/Disled/commit/aab0e8ab09d3158b4fc7b7633cadc5dd9baaf7be))
* **app:** tile names with a fixed margin ([c217e19](https://github.com/Oui-Dev/Disled/commit/c217e19d63096eebfba32fdd2379f00ee419e72c))
* **app:** VueUse in place of hand-made timers and loaders ([fc01103](https://github.com/Oui-Dev/Disled/commit/fc01103ffef2b1addfed7728e1f3f297282564fe))
* **app:** window code without its unused exports and defaults ([3b3be62](https://github.com/Oui-Dev/Disled/commit/3b3be620bad0e3427136fecf9c8258b49e68b2e0))
* **protocol:** INFO and the codec without what no host uses, and reset causes 5 and 6 reserved ([1068ae7](https://github.com/Oui-Dev/Disled/commit/1068ae7adbf2f13e43bee3c975d651abc2e6d5f9))
* **protocol:** the HID report framing in the codec ([3b4776e](https://github.com/Oui-Dev/Disled/commit/3b4776ede85071147f7c62276b566b48614cf797))

## [2.1.0](https://github.com/Oui-Dev/Disled/compare/app-v2.0.1...app-v2.1.0) (2026-11-20)


### 🚀 Enhancements

* **app:** the programs of the plugins read the lamp and its presses, and the integrations window installs the plugins of the catalog ([faa3094](https://github.com/Oui-Dev/Disled/commit/faa3094fcc34b7402a55457dc3c316e3e6583887))
* **firmware:** the lamp tells the host of every short press, with the TOUCH message of protocol 2.1 ([9601b23](https://github.com/Oui-Dev/Disled/commit/9601b23e088d6d45d0036ebf7b29e4480255ed14))


### 🩹 Fixes

* **deps:** update the dependencies of the app, the firmware tools and the CI ([35996f2](https://github.com/Oui-Dev/Disled/commit/35996f2b11076d03b39fc7a6bda61b45548434ce))

## [2.0.1](https://github.com/Oui-Dev/Disled/compare/app-v2.0.0...app-v2.0.1) (2026-11-19)


### 🩹 Fixes

* **app:** the lamp handed back to its own behavior when macOS ends the app, on Cmd+Q or at logout ([54a34b2](https://github.com/Oui-Dev/Disled/commit/54a34b29404ba4bf4e26e5f638729e02f854c5a0))
* **app:** the macOS popup under its icon from the first click, even after the app was hidden ([19f7ada](https://github.com/Oui-Dev/Disled/commit/19f7adac4a026f92a9246b9a59f5123148c27cb9))

## [2.0.0](https://github.com/Oui-Dev/Disled/compare/app-v1.1.2...app-v2.0.0) (2026-11-12)


### ⚠ BREAKING CHANGES

* **app:** drive protocol 2.0 lamps and lock version 1 lamps

### 🚀 Enhancements

* **app:** a color chosen in Diagnostics takes the lamp back from an integration ([db39b1b](https://github.com/Oui-Dev/Disled/commit/db39b1bc7ba4124e04715af8d180c3b179129b11))
* **app:** alerts of three breaths of 1.5 s ([342e7c0](https://github.com/Oui-Dev/Disled/commit/342e7c05c5c45a93943960cc05f3006e6d083e9d))
* **app:** button and menu labels without an ellipsis, and guards on the texts ([0d8164e](https://github.com/Oui-Dev/Disled/commit/0d8164e352bf8244000c8020b6908d5b16808ca4))
* **app:** calls told by the microphone in use, on Windows, macOS and Linux, with libpulse loaded at run time ([950c990](https://github.com/Oui-Dev/Disled/commit/950c9901f1cb819ee6759055f8336a9e81ce17e9))
* **app:** custom colors in the popup: six tiles, library and touch cycle, editors, sharing ([8d321d7](https://github.com/Oui-Dev/Disled/commit/8d321d776fd1aae1c90cb8f26b2099219cf21eaf))
* **app:** custom colors, touch cycle, deletion fallbacks and share codes in the backend ([fa5d5c3](https://github.com/Oui-Dev/Disled/commit/fa5d5c312c5811e1dda92f6567c1cb9d69e6922b))
* **app:** Diagnostics for protocol 2.0, closed to a lamp the app does not drive ([90b90ad](https://github.com/Oui-Dev/Disled/commit/90b90ad877eb44b2aecc29a3806a1066ffe9b1ee))
* **app:** drive protocol 2.0 lamps and lock version 1 lamps ([69b5298](https://github.com/Oui-Dev/Disled/commit/69b52989b9b651c42d4fa15b54f32196357e40df))
* **app:** every firmware 2.x and 1.0.0 on offer, the install of a chosen version, the whole flash erased first ([acdecb0](https://github.com/Oui-Dev/Disled/commit/acdecb0ab85cc8c697ef3bb8765a4386a1c07704))
* **app:** name the app Disled, at the new address of the repository ([f90f809](https://github.com/Oui-Dev/Disled/commit/f90f809ad27447564bb4e522c6adc4deba4d4e68))
* **app:** pass the color choices of the popup to the backend with lamp_choose ([6b4d6d1](https://github.com/Oui-Dev/Disled/commit/6b4d6d18a4c96aa9035b3f6dd5b046a527d8f8fb))
* **app:** show the lock of a version 1 lamp, with the update it needs ([f5ff9dd](https://github.com/Oui-Dev/Disled/commit/f5ff9dd383085dec0afbd67c32a55613a9f5116e))
* **app:** the control of the lamp, with its heartbeat, its alert guards and one way out that hands the lamp back ([641ab4e](https://github.com/Oui-Dev/Disled/commit/641ab4e421d6f21d6cc5c1e3d646d10a11e3af27))
* **app:** the integrations window, its approval question, and the plugins in the mode menu ([a8c47a1](https://github.com/Oui-Dev/Disled/commit/a8c47a13586a8076496494fa28ab7a518f704301))
* **app:** the line of the calls under the tiles, with the color of the calls ([4f597b0](https://github.com/Oui-Dev/Disled/commit/4f597b058ecbb700cf68b447e90810adaa6f146c))
* **app:** the mode menu of the lamp and the row of an integration ([13828d4](https://github.com/Oui-Dev/Disled/commit/13828d40fce10798c3db09991927c473e47e49a9))
* **app:** the plugin host: manifests, rules and states by key, fingerprints and approvals, the local server and its tokens, the programs ([78d5f5b](https://github.com/Oui-Dev/Disled/commit/78d5f5bcc3e0a841e62cce11545d2d3b49510a5e))
* **app:** the reinstall window and its version list, and the guided update fit for a version 1 lamp ([fe7701f](https://github.com/Oui-Dev/Disled/commit/fe7701f4c5250a78c1035e5a736e31a45c50c52a))
* **app:** touch sensitivity setting, true minus signs and the vertical settings menu ([f4d02b7](https://github.com/Oui-Dev/Disled/commit/f4d02b7b81eeec42bb615f595c168837c3e2b15e))
* **app:** widen the popup to 340 px ([a7c2293](https://github.com/Oui-Dev/Disled/commit/a7c2293644325b6f7d04492bfe8d3dd533b629a6))


### 🩹 Fixes

* **app:** a remove link for every plugin, and a red or orange dot for a stopped program ([7e012a4](https://github.com/Oui-Dev/Disled/commit/7e012a437e245dedaa7fd033e3433a6803a15044))
* **app:** English texts without the turns of phrase of the French ([6c8a14c](https://github.com/Oui-Dev/Disled/commit/6c8a14cff7281d62964514e9d0e9b8ced23e8b18))
* **app:** radios named once, in the current language, and arrows that browse the colors of a popover ([3e50891](https://github.com/Oui-Dev/Disled/commit/3e508919df63431d22cfe0927d6e36d6db0458d4))
* **app:** the arrows browse the suggestions of the color editor ([dead161](https://github.com/Oui-Dev/Disled/commit/dead161fc01d54f3359b7c439f2b7e919b96a6aa))
* **app:** the card of an integration level with the Off tile ([c9d5739](https://github.com/Oui-Dev/Disled/commit/c9d573924541717a436f6f11725251359a4b1f95))
* **app:** the left column of Diagnostics as wide as the popup, so that the tile names fit as there ([1e5a614](https://github.com/Oui-Dev/Disled/commit/1e5a61451a5d4d1a9bc1a0531f8276381362656a))
* **app:** the not-allowed cursor on the colors of a lamp the app does not drive ([582585f](https://github.com/Oui-Dev/Disled/commit/582585f76a38ccd9dcc3fa16ad8580d019168620))
* **app:** the pointer cursor on the enabled buttons, which Tailwind 4 no longer gives ([714869a](https://github.com/Oui-Dev/Disled/commit/714869a2445161edbf405ac126116bab13d89652))
* **app:** ticks of the light chart that a dim room no longer shows twice ([c312c6e](https://github.com/Oui-Dev/Disled/commit/c312c6ecf3f219a7d9038c4e9159525bf17dec11))
* **deps:** update the dependencies of the app, the firmware tools and the CI ([1dcd7b1](https://github.com/Oui-Dev/Disled/commit/1dcd7b172665ce324599f7859e45112c9c32797d))


### 💅 Refactors

* **app:** no pointer class on the buttons, which get it from the base style ([74af02a](https://github.com/Oui-Dev/Disled/commit/74af02a1d36daba6b6b77310d34b11e5d6cbb111))
* **app:** Nuxt UI buttons instead of native ones ([48ab010](https://github.com/Oui-Dev/Disled/commit/48ab01093d19455dccbcdb1bf87e27d2af8894b9))
* **app:** Nuxt UI components in place of hand-made markup ([ff4b8fb](https://github.com/Oui-Dev/Disled/commit/ff4b8fbf6f98f882ecd0ffff7a432a2c8146335c))
* **app:** Tailwind scale values in place of arbitrary ones ([8de3ef4](https://github.com/Oui-Dev/Disled/commit/8de3ef4b540f30754374928a99aa1f0cb34f5e9f))

## [1.1.2](https://github.com/Oui-Dev/DISLED/compare/app-v1.1.1...app-v1.1.2) (2026-10-30)


### 🩹 Fixes

* **app:** adress dependencies vulnerabilities ([0292c3c](https://github.com/Oui-Dev/DISLED/commit/0292c3cfc725742d76b56fc6994569869991ca8b))
* **app:** make popup a macOS panel to prevent issue on full screen apps ([77d6591](https://github.com/Oui-Dev/DISLED/commit/77d65917361d230dafdc647b6878fab6fbf324a0))

## [1.1.1](https://github.com/Oui-Dev/DISLED/compare/app-v1.1.0...app-v1.1.1) (2026-10-27)


### 🩹 Fixes

* **app:** apply the settings at the first INFO of a connection only ([4669424](https://github.com/Oui-Dev/DISLED/commit/4669424d65c432081e866fffc8001ede538506b8))
* **app:** cancel the lamp update when its window closes ([5a97cf8](https://github.com/Oui-Dev/DISLED/commit/5a97cf843233a334fe37fa9eddfd6e057bb59238))
* **app:** grant the devtools shortcut to debug builds only and tighten the CSP ([6e3397c](https://github.com/Oui-Dev/DISLED/commit/6e3397c8d0b6eea87a35157199ff584f38d6d287))
* **app:** hide the popup instead of closing it on Alt+F4 ([f6c531e](https://github.com/Oui-Dev/DISLED/commit/f6c531ed2f8eafb09a00a5542a7038c21488fe68))
* **app:** keep a saved setting from failing when the lamp leaves ([ec63c53](https://github.com/Oui-Dev/DISLED/commit/ec63c537136bbd2851182c4f05316151ea5e11d4))
* **app:** keep backend values from being overwritten by older command answers ([e866b20](https://github.com/Oui-Dev/DISLED/commit/e866b2063b74d6d688e4d71a323ba6b91200c14b))
* **app:** let a second install click join the running lamp update ([7307b9f](https://github.com/Oui-Dev/DISLED/commit/7307b9f6b18cf20ec312551103d1f0a46077716b))
* **app:** open Diagnostics from the tray menu off the event loop ([ef7f5b7](https://github.com/Oui-Dev/DISLED/commit/ef7f5b7f42d564076b0416434ee8728eda8e8a53))
* **app:** run one update check at a time ([2f1a4c0](https://github.com/Oui-Dev/DISLED/commit/2f1a4c01dd318efd28350da04cad85000d3aa3ad))
* **app:** show when the telemetry stream is off with figures on show ([e5ecce7](https://github.com/Oui-Dev/DISLED/commit/e5ecce757aee8dc1c97d454fc7960cda2390c1c6))
* **app:** time out stalled update requests ([fd16a77](https://github.com/Oui-Dev/DISLED/commit/fd16a779f39b0d88003d60f322fae9dff22324d8))


### 🔥 Performance

* **app:** format each sample's uptime once and pause hidden charts ([902cd6f](https://github.com/Oui-Dev/DISLED/commit/902cd6f62f783a47311478f459104c6dac8322f2))
* **app:** save CSV recordings off the event loop ([e153bbb](https://github.com/Oui-Dev/DISLED/commit/e153bbba293af7f6829bf469dfc96a7829dc9f7c))
* **app:** send the telemetry to the Diagnostics window only ([511bd92](https://github.com/Oui-Dev/DISLED/commit/511bd92e88a9a358bf6e7d44b5119a63ad671497))
* **app:** write the settings file once the changes settle ([ba940c5](https://github.com/Oui-Dev/DISLED/commit/ba940c52409074f0098dc1f8cdc4feaeadab6501))

## [1.1.0](https://github.com/Oui-Dev/DISLED/compare/app-v1.0.1...app-v1.1.0) (2026-10-26)


### 🚀 Enhancements

* **app:** check for updates at once after an app update ([7cb2763](https://github.com/Oui-Dev/DISLED/commit/7cb27632c2bdfce08fb7c09bc96a3b0fd3f3065b))
* **app:** mark the settings button when an update is on offer ([92b761b](https://github.com/Oui-Dev/DISLED/commit/92b761bc9ff2a38464ccf9d6a77a4a24a8148f93))
* **app:** show the lamp update along with the app update ([5d8b4a8](https://github.com/Oui-Dev/DISLED/commit/5d8b4a8649fb295a81bbf42fbc13a9983958de14))


### 🩹 Fixes

* **app:** show the first-launch steps in the macOS disk image window ([6d6cb22](https://github.com/Oui-Dev/DISLED/commit/6d6cb224ab30a485f3f92990f4b3fda35ecf9e3f)), closes [#36](https://github.com/Oui-Dev/DISLED/issues/36)

## [1.0.1](https://github.com/Oui-Dev/DISLED/compare/app-v1.0.0...app-v1.0.1) (2026-10-26)


### 🩹 Fixes

* **app:** grant each window only the permissions it uses ([ded5ebd](https://github.com/Oui-Dev/DISLED/commit/ded5ebd24bd482ae173c17a723cf9b3f3ff601d5))


### 🔥 Performance

* **app:** redraw the tray icon only when the lamp look changes ([8caab6b](https://github.com/Oui-Dev/DISLED/commit/8caab6b72819081cbeb1f8c9d7327b2a7acb2ba2))


### 💅 Refactors

* **app:** concrete AppHandle, and no bootloader by command ([2cea903](https://github.com/Oui-Dev/DISLED/commit/2cea903ba387b884f4626d3db9beaac3f4e1f9dc))
* **app:** single-use modules folded in, and the offer computed once ([22c0501](https://github.com/Oui-Dev/DISLED/commit/22c050147ceffb609db9c99bc6fbff7c51cb9438))

## [1.0.0](https://github.com/Oui-Dev/DISLED/compare/app-v0.5.1...app-v1.0.0) (2026-10-25)


### 🚀 Enhancements

* **app:** brightness on one slider, as on a phone ([74d8f0b](https://github.com/Oui-Dev/DISLED/commit/74d8f0b25693a72ee9cfeefd315aedbea99c3471))
* **app:** color mode and language settings, translated tray and window titles ([5401de9](https://github.com/Oui-Dev/DISLED/commit/5401de98ee5863df8d5ae7b9ae5028694d979086))
* **app:** diagnostics window with tabs on the left, telemetry on the right ([c4805ed](https://github.com/Oui-Dev/DISLED/commit/c4805edda3f0295b91ebae54333d458e613d9851))
* **app:** popup rebuilt with Nuxt UI and sized to its content ([ac52053](https://github.com/Oui-Dev/DISLED/commit/ac52053ba07639508ef9a27e2d95eabc12c642e7))
* **app:** popup settings with the rare actions in a "more" menu ([ded65fd](https://github.com/Oui-Dev/DISLED/commit/ded65fdf18dc9f1750aff39cb334f38771d8ca0f))
* **app:** the brightness slider sets the correction in automatic mode ([b525caa](https://github.com/Oui-Dev/DISLED/commit/b525caa0343bc48b713a8bb2caed575c85bf169e))


### 🩹 Fixes

* **app:** keep the last error on show while the app polls the lamp ([800018c](https://github.com/Oui-Dev/DISLED/commit/800018ce1148bb2c0afccb997435d743f05fc07a))
* **app:** no scrollbar flash while the popup grows ([e47819c](https://github.com/Oui-Dev/DISLED/commit/e47819c4c5439ee9c8cf0445f781236eee0730ad))
* **app:** reinstall the lamp firmware from the settings menu at any time ([2fc9d86](https://github.com/Oui-Dev/DISLED/commit/2fc9d86c051b8350ba963eecb0c08c1a0bdeb206))
* **app:** stop the lamp telemetry when the Diagnostics window closes ([4ca422e](https://github.com/Oui-Dev/DISLED/commit/4ca422e50958eab102594fd31c10e5699937f756))
* **app:** the brightness slider stays put when the mode changes ([4bfdb20](https://github.com/Oui-Dev/DISLED/commit/4bfdb20b1ebf554779630f4892ae494493bc02cc))
* **app:** the popup opens again on the lamp page ([4d4cec3](https://github.com/Oui-Dev/DISLED/commit/4d4cec38db6e4dae097cb8ccde5574b51bb0374a))
* **hid:** Limit device scan to lamp vendor to prevent stalls ([4531fa4](https://github.com/Oui-Dev/DISLED/commit/4531fa43ef9c899a956127841a64f527e22510d1))


### 💅 Refactors

* **app:** accessibility fixes from an axe audit ([298beb8](https://github.com/Oui-Dev/DISLED/commit/298beb8a2b0e4de47bb04822bb9eb52934738d06))
* **app:** diagnostics window and charts with Nuxt UI ([e87f0be](https://github.com/Oui-Dev/DISLED/commit/e87f0be0d6929e2ff3f59523517dbc19b5686c97))
* **app:** fainter tint for the neutral palette, grey off swatch ([9013f7a](https://github.com/Oui-Dev/DISLED/commit/9013f7adb55364fa9e014c90eca3ca07c6c0a182))
* **app:** improve CI, documentation, and app clarity ([1cbe034](https://github.com/Oui-Dev/DISLED/commit/1cbe0342aa6beb3162097a0a6e8ce847be90b943))
* **app:** keep module helpers private ([e6c15a4](https://github.com/Oui-Dev/DISLED/commit/e6c15a4ad411e9dad739d4ec19b26bd1d6664a8a))
* **app:** lamp update window with Nuxt UI ([5d35178](https://github.com/Oui-Dev/DISLED/commit/5d3517881bb390a198f28ee23586f94f30bb708e))
* **app:** Pinia stores over a typed IPC layer ([ebc4344](https://github.com/Oui-Dev/DISLED/commit/ebc4344606c3740af82a3c12ef0f939e81e02aa6))
* **app:** popup footer at the bottom and smaller, compact alert, cards for the settings ([123a0b9](https://github.com/Oui-Dev/DISLED/commit/123a0b96a8e9f82374c70c3ec92b68c6fe9de2d7))
* **app:** smaller popup controls, a 360 px popup, tidier diagnostics header ([5d5565e](https://github.com/Oui-Dev/DISLED/commit/5d5565eb85e3e322cba15b18e28ad3a3a9380c1e))
* **app:** telemetry at 250 ms, recording at the bottom, a legend per chart ([53f0f6d](https://github.com/Oui-Dev/DISLED/commit/53f0f6d31b133e99c73fd79e6cfd7e74f0f1d768))
* **app:** TypeScript types generated from the Rust types ([3c6192b](https://github.com/Oui-Dev/DISLED/commit/3c6192b415d6458805215cae452245cd854bcef4))
* global cleanup & some fix ([5c8ce22](https://github.com/Oui-Dev/DISLED/commit/5c8ce22ec7076839bf598693b397fc92db1bbf03))

## [0.5.1](https://github.com/Oui-Dev/DISLED/compare/app-v0.5.0...app-v0.5.1) (2026-10-24)


### Bug Fixes

* **app:** rolldown to tauri 2.11 to prevent update issue ([b3a22af](https://github.com/Oui-Dev/DISLED/commit/b3a22afc5d4497cc8beb9538678c99f2aa908247))

## [0.5.0](https://github.com/Oui-Dev/DISLED/compare/app-v0.4.0...app-v0.5.0) (2026-10-24)


### Features

* **app:** logo of the lamp on the app and its installers ([95b1244](https://github.com/Oui-Dev/DISLED/commit/95b124405ff349f470dd356b6d9a145634246b81))
* **app:** rounded squares for the tray icon and swatches, and an off swatch ([64b92f5](https://github.com/Oui-Dev/DISLED/commit/64b92f58dba1b57e5b277a918e793dc490082d31))


### Bug Fixes

* **app:** ensure exported CSV ends with newline ([6650786](https://github.com/Oui-Dev/DISLED/commit/6650786ca43e8b4960111b30931f79798ba36723))
* **app:** sign the macOS app ad hoc so that it opens on Apple Silicon ([74ba1b4](https://github.com/Oui-Dev/DISLED/commit/74ba1b47e7bb89746912503c918719d593f1a4d4)), closes [#19](https://github.com/Oui-Dev/DISLED/issues/19)

## [0.4.0](https://github.com/Oui-Dev/DISLED/compare/app-v0.3.0...app-v0.4.0) (2026-10-21)


### Features

* **app:** update the lamp firmware from the app ([4036ce7](https://github.com/Oui-Dev/DISLED/commit/4036ce7319b2b1e73ed96e3bc98e5aa9bfaca32a))


### Bug Fixes

* **deps:** bump the production group across 1 directory with 3 updates ([4746a45](https://github.com/Oui-Dev/DISLED/commit/4746a45daea3a3cbeddf4b569e0fdfc1f544c6a9))

## [0.3.0](https://github.com/Oui-Dev/DISLED/compare/app-v0.2.0...app-v0.3.0) (2026-10-20)


### Features

* **app:** live in the tray with a popup, settings applied at every connection ([45294be](https://github.com/Oui-Dev/DISLED/commit/45294bec26cbddc953472cca6c18ad5726dcab03))
* **app:** updates from GitHub Releases, offered in the popup ([1c16054](https://github.com/Oui-Dev/DISLED/commit/1c160546f981723c3c8b34a3b4c4cf1afa298ba2))

## [0.2.0](https://github.com/Oui-Dev/DISLED/compare/app-v0.1.0...app-v0.2.0) (2026-10-20)


### Features

* **protocol:** protocol 1.1 draft and a shared Rust codec ([fe8cd0b](https://github.com/Oui-Dev/DISLED/commit/fe8cd0befb2c218c26578ec54271b1fb7fae4e72))


### Bug Fixes

* **app:** let the logged-in user reach the lamp on Linux ([5eabaf4](https://github.com/Oui-Dev/DISLED/commit/5eabaf42945191df8ddf31852413a51e98097edf))

## 0.1.0 (2026-10-19)


### Features

* **app:** Tauri 2 test and tuning app ([ce35758](https://github.com/Oui-Dev/DISLED/commit/ce357583239df80283a08d9e839a23703b87d173))
