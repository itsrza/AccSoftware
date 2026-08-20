from pathlib import Path
import re, zipfile, sys
ROOT=Path(__file__).resolve().parents[1]
main=(ROOT/'apps/desktop-host/src-tauri/src/main.rs').read_text()
db=(ROOT/'apps/desktop-host/src-tauri/src/db/mod.rs').read_text()
app=(ROOT/'apps/desktop-ui/src/App.tsx').read_text()
api=(ROOT/'apps/desktop-ui/src/api.ts').read_text()
checks=[]
def ck(name,cond): checks.append((name,bool(cond)))
ck('v1.7 schema print_templates', 'CREATE TABLE IF NOT EXISTS print_templates' in db)
ck('v1.7 schema import_batches', 'CREATE TABLE IF NOT EXISTS import_batches' in db)
for cmd in ['get_demo_status','delete_demo_data','list_print_templates','save_print_template','delete_print_template','import_data']:
    ck(f'command {cmd}', f'fn {cmd}' in main and cmd in main[main.find('generate_handler!'):])
for perm in ['printing.template.view','printing.template.manage']:
    ck(f'permission {perm}', perm in db)
for api_fn in ['getDemoStatus','getPrintTemplates','savePrintTemplate','deletePrintTemplate','importData']:
    ck(f'api {api_fn}', f'export const {api_fn}' in api)
for page in ['DataTools','PrintTemplates']:
    ck(f'page {page}', f"./pages/{page}'" in app)
ck('temporary demo header control', 'DEMO_BUILD' in app and 'demo-delete' in app)
ck('demo seed connected invoice', 'demo-sale-1' in db and 'demo-sale-line-1' in db)
ck('demo seed accounting', 'demo-journal-1' in db and 'demo-journal-2' in db)
ck('demo seed inventory movement', 'demo-mov-1' in db and 'demo-mov-2' in db)
ck('demo seed check', 'demo-check-1' in db)
ck('demo template', 'tpl-demo-invoice' in db)
ck('demo deletion includes invoices', 'DELETE FROM sales_invoices' in main)
ck('demo deletion includes inventory', 'DELETE FROM inventory_movements' in main and 'DELETE FROM inventory_balances' in main)
ck('demo deletion includes templates/imports', 'DELETE FROM print_templates' in main and 'DELETE FROM import_batches' in main)
ck('import row limit', 'rows.len()>10000' in main)
ck('import transaction', 'let tx=c.transaction()' in main[main.find('fn import_data'):main.find('fn import_data')+1000])
ck('print template audit', 'print.template.save' in main and 'print.template.delete' in main)
ck('readme version', 'Version: **1.7.0**' in (ROOT/'README.md').read_text())
failed=[n for n,c in checks if not c]
print(f'PASS {sum(c for _,c in checks)}/{len(checks)}')
if failed:
 print('FAILED:'); print('\n'.join(failed)); sys.exit(1)
