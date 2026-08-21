import {useEffect,useMemo,useState} from 'react'
import {getAccountLedgerSummary,getInventoryValuation,getPurchaseReport,getSalesReport,listCustomReports,saveCustomReport,deleteCustomReport,SalesReportRow,PurchaseReportRow,InventoryValuation,AccountLedgerSummary} from '../api'
import {Icon} from '../components/Icon'
import {errorText} from '../lib/errors'
import {formatRials as money, formatNumber as n} from '../lib/format'

type Source='sales'|'purchase'|'inventory'|'ledger'
type Row=Record<string,string|number>
type Column={key:string,label:string}
type Config={columns:string[],filter:string,sortKey:string,sortDir:'asc'|'desc',groupKey:string,from:string,to:string}
const sourceLabels:Record<Source,string>={sales:'گزارش فروش',purchase:'گزارش خرید',inventory:'ارزش موجودی',ledger:'گردش حساب'}
const columns:Record<Source,Column[]>={
 sales:[['date','تاریخ'],['invoice_number','شماره'],['contact_name','شخص'],['subtotal','خالص'],['discount','تخفیف'],['tax','مالیات'],['total','مبلغ'],['payment_status','وضعیت']].map(([key,label])=>({key,label})),
 purchase:[['date','تاریخ'],['invoice_number','شماره'],['contact_name','تأمین‌کننده'],['subtotal','خالص'],['discount','تخفیف'],['tax','مالیات'],['total','مبلغ'],['payment_status','وضعیت']].map(([key,label])=>({key,label})),
 inventory:[['product_name','کالا'],['warehouse_name','انبار'],['quantity','موجودی'],['average_cost','میانگین بها'],['value','ارزش']].map(([key,label])=>({key,label})),
 ledger:[['code','کد'],['name','حساب'],['debit','بدهکار'],['credit','بستانکار'],['balance','مانده']].map(([key,label])=>({key,label}))
}
const today=()=>new Date().toISOString().slice(0,10)
const escapeHtml=(v:string)=>v.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;')
function normalize(source:Source,data:unknown):Row[]{
 if(source==='sales'||source==='purchase') return (data as (SalesReportRow|PurchaseReportRow)[]).map(x=>({...x,contact_name:x.contact_name||'بدون شخص'}))
 if(source==='inventory') return (data as InventoryValuation[]).map(x=>({...x}))
 return (data as AccountLedgerSummary[]).map(x=>({...x}))
}
function download(name:string,content:string,type:string){const blob=new Blob([content],{type});const a=document.createElement('a');a.href=URL.createObjectURL(blob);a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(a.href),500)}
function csv(rows:Row[],cols:Column[]){const q=(v:unknown)=>`"${String(v??'').replaceAll('"','""')}"`;return '\ufeff'+[cols.map(c=>q(c.label)).join(','),...rows.map(r=>cols.map(c=>q(r[c.key])).join(','))].join('\r\n')}
export function ReportBuilder(){
 const [source,setSource]=useState<Source>('sales'); const [data,setData]=useState<Row[]>([]); const [config,setConfig]=useState<Config>({columns:columns.sales.map(c=>c.key),filter:'',sortKey:'',sortDir:'asc',groupKey:'',from:'',to:today()}); const [name,setName]=useState('گزارش سفارشی'); const [saved,setSaved]=useState<{id:string,name:string,source:string,config_json:string}[]>([]); const [loading,setLoading]=useState(false); const [error,setError]=useState('')
 const available=columns[source]
 useEffect(()=>{setConfig(c=>({...c,columns:available.map(x=>x.key),sortKey:'',groupKey:''}));loadSource(source)},[source])
 const loadSource=async(s:Source)=>{setLoading(true);setError('');try{const d=s==='sales'?await getSalesReport(config.from||undefined,config.to||undefined):s==='purchase'?await getPurchaseReport(config.from||undefined,config.to||undefined):s==='inventory'?await getInventoryValuation():await getAccountLedgerSummary(config.from||undefined,config.to||undefined);setData(normalize(s,d))}catch(e){setError(errorText(e))}finally{setLoading(false)}}
 const refreshSaved=async()=>{try{setSaved(await listCustomReports())}catch(e){setError(errorText(e))}}
 useEffect(()=>{refreshSaved()},[])
 const result=useMemo(()=>{let r=data.filter(row=>!config.filter||Object.values(row).some(v=>String(v??'').toLowerCase().includes(config.filter.toLowerCase())));if(config.sortKey)r=[...r].sort((a,b)=>{const av=a[config.sortKey],bv=b[config.sortKey];const cmp=String(av??'').localeCompare(String(bv??''),'fa',{numeric:true});return config.sortDir==='asc'?cmp:-cmp});return r},[data,config.filter,config.sortKey,config.sortDir])
 const selected=available.filter(c=>config.columns.includes(c.key));
 const grouped=useMemo(()=>{if(!config.groupKey)return [{key:'',rows:result}];const m=new Map<string,Row[]>();for(const row of result){const k=String(row[config.groupKey]??'—');if(!m.has(k))m.set(k,[]);m.get(k)!.push(row)}return [...m].map(([key,rows])=>({key,rows}))},[result,config.groupKey])
 const save=async()=>{try{setError('');await saveCustomReport(undefined,name,source,JSON.stringify(config));await refreshSaved()}catch(e){setError(errorText(e))}}
 const remove=async(id:string)=>{try{await deleteCustomReport(id);await refreshSaved()}catch(e){setError(errorText(e))}}
 const loadSaved=(x:{name:string,source:string,config_json:string})=>{try{const c=JSON.parse(x.config_json) as Config;setSource(x.source as Source);setName(x.name);setConfig(c);loadSource(x.source as Source)}catch{setError('تنظیمات گزارش ذخیره‌شده نامعتبر است')}}
 const exportCsv=()=>download(`${name}.csv`,csv(result,selected),'text/csv;charset=utf-8')
 const exportExcel=()=>{const html=`<html><head><meta charset="utf-8"></head><body dir="rtl"><table border="1"><thead><tr>${selected.map(c=>`<th>${escapeHtml(c.label)}</th>`).join('')}</tr></thead><tbody>${result.map(r=>`<tr>${selected.map(c=>`<td>${escapeHtml(String(r[c.key]??''))}</td>`).join('')}</tr>`).join('')}</tbody></table></body></html>`;download(`${name}.xls`,html,'application/vnd.ms-excel;charset=utf-8')}
 const print=()=>{const w=window.open('','_blank','width=1100,height=800');if(!w){setError('پنجره چاپ توسط مرورگر مسدود شد');return}w.document.write(`<html dir="rtl"><head><meta charset="utf-8"><title>${escapeHtml(name)}</title><style>body{font-family:Tahoma,Arial;padding:28px;color:#21254E}h1{font-size:20px}p{color:#62748E}table{width:100%;border-collapse:collapse}th,td{border:1px solid #d9dfe9;padding:8px;text-align:right}th{background:#F6F9FF}@media print{button{display:none}}</style></head><body><h1>${escapeHtml(name)}</h1><p>${sourceLabels[source]} | ${config.from||'ابتدا'} تا ${config.to||'امروز'}</p><table><thead><tr>${selected.map(c=>`<th>${escapeHtml(c.label)}</th>`).join('')}</tr></thead><tbody>${result.map(r=>`<tr>${selected.map(c=>`<td>${escapeHtml(String(r[c.key]??''))}</td>`).join('')}</tr>`).join('')}</tbody></table><script>window.onload=()=>window.print()</script></body></html>`);w.document.close()}
 return <section className="page"><div className="page-head"><div><div className="eyebrow">گزارش‌ساز حرفه‌ای</div><h1>Report Builder</h1><p>گزارش را بساز، ذخیره کن و خروجی بگیر.</p></div><div className="quick-actions-inline"><button className="secondary" onClick={()=>loadSource(source)}><Icon name="refresh"/> بروزرسانی</button><button className="primary" onClick={save}>ذخیره گزارش</button></div></div>
  {error&&<div className="error-box">{error}</div>}
  <div className="builder-grid"><div className="panel builder-controls"><div className="builder-section"><label>نام گزارش<input value={name} onChange={e=>setName(e.target.value)}/></label><label>منبع<select value={source} onChange={e=>setSource(e.target.value as Source)}>{Object.entries(sourceLabels).map(([k,v])=><option key={k} value={k}>{v}</option>)}</select></label></div>
   <div className="builder-section"><label>جستجوی عمومی<input value={config.filter} onChange={e=>setConfig({...config,filter:e.target.value})} placeholder="فیلتر متن..."/></label><label>از<input type="date" value={config.from} onChange={e=>setConfig({...config,from:e.target.value})}/></label><label>تا<input type="date" value={config.to} onChange={e=>setConfig({...config,to:e.target.value})}/></label></div>
   <div className="builder-section"><label>مرتب‌سازی<select value={config.sortKey} onChange={e=>setConfig({...config,sortKey:e.target.value})}><option value="">بدون مرتب‌سازی</option>{available.map(c=><option key={c.key} value={c.key}>{c.label}</option>)}</select></label><label>جهت<select value={config.sortDir} onChange={e=>setConfig({...config,sortDir:e.target.value as 'asc'|'desc'})}><option value="asc">صعودی</option><option value="desc">نزولی</option></select></label><label>گروه‌بندی<select value={config.groupKey} onChange={e=>setConfig({...config,groupKey:e.target.value})}><option value="">بدون گروه‌بندی</option>{available.map(c=><option key={c.key} value={c.key}>{c.label}</option>)}</select></label></div>
   <div><strong>ستون‌ها</strong><div className="column-picks">{available.map(c=><label key={c.key}><input type="checkbox" checked={config.columns.includes(c.key)} onChange={e=>setConfig({...config,columns:e.target.checked?[...config.columns,c.key]:config.columns.filter(x=>x!==c.key)})}/>{c.label}</label>)}</div></div>
   <div className="export-actions"><button onClick={exportCsv}>CSV</button><button onClick={exportExcel}>Excel</button><button onClick={print}>چاپ / PDF</button></div></div>
   <div className="panel saved-reports"><div className="panel-head"><h3>گزارش‌های ذخیره‌شده</h3></div>{saved.length?saved.map(x=><div className="saved-report" key={x.id}><button onClick={()=>loadSaved(x)}>{x.name}<small>{sourceLabels[x.source as Source]||x.source}</small></button><button className="danger-link" onClick={()=>remove(x.id)}>حذف</button></div>):<div className="empty">گزارش ذخیره‌شده‌ای وجود ندارد.</div>}</div></div>
  <div className="panel report-preview"><div className="panel-head"><h3>پیش‌نمایش</h3><span>{result.length} ردیف</span></div>{loading?<div className="empty-state">در حال آماده‌سازی...</div>:<div className="table-wrap"><table className="large-table"><thead><tr>{selected.map(c=><th key={c.key}>{c.label}</th>)}</tr></thead><tbody>{grouped.map(g=><>{g.key&&<tr className="group-row"><td colSpan={selected.length}><b>{g.key}</b> — {g.rows.length} ردیف</td></tr>}{g.rows.map((r,i)=><tr key={`${g.key}-${i}`}>{selected.map(c=><td key={c.key}>{typeof r[c.key]==='number'?money(r[c.key] as number):String(r[c.key]??'')}</td>)}</tr>)}</>)}</tbody></table></div>}</div>
 </section>
}
