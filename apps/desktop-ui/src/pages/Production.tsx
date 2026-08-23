import { useCallback, useEffect, useMemo, useState } from 'react'
import { Icon } from '../components/Icon'
import {
  deleteProductionFormula,
  expandProductionFormula,
  getCostAllocations,
  getProductionExpenseAccounts,
  getProductionFormula,
  getProductionFormulas,
  getProductionOrders,
  getProducts,
  getWarehouses,
  postProduction,
  previewProduction,
  saveProductionFormula,
  AllocationInfo,
  CostingPreview,
  ExpenseAccountRow,
  FormulaComponentInput,
  FormulaDetail,
  FormulaRow,
  Product,
  ProductionInput,
  ProductionOrderRow,
  Warehouse,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials as money } from '../lib/format'

type Tab = 'receipt' | 'formulas'
type Line = { key: number; product_id: string; quantity: number; market_unit_price?: number }
type Expense = { key: number; account_id: string; title: string; amount: number }

let nextKey = 1
const blankLine = (): Line => ({ key: nextKey++, product_id: '', quantity: 0 })
const blankExpense = (): Expense => ({ key: nextKey++, account_id: '', title: '', amount: 0 })

/**
 * رسید تولید و فرمول تولید.
 *
 * معادله‌ای که این صفحه اجرا می‌کند:
 * `مواد مصرفی + هزینه‌های تولید = بهای تمام‌شده‌ی محصولات`
 *
 * تولید سود نمی‌سازد؛ فقط شکل دارایی از «مواد اولیه» به «کالای ساخته‌شده»
 * تغییر می‌کند. سود در لحظه‌ی فروش محقق می‌شود.
 */
