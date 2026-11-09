import { fileURLToPath } from 'node:url';
import VueI18nPlugin from '@intlify/unplugin-vue-i18n/vite';
import ui from '@nuxt/ui/vite';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vite';

const path = (relative: string) => fileURLToPath(new URL(relative, import.meta.url));

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    ui({
      // Each window picks its view from its label, so there is no router.
      router: false,
      // The components of Nuxt UI and of src/components are auto-imported, a
      // file name starting with its folder: lamp/LampColorPicker.vue is
      // <LampColorPicker>. Functions are imported explicitly.
      autoImport: false,
      components: {
        dts: 'src/components.d.ts',
        directoryAsNamespace: true,
        collapseSamePrefixes: true,
      },
      // Only the themes of the Nuxt UI components in use reach the CSS. They
      // are detected when Vite starts: restart `tauri dev` after using a Nuxt
      // UI component for the first time.
      experimental: { componentDetection: true },
      ui: {
        colors: {
          primary: 'brand',
          // The type lists only the Tailwind neutrals; `ink` is ours
          // (src/assets/css/palettes.css).
          neutral: 'ink' as 'neutral',
        },
        // The alerts, all subtle, keep their color on the icon, the
        // background and the border, and their text in the neutral tokens,
        // because amber or green text on a tinted background is not readable.
        alert: {
          slots: { title: 'text-highlighted', description: 'text-default opacity-100' },
        },
        // The badges likewise keep their color on the background, the ring
        // and the icon, and their text in the neutral tokens.
        badge: { slots: { label: 'text-default' } },
      },
      // The app works offline and its CSP forbids other origins, so the icons
      // it uses are bundled, never fetched.
      icon: { clientBundle: { scan: true } },
    }),
    // The messages are compiled at build time, so a malformed message fails
    // the build and the bundle ships without the message compiler.
    VueI18nPlugin({ include: [path('./src/locales/**')] }),
  ],

  resolve: {
    alias: { '@': path('./src') },
  },

  // `tauri dev` shows the Rust errors in the same terminal and expects the
  // fixed port of tauri.conf.json.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Cargo rebuilds the backend itself.
      ignored: ['**/src-tauri/**'],
    },
  },
});
