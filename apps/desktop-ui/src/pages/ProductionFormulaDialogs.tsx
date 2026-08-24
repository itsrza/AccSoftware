/**
 * پنجره‌های فرمول تولید — ساخت/ویرایش و مشاهده‌ی جزئیات.
 *
 * ## چرا از صفحه‌ی تولید جدا شد
 *
 * صفحه‌ی تولید سه مسئولیت داشت: رسید تولید، فهرست فرمول‌ها و دو پنجره‌ی
 * فرمول. با نهصد خط، مرور کد و پیدا کردن باگ عملاً غیرممکن می‌شد.
 * پنجره‌ها اینجا آمدند و صفحه‌ی اصلی فقط جریان رسید تولید را نگه داشت.
 */
import { Icon } from '../components/Icon'
import { formatRials as money } from '../lib/format'
import {useI18n} from '../lib/i18n'
import type { FormulaComponentInput, FormulaDetail, Product } from '../api'
import {Select} from '../components/Select'

export type FormulaDraft = {
  id?: string
  product_id: string
  title: string
  output_quantity: number
  components: (FormulaComponentInput & { key: number })[]
}

let nextKey = 1

export function ProductionFormulaDialogs({
  formulaForm,
  setFormulaForm,
  formulaDetail,
  setFormulaDetail,
  products,
  busy,
  onSave,
}: {
  formulaForm: FormulaDraft | null
  setFormulaForm: (value: FormulaDraft | null) => void
  formulaDetail: FormulaDetail | undefined
  setFormulaDetail: (value: FormulaDetail | undefined) => void
  products: Product[]
  busy: boolean
  onSave: () => void
}) {
  const { t } = useI18n()
  return (
    <>
    {formulaForm && (
      <div className="modal-backdrop" role="presentation">
        <div className="modal party-modal">
          <div className="modal-head">
            <div>
              <h2>{t('prodDlg.title')}</h2>
              <p>{t('prodDlg.wasteNote')}</p>
            </div>
            <button aria-label={t('common.close')} className="icon-btn" onClick={() => setFormulaForm(null)}>
              <Icon name="close" />
            </button>
          </div>
          <div className="tab-body">
            <div className="filter-grid">
              <label className="grow">
                <span>{t('prodDlg.productRequired')}</span>
                <Select
                  value={formulaForm.product_id}
                  onChange={(e) => setFormulaForm({ ...formulaForm, product_id: e.target.value })}
                >
                  <option value="">{t('invoiceForm.selectPlaceholder')}</option>
                  {products.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </Select>
              </label>
              <label className="grow">
                <span>{t('prodDlg.titleRequired')}</span>
                <input
                  value={formulaForm.title}
                  onChange={(e) => setFormulaForm({ ...formulaForm, title: e.target.value })}
                  placeholder={t('prodDlg.standardFormula')}
                />
              </label>
              <label>
                <span>{t('prodDlg.batchQuantity')}</span>
                <input
                  type="number"
                  min={0}
                  step="any"
                  value={formulaForm.output_quantity || ''}
                  onChange={(e) =>
                    setFormulaForm({
                      ...formulaForm,
                      output_quantity: Number(e.target.value) || 0,
                    })
                  }
                />
              </label>
            </div>

            <div className="repeat-head">
              <h4 className="section-title">{t('prodDlg.components')}</h4>
              <button
                className="ghost"
                onClick={() =>
                  setFormulaForm({
                    ...formulaForm,
                    components: [
                      ...formulaForm.components,
                      { key: nextKey++, product_id: '', quantity_per_unit: 1, waste_percent: 0 },
                    ],
                  })
                }
              >
                <Icon name="plus" /> {t('prodDlg.addComponent')}
              </button>
            </div>
            {formulaForm.components.map((component, index) => (
              <div className="line-row" key={component.key}>
                <label className="grow">
                  <span>{t('prod.material')}</span>
                  <Select
                    value={component.product_id}
                    onChange={(e) =>
                      setFormulaForm({
                        ...formulaForm,
                        components: formulaForm.components.map((c, i) =>
                          i === index ? { ...c, product_id: e.target.value } : c,
                        ),
                      })
                    }
                  >
                    <option value="">{t('invoiceForm.selectPlaceholder')}</option>
                    {products
                      .filter((p) => p.id !== formulaForm.product_id)
                      .map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                  </Select>
                </label>
                <label>
                  <span>{t('prodDlg.perUnit')}</span>
                  <input
                    type="number"
                    min={0}
                    step="any"
                    value={component.quantity_per_unit || ''}
                    onChange={(e) =>
                      setFormulaForm({
                        ...formulaForm,
                        components: formulaForm.components.map((c, i) =>
                          i === index
                            ? { ...c, quantity_per_unit: Number(e.target.value) || 0 }
                            : c,
                        ),
                      })
                    }
                  />
                </label>
                <label>
                  <span>{t('prodDlg.waste')}</span>
                  <input
                    type="number"
                    min={0}
                    max={99}
                    step="any"
                    value={component.waste_percent || ''}
                    onChange={(e) =>
                      setFormulaForm({
                        ...formulaForm,
                        components: formulaForm.components.map((c, i) =>
                          i === index ? { ...c, waste_percent: Number(e.target.value) || 0 } : c,
                        ),
                      })
                    }
                  />
                </label>
                <button aria-label={t('partyForm.remove')}
                  className="icon-btn danger-icon"
                  disabled={formulaForm.components.length === 1}
                  onClick={() =>
                    setFormulaForm({
                      ...formulaForm,
                      components: formulaForm.components.filter((_, i) => i !== index),
                    })
                  }
               
                >
                  <Icon name="trash" />
                </button>
              </div>
            ))}
            <p className="hint">
              {t('prodDlg.selfRefNote')}
            </p>
          </div>
          <div className="modal-actions">
            <button
              className="primary"
              onClick={onSave}
              disabled={busy || !formulaForm.product_id || !formulaForm.title.trim()}
            >
              {t('common.save')}
            </button>
            <button className="ghost" onClick={() => setFormulaForm(null)}>
              {t('common.cancel')}
            </button>
          </div>
        </div>
      </div>
    )}

    {formulaDetail && (
      <div className="modal-backdrop" onClick={() => setFormulaDetail(undefined)}>
        <div className="modal form-modal">
          <div className="modal-head">
            <div>
              <h2>{formulaDetail.header.title}</h2>
              <p>
                محصول: {formulaDetail.header.product_name} — بهای برآوردی واحد:{' '}
                {money(formulaDetail.header.estimated_unit_cost)} ریال
              </p>
            </div>
            <button aria-label={t('common.close')}
              className="icon-btn"
              onClick={() => setFormulaDetail(undefined)}
           
            >
              <Icon name="close" />
            </button>
          </div>
          <table className="mini-table">
            <thead>
              <tr>
                <th>{t('prod.material')}</th>
                <th>{t('prodDlg.baseConsumption')}</th>
                <th>{t('prodDlg.wasteShort')}</th>
                <th>{t('prodDlg.actualConsumption')}</th>
                <th>{t('ops.unitCost')}</th>
                <th>{t('products.stock')}</th>
              </tr>
            </thead>
            <tbody>
              {formulaDetail.components.map((component) => (
                <tr key={component.id}>
                  <td>{component.product_name}</td>
                  <td className="num">
                    {component.quantity_per_unit} {component.unit}
                  </td>
                  <td className="num">{component.waste_percent}٪</td>
                  <td className="num">{component.effective_quantity.toFixed(4)}</td>
                  <td className="num">{money(component.unit_cost)}</td>
                  <td className={`num${component.available_stock <= 0 ? ' red-text' : ''}`}>
                    {component.available_stock}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="muted">
            با موجودی فعلی می‌توان {formulaDetail.header.producible_now.toFixed(2)} واحد تولید
            کرد. ماده‌ای که کمترین ظرفیت را می‌دهد گلوگاه تولید است.
          </p>
        </div>
      </div>
    )}
    </>
  )
}
