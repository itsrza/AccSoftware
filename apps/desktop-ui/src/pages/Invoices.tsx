import {useEffect,useMemo,useState} from 'react'
import {Icon} from '../components/Icon'
import {getPurchaseInvoices,getSalesInvoices,InvoiceSummary} from '../api'
import {errorText} from '../lib/errors'
import {formatRials as money, formatNumber as n} from '../lib/format'

const statusLabel=(s:string)=>s==='posted'?'ثبت شده':s==='draft'?'پیش‌نویس':s==='cancelled'?'باطل شده':s==='reversed'?'برگشت شده':s
const paymentLabel=(s:string)=>s==='paid'?'تسویه کامل':s==='partial'?'تسویه جزئی':'تسویه نشده'

export function Invoices({page}:{page:string,onNavigate?:(p:string)=>void}){
 const [search,setSearch]=useState(''); const [rows,setRows]=useState<InvoiceSummary[]>([]); const [loading,setLoading]=useState(true); const [error,setError]=useState('')
 const sale=page==='sales'
 const title=sale?'فاکتورهای فروش':'فاکتورهای خرید'
 useEffect(()=>{let active=true;setLoading(true);setError('');(sale?getSalesInvoices():getPurchaseInvoices()).then(v=>{if(active)setRows(v)}).catch(e=>{if(active)setError(errorText(e))}).finally(()=>{if(active)setLoading(false)});return()=>{active=false}},[sale])
 const filtered=useMemo(()=>rows.filter(r=>`${r.number} ${r.invoice_date} ${r.status} ${r.payment_status} ${r.total}`.includes(search.trim())),[rows,search])
 const total=rows.reduce((a,r)=>a+r.total,0); const posted=rows.filter(r=>r.status==='posted').length; const pending=rows.filter(r=>r.payment_status!=='paid'&&r.status==='posted').length
 return <section className="page" dir="rtl">
  <div className="page-head"><div><div className="eyebrow">مدیریت و عملیات</div><h1>{title}</h1><p>اطلاعات از پایگاه داده واقعی برنامه خوانده می‌شود.</p></div></div>
  {error&&<div className="error-box">بارگذاری اطلاعات انجام نشد. {error}</div>}
  <div className="metric-strip"><div><span>تعداد کل</span><b>{money(rows.length)}</b></div><div><span>ثبت شده</span><b>{money(posted)}</b></div><div><span>جمع مبلغ</span><b>{money(total)} <small>ریال</small></b></div><div><span>در انتظار تسویه</span><b className="amber">{money(pending)}</b></div></div>
  <div className="panel list-panel"><div className="toolbar"><div className="global-search inner"><Icon name="search" size={17}/><input value={search} onChange={e=>setSearch(e.target.value)} placeholder="جستجو در فاکتورها..."/></div><button className="filter-btn" disabled><Icon name="filter"/> فیلترها</button><button className="filter-btn" disabled><Icon name="download"/> خروجی</button><button className="icon-btn" onClick={()=>{setLoading(true);(sale?getSalesInvoices():getPurchaseInvoices()).then(setRows).catch(e=>setError(errorText(e))).finally(()=>setLoading(false))}}><Icon name="refresh"/></button></div>
   {loading?<div className="empty">در حال بارگذاری...</div>:filtered.length===0?<div className="empty">فاکتوری برای نمایش وجود ندارد.</div>:<div className="table-wrap"><table className="large-table"><thead><tr><th>شماره</th><th>تاریخ</th><th>مبلغ</th><th>وضعیت</th><th>تسویه</th></tr></thead><tbody>{filtered.map(r=><tr key={r.id}><td><b className="code">{r.number}</b></td><td>{r.invoice_date}</td><td>{money(r.total)} ریال</td><td><span className={r.status==='posted'?'status done':'status pending'}>{statusLabel(r.status)}</span></td><td>{paymentLabel(r.payment_status)}</td></tr>)}</tbody></table></div>}
  </div>
 </section>
}
