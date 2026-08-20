import {useEffect,useMemo,useState} from 'react'
import {Icon} from './Icon'

type Item={id:string,label:string,group:string,icon?:string}
export function CommandPalette({open,onClose,onSelect}:{open:boolean,onClose:()=>void,onSelect:(id:string)=>void}){
 const [q,setQ]=useState('')
 const items:Item[]=[
  {id:'dashboard',label:'داشبورد',group:'ناوبری',icon:'grid'},{id:'sales',label:'فروش',group:'ناوبری',icon:'receipt'},{id:'purchase',label:'خرید',group:'ناوبری',icon:'cart'},
  {id:'inventory',label:'کالا و انبار',group:'ناوبری',icon:'package'},{id:'contacts',label:'اشخاص',group:'ناوبری',icon:'users'},{id:'treasury',label:'خزانه',group:'ناوبری',icon:'wallet'},
  {id:'checks',label:'چک‌ها',group:'ناوبری',icon:'check'},{id:'accounting',label:'حسابداری',group:'ناوبری',icon:'file'},{id:'reports',label:'گزارشات',group:'ناوبری',icon:'bar'},{id:'integrations',label:'اتصالات و افزونه‌ها',group:'ناوبری',icon:'settings'}]
 const filtered=useMemo(()=>items.filter(x=>x.label.includes(q.trim())),[q])
 useEffect(()=>{if(!open)return;setQ('');const f=(e:KeyboardEvent)=>{if(e.key==='Escape')onClose()};window.addEventListener('keydown',f);return()=>window.removeEventListener('keydown',f)},[open,onClose])
 if(!open)return null
 return <div className="command-backdrop" onClick={onClose}><div className="command-palette" onClick={e=>e.stopPropagation()}>
  <div className="command-input"><Icon name="search"/><input autoFocus value={q} onChange={e=>setQ(e.target.value)} placeholder="دستور یا صفحه را جستجو کنید..."/><kbd>ESC</kbd></div>
  <div className="command-list">{filtered.map(x=><button key={x.id} onClick={()=>{onSelect(x.id);onClose()}}><Icon name={x.icon as any}/><span>{x.label}</span><small>{x.group}</small><Icon name="chevron" size={14}/></button>)}{!filtered.length&&<div className="empty-state"><p>نتیجه‌ای پیدا نشد.</p></div>}</div>
  <div className="command-foot"><span>Enter انتخاب</span><span>↑ ↓ حرکت</span><span>Esc بستن</span></div>
 </div></div>
}
