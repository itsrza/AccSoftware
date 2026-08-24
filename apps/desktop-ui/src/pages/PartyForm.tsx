import { useEffect, useState } from 'react'
import { Icon } from '../components/Icon'
import {
  findDuplicateParty,
  getPartyDetail,
  getPartyGroups,
  getPartyOptions,
  getPartyRoutes,
  getParties,
  savePartyFull,
  BankAccountInput,
  ImageInput,
  OccasionInput,
  PartyGroupRow,
  PartyInput,
  PartyOptions,
  PhoneInput,
  RouteRow,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials, formatTomans, rialUnit } from '../lib/format'
import { useI18n, type TranslationKey } from '../lib/i18n'
import {Select} from '../components/Select'

/** هفت زبانه، دقیقاً مطابق فرم مرجع. */
const TABS = [
  { id: 'general', labelKey: 'partyForm.tab.general' },
  { id: 'contact', labelKey: 'partyForm.tab.contact' },
  { id: 'bank', labelKey: 'partyForm.tab.bank' },
  { id: 'images', labelKey: 'partyForm.tab.images' },
  { id: 'portal', labelKey: 'partyForm.tab.account' },
  { id: 'other', labelKey: 'partyForm.tab.other' },
  { id: 'occasions', labelKey: 'partyForm.tab.events' },
] as const satisfies readonly { id: string; labelKey: TranslationKey }[]

type TabId = (typeof TABS)[number]['id']

/** دوازده ماه شمسی؛ نام هر ماه از دیکشنری زبان فعال می‌آید. */
const JALALI_MONTH_KEYS = Array.from(
  { length: 12 },
  (_, index) => `month.${index + 1}` as TranslationKey,
)

/** شش ماه اول سال شمسی ۳۱ روزه‌اند، شش ماه دوم ۳۰ روزه. */
const daysInJalaliMonth = (month: number) => (month <= 6 ? 31 : 30)

const emptyParty = (): PartyInput => ({
  party_type: 'natural',
  party_function: 'person',
  is_customer: true,
  is_supplier: false,
  is_active: true,
  credit_limit: 0,
  phones: [],
  bank_accounts: [],
  images: [],
  occasions: [],
})

