import { invoke } from '@tauri-apps/api/core'
import { designPreviewInvoke, isDesignPreview } from './lib/devPreview'
import { toAppError } from './lib/errors'

export type Contact={id:string,name:string,kind:string,mobile?:string,is_customer:boolean,is_supplier:boolean}
export type Product={id:string,sku:string,barcode?:string,name:string,unit:string,sale_price:number,purchase_price:number,min_stock:number}
export type Account={id:string,code:string,name:string,level:string,parent_id?:string,nature:string}
export type Journal={id:string,number:number,entry_date:string,description:string,status:string,total_debit:number,total_credit:number}
export type Warehouse={id:string,name:string,code:string,is_active:boolean}
export type StockBalance={product_id:string,warehouse_id:string,quantity:number,reserved_quantity:number,available_quantity:number}

/**
 * تنها دروازه‌ی فراخوانی بک‌اند.
 *
 * هر خطای IPC اینجا به `AppError` ساخت‌یافته تبدیل می‌شود تا هیچ متن فنی خامی
 * به لایه‌ی نمایش نشت نکند.
 */
export async function api<T>(command:string,args?:Record<string,unknown>):Promise<T>{
 try{
  // در مرورگرِ حالت توسعه پل IPC وجود ندارد؛ فقط برای بازبینی چیدمان از
  // شبیه‌ساز پیش‌نمایش استفاده می‌شود. در بیلد Tauri هرگز اجرا نمی‌شود.
  if(isDesignPreview()) return await designPreviewInvoke<T>(command,args)
  return await invoke<T>(command,args)
 }catch(error){
  throw toAppError(error)
 }
}
export const login=(username:string,password:string)=>api<{id:string,username:string,display_name:string}>('login',{username,password})
export const getContacts=()=>api<Contact[]>('list_contacts')
export const getProducts=()=>api<Product[]>('list_products')
export const getAccounts=()=>api<Account[]>('list_accounts')
export const getJournals=()=>api<Journal[]>('list_journals')
export const deleteDemo=()=>api<void>('delete_demo_data')
export const createJournal=(entry_date:string,description:string,lines:Array<[string,number,number]>)=>api<string>('create_journal',{entryDate:entry_date,description,lines})

export const getWarehouses=()=>api<Warehouse[]>('list_warehouses')
export const getStockBalances=()=>api<StockBalance[]>('list_stock_balances')
export const createContact=(name:string,kind:'person'|'company',mobile:string|undefined,is_customer:boolean,is_supplier:boolean)=>api<string>('create_contact',{name,kind,mobile,isCustomer:is_customer,isSupplier:is_supplier})
export const createProduct=(sku:string,barcode:string|undefined,name:string,unit:string,sale_price:number,purchase_price:number,min_stock:number)=>api<string>('create_product',{sku,barcode,name,unit,salePrice:sale_price,purchasePrice:purchase_price,minStock:min_stock})
export const receiveStock=(product_id:string,warehouse_id:string,quantity:number,unit_cost:number,note?:string)=>api<string>('receive_stock',{productId:product_id,warehouseId:warehouse_id,quantity,unitCost:unit_cost,note})
export const issueStock=(product_id:string,warehouse_id:string,quantity:number,note?:string)=>api<string>('issue_stock',{productId:product_id,warehouseId:warehouse_id,quantity,note})

export const updateContact=(id:string,name:string,kind:'person'|'company',mobile:string|undefined,is_customer:boolean,is_supplier:boolean)=>api<void>('update_contact',{id,name,kind,mobile,isCustomer:is_customer,isSupplier:is_supplier})
export const deleteContact=(id:string)=>api<void>('delete_contact',{id})
export const updateProduct=(id:string,sku:string,barcode:string|undefined,name:string,unit:string,sale_price:number,purchase_price:number,min_stock:number)=>api<void>('update_product',{id,sku,barcode,name,unit,salePrice:sale_price,purchasePrice:purchase_price,minStock:min_stock})
export const deleteProduct=(id:string)=>api<void>('delete_product',{id})
export const transferStock=(product_id:string,from_warehouse_id:string,to_warehouse_id:string,quantity:number,note?:string)=>api<string>('transfer_stock',{productId:product_id,fromWarehouseId:from_warehouse_id,toWarehouseId:to_warehouse_id,quantity,note})
export const adjustStock=(product_id:string,warehouse_id:string,new_quantity:number,note:string)=>api<string>('adjust_stock',{productId:product_id,warehouseId:warehouse_id,newQuantity:new_quantity,note})

