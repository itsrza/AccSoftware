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
import type { FormulaComponentInput, FormulaDetail, Product } from '../api'

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
  return (
    <>
    {formulaForm && (
      <div className="modal-backdrop" onClick={() => setFormulaForm(null)}>
        <div className="modal party-modal" onClick={(e) => e.stopPropagation()}>
          <div className="modal-head">
            <div>
              <h2>فرمول تولید</h2>
              <p>ضایعات بخشی از بهای تمام‌شده است، نه هزینه‌ی جداگانه.</p>
            </div>
            <button aria-label="بستن" className="icon-btn" onClick={() => setFormulaForm(null)}>
              <Icon name="close" />
            </button>
          </div>
          <div className="tab-body">
            <div className="filter-grid">
              <label className="grow">
                <span>محصول تولیدی *</span>
                <select
                  value={formulaForm.product_id}
                  onChange={(e) => setFormulaForm({ ...formulaForm, product_id: e.target.value })}
                >
                  <option value="">انتخاب کنید…</option>
                  {products.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </label>
              <label className="grow">
                <span>عنوان فرمول *</span>
                <input
                  value={formulaForm.title}
                  onChange={(e) => setFormulaForm({ ...formulaForm, title: e.target.value })}
                  placeholder="فرمول استاندارد"
                />
              </label>
              <label>
                <span>مقدار تولید فرمول</span>
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
              <h4 className="section-title">اجزای فرمول</h4>
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
                <Icon name="plus" /> افزودن جزء
              </button>
            </div>
            {formulaForm.components.map((component, index) => (
              <div className="line-row" key={component.key}>
                <label className="grow">
                  <span>ماده</span>
                  <select
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
                    <option value="">انتخاب کنید…</option>
                    {products
                      .filter((p) => p.id !== formulaForm.product_id)
                      .map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                  </select>
                </label>
                <label>
                  <span>مصرف هر واحد</span>
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
                  <span>ضایعات (٪)</span>
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
                <button aria-label="حذف"
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
              محصول نمی‌تواند در اجزای فرمول خودش باشد — بهای تمام‌شده‌اش به خودش وابسته
              می‌شود و محاسبه بی‌معنا می‌گردد.
            </p>
          </div>
          <div className="modal-actions">
            <button
              className="primary"
              onClick={onSave}
              disabled={busy || !formulaForm.product_id || !formulaForm.title.trim()}
            >
              ذخیره
            </button>
            <button className="ghost" onClick={() => setFormulaForm(null)}>
              انصراف
            </button>
          </div>
        </div>
      </div>
    )}

    {formulaDetail && (
      <div className="modal-backdrop" onClick={() => setFormulaDetail(undefined)}>
        <div className="modal form-modal" onClick={(e) => e.stopPropagation()}>
          <div className="modal-head">
            <div>
              <h2>{formulaDetail.header.title}</h2>
              <p>
                محصول: {formulaDetail.header.product_name} — بهای برآوردی واحد:{' '}
                {money(formulaDetail.header.estimated_unit_cost)} ریال
              </p>
            </div>
            <button aria-label="بستن"
              className="icon-btn"
              onClick={() => setFormulaDetail(undefined)}
           
            >
              <Icon name="close" />
            </button>
          </div>
          <table className="mini-table">
            <thead>
              <tr>
                <th>ماده</th>
                <th>مصرف پایه</th>
                <th>ضایعات</th>
                <th>مصرف واقعی</th>
                <th>بهای واحد</th>
                <th>موجودی</th>
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
