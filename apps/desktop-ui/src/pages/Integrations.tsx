import {useEffect,useState} from 'react'
import {createApiProfile,getApiProfiles,getPlugins,setApiProfileEnabled,setPluginEnabled,ApiProfile,PluginInfo} from '../api'

export function Integrations(){
  const [profiles,setProfiles]=useState<ApiProfile[]>([])
  const [plugins,setPlugins]=useState<PluginInfo[]>([])
  const [loading,setLoading]=useState(true)
  const [form,setForm]=useState({name:'',baseUrl:'https://',authType:'none' as ApiProfile['auth_type'],authHeader:'',timeout:'10000',domains:'',secret:''})
  const [error,setError]=useState('')
  const load=async()=>{setLoading(true);setError('');try{const [p,g]=await Promise.all([getApiProfiles(),getPlugins()]);setProfiles(p);setPlugins(g)}catch(e){setError(String(e))}finally{setLoading(false)}}
  useEffect(()=>{void load()},[])
  const create=async()=>{try{setError('');await createApiProfile(form.name,form.baseUrl,form.authType,form.authHeader||undefined,Number(form.timeout),form.domains,form.secret||undefined);setForm({...form,name:'',secret:''});await load()}catch(e){setError(String(e))}}
  return <section className="page" dir="rtl"><div className="page-head"><div><h1>اتصالات و افزونه‌ها</h1><p>مدیریت API و Native Workerها با کنترل دسترسی</p></div></div>
    {error&&<div className="error-box">{error}</div>}
    <div className="panel"><div className="panel-head"><h2>اتصالات API</h2></div><div className="form-grid">
      <input placeholder="نام اتصال" value={form.name} onChange={e=>setForm({...form,name:e.target.value})}/><input placeholder="Base URL" value={form.baseUrl} onChange={e=>setForm({...form,baseUrl:e.target.value})}/>
      <select value={form.authType} onChange={e=>setForm({...form,authType:e.target.value as ApiProfile['auth_type']})}><option value="none">بدون احراز هویت</option><option value="api_key">API Key</option><option value="bearer">Bearer</option><option value="basic">Basic</option></select>
      <input placeholder="Allowed Domain" value={form.domains} onChange={e=>setForm({...form,domains:e.target.value})}/><input placeholder="نام Header برای API Key" value={form.authHeader} onChange={e=>setForm({...form,authHeader:e.target.value})}/><input type="number" placeholder="Timeout (ms)" value={form.timeout} onChange={e=>setForm({...form,timeout:e.target.value})}/>
      {form.authType!=='none'&&<input type="password" placeholder="Secret — در Secure Storage سیستم ذخیره می‌شود" value={form.secret} onChange={e=>setForm({...form,secret:e.target.value})}/>}<button onClick={()=>void create()}>ثبت اتصال</button>
    </div><div className="data-list">{loading?<div>در حال بارگذاری...</div>:profiles.length===0?<div className="empty">اتصال API ثبت نشده است.</div>:profiles.map(p=><div className="data-row" key={p.id}><div><b>{p.name}</b><span>{p.base_url}</span></div><button className={p.enabled?'switch on':'switch'} onClick={()=>void setApiProfileEnabled(p.id,!p.enabled).then(load)}><i/></button></div>)}</div></div>
    <div className="panel"><div className="panel-head"><h2>Plugin / Native Worker</h2></div>{plugins.length===0?<div className="empty">Plugin ثبت نشده است.</div>:plugins.map(p=><div className="data-row" key={p.id}><div><b>{p.name} <small>{p.version}</small></b><span>{p.description||p.id}</span></div><button className={p.enabled?'switch on':'switch'} onClick={()=>void setPluginEnabled(p.id,!p.enabled).then(load)}><i/></button></div>)}</div>
  </section>
}
