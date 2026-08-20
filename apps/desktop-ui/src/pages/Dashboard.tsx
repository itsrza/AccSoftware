import {useEffect,useMemo,useState} from 'react'
import {Icon} from '../components/Icon'
import {DashboardKpi,SalesTrend,TopProduct,LowStock,RecentInvoice,getDashboardKpis,getSalesTrend,getTopProducts,getLowStockReport,getRecentInvoices} from '../api'
const money=(n:number)=>new Intl.NumberFormat('fa-IR').format(Math.round(n))
const pct=(n:number)=>new Intl.NumberFormat('fa-IR',{maximumFractionDigits:1}).format(n)

function Chart({data}:{data:SalesTrend[]}){
 const max=Math.max(1,...data.map(x=>Math.max(x.sales,x.purchases)))
 const points=(key:'sales'|'purchases')=>data.map((x,i)=>`${(i/(Math.max(1,data.length-1)))*100},${100-(x[key]/max)*86}`).join(' ')
 return <div className="chart" style={{minHeight:260}}>
  {data.length===0?<div className="empty-state"><p>داده کافی برای نمودار وجود ندارد.</p></div>:<>
   <div className="ylabels"><span>{money(max)}</span><span>{money(max*.75)}</span><span>{money(max*.5)}</span><span>{money(max*.25)}</span><span>۰</span></div>
   <svg viewBox="0 0 100 100" preserveAspectRatio="none" className="line-chart" aria-label="روند فروش و خرید">
    <polyline points={points('sales')} fill="none" stroke="#d79d4b" strokeWidth="1.6" vectorEffect="non-scaling-stroke" strokeLinecap="round"/>
    <polyline points={points('purchases')} fill="none" stroke="#7c86ff" strokeWidth="1.2" vectorEffect="non-scaling-stroke" strokeDasharray="5 5" strokeLinecap="round"/>
   </svg>
   <div className="xlabels">{data.map(x=><span key={x.period}>{x.period.slice(5)}</span>)}</div>
  </>}
 </div>
}