export type InvoiceSummary={id:string,number:number,invoice_date:string,contact_id?:string,warehouse_id?:string,status:string,payment_status:string,subtotal:number,discount:number,tax:number,total:number}
export const createSalesInvoice=(invoice_date:string,contact_id:string|undefined,warehouse_id:string|undefined,lines:Array<[string,number,number,number,number]>)=>api<string>('create_sales_invoice',{invoiceDate:invoice_date,contactId:contact_id,warehouseId:warehouse_id,lines})
export const createPurchaseInvoice=(invoice_date:string,contact_id:string|undefined,warehouse_id:string|undefined,lines:Array<[string,number,number,number,number]>)=>api<string>('create_purchase_invoice',{invoiceDate:invoice_date,contactId:contact_id,warehouseId:warehouse_id,lines})
export const getSalesInvoices=()=>api<InvoiceSummary[]>('list_sales_invoices')
export const getPurchaseInvoices=()=>api<InvoiceSummary[]>('list_purchase_invoices')
export const postSalesInvoice=(id:string)=>api<void>('post_sales_invoice',{id})
export const postPurchaseInvoice=(id:string)=>api<void>('post_purchase_invoice',{id})

export const settleInvoice=(invoice_id:string,sale:boolean,amount:number,settlement_date:string)=>api<string>('settle_invoice',{invoiceId:invoice_id,sale,amount,settlementDate:settlement_date})

export type TreasuryAccount={id:string,name:string,account_type:string,account_number?:string,iban?:string,is_active:boolean}
export type TreasuryTransaction={id:string,transaction_type:string,amount:number,transaction_date:string,description:string,treasury_account_id:string,reference_type?:string,reference_id?:string}
export type CheckSummary={id:string,check_type:string,check_number:string,party_id?:string,amount:number,issue_date:string,due_date:string,status:string,bank_name?:string,treasury_account_id?:string}
export const getTreasuryAccounts=()=>api<TreasuryAccount[]>('list_treasury_accounts')
export const createTreasuryAccount=(name:string,account_type:'cash'|'bank'|'petty_cash',account_number?:string,iban?:string,linked_account_id?:string)=>api<string>('create_treasury_account',{name,accountType:account_type,accountNumber:account_number,iban,linkedAccountId:linked_account_id})
export const updateTreasuryAccount=(id:string,name:string,account_number?:string,iban?:string,linked_account_id?:string,is_active=true)=>api<void>('update_treasury_account',{id,name,accountNumber:account_number,iban,linkedAccountId:linked_account_id,isActive:is_active})
export const getTreasuryTransactions=()=>api<TreasuryTransaction[]>('list_treasury_transactions')
export const getTreasuryTransactionsFiltered=(treasury_account_id?:string,from_date?:string,to_date?:string)=>api<TreasuryTransaction[]>('list_treasury_transactions_filtered',{treasuryAccountId:treasury_account_id,fromDate:from_date,toDate:to_date})
export type TreasuryStatementLine={id:string,transaction_type:string,amount:number,transaction_date:string,description:string,running_balance:number,reference_type?:string,reference_id?:string}
export const getTreasuryStatement=(treasury_account_id:string,from_date?:string,to_date?:string)=>api<TreasuryStatementLine[]>('get_treasury_statement',{treasuryAccountId:treasury_account_id,fromDate:from_date,toDate:to_date})
export type TreasurySummary={id:string,name:string,account_type:string,balance:number,inflow:number,outflow:number,transaction_count:number,linked_account_id?:string}
export const getTreasurySummary=()=>api<TreasurySummary[]>('get_treasury_summary')
export const getChecks=()=>api<CheckSummary[]>('list_checks')
export const getChecksFiltered=(check_type?:'received'|'issued',status?:string,from_due_date?:string,to_due_date?:string)=>api<CheckSummary[]>('list_checks_filtered',{checkType:check_type,status,fromDueDate:from_due_date,toDueDate:to_due_date})
export type CheckDashboard={total_received:number,total_issued:number,received_count:number,issued_count:number,due_soon_count:number,overdue_count:number,bounced_count:number}
export const getCheckDashboard=()=>api<CheckDashboard>('get_check_dashboard')
export const createCheck=(check_type:'received'|'issued',check_number:string,party_id:string|undefined,treasury_account_id:string|undefined,amount:number,issue_date:string,due_date:string,bank_name?:string,description?:string)=>api<string>('create_check',{checkType:check_type,checkNumber:check_number,partyId:party_id,treasuryAccountId:treasury_account_id,amount,issueDate:issue_date,dueDate:due_date,bankName:bank_name,description})
export const updateCheckStatus=(check_id:string,new_status:string)=>api<void>('update_check_status',{checkId:check_id,newStatus:new_status})
export const createSalesReturn=(original_invoice_id:string,return_date:string,lines:Array<[string,number,number]>)=>api<string>('create_sales_return',{originalInvoiceId:original_invoice_id,returnDate:return_date,lines})
export const createPurchaseReturn=(original_invoice_id:string,return_date:string,lines:Array<[string,number,number]>)=>api<string>('create_purchase_return',{originalInvoiceId:original_invoice_id,returnDate:return_date,lines})
export const postSalesReturn=(id:string)=>api<void>('post_sales_return',{id})
export const postPurchaseReturn=(id:string)=>api<void>('post_purchase_return',{id})

