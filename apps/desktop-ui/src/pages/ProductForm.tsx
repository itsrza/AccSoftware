import { useEffect, useMemo, useState } from 'react'
import { Plus, Trash2 } from 'lucide-react'
import {
  getProductGroups,
  getProductKinds,
  getProductProfile,
  previewGoldPrice,
  saveProductProfile,
  type PriceLevelOption,
  type ProductDetail,
  type ProductGold,
  type ProductGroupRow,
  type ProductInput,
  type ProductKindOption,
  type ProductTierRow,
  type ProductUnitRow,
} from '../api'
import { errorText } from '../lib/errors'
import { formatNumber, formatRials as money, parseAmount, rialUnit } from '../lib/format'
import { useI18n, type TranslationKey } from '../lib/i18n'
import { Select } from '../components/Select'
import { Badge } from '../components/ui'

/**
 * فرم تعریف کالا — هفت زبانه.
 *
 * مرجع: تصویر `NztJl5` (فرم تعریف کالا) و `6FM9Ow` (انتخاب نوع کالا).
 *
 * ## چرا زبانه‌بندی
 * فرم کالای یک نرم‌افزار حسابداری واقعی سی‌وچند فیلد دارد. ریختن همه در یک
 * صفحه یعنی کاربر هیچ‌وقت پایین را نمی‌بیند. زبانه‌ها همان گروه‌بندی
 * نرم‌افزار فعلی را دارند تا کاربر مهاجرت‌کننده گم نشود.
 *
 * ## چرا محاسبه‌ی طلا از موتور می‌آید
 * قیمت طلا = وزن × نرخ + اجرت + سود + ارزش افزوده. این یک محاسبه‌ی مالی
 * است و در هسته زندگی می‌کند، نه در فرم.
 */

type Tab = 'general' | 'prices' | 'units' | 'tax' | 'stock' | 'tiers' | 'gold'

const TABS: { id: Tab; labelKey: TranslationKey }[] = [
  { id: 'general', labelKey: 'productForm.tab.general' },
  { id: 'prices', labelKey: 'productForm.tab.prices' },
  { id: 'units', labelKey: 'productForm.tab.units' },
  { id: 'tax', labelKey: 'productForm.tab.tax' },
  { id: 'stock', labelKey: 'productForm.tab.stock' },
  { id: 'tiers', labelKey: 'productForm.tab.tiers' },
  { id: 'gold', labelKey: 'productForm.tab.gold' },
]

/** واحد پیش‌فرض کالای تازه؛ به زبان فعال، چون در فیلد قابل ویرایش می‌نشیند. */
const emptyInput = (
  kind: string,
  levels: PriceLevelOption[],
  defaultUnit: string,
): ProductInput => ({
  kind,
  sku: '',
  name: '',
  unit: defaultUnit,
  purchase_price: 0,
  min_stock: 0,
  max_stock: 0,
  reorder_point: 0,
  vat_basis_points: 900,
  duty_basis_points: 0,
  tax_exempt: false,
  prices: levels.map((level) => ({ level: level.value, price: null })),
  units: [],
  tiers: [],
})

/** ورودی مبلغ: نمایش جداشده، ویرایش با رقم خام. */
function MoneyInput({
  value,
  onChange,
  disabled,
}: {
  value: number
  onChange: (value: number) => void
  disabled?: boolean
}) {
  return (
    <input
      key={value}
      defaultValue={value ? money(value) : ''}
      disabled={disabled}
      inputMode="numeric"
      placeholder="۰"
      onFocus={(event) => {
        event.currentTarget.value = value ? String(value) : ''
        event.currentTarget.select()
      }}
      onBlur={(event) => onChange(parseAmount(event.target.value) ?? 0)}
    />
  )
}

