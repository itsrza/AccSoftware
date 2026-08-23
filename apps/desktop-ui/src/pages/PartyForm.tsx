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
import { formatRials, formatTomans } from '../lib/format'
import {Select} from '../components/Select'

/** هفت زبانه، دقیقاً مطابق فرم مرجع. */
const TABS = [
  { id: 'general', label: 'مشخصات عمومی' },
  { id: 'contact', label: 'مشخصات ارتباطی' },
  { id: 'bank', label: 'حساب‌های بانکی' },
  { id: 'images', label: 'تصاویر' },
  { id: 'portal', label: 'مشخصات کاربری' },
  { id: 'other', label: 'سایر مشخصات' },
  { id: 'occasions', label: 'تقویم مناسبت‌ها' },
] as const

type TabId = (typeof TABS)[number]['id']

const JALALI_MONTHS = [
  'فروردین',
  'اردیبهشت',
  'خرداد',
  'تیر',
  'مرداد',
  'شهریور',
  'مهر',
  'آبان',
  'آذر',
  'دی',
  'بهمن',
  'اسفند',
]

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
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal party-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <div>
            <h2>{partyId ? 'ویرایش شخص' : 'افزودن شخص'}</h2>
            <p>
              {isLegal
                ? 'شخص حقوقی: نام شرکت و شناسه ملی الزامی است.'
                : 'شخص حقیقی: نام و نام خانوادگی الزامی است.'}
            </p>
          </div>
          <button className="icon-btn" onClick={onClose} aria-label="بستن">
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
              {item.label}
            </button>
          ))}
        </div>

        <div className="tab-body">
          {tab === 'general' && (
            <div className="filter-grid">
              <label>
                <span>نوع شخصیت *</span>
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
                <span>نقش *</span>
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
                <span>کد شخص</span>
                <input
                  value={form.code ?? ''}
                  onChange={(e) => set({ code: e.target.value })}
                  placeholder="خالی بگذارید تا خودکار ساخته شود"
                />
              </label>
              <label>
                <span>تاریخ افتتاح</span>
                <input
                  value={form.opening_date ?? ''}
                  onChange={(e) => set({ opening_date: e.target.value })}
                  placeholder="1405/01/01"
                />
              </label>
              {!isLegal && (
                <>
                  <label>
                    <span>عنوان</span>
                    <input
                      value={form.title_prefix ?? ''}
                      onChange={(e) => set({ title_prefix: e.target.value })}
                      placeholder="آقای / خانم"
                    />
                  </label>
                  <label>
                    <span>نام *</span>
                    <input
                      value={form.first_name ?? ''}
                      onChange={(e) => set({ first_name: e.target.value })}
                    />
                  </label>
                  <label>
                    <span>نام خانوادگی *</span>
                    <input
                      value={form.last_name ?? ''}
                      onChange={(e) => set({ last_name: e.target.value })}
                    />
                  </label>
                </>
              )}
              {isLegal && (
                <label className="grow">
                  <span>نام شرکت *</span>
                  <input
                    value={form.company_name ?? ''}
                    onChange={(e) => set({ company_name: e.target.value })}
                  />
                </label>
              )}
              <label>
                <span>{isLegal ? 'شناسه ملی' : 'کد ملی'}</span>
                <input
                  value={form.national_id ?? ''}
                  onChange={(e) => set({ national_id: e.target.value })}
                  onBlur={checkDuplicate}
                />
              </label>
              <label>
                <span>شماره اقتصادی</span>
                <input
                  value={form.economic_code ?? ''}
                  onChange={(e) => set({ economic_code: e.target.value })}
                />
              </label>
              <label>
                <span>گروه</span>
                <Select
                  value={form.group_id ?? ''}
                  onChange={(e) => set({ group_id: e.target.value })}
                >
                  <option value="">بدون گروه</option>
                  {groups.map((g) => (
                    <option key={g.id} value={g.id}>
                      {g.code} — {g.title}
                    </option>
                  ))}
                </Select>
              </label>
              <label>
                <span>مسیر پخش</span>
                <Select
                  value={form.route_id ?? ''}
                  onChange={(e) => set({ route_id: e.target.value })}
                >
                  <option value="">بدون مسیر</option>
                  {routes.map((r) => (
                    <option key={r.id} value={r.id}>
                      {r.code} — {r.title}
                    </option>
                  ))}
                </Select>
              </label>
              <label>
                <span>بازاریاب</span>
                <Select
                  value={form.marketer_id ?? ''}
                  onChange={(e) => set({ marketer_id: e.target.value })}
                >
                  <option value="">بدون بازاریاب</option>
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
                  <span>مشتری</span>
                </label>
                <label className="inline-check">
                  <input
                    type="checkbox"
                    checked={form.is_supplier}
                    onChange={(e) => set({ is_supplier: e.target.checked })}
                  />
                  <span>تأمین‌کننده</span>
                </label>
                <label className="inline-check">
                  <input
                    type="checkbox"
                    checked={form.is_active}
                    onChange={(e) => set({ is_active: e.target.checked })}
                  />
                  <span>فعال</span>
                </label>
              </div>
              {form.party_function === 'person' && !form.is_customer && !form.is_supplier && (
                <p className="hint">
                  شخص با نقش «شخص» باید حداقل مشتری یا تأمین‌کننده باشد؛ وگرنه در هیچ فاکتوری
                  قابل انتخاب نیست.
                </p>
              )}
            </div>
          )}

          {tab === 'contact' && (
            <>
              <div className="filter-grid">
                <label>
                  <span>موبایل</span>
                  <input
                    value={form.mobile ?? ''}
                    onChange={(e) => set({ mobile: e.target.value })}
                    onBlur={checkDuplicate}
                    placeholder="09121234567"
                  />
                </label>
                <label>
                  <span>ایمیل</span>
                  <input
                    value={form.email ?? ''}
                    onChange={(e) => set({ email: e.target.value })}
                  />
                </label>
                <label>
                  <span>وب‌سایت</span>
                  <input
                    value={form.website ?? ''}
                    onChange={(e) => set({ website: e.target.value })}
                  />
                </label>
                <label>
                  <span>استان</span>
                  <input
                    value={form.province ?? ''}
                    onChange={(e) => set({ province: e.target.value })}
                  />
                </label>
                <label>
                  <span>شهر</span>
                  <input value={form.city ?? ''} onChange={(e) => set({ city: e.target.value })} />
                </label>
                <label>
                  <span>کد پستی</span>
                  <input
                    value={form.postal_code ?? ''}
                    onChange={(e) => set({ postal_code: e.target.value })}
                    placeholder="۱۰ رقم"
                  />
                </label>
                <label className="grow">
                  <span>نشانی</span>
                  <input
                    value={form.address ?? ''}
                    onChange={(e) => set({ address: e.target.value })}
                  />
                </label>
              </div>

              <div className="repeat-head">
                <h4 className="section-title">تلفن‌ها</h4>
                <button className="ghost" onClick={addPhone}>
                  <Icon name="plus" /> افزودن تلفن
                </button>
              </div>
              {form.phones.map((phone, index) => (
                <div className="line-row" key={index}>
                  <label>
                    <span>عنوان</span>
                    <input
                      value={phone.title ?? ''}
                      onChange={(e) => setPhone(index, { title: e.target.value })}
                      placeholder="دفتر / منزل"
                    />
                  </label>
                  <label className="grow">
                    <span>شماره</span>
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
                    <span>پیش‌فرض</span>
                  </label>
                  <button aria-label="حذف"
                    className="icon-btn danger-icon"
                    onClick={() => removePhone(index)}
                   
                  >
                    <Icon name="trash" />
                  </button>
                </div>
              ))}
              {form.phones.length === 0 && <p className="muted">تلفنی ثبت نشده است.</p>}
            </>
          )}

          {tab === 'bank' && (
            <>
              <div className="repeat-head">
                <h4 className="section-title">حساب‌های بانکی شخص</h4>
                <button className="ghost" onClick={addBank}>
                  <Icon name="plus" /> افزودن حساب
                </button>
              </div>
              <p className="muted">
                شبا و شماره کارت با الگوریتم رسمی بررسی می‌شوند؛ شماره‌ی نادرست هنگام ذخیره رد
                می‌شود.
              </p>
              {form.bank_accounts.map((account, index) => (
                <div className="line-row" key={index}>
                  <label>
                    <span>بانک *</span>
                    <input
                      value={account.bank_name}
                      onChange={(e) => setBank(index, { bank_name: e.target.value })}
                    />
                  </label>
                  <label>
                    <span>شعبه</span>
                    <input
                      value={account.branch_name ?? ''}
                      onChange={(e) => setBank(index, { branch_name: e.target.value })}
                    />
                  </label>
                  <label>
                    <span>شماره حساب</span>
                    <input
                      value={account.account_number ?? ''}
                      onChange={(e) => setBank(index, { account_number: e.target.value })}
                    />
                  </label>
                  <label className="grow">
                    <span>شبا</span>
                    <input
                      value={account.iban ?? ''}
                      onChange={(e) => setBank(index, { iban: e.target.value })}
                      placeholder="IR..."
                    />
                  </label>
                  <label>
                    <span>شماره کارت</span>
                    <input
                      value={account.card_number ?? ''}
                      onChange={(e) => setBank(index, { card_number: e.target.value })}
                    />
                  </label>
                  <label>
                    <span>صاحب حساب</span>
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
                    <span>پیش‌فرض</span>
                  </label>
                  <button aria-label="حذف"
                    className="icon-btn danger-icon"
                    onClick={() => removeBank(index)}
                   
                  >
                    <Icon name="trash" />
                  </button>
                </div>
              ))}
              {form.bank_accounts.length === 0 && <p className="muted">حسابی ثبت نشده است.</p>}
            </>
          )}

          {tab === 'images' && (
            <>
              <div className="repeat-head">
                <h4 className="section-title">تصاویر</h4>
                <button className="ghost" onClick={addImage}>
                  <Icon name="plus" /> افزودن تصویر
                </button>
              </div>
              <p className="muted">
                فقط مسیر فایل ذخیره می‌شود، نه خود تصویر — تا حجم پایگاه داده و زمان پشتیبان‌گیری
                کنترل‌شده بماند.
              </p>
              {form.images.map((image, index) => (
                <div className="line-row" key={index}>
                  <label>
                    <span>عنوان</span>
                    <input
                      value={image.title ?? ''}
                      onChange={(e) => setImage(index, { title: e.target.value })}
                      placeholder="کارت ملی / لوگو"
                    />
                  </label>
                  <label className="grow">
                    <span>مسیر فایل *</span>
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
                    <span>اصلی</span>
                  </label>
                  <button aria-label="حذف"
                    className="icon-btn danger-icon"
                    onClick={() => removeImage(index)}
                   
                  >
                    <Icon name="trash" />
                  </button>
                </div>
              ))}
              {form.images.length === 0 && <p className="muted">تصویری ثبت نشده است.</p>}
            </>
          )}

          {tab === 'portal' && (
            <div className="filter-grid">
              <label>
                <span>نام کاربری فروشگاه اینترنتی</span>
                <input
                  value={form.portal_username ?? ''}
                  onChange={(e) => set({ portal_username: e.target.value })}
                  placeholder="حداقل چهار نویسه"
                />
              </label>
              <label>
                <span>رمز عبور</span>
                <input
                  type="password"
                  value={form.portal_password ?? ''}
                  onChange={(e) => set({ portal_password: e.target.value })}
                  placeholder={partyId ? 'برای تغییر، رمز تازه وارد کنید' : 'حداقل هشت نویسه'}
                />
              </label>
              <p className="hint">
                رمز عبور هرگز به‌صورت خام ذخیره یا نمایش داده نمی‌شود؛ فقط هش آن نگهداری می‌شود.
              </p>
            </div>
          )}

          {tab === 'other' && (
            <div className="filter-grid">
              <label>
                <span>شغل</span>
                <input
                  value={form.job_title ?? ''}
                  onChange={(e) => set({ job_title: e.target.value })}
                />
              </label>
              <label>
                <span>نحوه آشنایی</span>
                <input
                  value={form.introduction ?? ''}
                  onChange={(e) => set({ introduction: e.target.value })}
                  placeholder="معرفی همکار / تبلیغات / …"
                />
              </label>
              <label>
                <span>سقف اعتبار (ریال)</span>
                <input
                  type="number"
                  min={0}
                  value={form.credit_limit || ''}
                  onChange={(e) => set({ credit_limit: Number(e.target.value) || 0 })}
                  placeholder="صفر یعنی بدون محدودیت"
                />
                {form.credit_limit > 0 && (
                  <small className="field-hint">
                    {formatRials(form.credit_limit)} ریال ({formatTomans(form.credit_limit)})
                  </small>
                )}
              </label>
              <label className="grow">
                <span>یادداشت</span>
                <input value={form.note ?? ''} onChange={(e) => set({ note: e.target.value })} />
              </label>
              <p className="hint">
                سقف اعتبار هنگام ثبت فاکتور نسیه بررسی می‌شود؛ صفر یعنی محدودیتی اعمال نمی‌شود.
              </p>
            </div>
          )}

          {tab === 'occasions' && (
            <>
              <div className="repeat-head">
                <h4 className="section-title">مناسبت‌های تکرارشونده</h4>
                <button className="ghost" onClick={addOccasion}>
                  <Icon name="plus" /> افزودن مناسبت
                </button>
              </div>
              <p className="muted">
                تاریخ شمسی بدون سال ثبت می‌شود چون مناسبت هر سال تکرار می‌شود. شش ماه دوم سال
                ۳۰ روزه است.
              </p>
              {form.occasions.map((occasion, index) => (
                <div className="line-row" key={index}>
                  <label className="grow">
                    <span>عنوان *</span>
                    <input
                      value={occasion.title}
                      onChange={(e) => setOccasion(index, { title: e.target.value })}
                      placeholder="تولد / سالگرد تأسیس"
                    />
                  </label>
                  <label>
                    <span>ماه</span>
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
                      {JALALI_MONTHS.map((name, i) => (
                        <option key={name} value={i + 1}>
                          {name}
                        </option>
                      ))}
                    </Select>
                  </label>
                  <label>
                    <span>روز</span>
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
                    <span>یادآوری (روز قبل)</span>
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
                  <button aria-label="حذف"
                    className="icon-btn danger-icon"
                    onClick={() => removeOccasion(index)}
                   
                  >
                    <Icon name="trash" />
                  </button>
                </div>
              ))}
              {form.occasions.length === 0 && <p className="muted">مناسبتی ثبت نشده است.</p>}
            </>
          )}
        </div>

        <div className="modal-actions">
          <button className="primary" onClick={submit} disabled={busy}>
            {partyId ? 'ذخیره تغییرات' : 'ثبت شخص'}
          </button>
          <button className="ghost" onClick={onClose}>
            انصراف
          </button>
        </div>
      </div>
    </div>
  )
}
