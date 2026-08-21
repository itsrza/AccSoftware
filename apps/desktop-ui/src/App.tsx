import {useEffect,useState} from 'react'
import {Dashboard} from './pages/Dashboard'
import {Invoices} from './pages/Invoices'
import {DataPage} from './pages/DataPage'
import {Reports} from './pages/Reports'
import {ReportBuilder} from './pages/ReportBuilder'
import {Integrations} from './pages/Integrations'
import {Treasury} from './pages/Treasury'
import {Checks} from './pages/Checks'
import {Icon} from './components/Icon'
import {CommandPalette} from './components/CommandPalette'
import {SettingsCenter} from './components/SettingsCenter'
import {Operations} from './pages/Operations'
import {AdvancedInventory} from './pages/AdvancedInventory'
import {DataTools} from './pages/DataTools'
import {PrintTemplates} from './pages/PrintTemplates'
import {getDemoStatus,deleteDemo,login} from './api'
import './styles.css'

const menu = [
  {id:'dashboard',label:'داشبورد',icon:'grid'},
  {id:'sales',label:'فروش',icon:'receipt',children:['فاکتور فروش','برگشت از فروش','پیش‌فاکتورها']},
  {id:'purchase',label:'خرید',icon:'cart',children:['فاکتور خرید','برگشت از خرید','سفارش خرید']},
  {id:'inventory',label:'انبار و کالا',icon:'package',children:['کالاها','موجودی انبار','انتقال بین انبارها','انبارگردانی','تولید و فرمول']},
  {id:'contacts',label:'اشخاص',icon:'users',children:['مشتریان','تأمین‌کنندگان','گروه‌بندی اشخاص']},
  {id:'treasury',label:'خزانه',icon:'wallet',children:['دریافت','پرداخت','بانک‌ها','صندوق‌ها']},
  {id:'checks',label:'چک‌ها',icon:'check',children:['چک‌های دریافتی','چک‌های پرداختی','سررسیدها']},
  {id:'accounting',label:'حسابداری',icon:'file',children:['اسناد حسابداری','کدینگ حساب‌ها','دفاتر','تراز آزمایشی']},
  {id:'reports',label:'گزارشات',icon:'bar',children:['مرکز گزارشات','گزارش‌ساز','گزارش فروش','گزارش خرید','گزارش انبار','گزارش مالی']},
  {id:'integrations',label:'اتصالات و افزونه‌ها',icon:'settings'},
  {id:'data-tools',label:'ورود و خروج اطلاعات',icon:'file'},
  {id:'print-templates',label:'قالب‌های چاپ',icon:'file'},
]

