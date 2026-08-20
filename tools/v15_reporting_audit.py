from pathlib import Path
import sys, zipfile, json
ROOT=Path(__file__).resolve().parents[1]
R=(ROOT/'apps/desktop-host/src-tauri/src/main.rs').read_text()
A=(ROOT/'apps/desktop-ui/src/api.ts').read_text()
U=(ROOT/'apps/desktop-ui/src/pages/Reports.tsx').read_text()
P=(ROOT/'apps/desktop-ui/package.json').read_text()
checks={
 'trial_scope_company':'a.company_id=?1' in R and "j.company_id=?1" in R and "j.fiscal_year_id=?2" in R,
 'trial_posted_only':"j.status='posted'" in R,
 'trial_totals': 'total_debit:accounts.iter().map' in R and 'total_credit:accounts.iter().map' in R,
 'journal_book_command':'fn get_journal_book' in R and "j.status='posted'" in R,
 'journal_book_date_filter':'j.entry_date BETWEEN ?3 AND ?4' in R,
 'financial_statement_command':'fn get_financial_statement' in R,
 'balance_sheet_classes':"substr(a.code,1,1) IN ('1','2','3')" in R,
 'income_statement_classes':"substr(a.code,1,1) IN ('4','5','6')" in R,
 'financial_statement_company_scope':'a.company_id=?1' in R,
 'financial_statement_fy_scope':'j.fiscal_year_id=?2' in R,
 'aging_command':'fn get_party_aging' in R,
 'aging_sales_purchase':'table=if sales{"sales_invoices"}else{"purchase_invoices"}' in R,
 'aging_settlements_scoped':'settlement_date<=?3' in R and 'company_id=?1' in R,
 'aging_buckets': 'days<=30' in R and 'days<=60' in R and 'days<=90' in R,
 'report_permission_existing':'reporting.view' in R,
 'api_journal':'getJournalBook' in A and 'get_journal_book' in A,
 'api_statements':'getFinancialStatement' in A and 'get_financial_statement' in A,
 'api_aging':'getPartyAging' in A and 'get_party_aging' in A,
 'ui_journal_tab':"['journal','دفتر روزنامه']" in U,
 'ui_balance_tab':"['balance','ترازنامه']" in U,
 'ui_income_tab':"['income','سود و زیان تفصیلی']" in U,
 'ui_aging_tabs':"['agingReceivable','سن مطالبات']" in U and "['agingPayable','سن بدهی‌ها']" in U,
 'ui_loading_error_empty':'در حال محاسبه گزارش' in U and 'error-box' in U and 'داده‌ای برای نمایش وجود ندارد' in U,
 'version_1_5':'1.5.0' in P,
 'handler_registration': all(x in R for x in ['get_journal_book','get_financial_statement','get_party_aging']),
}
failed=[k for k,v in checks.items() if not v]
print(f"REPORT AUDIT: {len(checks)-len(failed)}/{len(checks)} PASS")
if failed:
 print('FAILED:',*failed,sep='\n- '); sys.exit(1)
