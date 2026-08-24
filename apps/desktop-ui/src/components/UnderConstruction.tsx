import {Icon} from './Icon'
import {useI18n} from '../lib/i18n'

/**
 * صفحه‌ی «در دست ساخت».
 *
 * قاعده‌ی محصول: هیچ بخشی نباید صفحه‌ی خالی یا صفحه‌ی اشتباه نشان بدهد. اگر
 * قابلیتی هنوز ساخته نشده، صریحاً اعلام می‌شود که در کدام فاز و بر اساس کدام
 * صفحه‌ی نرم‌افزار فعلی ساخته خواهد شد.
 */
export function UnderConstruction({
  title,
  description,
  reference,
  phase,
}: {
  title: string
  description: string
  /** شناسه‌ی اسکرین‌شات مرجع از نرم‌افزار فعلی نوین پرداز */
  reference?: string
  phase?: string
}) {
  const { t } = useI18n()
  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('underConstruction.eyebrow')}</div>
          <h1>{title}</h1>
        </div>
      </div>
      <div className="under-construction">
        <div className="uc-icon">
          <Icon name="settings" size={24} />
        </div>
        <h2>{t('underConstruction.title')}</h2>
        <p>{description}</p>
        <div style={{display: 'flex', gap: 8, flexWrap: 'wrap', justifyContent: 'center'}}>
          {reference && <span className="uc-ref">مرجع طراحی: تصویر {reference}</span>}
          {phase && <span className="uc-ref">{phase}</span>}
        </div>
      </div>
    </section>
  )
}
