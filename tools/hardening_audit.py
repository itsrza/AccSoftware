from pathlib import Path
import re, json, sys

root=Path(__file__).resolve().parents[1]
_host=root/'apps/desktop-host/src-tauri/src'
rust=(_host/'main.rs').read_text(encoding='utf-8')
host_tree='\n'.join(f.read_text(encoding='utf-8') for f in sorted(_host.rglob('*.rs')))
db=(root/'crates/novin-core/src/db/mod.rs').read_text(encoding='utf-8')  # دیتابیس در هسته است
ui_pkg=json.loads((root/'apps/desktop-ui/package.json').read_text(encoding='utf-8'))
api=(root/'apps/desktop-ui/src/api.ts').read_text(encoding='utf-8')
app=(root/'apps/desktop-ui/src/App.tsx').read_text(encoding='utf-8')
errors=[]

def need(cond,msg):
    if not cond: errors.append(msg)

# Pass A: architecture and version integrity
_v=str(ui_pkg['version'])
need(len(_v.split('.'))==3,'UI version must be semver')
need('version.workspace = true' in (root/'apps/desktop-host/src-tauri/Cargo.toml').read_text(encoding='utf-8'),'Rust package must use workspace version')
need(f'"version": "{_v}"' in (root/'package.json').read_text(encoding='utf-8'),'Root package version mismatch')
need(f'"version": "{_v}"' in (root/'apps/desktop-host/src-tauri/tauri.conf.json').read_text(encoding='utf-8'),'Tauri version mismatch')
need(f'version = "{_v}"' in (root/'Cargo.toml').read_text(encoding='utf-8'),'Workspace version mismatch')
need('SQLite' in (root/'docs/ARCHITECTURE.md').read_text(encoding='utf-8'),'Architecture documentation missing SQLite')
need('Tauri' in (root/'docs/ARCHITECTURE.md').read_text(encoding='utf-8'),'Architecture documentation missing Tauri')
need('React' in (root/'docs/ARCHITECTURE.md').read_text(encoding='utf-8'),'Architecture documentation missing React')
need('tauri::generate_handler!' in rust,'Tauri command handler missing')
need('env_clear()' in rust and 'Duration::from_secs(15)' in rust,'Plugin worker isolation controls missing')
need('keyring::Entry' in host_tree,'OS secure secret storage missing')
need(re.search(r'base\.scheme\(\)\s*!=\s*"https"',host_tree),'External API HTTPS enforcement missing')

# Pass B: accounting/data integrity
need(re.search(r'debit\s*!=\s*credit',rust),'Double-entry balance guard missing')
need(re.search(r'validate_fiscal_date\(&tx,\s*&fy,\s*&date\)',rust),'Invoice fiscal-date validation missing')
need(re.search(r'validate_fiscal_date\(&tx,\s*&fiscal_id,\s*&entry_date\)',rust),'Journal posting fiscal-date validation missing')
need('WHERE company_id=?1 AND fiscal_year_id=?2' in rust,'Company/fiscal scoping pattern missing')
need('ON CONFLICT(product_id,warehouse_id)' in rust,'Inventory upsert missing')
need('FOREIGN KEY' not in db or 'REFERENCES' in db,'SQLite references missing')
need('pragma_update(None, "foreign_keys", "ON")' in db and 'pragma_update(None, "journal_mode", "WAL")' in db,'SQLite integrity pragmas missing')
need('CREATE TABLE IF NOT EXISTS journal_entries' in db and 'CREATE TABLE IF NOT EXISTS journal_lines' in db,'Journal schema missing')
need('CREATE TABLE IF NOT EXISTS treasury_accounts' in db and 'CREATE TABLE IF NOT EXISTS treasury_transactions' in db,'Treasury schema missing')
need('CREATE TABLE IF NOT EXISTS checks' in db,'Checks schema missing')

# Pass C: security/integration/UI consistency
for perm in ['plugins.view','plugins.manage','plugins.execute','native.execute','integrations.view','integrations.manage','integrations.execute']:
    need(f"'{perm}','{perm}'" in db,f'Missing permission {perm}')
need('company_users WHERE user_id=?2 AND cu.is_active=1' in rust or 'cu.user_id=?2 AND cu.is_active=1' in rust,'Company membership enforcement missing')
need('api_profiles' in host_tree and 'allowed_domains' in host_tree,'API allowlist missing')
need(re.search(r'"host"\s*\|\s*"authorization"\s*\|\s*"cookie"',host_tree),'Sensitive request header block missing')
need('plugins.execute' in rust and 'native.execute' in rust,'Plugin execution permission gate missing')
need('Plugin / Native Worker' in (root/'apps/desktop-ui/src/pages/Integrations.tsx').read_text(encoding='utf-8'),'Integration UI missing')
need('getApiProfiles' in api and 'getPlugins' in api,'Frontend integration API bindings missing')
need('Integrations' in app,'Integrations route/UI missing')

# No obvious fake-form residue in active production pages.
for f in root.glob('apps/desktop-ui/src/**/*.tsx'):
    t=f.read_text(encoding='utf-8')
    if 'فرم نمونه برای مرحله UI نسخه 0.2' in t:
        errors.append(f'Old fake/demo UI marker remains in {f}')

if errors:
    print('FAIL')
    for e in errors: print('-',e)
    sys.exit(1)
print('PASS: architecture/data-integrity/security/UI consistency checks')