export function ProductForm({
  productId,
  onClose,
  onSaved,
}: {
  /** خالی یعنی کالای تازه. */
  productId?: string
  onClose: () => void
  onSaved: (id: string) => void
}) {
  const { t } = useI18n()
  const [tab, setTab] = useState<Tab>('general')
  const [kinds, setKinds] = useState<ProductKindOption[]>([])
  const [levels, setLevels] = useState<PriceLevelOption[]>([])
  const [groups, setGroups] = useState<ProductGroupRow[]>([])
  const [input, setInput] = useState<ProductInput | null>(null)
  const [detail, setDetail] = useState<ProductDetail>()
  const [goldRate, setGoldRate] = useState('')
  const [goldPreview, setGoldPreview] = useState<Record<string, number>>()
  const [error, setError] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    ;(async () => {
      try {
        const [options, groupRows] = await Promise.all([
          getProductKinds(),
          getProductGroups().catch(() => []),
        ])
        setKinds(options.kinds)
        setLevels(options.levels)
        setGroups(groupRows)

        if (productId) {
          const profile = await getProductProfile(productId)
          setDetail(profile)
          setInput({
            id: profile.id,
            kind: profile.kind,
            sku: profile.sku,
            barcode: profile.barcode,
            name: profile.name,
            display_name: profile.display_name,
            brand: profile.brand,
            group_id: profile.group_id,
            unit: profile.unit,
            purchase_price: profile.purchase_price,
            min_stock: profile.min_stock,
            max_stock: profile.max_stock,
            reorder_point: profile.reorder_point,
            vat_basis_points: profile.vat_basis_points,
            duty_basis_points: profile.duty_basis_points,
            tax_code: profile.tax_code,
            tax_exempt: profile.tax_exempt,
            prices: profile.prices.map((row) => ({ level: row.level, price: row.price })),
            units: profile.units,
            tiers: profile.tiers,
            gold: profile.gold,
          })
        } else {
          setInput(
            emptyInput(
              options.kinds[0]?.value ?? 'simple',
              options.levels,
              t('productForm.defaultUnit'),
            ),
          )
        }
      } catch (e) {
        setError(errorText(e))
      }
    })()
  }, [productId])

  const patch = (changes: Partial<ProductInput>) =>
    setInput((current) => (current ? { ...current, ...changes } : current))

  const isGold = input?.kind === 'gold_jewelry'
  const tracksInventory = useMemo(
    () => kinds.find((item) => item.value === input?.kind)?.tracks_inventory ?? true,
    [kinds, input?.kind],
  )

  const visibleTabs = TABS.filter((item) => {
    if (item.id === 'gold') return isGold
    if (item.id === 'stock' || item.id === 'units') return tracksInventory
    return true
  })

  const save = async () => {
    if (!input) return
    setSaving(true)
    setError('')
    try {
      const id = await saveProductProfile(input)
      onSaved(id)
    } catch (e) {
      setError(errorText(e))
      // زبانه‌ی مربوط به خطا باز شود تا کاربر دنبالش نگردد.
      const message = errorText(e)
      if (/PRD-00[789]|PRD-01[012]/.test(message)) setTab('prices')
      if (/PRD-007/.test(message)) setTab('tax')
      if (/PRD-01[678]/.test(message)) setTab('gold')
    } finally {
      setSaving(false)
    }
  }

  const runGoldPreview = async () => {
    if (!productId) return
    const rate = parseAmount(goldRate)
    if (!rate) return
    try {
      setGoldPreview(await previewGoldPrice(productId, rate))
    } catch (e) {
      setError(errorText(e))
    }
  }

  if (!input) {
    return (
      <div className="modal-backdrop" role="presentation">
        <div className="modal form-modal">
          <p className="empty-state">{t('common.loading')}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="modal-backdrop" role="presentation">
      <div className="modal party-modal">
        <div className="modal-head">
          <div>
            <div className="eyebrow">{t('productForm.eyebrow')}</div>
            <h2>{productId ? t('productForm.editTitle') : t('productForm.newTitle')}</h2>
            <p>{t('productForm.requiredNote')}</p>
          </div>
          <button aria-label={t('common.close')} type="button" className="icon-btn" onClick={onClose}>
            ✕
          </button>
        </div>

        {error && <div className="error-box">{error}</div>}

        <div className="tab-bar">
          {visibleTabs.map((item) => (
            <button
              key={item.id}
              type="button"
              className={tab === item.id ? 'active' : undefined}
              onClick={() => setTab(item.id)}
            >
              {t(item.labelKey)}
            </button>
          ))}
        </div>

        <div className="tab-body">
          {/* ------------------------------------------------ مشخصات عمومی */}
          {tab === 'general' && (
            <div className="form-grid">
              <label>
                {t('productForm.kindRequired')}
                <Select
                  value={input.kind}
                  aria-label={t('products.kind')}
                  onChange={(event) => patch({ kind: event.target.value })}
                >
                  {kinds.map((item) => (
                    <option key={item.value} value={item.value}>
                      {item.label}
                    </option>
                  ))}
                </Select>
              </label>
              <label>
                {t('productForm.skuRequired')}
                <input
                  value={input.sku}
                  onChange={(event) => patch({ sku: event.target.value })}
                  dir="ltr"
                />
              </label>
              <label>
                {t('productForm.barcode')}
                <input
                  value={input.barcode ?? ''}
                  onChange={(event) => patch({ barcode: event.target.value })}
                  dir="ltr"
                  placeholder={t('productForm.barcodeHint')}
                />
              </label>
              <label>
                {t('productForm.nameRequired')}
                <input value={input.name} onChange={(event) => patch({ name: event.target.value })} />
              </label>
              <label>
                {t('productForm.displayName')}
                <input
                  value={input.display_name ?? ''}
                  onChange={(event) => patch({ display_name: event.target.value })}
                  placeholder={t('productForm.displayNameHint')}
                />
              </label>
              <label>
                {t('productForm.brand')}
                <input
                  value={input.brand ?? ''}
                  onChange={(event) => patch({ brand: event.target.value })}
                />
              </label>
              <label>
                {t('products.group')}
                <Select
                  value={input.group_id ?? ''}
                  aria-label={t('products.group')}
                  onChange={(event) => patch({ group_id: event.target.value || undefined })}
                >
                  <option value="">{t('productForm.noGroup')}</option>
                  {groups.map((group) => (
                    <option key={group.id} value={group.id}>
                      {group.code} — {group.title}
                    </option>
                  ))}
                </Select>
              </label>
              <label>
                {t('productForm.mainUnitRequired')}
                <input value={input.unit} onChange={(event) => patch({ unit: event.target.value })} />
              </label>
              <label>
                {t('productForm.purchasePrice', { unit: rialUnit() })}
                <MoneyInput
                  value={input.purchase_price}
                  onChange={(value) => patch({ purchase_price: value })}
                />
              </label>
              <p className="hint">
                {tracksInventory
                  ? t('productForm.tracksInventory')
                  : t('productForm.serviceNote')}
              </p>
            </div>
          )}

          {/* ------------------------------------------------- سطوح قیمت */}
          {tab === 'prices' && (
            <>
              <p className="hint">
                {t('productForm.priceFallback')}
              </p>
              <div className="form-grid">
                {input.prices.map((row, index) => (
                  <label key={row.level}>
                    {levels.find((level) => level.value === row.level)?.label ?? row.level}
                    <MoneyInput
                      value={row.price ?? 0}
                      onChange={(value) => {
                        const next = [...input.prices]
                        next[index] = { ...row, price: value === 0 ? null : value }
                        patch({ prices: next })
                      }}
                    />
                  </label>
                ))}
              </div>
            </>
          )}

          {/* --------------------------------------------------- چند واحدی */}
          {tab === 'units' && (
            <>
              <div className="repeat-head">
                <h3 className="section-title">{t('productForm.subUnits')}</h3>
                <button
                  type="button"
                  className="table-action"
                  onClick={() =>
                    patch({
                      units: [
                        ...input.units,
                        { unit_name: '', factor: 1, is_default_sale: false } as ProductUnitRow,
                      ],
                    })
                  }
                >
                  <Plus className="size-3.5" aria-hidden /> {t('productForm.addUnit')}
                </button>
              </div>
              <p className="hint">
                {t('productForm.unitFactorHint')}
              </p>
              <div className="line-editor">
                {input.units.map((unit, index) => (
                  <div className="line-row" key={index}>
                    <label className="grow">
                      {t('productForm.unitName')}
                      <input
                        value={unit.unit_name}
                        onChange={(event) => {
                          const next = [...input.units]
                          next[index] = { ...unit, unit_name: event.target.value }
                          patch({ units: next })
                        }}
                      />
                    </label>
                    <label>
                      {t('productForm.unitFactor')}
                      <input
                        value={String(unit.factor)}
                        inputMode="decimal"
                        onChange={(event) => {
                          const next = [...input.units]
                          next[index] = { ...unit, factor: Number(event.target.value) || 0 }
                          patch({ units: next })
                        }}
                      />
                    </label>
                    <label className="inline-check">
                      <input
                        type="checkbox"
                        checked={unit.is_default_sale}
                        onChange={(event) => {
                          const next = input.units.map((item, position) => ({
                            ...item,
                            is_default_sale: position === index ? event.target.checked : false,
                          }))
                          patch({ units: next })
                        }}
                      />
                      {t('productForm.defaultSalesUnit')}
                    </label>
                    <button
                      type="button"
                      aria-label={t('productForm.removeUnit')}
                      className="icon-btn danger-icon"
                      onClick={() =>
                        patch({ units: input.units.filter((_, position) => position !== index) })
                      }
                    >
                      <Trash2 className="size-3.5" aria-hidden />
                    </button>
                  </div>
                ))}
                {input.units.length === 0 && (
                  <p className="empty-state">{t('productForm.noSubUnit')}</p>
                )}
              </div>
            </>
          )}

          {/* ---------------------------------------------- اطلاعات مالیاتی */}
          {tab === 'tax' && (
            <div className="form-grid">
              <label className="inline-check">
                <input
                  type="checkbox"
                  checked={input.tax_exempt}
                  onChange={(event) => patch({ tax_exempt: event.target.checked })}
                />
                {t('productForm.taxExemptItem')}
              </label>
              <label>
                {t('productForm.vatRate')}
                <Select
                  value={String(input.vat_basis_points)}
                  aria-label={t('productForm.vatRateShort')}
                  disabled={input.tax_exempt}
                  onChange={(event) => patch({ vat_basis_points: Number(event.target.value) })}
                >
                  <option value="0">{t('productForm.vat0')}</option>
                  <option value="900">{t('productForm.vat9')}</option>
                  <option value="1000">{t('productForm.vat10')}</option>
                </Select>
              </label>
              <label>
                {t('productForm.dutyRate')}
                <input
                  value={String(input.duty_basis_points)}
                  inputMode="numeric"
                  disabled={input.tax_exempt}
                  onChange={(event) =>
                    patch({ duty_basis_points: Number(event.target.value) || 0 })
                  }
                />
              </label>
              <label>
                {t('productForm.taxCode')}
                <input
                  value={input.tax_code ?? ''}
                  onChange={(event) => patch({ tax_code: event.target.value })}
                  dir="ltr"
                />
              </label>
              <p className="hint">
                {t('productForm.taxHint')}
              </p>
            </div>
          )}

          {/* -------------------------------------------- موجودی و سفارش */}
          {tab === 'stock' && (
            <>
              <div className="form-grid">
                <label>
                  {t('productForm.minStock')}
                  <input
                    value={String(input.min_stock)}
                    inputMode="decimal"
                    onChange={(event) => patch({ min_stock: Number(event.target.value) || 0 })}
                  />
                </label>
                <label>
                  {t('productForm.maxStock')}
                  <input
                    value={String(input.max_stock)}
                    inputMode="decimal"
                    onChange={(event) => patch({ max_stock: Number(event.target.value) || 0 })}
                  />
                </label>
                <label>
                  {t('productForm.reorderPoint')}
                  <input
                    value={String(input.reorder_point)}
                    inputMode="decimal"
                    onChange={(event) => patch({ reorder_point: Number(event.target.value) || 0 })}
                  />
                </label>
                <p className="hint">
                  {t('productForm.reorderHint')}
                </p>
              </div>

              {detail && detail.stock.length > 0 && (
                <>
                  <h3 className="section-title">{t('productForm.stockByWarehouse')}</h3>
                  <table className="mini-table">
                    <thead>
                      <tr>
                        <th>{t('common.warehouse')}</th>
                        <th className="num">{t('products.stock')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {detail.stock.map((row) => (
                        <tr key={row.warehouse_id}>
                          <td>{row.warehouse_name}</td>
                          <td className="num">{formatNumber(row.quantity)}</td>
                        </tr>
                      ))}
                      <tr className="total-row">
                        <td>{t('common.total')}</td>
                        <td className="num">{formatNumber(detail.total_stock)}</td>
                      </tr>
                    </tbody>
                  </table>
                </>
              )}
            </>
          )}

          {/* ------------------------------------------------ تخفیف پلکانی */}
          {tab === 'tiers' && (
            <>
              <div className="repeat-head">
                <h3 className="section-title">{t('productForm.tiersTitle')}</h3>
                <button
                  type="button"
                  className="table-action"
                  onClick={() =>
                    patch({
                      tiers: [...input.tiers, { min_quantity: 1, discount_bp: 0 } as ProductTierRow],
                    })
                  }
                >
                  <Plus className="size-3.5" aria-hidden /> {t('productForm.addTier')}
                </button>
              </div>
              <p className="hint">
                {t('productForm.tiersHint')}
              </p>
              <div className="line-editor">
                {input.tiers.map((tier, index) => (
                  <div className="line-row" key={index}>
                    <label>
                      {t('productForm.fromQuantity')}
                      <input
                        value={String(tier.min_quantity)}
                        inputMode="decimal"
                        onChange={(event) => {
                          const next = [...input.tiers]
                          next[index] = { ...tier, min_quantity: Number(event.target.value) || 0 }
                          patch({ tiers: next })
                        }}
                      />
                    </label>
                    <label>
                      {t('productForm.discountPercent')}
                      <input
                        value={String(tier.discount_bp / 100)}
                        inputMode="decimal"
                        onChange={(event) => {
                          const next = [...input.tiers]
                          next[index] = {
                            ...tier,
                            discount_bp: Math.round((Number(event.target.value) || 0) * 100),
                          }
                          patch({ tiers: next })
                        }}
                      />
                    </label>
                    <button
                      type="button"
                      aria-label={t('productForm.removeTier')}
                      className="icon-btn danger-icon"
                      onClick={() =>
                        patch({ tiers: input.tiers.filter((_, position) => position !== index) })
                      }
                    >
                      <Trash2 className="size-3.5" aria-hidden />
                    </button>
                  </div>
                ))}
                {input.tiers.length === 0 && <p className="empty-state">{t('productForm.noTier')}</p>}
              </div>
            </>
          )}

          {/* -------------------------------------------------- طلا و جواهر */}
          {tab === 'gold' && (
            <>
              <div className="form-grid">
                <label>
                  {t('productForm.weightRequired')}
                  <input
                    value={String(input.gold?.weight_grams ?? '')}
                    inputMode="decimal"
                    onChange={(event) =>
                      patch({
                        gold: {
                          ...(input.gold ?? { carat: 18, making_charge_bp: 0, profit_bp: 0 }),
                          weight_grams: Number(event.target.value) || 0,
                        } as ProductGold,
                      })
                    }
                  />
                </label>
                <label>
                  {t('productForm.carat')}
                  <Select
                    value={String(input.gold?.carat ?? 18)}
                    aria-label={t('productForm.carat')}
                    onChange={(event) =>
                      patch({
                        gold: {
                          ...(input.gold ?? {
                            weight_grams: 0,
                            making_charge_bp: 0,
                            profit_bp: 0,
                          }),
                          carat: Number(event.target.value),
                        } as ProductGold,
                      })
                    }
                  >
                    <option value="18">{t('productForm.carat18')}</option>
                    <option value="21">{t('productForm.carat21')}</option>
                    <option value="24">{t('productForm.carat24')}</option>
                  </Select>
                </label>
                <label>
                  {t('productForm.makingCharge')}
                  <input
                    value={String((input.gold?.making_charge_bp ?? 0) / 100)}
                    inputMode="decimal"
                    onChange={(event) =>
                      patch({
                        gold: {
                          ...(input.gold ?? { weight_grams: 0, carat: 18, profit_bp: 0 }),
                          making_charge_bp: Math.round((Number(event.target.value) || 0) * 100),
                        } as ProductGold,
                      })
                    }
                  />
                </label>
                <label>
                  {t('productForm.sellerProfit')}
                  <input
                    value={String((input.gold?.profit_bp ?? 0) / 100)}
                    inputMode="decimal"
                    onChange={(event) =>
                      patch({
                        gold: {
                          ...(input.gold ?? { weight_grams: 0, carat: 18, making_charge_bp: 0 }),
                          profit_bp: Math.round((Number(event.target.value) || 0) * 100),
                        } as ProductGold,
                      })
                    }
                  />
                </label>
              </div>

              {productId && (
                <>
                  <h3 className="section-title">{t('productForm.goldPricing')}</h3>
                  <div className="line-row">
                    <label className="grow">
                      {t('productForm.goldRate', { unit: rialUnit() })}
                      <input
                        value={goldRate}
                        inputMode="numeric"
                        onChange={(event) => setGoldRate(event.target.value)}
                      />
                    </label>
                    <button type="button" className="table-action" onClick={runGoldPreview}>
                      {t('productForm.calculate')}
                    </button>
                  </div>
                  {goldPreview && (
                    <div className="inline-summary">
                      <span>
                        {t('productForm.metalValue')} <b>{money(goldPreview.metal_value)}</b>
                      </span>
                      <span>
                        {t('productForm.makingValue')} <b>{money(goldPreview.making_charge)}</b>
                      </span>
                      <span>
                        {t('productForm.profitValue')} <b>{money(goldPreview.profit)}</b>
                      </span>
                      <span>
                        {t('productForm.vatValue')} <b>{money(goldPreview.vat)}</b>
                      </span>
                      <span>
                        {t('productForm.payable')} <b>{money(goldPreview.total)} ریال</b>
                      </span>
                    </div>
                  )}
                </>
              )}
            </>
          )}
        </div>

        <div className="modal-actions">
          <button type="button" className="secondary" onClick={onClose}>
            {t('common.cancel')}
          </button>
          <button type="button" className="primary" disabled={saving} onClick={save}>
            {saving ? t('productForm.saving') : t('productForm.save')}
          </button>
          {detail && (
            <Badge tone="neutral" dot={false}>
              {detail.kind_label}
            </Badge>
          )}
        </div>
      </div>
    </div>
  )
}
