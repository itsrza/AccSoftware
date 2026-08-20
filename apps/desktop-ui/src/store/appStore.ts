import {create} from 'zustand';
export type Invoice={id:string,number:string,customer:string,date:string,total:number,status:'draft'|'posted'|'paid'};
export type Product={id:string,sku:string,name:string,stock:number,unit:string,price:number};
const seedInvoices:Invoice[]=[{id:'1',number:'1405-00021',customer:'شرکت آریا تجارت',date:'1405/05/29',total:185000000,status:'paid'},{id:'2',number:'1405-00022',customer:'فروشگاه پارس',date:'1405/05/30',total:74000000,status:'posted'},{id:'3',number:'1405-00023',customer:'محمد رضایی',date:'1405/05/30',total:28500000,status:'draft'}];
const seedProducts:Product[]=[{id:'1',sku:'1001',name:'پرینتر حرارتی X100',stock:42,unit:'دستگاه',price:12500000},{id:'2',sku:'1002',name:'بارکدخوان Pro',stock:18,unit:'دستگاه',price:8900000},{id:'3',sku:'1003',name:'لیبل حرارتی 50×30',stock:1260,unit:'رول',price:185000}];
type State={invoices:Invoice[];products:Product[];dark:boolean;addInvoice:(i:Invoice)=>void;toggleTheme:()=>void};
export const useAppStore=create<State>((set)=>({invoices:seedInvoices,products:seedProducts,dark:false,addInvoice:(i)=>set(s=>({invoices:[i,...s.invoices]})),toggleTheme:()=>set(s=>({dark:!s.dark}))}));