export type TreasuryBalance={id:string,name:string,account_type:string,balance:number,linked_account_id?:string}
export type AccountBalance={id:string,code:string,name:string,debit:number,credit:number,balance:number,nature:string}
export type TrialBalance={total_debit:number,total_credit:number,accounts:AccountBalance[]}
export const createTreasuryTransaction=(transaction_type:'receipt'|'payment',treasury_account_id:string,offset_account_id:string,amount:number,transaction_date:string,description:string)=>api<string>('create_treasury_transaction',{transactionType:transaction_type,treasuryAccountId:treasury_account_id,offsetAccountId:offset_account_id,amount,transactionDate:transaction_date,description})
export const createTreasuryTransfer=(from_account_id:string,to_account_id:string,amount:number,transaction_date:string,description:string)=>api<string>('create_treasury_transfer',{fromAccountId:from_account_id,toAccountId:to_account_id,amount,transactionDate:transaction_date,description})
export const getTreasuryBalances=()=>api<TreasuryBalance[]>('list_treasury_balances')
export const getTrialBalance=()=>api<TrialBalance>('get_trial_balance')

export type LedgerLine={date:string,journal_number:number,journal_id:string,description:string,account_id:string,debit:number,credit:number,running_balance:number}
export type PartyBalance={contact_id:string,contact_name:string,invoice_count:number,invoiced:number,settled:number,remaining:number}
export type CashPosition={total:number,accounts:TreasuryBalance[]}
export type PeriodStatus={id:string,title:string,start_date:string,end_date:string,is_closed:boolean,draft_journals:number,posted_journals:number}
export const getAccountLedger=(account_id:string,from_date?:string,to_date?:string)=>api<LedgerLine[]>('get_account_ledger',{accountId:account_id,fromDate:from_date,toDate:to_date})
export const getReceivables=()=>api<PartyBalance[]>('get_receivables')
export const getPayables=()=>api<PartyBalance[]>('get_payables')
export const getCashPosition=()=>api<CashPosition>('get_cash_position')
export const getFiscalPeriodStatus=()=>api<PeriodStatus>('get_fiscal_period_status')
export const closeFiscalYear=()=>api<void>('close_fiscal_year')
export const verifyBackupFile=(name:string)=>api<string>('verify_backup_file',{name})

export type DashboardKpi={sales:number,purchases:number,gross_profit:number,receivables:number,payables:number,cash:number,inventory_value:number,low_stock_count:number}
export type SalesTrend={period:string,sales:number,purchases:number}
export type TopProduct={product_id:string,name:string,quantity:number,revenue:number}
export type LowStock={product_id:string,name:string,quantity:number,min_stock:number,warehouse_count:number}
export type RecentInvoice={id:string,number:number,invoice_date:string,contact_name?:string,total:number,payment_status:string,invoice_type:string}
export type ProfitLoss={revenue:number,sales_returns:number,net_revenue:number,cogs:number,gross_profit:number,gross_margin_percent:number}
export const getDashboardKpis=()=>api<DashboardKpi>('get_dashboard_kpis')
export const getSalesTrend=()=>api<SalesTrend[]>('get_sales_trend')
export const getTopProducts=()=>api<TopProduct[]>('get_top_products')
export const getLowStockReport=()=>api<LowStock[]>('get_low_stock_report')
export const getRecentInvoices=()=>api<RecentInvoice[]>('get_recent_invoices')
export const getProfitLoss=()=>api<ProfitLoss>('get_profit_loss')