export function PartyForm({
  partyId,
  onClose,
  onSaved,
}: {
  partyId?: string
  onClose: () => void
  onSaved: (id: string) => void
}) {
  const { t } = useI18n()
  const [tab, setTab] = useState<TabId>('general')
  const [form, setForm] = useState<PartyInput>(emptyParty())
  const [options, setOptions] = useState<PartyOptions>()
  const [groups, setGroups] = useState<PartyGroupRow[]>([])
  const [routes, setRoutes] = useState<RouteRow[]>([])
  const [marketers, setMarketers] = useState<{ id: string; name: string }[]>([])
  const [error, setError] = useState('')
  const [duplicate, setDuplicate] = useState('')
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    ;(async () => {
      try {
        setOptions(await getPartyOptions())
        setGroups(await getPartyGroups())
      } catch (e) {
        setError(errorText(e))
      }
      try {
        setRoutes(await getPartyRoutes())
      } catch {
        /* مسیر اختیاری است */
      }
      try {
        const list = await getParties()
        setMarketers(
          list.rows
            .filter((p) => p.party_function === 'marketer' || p.party_function === 'supervisor')
            .map((p) => ({ id: p.id, name: p.display_name })),
        )
      } catch {
        /* بازاریاب اختیاری است */
      }
    })()
  }, [])

  useEffect(() => {
    if (!partyId) {
      setForm(emptyParty())
      return
    }
    ;(async () => {
      try {
        const detail = await getPartyDetail(partyId)
        setForm({
          ...detail,
          phones: detail.phones.map(({ id: _id, ...rest }) => rest),
          bank_accounts: detail.bank_accounts.map(({ id: _id, ...rest }) => rest),
          images: detail.images.map(({ id: _id, ...rest }) => rest),
          occasions: detail.occasions.map(({ id: _id, ...rest }) => rest),
          portal_password: undefined,
        })
      } catch (e) {
        setError(errorText(e))
      }
    })()
  }, [partyId])

  const set = (patch: Partial<PartyInput>) => setForm((current) => ({ ...current, ...patch }))

  const isLegal = form.party_type !== 'natural'

  // تشخیص تکراری پیش از ذخیره — بهتر از خطا خوردن بعد از پرکردن هفت زبانه.
  const checkDuplicate = async () => {
    try {
      const found = await findDuplicateParty(form.mobile, form.national_id, form.id)
      setDuplicate(found ? `هشدار: «${found}» با همین موبایل یا کد ملی از قبل ثبت شده است.` : '')
    } catch {
      setDuplicate('')
    }
  }

  const submit = async () => {
    setBusy(true)
    setError('')
    try {
      const id = await savePartyFull(form)
      onSaved(id)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  // --- کمک‌کننده‌های ردیف‌های تکرارشونده ---
  const addPhone = () =>
    set({ phones: [...form.phones, { number: '', is_primary: form.phones.length === 0 }] })
  const setPhone = (index: number, patch: Partial<PhoneInput>) =>
    set({ phones: form.phones.map((p, i) => (i === index ? { ...p, ...patch } : p)) })
  const removePhone = (index: number) =>
    set({ phones: form.phones.filter((_, i) => i !== index) })

  const addBank = () =>
    set({
      bank_accounts: [
        ...form.bank_accounts,
        { bank_name: '', is_default: form.bank_accounts.length === 0 },
      ],
    })
  const setBank = (index: number, patch: Partial<BankAccountInput>) =>
    set({ bank_accounts: form.bank_accounts.map((b, i) => (i === index ? { ...b, ...patch } : b)) })
  const removeBank = (index: number) =>
    set({ bank_accounts: form.bank_accounts.filter((_, i) => i !== index) })

  const addImage = () =>
    set({
      images: [...form.images, { file_path: '', is_primary: form.images.length === 0 }],
    })
  const setImage = (index: number, patch: Partial<ImageInput>) =>
    set({ images: form.images.map((m, i) => (i === index ? { ...m, ...patch } : m)) })
  const removeImage = (index: number) => set({ images: form.images.filter((_, i) => i !== index) })

  const addOccasion = () =>
    set({
      occasions: [
        ...form.occasions,
        { title: '', jalali_month: 1, jalali_day: 1, remind_days_before: 3 },
      ],
    })
  const setOccasion = (index: number, patch: Partial<OccasionInput>) =>
    set({ occasions: form.occasions.map((o, i) => (i === index ? { ...o, ...patch } : o)) })
  const removeOccasion = (index: number) =>
    set({ occasions: form.occasions.filter((_, i) => i !== index) })

  return (
    <div className="modal-backdrop" role="presentation">
      <div className="modal party-modal">
        <div className="modal-head">
          <div>
            <h2>{partyId ? t('partyForm.editTitle') : t('partyForm.newTitle')}</h2>
            <p>
              {isLegal
                ? t('partyForm.legalRequired')
                : t('partyForm.naturalRequired')}
            </p>
          </div>
          <button className="icon-btn" onClick={onClose} aria-label={t('common.close')}>
            <Icon name="close" />
          </button>
        </div>

        {error && <div className="error-box">{error}</div>}
        {duplicate && <div className="warn-box">{duplicate}</div>}

        <div className="tab-bar">
          {TABS.map((item) => (
            <button
              key={item.id}
              className={tab === item.id ? 'active' : ''}
              onClick={() => setTab(item.id)}
            >
              {t(item.labelKey)}
            </button>
          ))}
        </div>

        <div className="tab-body">
          {tab === 'general' && (
            <div className="filter-grid">
              <label>
                <span>{t('partyForm.personTypeRequired')}</span>
                <Select
                  value={form.party_type}
                  onChange={(e) => set({ party_type: e.target.value })}
                >
                  {options?.party_types.map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </Select>
              </label>
              <label>
                <span>{t('partyForm.roleRequired')}</span>
                <Select
                  value={form.party_function}
                  onChange={(e) => set({ party_function: e.target.value })}
                >
                  {options?.party_functions.map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </Select>
              </label>
              <label>
                <span>{t('partyForm.code')}</span>
                <input
                  value={form.code ?? ''}
                  onChange={(e) => set({ code: e.target.value })}
                  placeholder={t('partyForm.autoCode')}
                />
              </label>
              <label>
                <span>{t('partyForm.openingDate')}</span>
                <input
                  value={form.opening_date ?? ''}
                  onChange={(e) => set({ opening_date: e.target.value })}
                  placeholder="1405/01/01"
                />
              </label>
              {!isLegal && (
                <>
                  <label>
                    <span>{t('partyForm.titlePrefix')}</span>
                    <input
                      value={form.title_prefix ?? ''}
                      onChange={(e) => set({ title_prefix: e.target.value })}
                      placeholder={t('partyForm.salutation')}
                    />
                  </label>
                  <label>
                    <span>{t('partyForm.firstNameRequired')}</span>
                    <input
                      value={form.first_name ?? ''}
                      onChange={(e) => set({ first_name: e.target.value })}
                    />
                  </label>
                  <label>
                    <span>{t('partyForm.lastNameRequired')}</span>
                    <input
                      value={form.last_name ?? ''}
                      onChange={(e) => set({ last_name: e.target.value })}
                    />
                  </label>
                </>
              )}
              {isLegal && (
                <label className="grow">
                  <span>{t('partyForm.companyNameRequired')}</span>
                  <input
                    value={form.company_name ?? ''}
                    onChange={(e) => set({ company_name: e.target.value })}
                  />
                </label>
              )}
              <label>
                <span>{isLegal ? t('partyForm.nationalId') : t('partyForm.personalId')}</span>
                <input
                  value={form.national_id ?? ''}
                  onChange={(e) => set({ national_id: e.target.value })}
                  onBlur={checkDuplicate}
                />
              </label>
              <label>
                <span>{t('partyForm.economicCode')}</span>
                <input
                  value={form.economic_code ?? ''}
                  onChange={(e) => set({ economic_code: e.target.value })}
                />
              </label>
              <label>
                <span>{t('common.group')}</span>
                <Select
                  value={form.group_id ?? ''}
                  onChange={(e) => set({ group_id: e.target.value })}
                >
                  <option value="">{t('partyForm.noGroup')}</option>
                  {groups.map((g) => (
                    <option key={g.id} value={g.id}>
                      {g.code} — {g.title}
                    </option>
                  ))}
                </Select>
              </label>
              <label>
                <span>{t('partyForm.route')}</span>
                <Select
                  value={form.route_id ?? ''}
                  onChange={(e) => set({ route_id: e.target.value })}
                >
                  <option value="">{t('partyForm.noRoute')}</option>
                  {routes.map((r) => (
                    <option key={r.id} value={r.id}>
                      {r.code} — {r.title}
                    </option>
                  ))}
                </Select>
              </label>
              <label>
                <span>{t('partyForm.marketer')}</span>
                <Select
                  value={form.marketer_id ?? ''}
                  onChange={(e) => set({ marketer_id: e.target.value })}
                >
                  <option value="">{t('partyForm.noMarketer')}</option>
                  {marketers.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.name}
                    </option>
                  ))}
                </Select>
              </label>
              <div className="checkbox-row">
                <label className="inline-check">
                  <input
                    type="checkbox"
                    checked={form.is_customer}
                    onChange={(e) => set({ is_customer: e.target.checked })}
                  />
                  <span>{t('partyForm.customer')}</span>
                </label>
                <label className="inline-check">
                  <input
                    type="checkbox"
                    checked={form.is_supplier}
                    onChange={(e) => set({ is_supplier: e.target.checked })}
                  />
                  <span>{t('partyForm.supplier')}</span>
                </label>
                <label className="inline-check">
                  <input
                    type="checkbox"
                    checked={form.is_active}
                    onChange={(e) => set({ is_active: e.target.checked })}
                  />
                  <span>{t('partyForm.active')}</span>
                </label>
              </div>
              {form.party_function === 'person' && !form.is_customer && !form.is_supplier && (
                <p className="hint">
                  {t('partyForm.roleHint')}
                </p>
              )}
            </div>
          )}

          {tab === 'contact' && (
            <>
              <div className="filter-grid">
                <label>
                  <span>{t('partyForm.mobile')}</span>
                  <input
                    value={form.mobile ?? ''}
                    onChange={(e) => set({ mobile: e.target.value })}
                    onBlur={checkDuplicate}
                    placeholder="09121234567"
                  />
                </label>
                <label>
                  <span>{t('partyForm.email')}</span>
                  <input
                    value={form.email ?? ''}
                    onChange={(e) => set({ email: e.target.value })}
                  />
                </label>
                <label>
                  <span>{t('partyForm.website')}</span>
                  <input
                    value={form.website ?? ''}
                    onChange={(e) => set({ website: e.target.value })}
                  />
                </label>
                <label>
                  <span>{t('partyForm.province')}</span>
                  <input
                    value={form.province ?? ''}
                    onChange={(e) => set({ province: e.target.value })}
                  />
                </label>
                <label>
                  <span>{t('partyForm.city')}</span>
                  <input value={form.city ?? ''} onChange={(e) => set({ city: e.target.value })} />
                </label>
                <label>
                  <span>{t('partyForm.postalCode')}</span>
                  <input
                    value={form.postal_code ?? ''}
                    onChange={(e) => set({ postal_code: e.target.value })}
                    placeholder={t('partyForm.tenDigits')}
                  />
                </label>
                <label className="grow">
                  <span>{t('partyForm.address')}</span>
                  <input
                    value={form.address ?? ''}
                    onChange={(e) => set({ address: e.target.value })}
                  />
                </label>
              </div>

              <div className="repeat-head">
                <h4 className="section-title">{t('partyForm.phones')}</h4>
                <button className="ghost" onClick={addPhone}>
                  <Icon name="plus" /> {t('partyForm.addPhone')}
                </button>
              </div>
              {form.phones.map((phone, index) => (
                <div className="line-row" key={index}>
                  <label>
                    <span>{t('partyForm.titlePrefix')}</span>
                    <input
                      value={phone.title ?? ''}
                      onChange={(e) => setPhone(index, { title: e.target.value })}
                      placeholder={t('partyForm.officeHome')}
                    />
                  </label>
                  <label className="grow">
                    <span>{t('partyForm.phoneNumber')}</span>
                    <input
                      value={phone.number}
                      onChange={(e) => setPhone(index, { number: e.target.value })}
                    />
                  </label>
                  <label className="inline-check">
                    <input
                      type="radio"
                      name="primary-phone"
                      checked={phone.is_primary}
                      onChange={() =>
                        set({
                          phones: form.phones.map((p, i) => ({ ...p, is_primary: i === index })),
                        })
                      }
                    />
                    <span>{t('partyForm.default')}</span>
                  </label>
                  <button aria-label={t('partyForm.remove')}
                    className="icon-btn danger-icon"
                    onClick={() => removePhone(index)}
                   
                  >
                    <Icon name="trash" />
                  </button>
                </div>
              ))}
              {form.phones.length === 0 && <p className="muted">{t('partyForm.noPhone')}</p>}
            </>
          )}

          {tab === 'bank' && (
            <>
              <div className="repeat-head">
                <h4 className="section-title">{t('partyForm.bankAccounts')}</h4>
                <button className="ghost" onClick={addBank}>
                  <Icon name="plus" /> {t('partyForm.addAccount')}
                </button>
              </div>
              <p className="muted">
                {t('partyForm.ibanHint')}
              </p>
              {form.bank_accounts.map((account, index) => (
                <div className="line-row" key={index}>
                  <label>
                    <span>{t('partyForm.bankRequired')}</span>
                    <input
                      value={account.bank_name}
                      onChange={(e) => setBank(index, { bank_name: e.target.value })}
                    />
                  </label>
                  <label>
                    <span>{t('partyForm.branch')}</span>
                    <input
                      value={account.branch_name ?? ''}
                      onChange={(e) => setBank(index, { branch_name: e.target.value })}
                    />
                  </label>
                  <label>
                    <span>{t('partyForm.accountNumber')}</span>
                    <input
                      value={account.account_number ?? ''}
                      onChange={(e) => setBank(index, { account_number: e.target.value })}
                    />
                  </label>
                  <label className="grow">
                    <span>{t('partyForm.iban')}</span>
                    <input
                      value={account.iban ?? ''}
                      onChange={(e) => setBank(index, { iban: e.target.value })}
                      placeholder="IR..."
                    />
                  </label>
                  <label>
                    <span>{t('partyForm.cardNumber')}</span>
                    <input
                      value={account.card_number ?? ''}
                      onChange={(e) => setBank(index, { card_number: e.target.value })}
                    />
                  </label>
                  <label>
                    <span>{t('partyForm.accountHolder')}</span>
                    <input
                      value={account.holder_name ?? ''}
                      onChange={(e) => setBank(index, { holder_name: e.target.value })}
                    />
                  </label>
                  <label className="inline-check">
                    <input
                      type="radio"
                      name="default-bank"
                      checked={account.is_default}
                      onChange={() =>
                        set({
                          bank_accounts: form.bank_accounts.map((b, i) => ({
                            ...b,
                            is_default: i === index,
                          })),
                        })
                      }
                    />
                    <span>{t('partyForm.default')}</span>
                  </label>
                  <button aria-label={t('partyForm.remove')}
                    className="icon-btn danger-icon"
                    onClick={() => removeBank(index)}
                   
                  >
                    <Icon name="trash" />
                  </button>
                </div>
              ))}
              {form.bank_accounts.length === 0 && <p className="muted">{t('partyForm.noAccount')}</p>}
            </>
          )}

          {tab === 'images' && (
            <>
              <div className="repeat-head">
                <h4 className="section-title">{t('partyForm.tab.images')}</h4>
                <button className="ghost" onClick={addImage}>
                  <Icon name="plus" /> {t('partyForm.addImage')}
                </button>
              </div>
              <p className="muted">
                {t('partyForm.imageHint')}
              </p>
              {form.images.map((image, index) => (
                <div className="line-row" key={index}>
                  <label>
                    <span>{t('partyForm.titlePrefix')}</span>
                    <input
                      value={image.title ?? ''}
                      onChange={(e) => setImage(index, { title: e.target.value })}
                      placeholder={t('partyForm.idCardLogo')}
                    />
                  </label>
                  <label className="grow">
                    <span>{t('partyForm.filePathRequired')}</span>
                    <input
                      value={image.file_path}
                      onChange={(e) => setImage(index, { file_path: e.target.value })}
                      placeholder="C:\\Documents\\customer.jpg"
                    />
                  </label>
                  <label className="inline-check">
                    <input
                      type="radio"
                      name="primary-image"
                      checked={image.is_primary}
                      onChange={() =>
                        set({
                          images: form.images.map((m, i) => ({ ...m, is_primary: i === index })),
                        })
                      }
                    />
                    <span>{t('partyForm.primary')}</span>
                  </label>
                  <button aria-label={t('partyForm.remove')}
                    className="icon-btn danger-icon"
                    onClick={() => removeImage(index)}
                   
                  >
                    <Icon name="trash" />
                  </button>
                </div>
              ))}
              {form.images.length === 0 && <p className="muted">{t('partyForm.noImage')}</p>}
            </>
          )}

          {tab === 'portal' && (
            <div className="filter-grid">
              <label>
                <span>{t('partyForm.username')}</span>
                <input
                  value={form.portal_username ?? ''}
                  onChange={(e) => set({ portal_username: e.target.value })}
                  placeholder={t('partyForm.minFour')}
                />
              </label>
              <label>
                <span>{t('partyForm.password')}</span>
                <input
                  type="password"
                  value={form.portal_password ?? ''}
                  onChange={(e) => set({ portal_password: e.target.value })}
                  placeholder={partyId ? t('partyForm.passwordHint') : t('partyForm.minEight')}
                />
              </label>
              <p className="hint">
                {t('partyForm.passwordNote')}
              </p>
            </div>
          )}

          {tab === 'other' && (
            <div className="filter-grid">
              <label>
                <span>{t('partyForm.job')}</span>
                <input
                  value={form.job_title ?? ''}
                  onChange={(e) => set({ job_title: e.target.value })}
                />
              </label>
              <label>
                <span>{t('partyForm.referral')}</span>
                <input
                  value={form.introduction ?? ''}
                  onChange={(e) => set({ introduction: e.target.value })}
                  placeholder={t('partyForm.referralHint')}
                />
              </label>
              <label>
                <span>{t('partyForm.creditLimit', { unit: rialUnit() })}</span>
                <input
                  type="number"
                  min={0}
                  value={form.credit_limit || ''}
                  onChange={(e) => set({ credit_limit: Number(e.target.value) || 0 })}
                  placeholder={t('partyForm.zeroNoLimit')}
                />
                {form.credit_limit > 0 && (
                  <small className="field-hint">
                    {formatRials(form.credit_limit)} ریال ({formatTomans(form.credit_limit)})
                  </small>
                )}
              </label>
              <label className="grow">
                <span>{t('partyForm.note')}</span>
                <input value={form.note ?? ''} onChange={(e) => set({ note: e.target.value })} />
              </label>
              <p className="hint">
                {t('partyForm.creditHint')}
              </p>
            </div>
          )}

          {tab === 'occasions' && (
            <>
              <div className="repeat-head">
                <h4 className="section-title">{t('partyForm.recurringEvents')}</h4>
                <button className="ghost" onClick={addOccasion}>
                  <Icon name="plus" /> {t('partyForm.addEvent')}
                </button>
              </div>
              <p className="muted">
                {t('partyForm.eventNote')}
              </p>
              {form.occasions.map((occasion, index) => (
                <div className="line-row" key={index}>
                  <label className="grow">
                    <span>{t('partyForm.eventTitleRequired')}</span>
                    <input
                      value={occasion.title}
                      onChange={(e) => setOccasion(index, { title: e.target.value })}
                      placeholder={t('partyForm.eventHint')}
                    />
                  </label>
                  <label>
                    <span>{t('partyForm.month')}</span>
                    <Select
                      value={occasion.jalali_month}
                      onChange={(e) => {
                        const month = Number(e.target.value)
                        setOccasion(index, {
                          jalali_month: month,
                          jalali_day: Math.min(occasion.jalali_day, daysInJalaliMonth(month)),
                        })
                      }}
                    >
                      {JALALI_MONTH_KEYS.map((key, i) => (
                        <option key={key} value={i + 1}>
                          {t(key)}
                        </option>
                      ))}
                    </Select>
                  </label>
                  <label>
                    <span>{t('partyForm.day')}</span>
                    <Select
                      value={occasion.jalali_day}
                      onChange={(e) => setOccasion(index, { jalali_day: Number(e.target.value) })}
                    >
                      {Array.from(
                        { length: daysInJalaliMonth(occasion.jalali_month) },
                        (_, i) => i + 1,
                      ).map((day) => (
                        <option key={day} value={day}>
                          {day}
                        </option>
                      ))}
                    </Select>
                  </label>
                  <label>
                    <span>{t('partyForm.remindDaysBefore')}</span>
                    <input
                      type="number"
                      min={0}
                      max={365}
                      value={occasion.remind_days_before}
                      onChange={(e) =>
                        setOccasion(index, { remind_days_before: Number(e.target.value) || 0 })
                      }
                    />
                  </label>
                  <button aria-label={t('partyForm.remove')}
                    className="icon-btn danger-icon"
                    onClick={() => removeOccasion(index)}
                   
                  >
                    <Icon name="trash" />
                  </button>
                </div>
              ))}
              {form.occasions.length === 0 && <p className="muted">{t('partyForm.noEvent')}</p>}
            </>
          )}
        </div>

        <div className="modal-actions">
          <button className="primary" onClick={submit} disabled={busy}>
            {partyId ? t('partyForm.saveChanges') : t('partyForm.savePerson')}
          </button>
          <button className="ghost" onClick={onClose}>
            {t('common.cancel')}
          </button>
        </div>
      </div>
    </div>
  )
}
