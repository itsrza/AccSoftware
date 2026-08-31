/**
 * @vitest-environment node
 *
 * ممیزی دور ۱۴ — مجموعه‌ی سخت‌گیرانه‌ی تجاری‌سازی (۱۱۳ تست) + ممیزی پایتونی.
 *
 * این پرونده همان دو ابزار مستقل را که در `scripts/` و `tools/` زندگی
 * می‌کنند، در CI اجرا می‌کند تا تک‌منبع بمانند؛ اینجا فقط ضمانت اجرا و
 * خروجی صفر است. اجرای پایتون فقط جایی که مفسر موجود است الزامی است
 * (رانر لینوکسی CI دارد؛ ویندوزِ توسعه‌دهنده ممکن است نداشته باشد).
 */
import { describe, expect, it } from 'vitest'
import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { resolve } from 'node:path'

const ROOT = resolve(__dirname, '../../../..')

describe('م۱۴ · مجموعه‌ی سخت‌گیرانه‌ی تجاری‌سازی', () => {
  it('س۱۴ — ۱۱۴ تست سخت‌گرایی تجاری همه پاس‌اند', () => {
    const run = spawnSync('node', ['scripts/commercial-hardening-tests.mjs'], {
      cwd: ROOT,
      encoding: 'utf8',
      timeout: 120_000,
    })
    expect(run.status, run.stdout + run.stderr).toBe(0)
    expect(run.stdout).toContain('114 passed, 0 failed')
  })

  it('س۱۵ — ممیزی پایتونی معماری/یکپارچگی/امنیت پاس است', () => {
    const python = existsSync('/usr/bin/python3') ? 'python3' : 'python'
    const probe = spawnSync(python, ['--version'], {encoding: 'utf8'})
    if (probe.status !== 0) {
      // بدون مفسر، این ممیوی محلی قابل اجرا نیست؛ نسخه‌ی node الزامی است.
      return
    }
    const run = spawnSync(python, ['tools/hardening_audit.py'], {
      cwd: ROOT,
      encoding: 'utf8',
      timeout: 60_000,
    })
    expect(run.status, run.stdout + run.stderr).toBe(0)
    expect(run.stdout).toContain('PASS')
  })
})