export type StockCardLine={date:string,movement_type:string,quantity:number,unit_cost:number,balance:number,reference_type?:string,note?:string}
export type InventoryValuation={product_id:string,product_name:string,warehouse_id:string,warehouse_name:string,quantity:number,average_cost:number,value:number}
export type SalesReportRow={date:string,invoice_number:number,contact_name?:string,subtotal:number,discount:number,tax:number,total:number,payment_status:string}
export type PurchaseReportRow=SalesReportRow
export type AccountLedgerSummary={account_id:string,code:string,name:string,debit:number,credit:number,balance:number}
export const getStockCard=(product_id:string,warehouse_id?:string,from_date?:string,to_date?:string)=>api<StockCardLine[]>('get_stock_card',{productId:product_id,warehouseId:warehouse_id,fromDate:from_date,toDate:to_date})
export const getInventoryValuation=()=>api<InventoryValuation[]>('get_inventory_valuation')
export const getSalesReport=(from_date?:string,to_date?:string)=>api<SalesReportRow[]>('get_sales_report',{fromDate:from_date,toDate:to_date})
export const getPurchaseReport=(from_date?:string,to_date?:string)=>api<PurchaseReportRow[]>('get_purchase_report',{fromDate:from_date,toDate:to_date})
export const getAccountLedgerSummary=(from_date?:string,to_date?:string)=>api<AccountLedgerSummary[]>('get_account_ledger_summary',{fromDate:from_date,toDate:to_date})

export type PluginInfo={id:string,name:string,version:string,description?:string,enabled:boolean,permissions:string[]}
export type ApiProfile={id:string,name:string,base_url:string,auth_type:'none'|'api_key'|'bearer'|'basic',auth_header?:string,timeout_ms:number,enabled:boolean,allowed_domains:string}
export type ApiResponse={status:number,body:string,content_type?:string}
export const getPlugins=()=>api<PluginInfo[]>('list_plugins')
export const registerPlugin=(manifest_json:string,executable_path:string)=>api<string>('register_plugin',{manifestJson:manifest_json,executablePath:executable_path})
export const setPluginEnabled=(plugin_id:string,enabled:boolean)=>api<void>('set_plugin_enabled',{pluginId:plugin_id,enabled})
export const executePlugin=(plugin_id:string,payload:string)=>api<string>('execute_plugin',{pluginId:plugin_id,payload})
export const getApiProfiles=()=>api<ApiProfile[]>('list_api_profiles')
export const createApiProfile=(name:string,base_url:string,auth_type:ApiProfile['auth_type'],auth_header:string|undefined,timeout_ms:number,allowed_domains:string,secret?:string)=>api<string>('create_api_profile',{name,baseUrl:base_url,authType:auth_type,authHeader:auth_header,timeoutMs:timeout_ms,allowedDomains:allowed_domains,secret})
export const executeApiRequest=(profile_id:string,method:string,path:string,headers_json?:string,body?:string)=>api<ApiResponse>('execute_api_request',{profileId:profile_id,method,path,headersJson:headers_json,body})
export const setApiProfileEnabled=(profile_id:string,enabled:boolean)=>api<void>('set_api_profile_enabled',{profileId:profile_id,enabled})

