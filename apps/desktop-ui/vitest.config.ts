import {defineConfig} from 'vitest/config'
import react from '@vitejs/plugin-react'

/**
 * پیکربندی تست‌ها.
 *
 * محیط پیش‌فرض `node` است تا تست‌های منطقی سریع بمانند؛ فایل‌هایی که به DOM
 * نیاز دارند با `@vitest-environment jsdom` در بالای فایل آن را می‌گیرند.
 */
export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'node',
  },
})
