/**
 * @vitest-environment node
 *
 * ممیزی دور ۱۶ — دو شکاف باقی‌مانده‌ی پرامپت‌های مرجع:
 *  ۱۱-ز نمایشگر لاگ عملکرد کاربران (audit_logs + UI)
 *  ۱۱-ح اتصال UI به دستورهای موجود پشتیبان‌گیری
 */
import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const ROOT = resolve(__dirname, '../../../..')
const read = (path: string) => readFileSync(resolve(ROOT, path), 'utf8')

describe('م۱۶ · لاگ عملکرد و پشتیبان‌گیری', () => {
  it('ل۱ — دستور list_audit_logs با مجوز و فیلترهای پارامتری', () => {
    const main = read('apps/desktop-host/src-tauri/src/main.rs')
    expect(main).toContain('fn list_audit_logs(')
    expect(main).toContain('list_audit_logs,')
    expect(main).toContain('"system.audit.view"')
    // ورودی کاربر در متن SQL نمی‌نشیند — شرط‌ها پارامتری‌اند
    const body = main.slice(main.indexOf('fn list_audit_logs('))
    expect(body).toContain('params_from_iter')
    expect(body).toContain('.clamp(1, 1000)')
  })

  it('ل۲ — صفحه‌ی لاگ وجود دارد و به منو/مسیر وصل است', () => {
    expect(read('apps/desktop-ui/src/pages/AuditLog.tsx')).toContain('getAuditLogs')
    const app = read('apps/desktop-ui/src/App.tsx')
    expect(app).toContain("import {AuditLog}")
    expect(app).toContain("case 'audit-log':")
    expect(app).toContain("page: 'audit-log'")
    const api = read('apps/desktop-ui/src/api.ts')
    expect(api).toContain("'list_audit_logs'")
  })

  it('ل۳ — کلیدهای لاگ در سه زبان', () => {
    for (const file of ['fa.ts', 'en.ts', 'ar.ts']) {
      const dict = read(`apps/desktop-ui/src/lib/i18n/${file}`)
      for (const key of ['page.audit-log', 'auditLog.col.user', 'auditLog.entity.invoice', 'auditLog.empty']) {
        expect(dict, `${file}: ${key}`).toContain(`'${key}'`)
      }
    }
  })

  it('ل۴ — پاسخ پیش‌نمایش برای هر دو دستور جدید/متصل', () => {
    const preview = read('apps/desktop-ui/src/lib/devPreview.ts')
    expect(preview).toContain('list_audit_logs:')
    expect(preview).toContain('list_backups:')
    expect(preview).toContain('backup_database:')
    expect(preview).toContain('restore_database:')
  })

  it('ل۵ — پنل پشتیبان‌گیری به دستورهای واقعی وصل است (نه UI ساختگی)', () => {
    const settings = read('apps/desktop-ui/src/components/SettingsCenter.tsx')
    expect(settings).toContain('BackupPanel')
    expect(settings).toContain('backupDatabase')
    expect(settings).toContain('restoreDatabase')
    expect(settings).toContain('verifyBackupFile')
    // بازگردانی مخرب دو‌مرحله‌ای تأیید می‌خواهد
    expect(settings).toContain('backupConfirm')
  })
})