export type InventoryAdvanced={product_id:string,warehouse_id:string,quantity:number,reserved_quantity:number,in_transit_quantity:number,available_quantity:number,valuation_method:string,average_cost:number,inventory_value:number,expiring_quantity:number}
export type InventoryLot={id:string,product_id:string,warehouse_id:string,lot_number:string,lot_type:string,serial_number?:string,manufacture_date?:string,expiry_date?:string,quantity:number,unit_cost:number,status:string}
export type InventoryCount={id:string,warehouse_id:string,title:string,count_date:string,status:string,line_count:number,variance_count:number}
export type InventoryTransferOrder={id:string,product_id:string,from_warehouse_id:string,to_warehouse_id:string,quantity:number,unit_cost:number,status:string,note?:string}
export const getInventoryAdvanced=()=>api<InventoryAdvanced[]>('list_inventory_advanced')
export const getInventoryValuationMethod=()=>api<string>('get_inventory_valuation_method')
export const setInventoryValuationMethod=(method:'fifo'|'moving_average'|'weighted_average')=>api<void>('set_inventory_valuation_method',{method})
export const reserveInventory=(product_id:string,warehouse_id:string,quantity:number,reference_type?:string,reference_id?:string)=>api<string>('reserve_inventory',{productId:product_id,warehouseId:warehouse_id,quantity,referenceType:reference_type,referenceId:reference_id})
export const releaseInventory=(reservation_id:string)=>api<void>('release_inventory',{reservationId:reservation_id})
export const createInventoryLot=(product_id:string,warehouse_id:string,lot_number:string,lot_type:'batch'|'serial',serial_number:string|undefined,manufacture_date:string|undefined,expiry_date:string|undefined,quantity:number,unit_cost:number)=>api<string>('create_inventory_lot',{productId:product_id,warehouseId:warehouse_id,lotNumber:lot_number,lotType:lot_type,serialNumber:serial_number,manufactureDate:manufacture_date,expiryDate:expiry_date,quantity,unitCost:unit_cost})
export const getInventoryLots=(product_id?:string,warehouse_id?:string)=>api<InventoryLot[]>('list_inventory_lots',{productId:product_id,warehouseId:warehouse_id})
export const createInventoryCount=(warehouse_id:string,title:string,count_date:string)=>api<string>('create_inventory_count',{warehouseId:warehouse_id,title,countDate:count_date})
export const getInventoryCounts=()=>api<InventoryCount[]>('list_inventory_counts')
export const setInventoryCountLine=(line_id:string,counted_quantity:number,recount_quantity?:number,note?:string)=>api<void>('set_inventory_count_line',{lineId:line_id,countedQuantity:counted_quantity,recountQuantity:recount_quantity,note})
export const postInventoryCount=(session_id:string)=>api<void>('post_inventory_count',{sessionId:session_id})
export const createInventoryTransferOrder=(product_id:string,from_warehouse_id:string,to_warehouse_id:string,quantity:number,unit_cost:number,note?:string)=>api<string>('create_inventory_transfer_order',{productId:product_id,fromWarehouseId:from_warehouse_id,toWarehouseId:to_warehouse_id,quantity,unitCost:unit_cost,note})
export const receiveInventoryTransfer=(transfer_id:string)=>api<void>('receive_inventory_transfer',{transferId:transfer_id})
export const getInventoryTransferOrders=()=>api<InventoryTransferOrder[]>('list_inventory_transfer_orders')

export type JournalBookLine={date:string,number:number,description:string,account_code:string,account_name:string,debit:number,credit:number}
export type ReportLine={code:string,name:string,amount:number,nature:string}
export type FinancialStatement={title:string,as_of:string,lines:ReportLine[],total:number}
export type PartyAging={contact_id:string,contact_name:string,current:number,days_1_30:number,days_31_60:number,days_61_90:number,over_90:number,total:number}
export const getJournalBook=(from_date?:string,to_date?:string)=>api<JournalBookLine[]>('get_journal_book',{fromDate:from_date,toDate:to_date})
export const getFinancialStatement=(statement:'balance_sheet'|'income_statement',as_of?:string)=>api<FinancialStatement>('get_financial_statement',{statement,asOf:as_of})
export const getPartyAging=(sales:boolean,as_of?:string)=>api<PartyAging[]>('get_party_aging',{sales,asOf:as_of})

export type SavedReport={id:string,name:string,source:string,config_json:string,created_at:string,updated_at:string}
export const listCustomReports=()=>api<SavedReport[]>('list_custom_reports')
export const saveCustomReport=(id:string|undefined,name:string,source:string,config_json:string)=>api<string>('save_custom_report',{id,name,source,configJson:config_json})
export const deleteCustomReport=(id:string)=>api<void>('delete_custom_report',{id})

