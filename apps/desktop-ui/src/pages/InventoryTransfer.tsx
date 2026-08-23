import { useCallback, useEffect, useMemo, useState } from 'react'
import { Icon } from '../components/Icon'
import {
  createInventoryTransferOrder,
  getInventoryTransferOrders,
  getProducts,
  getStockBalances,
  getWarehouses,
  receiveInventoryTransfer,
  InventoryTransferOrder,
  Product,
  StockBalance,
  Warehouse,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials as money } from '../lib/format'
import { useSort } from '../lib/useSort'

/**
 * انتقال کالا بین انبارها.
 *
 * ## چرا انتقال دو مرحله‌ای است
 *
 * کالا در لحظه‌ی خروج از انبار مبدأ به انبار مقصد نمی‌رسد؛ در فاصله‌ی این دو،
 * «در راه» است. اگر یک‌مرحله‌ای ثبت شود، موجودی مقصد چیزی را نشان می‌دهد که
 * هنوز نرسیده و انبارگردانی مقصد اختلاف می‌دهد.
 *
 * ## اثر حسابداری
 *
 * انتقال بین انبارهای یک شرکت **هیچ اثر مالی ندارد** — نه سود می‌سازد نه
 * هزینه. فقط موجودی جابه‌جا می‌شود، پس سند حسابداری صادر نمی‌شود.
 */
