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
import {useI18n} from '../lib/i18n'
import { useSort } from '../lib/useSort'
import {Select} from '../components/Select'

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
  const {t} = useI18n()
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
    if (fromId && toId && fromId === toId) list.push(t('transfer.errSame'))
    if (quantity > 0 && quantity > available)
      list.push(`مقدار انتقال از موجودی قابل انتقال (${available}) بیشتر است.`)
    return list
  }, [fromId, toId, quantity, available])

  const submit = async () => {
    setBusy(true)
    setNotice('')
    try {
      await createInventoryTransferOrder(productId, fromId, toId, quantity, unitCost, note || undefined)
      setNotice(t('transfer.issued'))
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
    status === 'in_transit' ? t('inv.inTransit') : status === 'received' ? t('transfer.received') : t('transfer.cancelled')

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('common.warehouse')}</div>
          <h1>{t('page.inventory-transfer')}</h1>
          <p>
            {t('transfer.lead')}
          </p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {notice && <div className="success-box">{notice}</div>}

      <div className="metric-strip">
        <div>
          <span>{t('inv.inTransit')}</span>
          <b className="amber">{inTransit.length}</b>
          <small>{t('transfer.awaiting')}</small>
        </div>
        <div>
          <span>{t('transfer.transitValue')}</span>
          <b>{money(inTransit.reduce((sum, o) => sum + o.quantity * o.unit_cost, 0))} ریال</b>
          <small>{t('transfer.atCost')}</small>
        </div>
        <div>
          <span>{t('transfer.received')}</span>
          <b>{orders.filter((o) => o.status === 'received').length}</b>
          <small>{t('transfer.closedNotes')}</small>
        </div>
        <div>
          <span>{t('transfer.activeWarehouses')}</span>
          <b>{warehouses.length}</b>
          <small>{t('transfer.possibleDestination')}</small>
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <div>
            <h3>{t('transfer.newNote')}</h3>
            <p>{t('transfer.availableHint')}</p>
          </div>
        </div>
        <div className="filter-grid">
          <label className="grow">
            <span>{t('transfer.productRequired')}</span>
            <Select value={productId} onChange={(e) => setProductId(e.target.value)}>
              <option value="">{t('invoiceForm.selectPlaceholder')}</option>
              {products.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>{t('transfer.fromRequired')}</span>
            <Select value={fromId} onChange={(e) => setFromId(e.target.value)}>
              <option value="">{t('invoiceForm.selectPlaceholder')}</option>
              {warehouses.map((w) => (
                <option key={w.id} value={w.id}>
                  {w.name}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>{t('transfer.toRequired')}</span>
            <Select value={toId} onChange={(e) => setToId(e.target.value)}>
              <option value="">{t('invoiceForm.selectPlaceholder')}</option>
              {warehouses
                .filter((w) => w.id !== fromId)
                .map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.name}
                  </option>
                ))}
            </Select>
          </label>
          <label>
            <span>{t('transfer.quantityRequired')}</span>
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
            <span>{t('transfer.note')}</span>
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
              {t('transfer.unitCost')} <b>{money(unitCost)} ریال</b>
            </span>
            {quantity > 0 && (
              <span>
                {t('transfer.value')} <b>{money(Math.round(quantity * unitCost))} ریال</b>
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
            {t('transfer.issue')}
          </button>
        </div>
      </div>

      <div className="panel list-panel">
        <div className="panel-head">
          <div>
            <h3>{t('transfer.notes')}</h3>
            <p>{sorted.length} مورد</p>
          </div>
          <div className="filter-actions">
            <Select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
              <option value="">{t('common.all')}</option>
              <option value="in_transit">{t('inv.inTransit')}</option>
              <option value="received">{t('transfer.received')}</option>
              <option value="cancelled">{t('transfer.cancelled')}</option>
            </Select>
            <button className="icon-btn" onClick={load} aria-label={t('common.refresh')}>
              <Icon name="refresh" />
            </button>
          </div>
        </div>
        <div className="table-wrap">
          <table className="large-table">
            <thead>
              <tr>
                <th>{t('invoiceForm.product')}</th>
                <th>{t('transfer.from')}</th>
                <th>{t('transfer.to')}</th>
                <th {...sortProps('quantity')}>{t('common.quantity')}</th>
                <th {...sortProps('unit_cost')}>{t('ops.unitCost')}</th>
                <th>{t('reports.value')}</th>
                <th>{t('transfer.note')}</th>
                <th {...sortProps('status')}>{t('common.status')}</th>
                <th>{t('common.actions')}</th>
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
                        {t('transfer.receiveAtDestination')}
                      </button>
                    )}
                  </td>
                </tr>
              ))}
              {sorted.length === 0 && (
                <tr>
                  <td colSpan={9} className="empty-row">
                    {t('transfer.empty')}
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