export type PrintTemplate={id:string,name:string,template_type:string,content_html:string,is_default:boolean}
export const getDemoStatus=()=>api<boolean>('get_demo_status')
export const getPrintTemplates=()=>api<PrintTemplate[]>('list_print_templates')
export const savePrintTemplate=(id:string|undefined,name:string,template_type:string,content_html:string,is_default:boolean)=>api<string>('save_print_template',{id,name,templateType:template_type,contentHtml:content_html,isDefault:is_default})
export const deletePrintTemplate=(id:string)=>api<void>('delete_print_template',{id})
export const importData=(entity_type:string,rows:unknown[])=>api<string>('import_data',{entityType:entity_type,rowsJson:JSON.stringify(rows)})

// --- فاز ۲: ابعاد مالی و سند یک‌سطری (مرجع: تصویر Rb2xiG) ---
export type PostableAccount = {
  id: string
  code: string
  name: string
  nature: string
  requires_subsidiary: boolean
  subsidiary_group_id?: string
  requires_cost_center: boolean
  requires_project: boolean
}
export type DimensionOption = {id: string; code: string; title: string}
export type PostingSideInput = {
  accountId: string
  subsidiaryId?: string
  costCenterId?: string
  projectId?: string
}

export const getPostableAccounts = () => api<PostableAccount[]>('list_postable_accounts')
export const getCostCenters = () => api<DimensionOption[]>('list_cost_centers')
export const getProjects = () => api<DimensionOption[]>('list_projects')
export const getSubsidiaryGroups = () => api<DimensionOption[]>('list_subsidiary_groups')

const toSide = (side: PostingSideInput) => ({
  account_id: side.accountId,
  subsidiary_id: side.subsidiaryId || null,
  cost_center_id: side.costCenterId || null,
  project_id: side.projectId || null,
})

export const createSingleLineJournal = (
  entry_date: string,
  description: string,
  amount: number,
  debit: PostingSideInput,
  credit: PostingSideInput,
) =>
  api<string>('create_single_line_journal', {
    entryDate: entry_date,
    description,
    amount,
    debit: toSide(debit),
    credit: toSide(credit),
  })

// --- فاز ۳: کاتالوگ کالا (مرجع: تصاویر 8Xmc1p و NztJl5) ---
export type ProductGroupRow = {
  id: string
  code: string
  title: string
  parent_id?: string
  product_count: number
}
export type PriceLevelRow = {level: string; label: string; price: number | null}
export type ProductPriceRow = {
  id: string
  sku: string
  name: string
  kind: string
  kind_label: string
  group_title?: string
  prices: PriceLevelRow[]
}

export const getProductGroups = () => api<ProductGroupRow[]>('list_product_groups')
export const getProductPrices = () => api<ProductPriceRow[]>('list_product_prices')
export const setProductPrice = (productId: string, level: string, price: number | null) =>
  api<void>('set_product_price', {productId, level, price})

// --- فاز ۴: اشخاص (مرجع: تصاویر c9pvYl و 1zkKV5) ---
export type PartyRow = {
  id: string
  code: string
  display_name: string
  party_type: string
  party_type_label: string
  party_function: string
  party_function_label: string
  group_title: string
  is_customer: boolean
  is_supplier: boolean
  mobile?: string
  route_title?: string
  marketer_name?: string
  credit_limit: number
  balance: number
  balance_status: string
  balance_indicator: string
}
export type PartySummary = {
  debtor_count: number
  debtor_total: number
  creditor_count: number
  creditor_total: number
  settled_count: number
  total_count: number
  net_total: number
}
export type PartyListResult = {rows: PartyRow[]; summary: PartySummary}
export type RouteRow = {id: string; code: string; title: string}

export const getParties = () => api<PartyListResult>('list_parties')
export const getPartyRoutes = () => api<RouteRow[]>('list_party_routes')

export const validatePartyIdentity = (input: {
  partyType: string
  nationalId: string | null
  economicCode: string | null
  postalCode: string | null
  mobile: string | null
  iban: string | null
  cardNumber: string | null
}) => api<string[]>('validate_party_identity', {...input})

export const updatePartyProfile = (input: {
  contactId: string
  partyType: string
  partyFunction: string
  nationalId: string | null
  economicCode: string | null
  postalCode: string | null
  creditLimit: number
  routeId: string | null
  marketerId: string | null
}) => api<void>('update_party_profile', {...input})