export function Dashboard({demo,onSettings,onNavigate}:{demo:boolean,onSettings:()=>void,onNavigate:(p:string)=>void}){
 const [kpi,setKpi]=useState<DashboardKpi>(); const [trend,setTrend]=useState<SalesTrend[]>([]); const [products,setProducts]=useState<TopProduct[]>([]); const [low,setLow]=useState<LowStock[]>([]); const [recent,setRecent]=useState<RecentInvoice[]>([]); const [loading,setLoading]=useState(true); const [error,setError]=useState('')
 const load=async()=>{setLoading(true);setError('');try{const [a,b,c,d,e]=await Promise.all([getDashboardKpis(),getSalesTrend(),getTopProducts(),getLowStockReport(),getRecentInvoices()]);setKpi(a);setTrend(b.slice(-6));setProducts(c);setLow(d);setRecent(e)}catch(e){setError(String(e))}finally{setLoading(false)}}
 useEffect(()=>{if(demo)load();else setLoading(false)},[demo])
 if(!demo)return <section className="page"><div className="empty-state"><div className="empty-icon"><Icon name="grid" size={28}/></div><h2>داشبورد آماده است</h2><p>داده‌های نمونه حذف شده‌اند. پس از ثبت اطلاعات واقعی، شاخص‌ها اینجا نمایش داده می‌شوند.</p><button className="primary" onClick={onSettings}><Icon name="settings"/> مدیریت داده‌ها</button></div></section>
 const stats=kpi?[{title:'فروش',value:kpi.sales,tone:'gold',icon:'receipt'},{title:'سود ناخالص',value:kpi.gross_profit,tone:'green',icon:'trend'},{title:'مطالبات مشتریان',value:kpi.receivables,tone:'purple',icon:'users'},{title:'ارزش موجودی',value:kpi.inventory_value,tone:'red',icon:'package'}]:[]
 return <section className="page">
  <div className="welcome"><div><div className="eyebrow">داشبورد مدیریتی</div><h1>وضعیت مالی و عملیاتی شرکت</h1><p>{loading?'در حال دریافت اطلاعات واقعی از پایگاه داده...':'آخرین اطلاعات ثبت‌شده در SQLite'}</p></div><div style={{display:'flex',gap:8}}><button className="ghost" onClick={load}><Icon name="refresh"/> بروزرسانی</button><button className="primary" onClick={()=>onNavigate('sales')}><Icon name="plus"/> ثبت فاکتور فروش</button></div></div>
  {error&&<div className="error-box">{error}</div>}
  <div className="stats-grid">{stats.map(s=><div className="stat-card" key={s.title}><div className="stat-top"><span className={'stat-icon '+s.tone}><Icon name={s.icon as any}/></span></div><span className="stat-title">{s.title}</span><strong>{money(s.value)}<small>ریال</small></strong><div className="stat-foot"><span>داده واقعی</span></div></div>)}</div>
  <div className="metric-strip"><div><span>خرید</span><b>{money(kpi?.purchases||0)} ریال</b></div><div><span>بدهی تأمین‌کنندگان</span><b>{money(kpi?.payables||0)} ریال</b></div><div><span>نقدینگی</span><b>{money(kpi?.cash||0)} ریال</b></div><div><span>کمبود موجودی</span><b>{pct(kpi?.low_stock_count||0)} کالا</b></div></div>
  <div className="dashboard-grid">
   <div className="panel chart-panel"><div className="panel-head"><div><h3>روند فروش و خرید</h3><p>داده‌های ثبت‌شده در سال مالی فعال</p></div><button className="text-btn" onClick={()=>onNavigate('reports')}>گزارشات</button></div><Chart data={trend}/><div className="legend"><span><i className="gold-dot"/> فروش</span><span><i className="purple-dot"/> خرید</span></div></div>
   <div className="panel"><div className="panel-head"><div><h3>پرفروش‌ترین کالاها</h3><p>بر اساس مبلغ فروش</p></div></div><div className="stock-list">{products.slice(0,6).map((x,i)=><div className="stock-row" key={x.product_id}><span className="product-icon">{i+1}</span><div><b>{x.name}</b><small>{money(x.quantity)} واحد</small></div><strong>{money(x.revenue)}</strong></div>)}{products.length===0&&<div className="empty-state"><p>داده‌ای ثبت نشده است.</p></div>}</div></div>
  </div>
  <div className="dashboard-grid bottom-grid">
   <div className="panel"><div className="panel-head"><div><h3>آخرین فاکتورها</h3><p>آخرین اسناد ثبت‌شده</p></div><button className="text-btn" onClick={()=>onNavigate('sales')}>مشاهده همه</button></div><div className="table-wrap"><table><thead><tr><th>نوع</th><th>شماره</th><th>شخص</th><th>مبلغ</th><th>وضعیت</th></tr></thead><tbody>{recent.map(x=><tr key={x.id}><td>{x.invoice_type==='sale'?'فروش':'خرید'}</td><td><b className="code">{x.number}</b></td><td>{x.contact_name||'بدون شخص'}</td><td>{money(x.total)}</td><td><span className={x.payment_status==='paid'?'status done':'status pending'}>{x.payment_status==='paid'?'تسویه':'در انتظار تسویه'}</span></td></tr>)}</tbody></table></div></div>
   <div className="panel"><div className="panel-head"><div><h3>هشدار موجودی</h3><p>{low.length} کالا زیر حد سفارش</p></div><button className="text-btn" onClick={()=>onNavigate('inventory')}>مشاهده انبار</button></div><div className="stock-list">{low.slice(0,6).map(x=><div className="stock-row" key={x.product_id}><span className="product-icon"><Icon name="box" size={17}/></span><div><b>{x.name}</b><small>حداقل {money(x.min_stock)}</small></div><div className="stock-num"><strong>{money(x.quantity)}</strong><small>موجودی</small></div><span className="status danger">کمبود</span></div>)}</div></div>
  </div>
 </section>
}