export default function App(){
  const [page,setPage]=useState('dashboard')
  const [open,setOpen]=useState<string[]>(['sales','inventory','accounting'])
  const [dark,setDark]=useState(false)
  const [settings,setSettings]=useState(false)
  const [palette,setPalette]=useState(false)
  const [collapsed,setCollapsed]=useState(false)
  // حالت دمو فقط وقتی فعال است که صراحتاً درخواست شود؛ پیش‌فرض امن = خاموش.
  // بیلد تجاری هرگز نباید ورود خودکار یا ابزار حذف داده‌ی نمونه داشته باشد.
  const DEMO_BUILD = import.meta.env.VITE_DEMO_MODE === 'true'
  const [demo,setDemo]=useState(false)
  const [demoBusy,setDemoBusy]=useState(false)
  const [booting,setBooting]=useState(true)
  const [bootError,setBootError]=useState<string | null>(null)
  useEffect(()=>{
    let alive=true
    const boot=async()=>{
      try{
        if(DEMO_BUILD){
          await login('admin','demo')
          const status=await getDemoStatus()
          if(alive)setDemo(status)
        }
      }catch(e){
        if(alive)setBootError(String(e))
      }finally{
        if(alive)setBooting(false)
      }
    }
    boot()
    return()=>{alive=false}
  },[DEMO_BUILD])
  useEffect(()=>{const f=(e:KeyboardEvent)=>{if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='k'){e.preventDefault();setPalette(true)};if(e.key==='Escape'){setPalette(false);setSettings(false)}};window.addEventListener('keydown',f);return()=>window.removeEventListener('keydown',f)},[])
  const toggle=(id:string)=>setOpen(v=>v.includes(id)?v.filter(x=>x!==id):[...v,id])
  if(booting) return <div className="boot-screen" dir="rtl"><div className="boot-card"><strong>نوین پرداز</strong><span>در حال آماده‌سازی محیط دمو...</span></div></div>
  if(bootError) return <div className="boot-screen" dir="rtl"><div className="boot-card error"><strong>اجرای برنامه انجام نشد</strong><span>{bootError}</span><small>فایل لاگ را بررسی کنید.</small></div></div>
  return <div className={(dark?'app dark':'app')+(collapsed?' sidebar-collapsed':'')} dir="rtl">
    <aside className="sidebar">
      <button className="collapse-btn" onClick={()=>setCollapsed(!collapsed)} title="جمع/باز کردن منو"><Icon name="chevron"/></button>
      <div className="brand">
        <div className="brand-mark"><img src="https://novinacc.ir/wp-content/uploads/2023/07/cropped-%D9%84%D9%88%DA%AF%D9%88-300x83.png" alt="نوین پرداز" onError={(e)=>{e.currentTarget.style.display='none'; e.currentTarget.parentElement!.textContent='NP'}}/></div>
        <div><strong>نوین پرداز</strong><span>Accounting Platform</span></div>
      </div>
      <div className="company-switch"><div><span>شرکت فعال</span><b>نوین پرداز — شعبه مرکزی</b></div><Icon name="chevron" size={15}/></div>
      <div className="nav-title">منوی اصلی</div>
      <nav>{menu.map(item=><div key={item.id}>
        <button className={page===item.id?'nav-item active':'nav-item'} onClick={()=>item.children?toggle(item.id):setPage(item.id)}>
          <span className="nav-icon"><Icon name={item.icon as any}/></span><span>{item.label}</span>{item.children&&<Icon name="chevron" size={14}/>} 
        </button>
        {item.children&&open.includes(item.id)&&<div className="subnav">{item.children.map((c,i)=><button key={c} onClick={()=>setPage(item.id==='reports'&&c==='گزارش‌ساز'?'report-builder':item.id)} className={i===0&&page===item.id?'selected':''}>{c}</button>)}</div>}
      </div>)}</nav>
      <div className="sidebar-bottom">
        <button className="nav-item" onClick={()=>setSettings(true)}><span className="nav-icon"><Icon name="settings"/></span><span>تنظیمات برنامه</span></button>
        <div className="storage"><span className="dot"/><div><b>وضعیت سیستم</b><small>داده‌ها روی این دستگاه ذخیره می‌شوند</small></div></div>
      </div>
    </aside>
    <main className="main">
      <header className="topbar">
        <div className="demo-control">{DEMO_BUILD&&demo&&<button className="demo-delete" disabled={demoBusy} onClick={async()=>{if(!confirm('تمام محتوای نمونه حذف می‌شود. این عملیات برای نسخه توسعه است. ادامه می‌دهید؟'))return;setDemoBusy(true);try{await deleteDemo();setDemo(false);window.location.reload()}catch(e){alert(String(e))}finally{setDemoBusy(false)}}}>{demoBusy?'در حال حذف...':'حذف محتوای دمو'}</button>}</div>
        <div className="breadcrumbs"><span>شرکت نوین پرداز</span><b>/</b><strong>{page==='dashboard'?'داشبورد':page==='sales'?'فروش':page==='inventory'?'انبار و کالا':page==='contacts'?'اشخاص':page==='treasury'?'خزانه':page==='checks'?'چک‌ها':page==='accounting'?'حسابداری':page==='integrations'?'اتصالات و افزونه‌ها':page==='data-tools'?'ورود و خروج اطلاعات':page==='print-templates'?'قالب‌های چاپ':'گزارشات'}</strong></div>
        <div className="top-actions">
          <button className="global-search command-trigger" onClick={()=>setPalette(true)}><Icon name="search" size={17}/><span>جستجوی سریع یا اجرای دستور...</span><kbd>Ctrl K</kbd></button>
          <button className="icon-btn" onClick={()=>setDark(!dark)}><Icon name={dark?'sun':'moon'}/></button>
          <button className="icon-btn notification"><Icon name="bell"/><i/></button>
          <div className="profile"><div className="avatar">م</div><div><b>مدیر سیستم</b><span>مدیریت</span></div><Icon name="chevron" size={14}/></div>
        </div>
      </header>
      {page==='dashboard'?<Dashboard demo={demo} onSettings={()=>setSettings(true)} onNavigate={setPage}/>:page==='contacts'?<DataPage kind="contacts"/>:page==='inventory'?<AdvancedInventory/>:page==='accounting'?<Operations mode="accounting"/>:page==='reports'?<Reports/>:page==='report-builder'?<ReportBuilder/>:page==='integrations'?<Integrations/>:page==='data-tools'?<DataTools/>:page==='print-templates'?<PrintTemplates/>:page==='treasury'?<Treasury/>:page==='checks'?<Checks/>:<Invoices page={page}/>} 
    </main>
    {settings&&<SettingsCenter onClose={()=>setSettings(false)} dark={dark} setDark={setDark}/>}
    <CommandPalette open={palette} onClose={()=>setPalette(false)} onSelect={setPage}/>
  </div>
}
