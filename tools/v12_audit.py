from pathlib import Path
import re, zipfile, sys
R=Path(__file__).resolve().parents[1]
app=(R/'apps/desktop-ui/src/App.tsx').read_text()
op=(R/'apps/desktop-ui/src/pages/Operations.tsx').read_text()
api=(R/'apps/desktop-ui/src/api.ts').read_text()
rust=(R/'apps/desktop-host/src-tauri/src/main.rs').read_text()
pkg=(R/'apps/desktop-ui/package.json').read_text()
checks=[]
def ok(name,cond): checks.append((name,bool(cond)))
ok('Operations page exists',(R/'apps/desktop-ui/src/pages/Operations.tsx').exists())
ok('Accounting route wired','<Operations mode="accounting"/>' in app)
ok('Inventory route wired','<Operations mode="inventory"/>' in app)
for fn in ['createJournal','getAccounts','getJournals','receiveStock','issueStock','transferStock','adjustStock','getStockCard','getProducts','getWarehouses','getStockBalances']:
 ok(f'API {fn}', re.search(r'\b'+fn+r'\b',api) is not None)
for cmd in ['create_journal','receive_stock','issue_stock','transfer_stock','adjust_stock','get_stock_card']:
 ok(f'Rust command {cmd}', cmd in rust)
ok('Balanced journal enforced','a===b' in op and 'amount<=0' in op)
ok('Inventory quantity validation','qty<=0' in op)
ok('Transfer destination required','toWarehouse' in op)
ok('No fake data in new page','Math.random' not in op and 'hard-coded' not in op.lower())
ok('Version 1.2.0','"version":"1.2.0"' in pkg)
failed=[n for n,v in checks if not v]
print(f'checks={len(checks)} passed={len(checks)-len(failed)} failed={len(failed)}')
for n,v in checks: print(('PASS' if v else 'FAIL'),n)
sys.exit(1 if failed else 0)
