
#[derive(Serialize)] struct TreasuryAccount { id:String, name:String, account_type:String, account_number:Option<String>, iban:Option<String>, is_active:bool }
#[derive(Serialize)] struct TreasuryTransaction { id:String, transaction_type:String, amount:i64, transaction_date:String, description:String, treasury_account_id:String, reference_type:Option<String>, reference_id:Option<String> }
#[derive(Serialize)] struct CheckSummary { id:String, check_type:String, check_number:String, party_id:Option<String>, amount:i64, issue_date:String, due_date:String, status:String, bank_name:Option<String>, treasury_account_id:Option<String> }

#[tauri::command]
fn list_treasury_accounts(state:State<AppState>)->Result<Vec<TreasuryAccount>,String>{
    let user=require_login(&state)?; let c=conn(&state)?;
    let mut st=c.prepare("SELECT t.id,t.name,t.account_type,t.account_number,t.iban,t.is_active FROM treasury_accounts t JOIN company_users cu ON cu.company_id=t.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY t.name").map_err(|e|e.to_string())?;
    let rows=st.query_map(params![user],|r|Ok(TreasuryAccount{id:r.get(0)?,name:r.get(1)?,account_type:r.get(2)?,account_number:r.get(3)?,iban:r.get(4)?,is_active:r.get::<_,i64>(5)?!=0})).map_err(|e|e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn create_treasury_account(state:State<AppState>,name:String,account_type:String,account_number:Option<String>,iban:Option<String>,linked_account_id:Option<String>)->Result<String,String>{
    if name.trim().is_empty(){return Err("TRE-001: نام حساب الزامی است".into())}
    if !["cash","bank","petty_cash"].contains(&account_type.as_str()){return Err("TRE-002: نوع حساب خزانه نامعتبر است".into())}
    let mut c=conn(&state)?; let user=require_permission(&state,&c,"treasury.account.create")?; let tx=c.transaction().map_err(|e|e.to_string())?; let (company,_fy)=active_context(&tx,&user)?;
    if let Some(a)=&linked_account_id { let ok:i64=tx.query_row("SELECT COUNT(*) FROM accounts WHERE id=?1 AND company_id=?2",params![a,company],|r|r.get(0)).unwrap_or(0); if ok==0{return Err("TRE-003: حساب حسابداری معتبر نیست".into())} }
    let id=format!("treasury-{}",chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default());
    tx.execute("INSERT INTO treasury_accounts(id,company_id,name,account_type,account_number,iban,linked_account_id) VALUES(?,?,?,?,?,?,?)",params![id,company,name,account_type,account_number,iban,linked_account_id]).map_err(|e|format!("TRE-004: {e}"))?;
    audit(&tx,&user,"treasury.account.create","treasury_account",&id,None,Some(&format!("{{\"name\":\"{}\",\"type\":\"{}\"}}",name,account_type)))?; tx.commit().map_err(|e|e.to_string())?; Ok(id)
}

#[tauri::command]
fn list_treasury_transactions(state:State<AppState>)->Result<Vec<TreasuryTransaction>,String>{
    let user=require_login(&state)?; let c=conn(&state)?;
    let mut st=c.prepare("SELECT t.id,t.transaction_type,t.amount,t.transaction_date,t.description,t.treasury_account_id,t.reference_type,t.reference_id FROM treasury_transactions t JOIN company_users cu ON cu.company_id=t.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY t.transaction_date DESC,t.created_at DESC").map_err(|e|e.to_string())?;
    let rows=st.query_map(params![user],|r|Ok(TreasuryTransaction{id:r.get(0)?,transaction_type:r.get(1)?,amount:r.get(2)?,transaction_date:r.get(3)?,description:r.get(4)?,treasury_account_id:r.get(5)?,reference_type:r.get(6)?,reference_id:r.get(7)?})).map_err(|e|e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn list_checks(state:State<AppState>)->Result<Vec<CheckSummary>,String>{
    let user=require_permission(&state,&conn(&state)?,"treasury.check.update")?; let c=conn(&state)?;
    let mut st=c.prepare("SELECT k.id,k.check_type,k.check_number,k.party_id,k.amount,k.issue_date,k.due_date,k.status,k.bank_name,k.treasury_account_id FROM checks k JOIN company_users cu ON cu.company_id=k.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY k.due_date").map_err(|e|e.to_string())?;
    let rows=st.query_map(params![user],|r|Ok(CheckSummary{id:r.get(0)?,check_type:r.get(1)?,check_number:r.get(2)?,party_id:r.get(3)?,amount:r.get(4)?,issue_date:r.get(5)?,due_date:r.get(6)?,status:r.get(7)?,bank_name:r.get(8)?,treasury_account_id:r.get(9)?})).map_err(|e|e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn create_check(state:State<AppState>,check_type:String,check_number:String,party_id:Option<String>,treasury_account_id:Option<String>,amount:i64,issue_date:String,due_date:String,bank_name:Option<String>,description:Option<String>)->Result<String,String>{
    if !["received","issued"].contains(&check_type.as_str()){return Err("CHK-001: نوع چک نامعتبر است".into())} if check_number.trim().is_empty(){return Err("CHK-002: شماره چک الزامی است".into())} if amount<=0{return Err("CHK-003: مبلغ چک باید بیشتر از صفر باشد".into())}
    let mut c=conn(&state)?; let user=require_permission(&state,&c,"treasury.check.create")?; let tx=c.transaction().map_err(|e|e.to_string())?; let (company,fy)=active_context(&tx,&user)?;
    if let Some(p)=&party_id {let ok:i64=tx.query_row("SELECT COUNT(*) FROM contacts WHERE id=?1 AND company_id=?2",params![p,company],|r|r.get(0)).unwrap_or(0); if ok==0{return Err("CHK-004: شخص معتبر نیست".into())}}
    if let Some(t)=&treasury_account_id {let ok:i64=tx.query_row("SELECT COUNT(*) FROM treasury_accounts WHERE id=?1 AND company_id=?2 AND is_active=1",params![t,company],|r|r.get(0)).unwrap_or(0); if ok==0{return Err("CHK-005: حساب خزانه معتبر نیست".into())}}
    let duplicate:i64=tx.query_row("SELECT COUNT(*) FROM checks WHERE company_id=?1 AND check_type=?2 AND check_number=?3 AND status<>'cancelled'",params![company,check_type,check_number],|r|r.get(0)).unwrap_or(0); if duplicate>0{return Err("CHK-006: شماره چک تکراری است".into())}
    let id=format!("check-{}",chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default());
    tx.execute("INSERT INTO checks(id,company_id,fiscal_year_id,check_type,check_number,party_id,treasury_account_id,amount,issue_date,due_date,status,bank_name,description,created_by) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)",params![id,company,fy,check_type,check_number,party_id,treasury_account_id,amount,issue_date,due_date,"registered",bank_name,description,user]).map_err(|e|format!("CHK-007: {e}"))?;
    audit(&tx,"user-demo","treasury.check.create","check",&id,None,Some("{\"status\":\"registered\"}"))?; tx.commit().map_err(|e|e.to_string())?; Ok(id)
}

#[tauri::command]
fn update_check_status(state:State<AppState>,check_id:String,new_status:String)->Result<(),String>{
    if !["registered","deposited","transferred","cleared","bounced","cancelled"].contains(&new_status.as_str()){return Err("CHK-008: وضعیت چک نامعتبر است".into())}
    let mut c=conn(&state)?; let user=require_permission(&state,&c,"treasury.check.update")?; let tx=c.transaction().map_err(|e|e.to_string())?;
    let old:String=tx.query_row("SELECT status FROM checks WHERE id=?1 AND company_id IN (SELECT company_id FROM company_users WHERE user_id=?2 AND is_active=1)",params![check_id,user],|r|r.get(0)).map_err(|_|"CHK-009: چک یافت نشد".to_string())?;
    let valid=matches!((old.as_str(),new_status.as_str()),("registered","deposited")|("registered","transferred")|("registered","cancelled")|("deposited","cleared")|("deposited","bounced")|("transferred","cleared")|("transferred","bounced"));
    if !valid{return Err(format!("CHK-010: انتقال وضعیت {} به {} مجاز نیست",old,new_status))}
    tx.execute("UPDATE checks SET status=?1 WHERE id=?2",params![new_status,check_id]).map_err(|e|e.to_string())?; audit(&tx,&user,"treasury.check.update","check",&check_id,Some(&format!("{{\"status\":\"{}\"}}",old)),Some(&format!("{{\"status\":\"{}\"}}",new_status)))?; tx.commit().map_err(|e|e.to_string())?; Ok(())
}

fn create_return_common(state:&State<AppState>,sale:bool,original_invoice_id:String,return_date:String,lines:Vec<(String,f64,i64)>)->Result<String,String>{
    let permission=if sale{"sales.return.create"}else{"purchase.return.create"}; let mut c=conn(state)?; let user=require_permission(state,&c,permission)?; let tx=c.transaction().map_err(|e|e.to_string())?;
    let table=if sale{"sales_invoices"}else{"purchase_invoices"}; let line_table=if sale{"sales_invoice_lines"}else{"purchase_invoice_lines"}; let return_table=if sale{"sales_returns"}else{"purchase_returns"}; let return_line=if sale{"sales_return_lines"}else{"purchase_return_lines"};
    let row:(String,String,String,Option<String>,Option<String>,String)= {let sql=format!("SELECT company_id,fiscal_year_id,status,contact_id,warehouse_id,payment_status FROM {table} WHERE id=?1"); tx.query_row(&sql,params![original_invoice_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(|_|"RET-001: فاکتور اصلی یافت نشد".to_string())?};
    if row.2!="posted"{return Err("RET-002: فقط فاکتور ثبت‌شده قابل برگشت است".into())} let wid=row.4.clone().ok_or("RET-003: فاکتور اصلی انبار ندارد".to_string())?;
    let mut total=0i64; for (pid,q,p) in &lines {if *q<=0.0{return Err("RET-004: مقدار برگشتی نامعتبر است".into())} let original:f64=tx.query_row(&format!("SELECT COALESCE(SUM(quantity),0) FROM {line_table} WHERE invoice_id=?1 AND product_id=?2"),params![original_invoice_id,pid],|r|r.get(0)).unwrap_or(0.0); let returned:f64=tx.query_row(&format!("SELECT COALESCE(SUM(rl.quantity),0) FROM {return_line} rl JOIN {return_table} rh ON rh.id=rl.return_id WHERE rh.original_invoice_id=?1 AND rl.product_id=?2 AND rh.status='posted'"),params![original_invoice_id,pid],|r|r.get(0)).unwrap_or(0.0); if *q>original-returned{return Err("RET-005: مقدار برگشتی بیشتر از مقدار قابل برگشت است".into())} total+=(*q*(*p as f64)).round() as i64;}
    let prefix=if sale{"sales-return"}else{"purchase-return"}; let number:i64=tx.query_row(&format!("SELECT COALESCE(MAX(number),0)+1 FROM {return_table} WHERE company_id=?1 AND fiscal_year_id=?2"),params![row.0,row.1],|r|r.get(0)).map_err(|e|e.to_string())?; let id=format!("{prefix}-{}",chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default());
    tx.execute(&format!("INSERT INTO {return_table}(id,company_id,fiscal_year_id,number,return_date,original_invoice_id,contact_id,warehouse_id,status,total,created_by) VALUES(?,?,?,?,?,?,?,?, 'draft',?,?)"),params![id,row.0,row.1,number,return_date,original_invoice_id,row.3,row.4,total,user]).map_err(|e|e.to_string())?;
    for (i,(pid,q,p)) in lines.iter().enumerate(){tx.execute(&format!("INSERT INTO {return_line}(id,return_id,product_id,quantity,unit_price,line_total) VALUES(?,?,?,?,?,?)"),params![format!("{id}-line-{}",i+1),id,pid,q,p,(*q*(*p as f64)).round() as i64]).map_err(|e|e.to_string())?;}
    audit(&tx,&user,permission,if sale{"sales_return"}else{"purchase_return"},&id,None,Some(&format!("{{\"total\":{}}}",total)))?; tx.commit().map_err(|e|e.to_string())?; Ok(id)
}

#[tauri::command] fn create_sales_return(state:State<AppState>,original_invoice_id:String,return_date:String,lines:Vec<(String,f64,i64)>)->Result<String,String>{create_return_common(&state,true,original_invoice_id,return_date,lines)}
#[tauri::command] fn create_purchase_return(state:State<AppState>,original_invoice_id:String,return_date:String,lines:Vec<(String,f64,i64)>)->Result<String,String>{create_return_common(&state,false,original_invoice_id,return_date,lines)}

fn post_return(state:&State<AppState>,return_id:String,sale:bool)->Result<(),String>{
    let permission=if sale{"sales.return.post"}else{"purchase.return.post"}; let mut c=conn(state)?; let user=require_permission(state,&c,permission)?; let tx=c.transaction().map_err(|e|e.to_string())?; let rt=if sale{"sales_returns"}else{"purchase_returns"}; let rl=if sale{"sales_return_lines"}else{"purchase_return_lines"};
    let row:(String,String,String,String,Option<String>,i64)=tx.query_row(&format!("SELECT company_id,fiscal_year_id,status,warehouse_id,journal_id,total FROM {rt} WHERE id=?1"),params![return_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(|_|"RET-006: برگشت یافت نشد".to_string())?;
    if row.2!="draft"{return Err("RET-007: فقط برگشت پیش‌نویس قابل ثبت است".into())} let wid=row.3.ok_or("RET-008: انبار برگشت مشخص نیست".to_string())?;
    let mut st=tx.prepare(&format!("SELECT product_id,quantity,unit_price FROM {rl} WHERE return_id=?1")).map_err(|e|e.to_string())?; let items:Vec<(String,f64,i64)>=st.query_map(params![return_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?.filter_map(Result::ok).collect(); drop(st);
    for (pid,q,p) in &items {let current:f64=tx.query_row("SELECT COALESCE(quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",params![pid,wid],|r|r.get(0)).unwrap_or(0.0); if !sale && current<*q{return Err("RET-009: موجودی برای برگشت خرید کافی نیست".into())} let newq=if sale{current+*q}else{current-*q}; tx.execute("INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?,?,?) ON CONFLICT(product_id,warehouse_id) DO UPDATE SET quantity=excluded.quantity,updated_at=CURRENT_TIMESTAMP",params![pid,wid,newq]).map_err(|e|e.to_string())?; let typ=if sale{"receipt"}else{"issue"}; tx.execute("INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,reference_type,reference_id,created_by) VALUES(?,?,?,?,?,?,?,?,?,?)",params![format!("return-stock-{}-{}",return_id,pid),row.0,pid,wid,typ,q,p,"invoice_return",return_id,user]).map_err(|e|e.to_string())?;}
    let jid=format!("journal-return-{return_id}"); let n:i64=tx.query_row("SELECT COALESCE(MAX(number),0)+1 FROM journal_entries WHERE company_id=?1 AND fiscal_year_id=?2",params![row.0,row.1],|r|r.get(0)).map_err(|e|e.to_string())?; tx.execute("INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,source_id,created_by) VALUES(?,?,?,?,?,'ثبت خودکار برگشت فاکتور','posted','invoice_return',?,?)",params![jid,row.0,row.1,n,chrono::Utc::now().format("%Y/%m/%d").to_string(),return_id,user]).map_err(|e|e.to_string())?;
    let (a,b) = if sale {("acc-4200","acc-1201")} else {("acc-2101","acc-5200")}; let lines=if sale{vec![(a,row.5,0),(b,0,row.5)]}else{vec![(a,row.5,0),(b,0,row.5)]}; for (i,(acc,d,cr)) in lines.iter().enumerate(){tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{jid}-line-{}",i+1),jid,acc,d,cr,"ثبت خودکار برگشت فاکتور"]).map_err(|e|e.to_string())?;}
    tx.execute(&format!("UPDATE {rt} SET status='posted',journal_id=?1 WHERE id=?2"),params![jid,return_id]).map_err(|e|e.to_string())?; audit(&tx,&user,permission,if sale{"sales_return"}else{"purchase_return"},&return_id,Some("{\"status\":\"draft\"}"),Some("{\"status\":\"posted\"}"))?; tx.commit().map_err(|e|e.to_string())?; Ok(())
}
#[tauri::command] fn post_sales_return(state:State<AppState>,id:String)->Result<(),String>{post_return(&state,id,true)}
#[tauri::command] fn post_purchase_return(state:State<AppState>,id:String)->Result<(),String>{post_return(&state,id,false)}