export function Production() {
  const [tab, setTab] = useState<Tab>('receipt')
  const [products, setProducts] = useState<Product[]>([])
  const [warehouses, setWarehouses] = useState<Warehouse[]>([])
  const [accounts, setAccounts] = useState<ExpenseAccountRow[]>([])
  const [allocations, setAllocations] = useState<AllocationInfo[]>([])
  const [formulas, setFormulas] = useState<FormulaRow[]>([])
  const [orders, setOrders] = useState<ProductionOrderRow[]>([])

  // --- رسید تولید ---
  const [productionDate, setProductionDate] = useState('')
  const [warehouseId, setWarehouseId] = useState('')
  const [allocation, setAllocation] = useState('by_quantity')
  const [formulaId, setFormulaId] = useState('')
  const [formulaQuantity, setFormulaQuantity] = useState(1)
  const [description, setDescription] = useState('')
  const [inputs, setInputs] = useState<Line[]>([blankLine()])
  const [outputs, setOutputs] = useState<Line[]>([blankLine()])
  const [expenses, setExpenses] = useState<Expense[]>([])
  const [preview, setPreview] = useState<CostingPreview>()

  // --- فرمول ---
  const [formulaForm, setFormulaForm] = useState<{
    id?: string
    product_id: string
    title: string
    output_quantity: number
    components: (FormulaComponentInput & { key: number })[]
  } | null>(null)
  const [formulaDetail, setFormulaDetail] = useState<FormulaDetail>()

  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const [busy, setBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      setFormulas(await getProductionFormulas())
      setOrders(await getProductionOrders())
      setError('')
    } catch (e) {
      setError(errorText(e))
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  useEffect(() => {
    ;(async () => {
      try {
        setProducts(await getProducts())
        setWarehouses((await getWarehouses()).filter((w) => w.is_active !== false))
        setAccounts(await getProductionExpenseAccounts())
        setAllocations(await getCostAllocations())
      } catch (e) {
        setError(errorText(e))
      }
    })()
  }, [])

  const draft: ProductionInput = useMemo(
    () => ({
      production_date: productionDate,
      warehouse_id: warehouseId,
      formula_id: formulaId || undefined,
      cost_allocation: allocation,
      description: description || undefined,
      inputs: inputs
        .filter((line) => line.product_id && line.quantity > 0)
        .map((line) => ({ product_id: line.product_id, quantity: line.quantity })),
      outputs: outputs
        .filter((line) => line.product_id && line.quantity > 0)
        .map((line) => ({
          product_id: line.product_id,
          quantity: line.quantity,
          market_unit_price: line.market_unit_price,
        })),
      expenses: expenses
        .filter((line) => line.account_id && line.amount > 0)
        .map((line) => ({ account_id: line.account_id, title: line.title || 'هزینه تولید', amount: line.amount })),
    }),
    [productionDate, warehouseId, formulaId, allocation, description, inputs, outputs, expenses],
  )

  // پیش‌نمایش بهای تمام‌شده از موتور می‌آید، نه از محاسبه‌ی مرورگر.
  useEffect(() => {
    if (draft.inputs.length === 0 || draft.outputs.length === 0 || !warehouseId) {
      setPreview(undefined)
      return
    }
    let cancelled = false
    const timer = setTimeout(async () => {
      try {
        const result = await previewProduction(draft)
        if (!cancelled) {
          setPreview(result)
          setError('')
        }
      } catch (e) {
        if (!cancelled) {
          setPreview(undefined)
          setError(errorText(e))
        }
      }
    }, 300)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [draft, warehouseId])

  /** پرکردن مواد مصرفی از روی فرمول — همان دکمه‌ی «استفاده از فرمول تولید». */
  const applyFormula = async () => {
    if (!formulaId || formulaQuantity <= 0) return
    setBusy(true)
    try {
      const expanded = await expandProductionFormula(formulaId, formulaQuantity)
      setInputs(
        expanded.map((component) => ({
          key: nextKey++,
          product_id: component.product_id,
          quantity: Number(component.required_quantity.toFixed(4)),
        })),
      )
      const formula = formulas.find((f) => f.id === formulaId)
      if (formula) {
        setOutputs([{ key: nextKey++, product_id: formula.product_id, quantity: formulaQuantity }])
      }
      setNotice('مواد مصرفی از فرمول پر شد. مقدارها با احتساب ضایعات محاسبه شده‌اند.')
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const submit = async () => {
    setBusy(true)
    setNotice('')
    try {
      const id = await postProduction(draft)
      setNotice(`رسید تولید ثبت شد (${id}). سند حسابداری و گردش انبار صادر شد.`)
      setInputs([blankLine()])
      setOutputs([blankLine()])
      setExpenses([])
      setPreview(undefined)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const saveFormula = async () => {
    if (!formulaForm) return
    setBusy(true)
    try {
      await saveProductionFormula({
        id: formulaForm.id,
        product_id: formulaForm.product_id,
        title: formulaForm.title,
        output_quantity: formulaForm.output_quantity,
        components: formulaForm.components.map(({ key: _key, ...rest }) => rest),
      })
      setNotice('فرمول تولید ذخیره شد.')
      setFormulaForm(null)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const nameOf = (id: string) => products.find((p) => p.id === id)?.name ?? id
  const canSubmit =
    !busy &&
    productionDate.trim() !== '' &&
    warehouseId !== '' &&
    !!preview &&
    preview.warnings.length === 0 &&
    preview.total_cost >= 0

  const selectedAllocation = allocations.find((a) => a.value === allocation)

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">عملیات</div>
          <h1>تولید</h1>
          <p>
            مواد مصرفی + هزینه‌های تولید = بهای تمام‌شده‌ی محصولات. تولید سود نمی‌سازد؛ فقط شکل
            دارایی عوض می‌شود — سود در لحظه‌ی فروش محقق می‌شود.
          </p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {notice && <div className="success-box">{notice}</div>}

      <div className="tab-bar">
        <button className={tab === 'receipt' ? 'active' : ''} onClick={() => setTab('receipt')}>
          رسید تولید
        </button>
        <button className={tab === 'formulas' ? 'active' : ''} onClick={() => setTab('formulas')}>
          فرمول‌های تولید
        </button>
      </div>

      {tab === 'receipt' && (
        <>
          <div className="panel">
            <div className="filter-grid">
              <label>
                <span>تاریخ تولید *</span>
                <input
                  value={productionDate}
                  onChange={(e) => setProductionDate(e.target.value)}
                  placeholder="1405/06/15"
                />
              </label>
              <label>
                <span>انبار *</span>
                <select value={warehouseId} onChange={(e) => setWarehouseId(e.target.value)}>
                  <option value="">انتخاب کنید…</option>
                  {warehouses.map((w) => (
                    <option key={w.id} value={w.id}>
                      {w.name}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>روش تخصیص بها</span>
                <select value={allocation} onChange={(e) => setAllocation(e.target.value)}>
                  {allocations.map((a) => (
                    <option key={a.value} value={a.value}>
                      {a.label}
                    </option>
                  ))}
                </select>
              </label>
              <label className="grow">
                <span>توضیح</span>
                <input value={description} onChange={(e) => setDescription(e.target.value)} />
              </label>
              {selectedAllocation && <p className="hint">{selectedAllocation.explanation}</p>}
            </div>
          </div>

          <div className="panel">
            <div className="panel-head">
              <div>
                <h3>استفاده از فرمول تولید</h3>
                <p>مواد مصرفی خودکار و با احتساب ضایعات پر می‌شود.</p>
              </div>
            </div>
            <div className="filter-grid">
              <label className="grow">
                <span>فرمول</span>
                <select value={formulaId} onChange={(e) => setFormulaId(e.target.value)}>
                  <option value="">بدون فرمول (ورود دستی)</option>
                  {formulas
                    .filter((f) => f.is_active)
                    .map((f) => (
                      <option key={f.id} value={f.id}>
                        {f.product_name} — {f.title} (قابل تولید: {f.producible_now.toFixed(1)})
                      </option>
                    ))}
                </select>
              </label>
              <label>
                <span>مقدار تولید</span>
                <input
                  type="number"
                  min={0}
                  step="any"
                  value={formulaQuantity || ''}
                  onChange={(e) => setFormulaQuantity(Number(e.target.value) || 0)}
                />
              </label>
              <div className="filter-actions">
                <button
                  className="ghost"
                  onClick={applyFormula}
                  disabled={busy || !formulaId || formulaQuantity <= 0}
                >
                  <Icon name="arrow" /> اعمال فرمول
                </button>
              </div>
            </div>
          </div>

          <div className="panel">
            <div className="panel-head">
              <div>
                <h3>مواد مصرفی</h3>
                <p>بهای هر ماده از بهای تمام‌شده‌ی همان کالا در انبار خوانده می‌شود.</p>
              </div>
              <button className="ghost" onClick={() => setInputs((c) => [...c, blankLine()])}>
                <Icon name="plus" /> افزودن ماده
              </button>
            </div>
            {inputs.map((line) => (
              <div className="line-row" key={line.key}>
                <label className="grow">
                  <span>ماده</span>
                  <select
                    value={line.product_id}
                    onChange={(e) =>
                      setInputs((c) =>
                        c.map((l) => (l.key === line.key ? { ...l, product_id: e.target.value } : l)),
                      )
                    }
                  >
                    <option value="">انتخاب کنید…</option>
                    {products.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.name}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  <span>مقدار</span>
                  <input
                    type="number"
                    min={0}
                    step="any"
                    value={line.quantity || ''}
                    onChange={(e) =>
                      setInputs((c) =>
                        c.map((l) =>
                          l.key === line.key ? { ...l, quantity: Number(e.target.value) || 0 } : l,
                        ),
                      )
                    }
                  />
                </label>
                <button
                  className="icon-btn danger-icon"
                  disabled={inputs.length === 1}
                  onClick={() => setInputs((c) => c.filter((l) => l.key !== line.key))}
                  aria-label="حذف"
                >
                  <Icon name="trash" />
                </button>
              </div>
            ))}
          </div>

          <div className="panel">
            <div className="panel-head">
              <div>
                <h3>کالاهای تولید شده</h3>
                <p>
                  {allocation === 'by_market_value'
                    ? 'برای تخصیص بر اساس ارزش، قیمت بازار هر محصول را وارد کنید.'
                    : 'بهای تمام‌شده به نسبت مقدار بین محصولات تقسیم می‌شود.'}
                </p>
              </div>
              <button className="ghost" onClick={() => setOutputs((c) => [...c, blankLine()])}>
                <Icon name="plus" /> افزودن محصول
              </button>
            </div>
            {outputs.map((line) => (
              <div className="line-row" key={line.key}>
                <label className="grow">
                  <span>محصول</span>
                  <select
                    value={line.product_id}
                    onChange={(e) =>
                      setOutputs((c) =>
                        c.map((l) => (l.key === line.key ? { ...l, product_id: e.target.value } : l)),
                      )
                    }
                  >
                    <option value="">انتخاب کنید…</option>
                    {products.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.name}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  <span>مقدار</span>
                  <input
                    type="number"
                    min={0}
                    step="any"
                    value={line.quantity || ''}
                    onChange={(e) =>
                      setOutputs((c) =>
                        c.map((l) =>
                          l.key === line.key ? { ...l, quantity: Number(e.target.value) || 0 } : l,
                        ),
                      )
                    }
                  />
                </label>
                {allocation === 'by_market_value' && (
                  <label>
                    <span>قیمت بازار واحد</span>
                    <input
                      type="number"
                      min={0}
                      value={line.market_unit_price || ''}
                      onChange={(e) =>
                        setOutputs((c) =>
                          c.map((l) =>
                            l.key === line.key
                              ? { ...l, market_unit_price: Number(e.target.value) || 0 }
                              : l,
                          ),
                        )
                      }
                    />
                  </label>
                )}
                <button
                  className="icon-btn danger-icon"
                  disabled={outputs.length === 1}
                  onClick={() => setOutputs((c) => c.filter((l) => l.key !== line.key))}
                  aria-label="حذف"
                >
                  <Icon name="trash" />
                </button>
              </div>
            ))}
          </div>

          <div className="panel">
            <div className="panel-head">
              <div>
                <h3>هزینه‌های تولید</h3>
                <p>دستمزد، سربار و انرژی — هر کدام به حساب هزینه‌ی خودش می‌نشیند.</p>
              </div>
              <button className="ghost" onClick={() => setExpenses((c) => [...c, blankExpense()])}>
                <Icon name="plus" /> افزودن هزینه
              </button>
            </div>
            {expenses.map((line) => (
              <div className="line-row" key={line.key}>
                <label className="grow">
                  <span>حساب هزینه</span>
                  <select
                    value={line.account_id}
                    onChange={(e) =>
                      setExpenses((c) =>
                        c.map((l) => (l.key === line.key ? { ...l, account_id: e.target.value } : l)),
                      )
                    }
                  >
                    <option value="">انتخاب کنید…</option>
                    {accounts.map((a) => (
                      <option key={a.id} value={a.id}>
                        {a.code} — {a.name}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="grow">
                  <span>شرح</span>
                  <input
                    value={line.title}
                    onChange={(e) =>
                      setExpenses((c) =>
                        c.map((l) => (l.key === line.key ? { ...l, title: e.target.value } : l)),
                      )
                    }
                  />
                </label>
                <label>
                  <span>مبلغ (ریال)</span>
                  <input
                    type="number"
                    min={0}
                    value={line.amount || ''}
                    onChange={(e) =>
                      setExpenses((c) =>
                        c.map((l) =>
                          l.key === line.key ? { ...l, amount: Number(e.target.value) || 0 } : l,
                        ),
                      )
                    }
                  />
                </label>
                <button
                  className="icon-btn danger-icon"
                  onClick={() => setExpenses((c) => c.filter((l) => l.key !== line.key))}
                  aria-label="حذف"
                >
                  <Icon name="trash" />
                </button>
              </div>
            ))}
            {expenses.length === 0 && <p className="muted">هزینه‌ای ثبت نشده است.</p>}
          </div>

          {preview && (
            <div className="panel">
              <div className="panel-head">
                <div>
                  <h3>بهای تمام‌شده</h3>
                  <p>این دقیقاً همان محاسبه‌ای است که هنگام ثبت اعمال می‌شود.</p>
                </div>
              </div>

              {preview.warnings.length > 0 && (
                <div className="warn-box">
                  {preview.warnings.map((warning) => (
                    <div key={warning}>{warning}</div>
                  ))}
                </div>
              )}

              <div className="inline-summary">
                <span>
                  مواد مصرفی: <b>{money(preview.materials_total)}</b>
                </span>
                <span>
                  هزینه‌های تولید: <b>{money(preview.expenses_total)}</b>
                </span>
                <span>
                  جمع بهای تمام‌شده: <b>{money(preview.total_cost)} ریال</b>
                </span>
              </div>

              <table className="mini-table">
                <thead>
                  <tr>
                    <th>محصول</th>
                    <th>مقدار</th>
                    <th>سهم از بهای تمام‌شده</th>
                    <th>بهای واحد جدید</th>
                    <th>بهای واحد قبلی</th>
                  </tr>
                </thead>
                <tbody>
                  {preview.outputs.map((row) => (
                    <tr key={row.product_id}>
                      <td>{row.product_name}</td>
                      <td className="num">{row.quantity}</td>
                      <td className="num">{money(row.allocated_cost)}</td>
                      <td className="num">{money(row.unit_cost)}</td>
                      <td className="num">
                        {row.previous_unit_cost ? money(row.previous_unit_cost) : '—'}
                      </td>
                    </tr>
                  ))}
                  <tr className="total-row">
                    <td colSpan={2}>جمع</td>
                    <td className="num">
                      {money(preview.outputs.reduce((sum, r) => sum + r.allocated_cost, 0))}
                    </td>
                    <td colSpan={2} />
                  </tr>
                </tbody>
              </table>

              <div className="modal-actions">
                <button className="primary" onClick={submit} disabled={!canSubmit}>
                  ثبت رسید تولید
                </button>
              </div>
            </div>
          )}

          <div className="panel list-panel">
            <div className="panel-head">
              <div>
                <h3>رسیدهای تولید</h3>
                <p>{orders.length} مورد</p>
              </div>
              <button className="icon-btn" onClick={load} aria-label="بروزرسانی">
                <Icon name="refresh" />
              </button>
            </div>
            <div className="table-wrap">
              <table className="large-table">
                <thead>
                  <tr>
                    <th>شماره</th>
                    <th>تاریخ</th>
                    <th>انبار</th>
                    <th>مواد</th>
                    <th>هزینه</th>
                    <th>بهای تمام‌شده</th>
                    <th>اقلام</th>
                    <th>شرح</th>
                  </tr>
                </thead>
                <tbody>
                  {orders.map((order) => (
                    <tr key={order.id}>
                      <td className="code">{order.number}</td>
                      <td>{order.production_date}</td>
                      <td>{order.warehouse_name}</td>
                      <td className="num">{money(order.materials_total)}</td>
                      <td className="num">{money(order.expenses_total)}</td>
                      <td className="num">{money(order.total_cost)}</td>
                      <td className="num">
                        {order.input_count} → {order.output_count}
                      </td>
                      <td>{order.description ?? '—'}</td>
                    </tr>
                  ))}
                  {orders.length === 0 && (
                    <tr>
                      <td colSpan={8} className="empty-row">
                        رسید تولیدی ثبت نشده است.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </>
      )}

      {tab === 'formulas' && (
        <div className="panel list-panel">
          <div className="panel-head">
            <div>
              <h3>فرمول‌های تولید</h3>
              <p>{formulas.length} فرمول — «قابل تولید» بر اساس موجودی فعلی مواد است.</p>
            </div>
            <button
              className="primary"
              onClick={() =>
                setFormulaForm({
                  product_id: '',
                  title: '',
                  output_quantity: 1,
                  components: [{ key: nextKey++, product_id: '', quantity_per_unit: 1, waste_percent: 0 }],
                })
              }
            >
              <Icon name="plus" /> فرمول جدید
            </button>
          </div>
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>محصول</th>
                  <th>عنوان فرمول</th>
                  <th>مقدار تولید</th>
                  <th>تعداد اجزا</th>
                  <th>بهای برآوردی واحد</th>
                  <th>قابل تولید با موجودی</th>
                  <th>عملیات</th>
                </tr>
              </thead>
              <tbody>
                {formulas.map((formula) => (
                  <tr key={formula.id} className={formula.is_active ? '' : 'row-muted'}>
                    <td>{formula.product_name}</td>
                    <td>{formula.title}</td>
                    <td className="num">{formula.output_quantity}</td>
                    <td className="num">{formula.component_count}</td>
                    <td className="num">{money(formula.estimated_unit_cost)}</td>
                    <td className={`num${formula.producible_now < 1 ? ' red-text' : ''}`}>
                      {formula.producible_now.toFixed(2)}
                    </td>
                    <td>
                      <button
                        className="table-action"
                        onClick={async () => {
                          try {
                            setFormulaDetail(await getProductionFormula(formula.id))
                          } catch (e) {
                            setError(errorText(e))
                          }
                        }}
                      >
                        جزئیات
                      </button>
                      <button
                        className="table-action"
                        disabled={busy}
                        onClick={async () => {
                          setBusy(true)
                          try {
                            await deleteProductionFormula(formula.id)
                            await load()
                          } catch (e) {
                            setError(errorText(e))
                          } finally {
                            setBusy(false)
                          }
                        }}
                      >
                        حذف
                      </button>
                    </td>
                  </tr>
                ))}
                {formulas.length === 0 && (
                  <tr>
                    <td colSpan={7} className="empty-row">
                      فرمولی تعریف نشده است.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {formulaForm && (
        <div className="modal-backdrop" onClick={() => setFormulaForm(null)}>
          <div className="modal party-modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-head">
              <div>
                <h2>فرمول تولید</h2>
                <p>ضایعات بخشی از بهای تمام‌شده است، نه هزینه‌ی جداگانه.</p>
              </div>
              <button className="icon-btn" onClick={() => setFormulaForm(null)} aria-label="بستن">
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
                  <button
                    className="icon-btn danger-icon"
                    disabled={formulaForm.components.length === 1}
                    onClick={() =>
                      setFormulaForm({
                        ...formulaForm,
                        components: formulaForm.components.filter((_, i) => i !== index),
                      })
                    }
                    aria-label="حذف"
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
                onClick={saveFormula}
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
              <button
                className="icon-btn"
                onClick={() => setFormulaDetail(undefined)}
                aria-label="بستن"
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
    </section>
  )
}