// --- فاز ۵: محاسبه‌ی زنده‌ی فاکتور (مرجع: sFpxWK، PI5uot، FRPBDr) ---
export type InvoiceLineInput = {
  product_id: string
  quantity: number
  unit_price: number
  discount_amount?: number
  discount_bp?: number
  vat_bp?: number
  duty_bp?: number
  commission_bp?: number
  unit_cost?: number
  serials?: string[]
  serial_tracked?: boolean
}
export type ComputedLineRow = {
  gross: number
  tier_discount: number
  line_discount: number
  header_discount_share: number
  coupon_share: number
  total_discount: number
  net: number
  freight_share: number
  duty: number
  vat: number
  total: number
  commission: number
  cost: number
  profit: number
}
export type InvoicePreview = {
  lines: ComputedLineRow[]
  subtotal: number
  discount_total: number
  net_total: number
  freight: number
  duty_total: number
  vat_total: number
  total: number
  commission_total: number
  cost_total: number
  profit: number
  profit_margin_bp: number
  balance_before: number
  balance_after: number
  invoice_remainder: number
}
export type InstallmentRow = {
  number: number
  due_date: string
  due_date_jalali: string
  amount: number
}

export const previewInvoice = (input: {
  lines: InvoiceLineInput[]
  headerDiscount: number
  freight: number
  freightAllocated: boolean
  contactId: string | null
  received: number
}) => api<InvoicePreview>('preview_invoice', {...input})

export const buildInstallmentPlan = (
  total: number,
  downPayment: number,
  count: number,
  firstDueJalali: string,
) => api<InstallmentRow[]>('build_installment_plan', {total, downPayment, count, firstDueJalali})

// --- فاز ۶: انبارگردانی و عملیات جمعی ---
export type StocktakeSessionRow = {
  id: string
  title: string
  warehouse_name: string
  count_date: string
  status: string
  status_label: string
  total_lines: number
  counted_lines: number
  variance_lines: number
}
export type StocktakeLineRow = {
  id: string
  product_id: string
  product_name: string
  sku: string
  frozen_quantity: number
  counted_quantity: number | null
  recount_quantity: number | null
  final_quantity: number | null
  variance: number | null
  variance_value: number
  variance_approved: boolean
  needs_recount: boolean
  unit_cost: number
}
export type StocktakeDetail = {
  id: string
  title: string
  status: string
  status_label: string
  warehouse_name: string
  count_date: string
  lines: StocktakeLineRow[]
  total_lines: number
  counted_lines: number
  uncounted_lines: number
  surplus_lines: number
  shortage_lines: number
  unapproved_variances: number
  surplus_value: number
  shortage_value: number
  net_value: number
  recount_threshold_percent: number
  can_post: boolean
  blocking_reason: string | null
}
export type BulkPriceRow = {
  product_id: string
  product_name: string
  old_price: number
  new_price: number
  difference: number
}
export type LowStockRow = {
  product_id: string
  product_name: string
  sku: string
  quantity: number
  reorder_point: number
}
export type ValuationInfo = {
  method: string
  label: string
  explanation: string
  is_active: boolean
}

export const getStocktakes = () => api<StocktakeSessionRow[]>('list_stocktakes')
export const getStocktake = (sessionId: string) =>
  api<StocktakeDetail>('get_stocktake', {sessionId})
export const createStocktake = (warehouseId: string, title: string, countDate: string) =>
  api<string>('create_stocktake', {warehouseId, title, countDate})
export const setStocktakeCount = (
  lineId: string,
  quantity: number | null,
  isRecount: boolean,
  approve: boolean | null,
) => api<void>('set_stocktake_count', {lineId, quantity, isRecount, approve})
export const approveAllVariances = (sessionId: string) =>
  api<number>('approve_all_variances', {sessionId})
export const postStocktake = (sessionId: string) => api<string>('post_stocktake', {sessionId})

export const previewBulkPrice = (
  productIds: string[],
  mode: 'percent' | 'amount' | 'set',
  value: number,
  roundTo: number,
) => api<BulkPriceRow[]>('preview_bulk_price_change', {productIds, mode, value, roundTo})
export const applyBulkPrice = (
  productIds: string[],
  mode: 'percent' | 'amount' | 'set',
  value: number,
  roundTo: number,
) => api<number>('apply_bulk_price_change', {productIds, mode, value, roundTo})

export const getLowStock = () => api<LowStockRow[]>('get_low_stock')
export const getValuationMethods = () => api<ValuationInfo[]>('list_valuation_methods')