export function InventoryTransfer() {
  const [orders, setOrders] = useState<InventoryTransferOrder[]>([])
  const [products, setProducts] = useState<Product[]>([])
  const [warehouses, setWarehouses] = useState<Warehouse[]>([])
  const [stock, setStock] = useState<StockBalance[]>([])
  const [productId, setProductId] = useState('')
  const [fromId, setFromId] = useState('')
  const [toId, setToId] = useState('')
  const [quantity, setQuantity] = useState(0)
  const [note, setNote] = useState('')
  const [statusFilter, setStatusFilter] = useState('')
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const [busy, setBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      const [list, balances] = await Promise.all([
        getInventoryTransferOrders(),
        getStockBalances(),
      ])
      setOrders(list)
      setStock(balances)
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
      } catch (e) {
        setError(errorText(e))
      }
    })()
  }, [])

  const nameOf = (id: string) => products.find((p) => p.id === id)?.name ?? id
  const warehouseName = (id: string) => warehouses.find((w) => w.id === id)?.name ?? id

  // موجودی قابل انتقال = موجودی منهای رزرو. رزروشده متعلق به سفارش دیگری است.
  const available = useMemo(() => {
    if (!productId || !fromId) return 0
    const row = stock.find((s) => s.product_id === productId && s.warehouse_id === fromId)
    return row ? row.available_quantity : 0
  }, [stock, productId, fromId])

  const selectedProduct = products.find((p) => p.id === productId)
  const unitCost = selectedProduct?.purchase_price ?? 0

  const problems = useMemo(() => {
    const list: string[] = []
    if (fromId && toId && fromId === toId) list.push('انبار مبدأ و مقصد نباید یکسان باشد.')
    if (quantity > 0 && quantity > available)
      list.push(`مقدار انتقال از موجودی قابل انتقال (${available}) بیشتر است.`)
    return list
  }, [fromId, toId, quantity, available])

  const submit = async () => {
    setBusy(true)
    setNotice('')
    try {
      await createInventoryTransferOrder(productId, fromId, toId, quantity, unitCost, note || undefined)
      setNotice('حواله‌ی انتقال صادر شد. کالا تا تحویل در انبار مقصد، «در راه» است.')
      setQuantity(0)
      setNote('')
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const receive = async (order: InventoryTransferOrder) => {
    setBusy(true)
    try {
      await receiveInventoryTransfer(order.id)
      setNotice(`${nameOf(order.product_id)} در ${warehouseName(order.to_warehouse_id)} تحویل شد.`)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const visible = useMemo(
    () => orders.filter((o) => !statusFilter || o.status === statusFilter),
    [orders, statusFilter],
  )
  const { sorted, sortProps } = useSort(visible, 'status')

  const inTransit = orders.filter((o) => o.status === 'in_transit')
  const canSubmit =
    !busy && productId !== '' && fromId !== '' && toId !== '' && quantity > 0 && problems.length === 0

  const statusLabel = (status: string) =>
    status === 'in_transit' ? 'در راه' : status === 'received' ? 'تحویل شده' : 'لغو شده'

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">انبار</div>
          <h1>انتقال بین انبارها</h1>
          <p>
            انتقال دو مرحله‌ای است: کالا از مبدأ خارج می‌شود و تا تحویل در مقصد «در راه» می‌ماند.
            این انتقال هیچ اثر مالی ندارد — فقط جای کالا عوض می‌شود.
          </p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {notice && <div className="success-box">{notice}</div>}

      <div className="metric-strip">
        <div>
          <span>در راه</span>
          <b className="amber">{inTransit.length}</b>
          <small>منتظر تحویل در مقصد</small>
        </div>
        <div>
          <span>ارزش کالای در راه</span>
          <b>{money(inTransit.reduce((sum, o) => sum + o.quantity * o.unit_cost, 0))} ریال</b>
          <small>به بهای تمام‌شده</small>
        </div>
        <div>
          <span>تحویل شده</span>
          <b>{orders.filter((o) => o.status === 'received').length}</b>
          <small>حواله‌ی بسته‌شده</small>
        </div>
        <div>
          <span>انبارهای فعال</span>
          <b>{warehouses.length}</b>
          <small>مقصد ممکن</small>
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <div>
            <h3>حواله‌ی انتقال جدید</h3>
            <p>موجودی قابل انتقال، موجودی منهای مقدار رزروشده است.</p>
          </div>
        </div>
        <div className="filter-grid">
          <label className="grow">
            <span>کالا *</span>
            <select value={productId} onChange={(e) => setProductId(e.target.value)}>
              <option value="">انتخاب کنید…</option>
              {products.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>از انبار *</span>
            <select value={fromId} onChange={(e) => setFromId(e.target.value)}>
              <option value="">انتخاب کنید…</option>
              {warehouses.map((w) => (
                <option key={w.id} value={w.id}>
                  {w.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>به انبار *</span>
            <select value={toId} onChange={(e) => setToId(e.target.value)}>
              <option value="">انتخاب کنید…</option>
              {warehouses
                .filter((w) => w.id !== fromId)
                .map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.name}
                  </option>
                ))}
            </select>
          </label>
          <label>
            <span>مقدار *</span>
            <input
              type="number"
              min={0}
              max={available}
              step="any"
              value={quantity || ''}
              onChange={(e) => setQuantity(Number(e.target.value) || 0)}
            />
          </label>
          <label className="grow">
            <span>توضیح</span>
            <input value={note} onChange={(e) => setNote(e.target.value)} />
          </label>
        </div>

        {productId && fromId && (
          <div className="inline-summary">
            <span>
              موجودی قابل انتقال در {warehouseName(fromId)}: <b>{available}</b>{' '}
              {selectedProduct?.unit ?? ''}
            </span>
            <span>
              بهای تمام‌شده‌ی واحد: <b>{money(unitCost)} ریال</b>
            </span>
            {quantity > 0 && (
              <span>
                ارزش انتقال: <b>{money(Math.round(quantity * unitCost))} ریال</b>
              </span>
            )}
          </div>
        )}

        {problems.length > 0 && (
          <div className="warn-box">
            {problems.map((problem) => (
              <div key={problem}>{problem}</div>
            ))}
          </div>
        )}

        <div className="modal-actions">
          <button className="primary" onClick={submit} disabled={!canSubmit}>
            صدور حواله‌ی انتقال
          </button>
        </div>
      </div>

      <div className="panel list-panel">
        <div className="panel-head">
          <div>
            <h3>حواله‌های انتقال</h3>
            <p>{sorted.length} مورد</p>
          </div>
          <div className="filter-actions">
            <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
              <option value="">همه</option>
              <option value="in_transit">در راه</option>
              <option value="received">تحویل شده</option>
              <option value="cancelled">لغو شده</option>
            </select>
            <button className="icon-btn" onClick={load} aria-label="بروزرسانی">
              <Icon name="refresh" />
            </button>
          </div>
        </div>
        <div className="table-wrap">
          <table className="large-table">
            <thead>
              <tr>
                <th>کالا</th>
                <th>از انبار</th>
                <th>به انبار</th>
                <th {...sortProps('quantity')}>مقدار</th>
                <th {...sortProps('unit_cost')}>بهای واحد</th>
                <th>ارزش</th>
                <th>توضیح</th>
                <th {...sortProps('status')}>وضعیت</th>
                <th>عملیات</th>
              </tr>
            </thead>
            <tbody>
              {sorted.map((order) => (
                <tr key={order.id} className={order.status === 'cancelled' ? 'row-muted' : ''}>
                  <td>{nameOf(order.product_id)}</td>
                  <td>{warehouseName(order.from_warehouse_id)}</td>
                  <td>{warehouseName(order.to_warehouse_id)}</td>
                  <td className="num">{order.quantity}</td>
                  <td className="num">{money(order.unit_cost)}</td>
                  <td className="num">{money(Math.round(order.quantity * order.unit_cost))}</td>
                  <td>{order.note ?? '—'}</td>
                  <td>
                    <span
                      className={`status ${
                        order.status === 'received'
                          ? 'done'
                          : order.status === 'cancelled'
                            ? 'neutral'
                            : 'pending'
                      }`}
                    >
                      {statusLabel(order.status)}
                    </span>
                  </td>
                  <td>
                    {order.status === 'in_transit' && (
                      <button
                        className="table-action"
                        disabled={busy}
                        onClick={() => receive(order)}
                      >
                        تحویل در مقصد
                      </button>
                    )}
                  </td>
                </tr>
              ))}
              {sorted.length === 0 && (
                <tr>
                  <td colSpan={9} className="empty-row">
                    حواله‌ی انتقالی ثبت نشده است.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  )
}
