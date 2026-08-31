# وضعیت فعلی ریپازیتوری AccSoftware + پرامپت‌های اجرایی

## خلاصه‌ی بررسی (بر اساس کد واقعی ریپازیتوری، بررسی‌شده در تاریخ این گزارش)

⚠️ توضیح مهم: امکان باز کردن تصاویر شما در snipboard.io وجود نداشت (آن دامنه در دسترس این ابزار نبود)، اما ریپازیتوری گیت‌هاب واقعی شما مستقیماً clone و بررسی شد؛ نتیجه بر اساس کد واقعی است، نه حدس.

### ۱) هیچ‌کدام از ۹ پرامپت قبلی (بخش ۱ تا ۹) اجرا نشده‌اند
آخرین کامیت ریپازیتوری فقط یک اصلاح بیلد ویندوز است. نشانه‌های زیر که باید بعد از اجرای پرامپت‌ها ظاهر می‌شدند، در کد وجود ندارند:
- ماژول `errors` در main.rs → **وجود ندارد**
- جدول/مکانیزم `account_mappings` → **وجود ندارد**
- تابع `gen_id` → **وجود ندارد**
- حساب‌های hardcoded (`acc-4100`, `acc-1101`, `acc-2101`, `acc-1201`, `acc-5100`) → **هنوز در main.rs موجودند (۹ مورد)**
- باگ `f64::EPSILON` در adjust_stock → **هنوز اصلاح نشده**
- `redirect::Policy::none()` در execute_api_request → **وجود ندارد (باگ SSRF هنوز باز است)**
- pragma های per-connection در تابع `conn()` → **هنوز اعمال نمی‌شوند**

**نتیجه: تمام ۹ پرامپت قبلی (که قبلاً در فایل AccSoftware-Hardening-Prompts.md داده شد) هنوز باید دقیقاً به همان ترتیب اجرا شوند.** آن فایل را دوباره پیوست این گزارش می‌بینید (بدون تغییر در محتوای پرامپت‌های ۱ تا ۸، با یک اصلاح کوچک در تصحیح بخش ۶ که قبلاً هم اعمال شده بود).

### ۲) یافته‌های جدید از مقایسه با اسکرین‌شات‌های شما (توضیحات متنی زیر تصاویر)

| ویژگی توصیف‌شده | وضعیت واقعی در کد |
|---|---|
| افزودن کالای جدید / کالای ساده | ✅ کاملاً پیاده‌سازی شده (`DataPage.tsx` + `create_product`/`update_product`) |
| اشخاص / افزودن شخص جدید | ✅ کاملاً پیاده‌سازی شده (`DataPage.tsx` + `create_contact`/`update_contact`) |
| حساب‌های بانکی / تعریف بانک جدید / صندوق‌ها | ✅ پیاده‌سازی شده (`Treasury.tsx` + `create_treasury_account`) |
| چک‌ها (دریافتی/پرداختی/سررسید) | ✅ پیاده‌سازی شده (`Checks.tsx` + `create_check`/`update_check_status`) — با باگ‌های حسابداری که در بخش ۵ پرامپت‌های قبلی مستند شد |
| فاکتور فروش / برگشت از فروش / فاکتور خرید | ✅ پیاده‌سازی شده (`Invoices.tsx` + `create_sales_return`/`create_purchase_return`) |
| سند دریافت/پرداخت | ✅ پیاده‌سازی شده (`create_treasury_transaction`) |
| سند حسابداری یک‌سطری | ✅ پیاده‌سازی شده (`create_journal`/`create_journal_internal`) |
| **تولید و فرمول تولید** | ❌ **در منو وجود دارد ولی هیچ پیاده‌سازی واقعی ندارد** — نه در Backend (main.rs) هیچ Command برای BOM/فرمول تولید/دستور کار تولید وجود دارد، نه در Frontend هیچ صفحه یا فرمی. کلیک روی «تولید و فرمول» در منو صرفاً به صفحه‌ی عمومی انبار (`AdvancedInventory.tsx`) هدایت می‌شود که هیچ ارتباطی با ساخت محصول از مواد اولیه ندارد. |
| منوها و امکانات (کلی) | ✅ ساختار منو کامل و منطقی است |
| **تنظیمات اصلی برنامه** | ⚠️ **فقط ۳ از ۱۲ گروه واقعی هستند.** گروه‌های «حسابداری»، «فروش و خرید»، «انبار»، «خزانه و چک»، «چاپ»، «پشتیبان‌گیری»، «اتصالات»، «ظاهر»، «امنیت و کاربران» در خود کد (`SettingsCenter.tsx`) با متن صریح «این بخش برای اتصال به تنظیمات واقعی ماژول آماده است» علامت‌گذاری شده‌اند — یعنی پوسته‌ی UI ساخته شده ولی هیچ تنظیمی واقعاً ذخیره/اعمال نمی‌شود. |

---

## نحوه‌ی استفاده از این فایل

این فایل شامل **۱۰ پرامپت** است. باید **دقیقاً به‌ترتیب زیر** به ایجنت کدنویسی (Claude Code) داده شوند، هرکدام بعد از تکمیل کامل و کامیت‌شدنِ قبلی:

۱ تا ۸: همان پرامپت‌های اصلاحی قبلی (Auth/DB، Contacts/Products، Inventory، Invoices، Treasury/Checks، Reports، Integrations، Frontend) — **بدون تغییر، چون هنوز هیچ‌کدام اجرا نشده‌اند.**
۹ (جدید): تکمیل ویژگی‌های ناقص/غایب که در بررسی این دور کشف شد (Manufacturing/BOM + تکمیل واقعی مرکز تنظیمات).
۱۰: بیلد نهایی + تست End-to-End + گزارش جامع (نسخه‌ی به‌روزشده که بخش ۹ جدید را هم پوشش می‌دهد).

---

## بخش ۱: Auth/Users + لایه اتصال به دیتابیس

```
نرم‌افزار حسابداری «نوین پرداز» (Tauri/Rust در apps/desktop-host/src-tauri/src + React-TS در apps/desktop-ui/src) را در ریپازیتوری گیت‌هاب باز کن و اصلاحات زیر را دقیقاً طبق جزئیات فنی داده‌شده، بخش‌به‌بخش و به ترتیب زیر اعمال کن. هر بخش را کامل تمام کن (کامیت جداگانه بزن) قبل از رفتن به بخش بعدی. برای هر اصلاح، تست واحد یا سناریوی تست دستی مشخص‌شده را هم اضافه/اجرا کن.

═══════════════════════════════════════════
بخش ۱: لایه اتصال به دیتابیس + Auth/Users
فایل‌های هدف: apps/desktop-host/src-tauri/src/main.rs (خطوط ۸۸-۲۰۰ تقریباً)، apps/desktop-host/src-tauri/src/db/mod.rs
═══════════════════════════════════════════

مشکل ۱ (بحرانی - امنیت/صحت داده):
تابع db::open در db/mod.rs این pragmaها را فقط یک‌بار در startup تنظیم می‌کند:
foreign_keys=ON, journal_mode=WAL, synchronous=NORMAL
اما تابع conn() در main.rs (که در همه‌ی ۱۳۸ Command Handler استفاده می‌شود) یک Connection::open() خام باز می‌کند بدون این pragmaها. چون foreign_keys در SQLite per-connection است، در عمل هیچ عملیات واقعی برنامه (ثبت فاکتور، سند حسابداری، حرکت انبار) از یکپارچگی ارجاعی محافظت نمی‌شود و رکورد یتیم کاملاً ممکن است.

اصلاح: تابع conn() در main.rs را طوری بازنویسی کن که بعد از Connection::open() این سه خط را اضافه کند:
conn.pragma_update(None, "foreign_keys", "ON")
conn.pragma_update(None, "journal_mode", "WAL")
conn.pragma_update(None, "synchronous", "NORMAL")
هر کدام با map_err جداگانه و کد خطای مشخص (مثلاً APP-002, APP-003, APP-004).

تست: یک تست واحد بنویس که با اتصال حاصل از conn() یک رکورد journal_lines با account_id نامعتبر (که در accounts وجود ندارد) درج کند و انتظار Err داشته باشد. همچنین یک تست که بلافاصله بعد از conn() مقدار PRAGMA foreign_keys و PRAGMA journal_mode را می‌خواند و برابر 1 و wal بودن را تایید می‌کند.
قبل از merge، روی دیتابیس‌های موجود (اگر نمونه‌ای برای تست هست) PRAGMA foreign_key_check اجرا کن تا رکوردهای یتیم قدیمی شناسایی شوند (این pragma از قبل در main.rs خط ۴۳۱۰ وجود دارد، از همان استفاده کن).

مشکل ۲ (امنیت متوسط):
Command با نام current_user در main.rs (خط ۱۷۵) کاربر را فقط بر اساس id از جدول users می‌خواند بدون شرط is_active=1، برخلاف تمام Query های مشابه دیگر در همان فایل که همه‌جا is_active=1 چک می‌کنند (مثلاً خط ۱۴۵ در login). یعنی اگر کاربری در حین اجرای برنامه در دیتابیس غیرفعال شود، تا ری‌استارت برنامه هنوز لاگین محسوب می‌شود.

اصلاح: در Query داخل current_user شرط "AND is_active=1" را به WHERE اضافه کن. اگر query_row نتیجه‌ای برنگرداند (QueryReturnedNoRows)، علاوه بر برگرداندن Ok(None)، مقدار state.user_id را هم به None ریست کن تا سشن واقعاً پاک شود، نه فقط پاسخ.

تست: یک کاربر را لاگین کن، سپس مستقیم در دیتابیس is_active آن کاربر را 0 کن، بدون ری‌استارت برنامه current_user را دوباره فراخوانی کن. باید None برگردد و بعد از آن state.user_id هم واقعاً خالی باشد (تست با فراخوانی current_user دوباره: نباید کاربر برگردد حتی اگر is_active دوباره 1 شود، مگر login مجدد انجام شود).

مشکل ۳ (تمیزی کد - mojibake):
تمام رشته‌های خطای فارسی در main.rs (به‌خصوص در بخش Auth: خطوط ۹۴، ۱۰۲، ۱۰۴، ۱۲۵، ۱۳۲، ۱۳۵، ۱۴۵، ۱۴۷، ۱۵۰، ۱۶۸، ۱۷۸) دچار Encoding خراب چندلایه هستند (نتیجه‌ی چندبار انکود/دیکود اشتباه UTF-8، نه یک‌بار ساده). این بایت‌ها قابل بازیابی خودکار مطمئن نیستند.

اصلاح: به‌جای تلاش برای decode کردن بایت‌های خراب، متن فارسی صحیح را بر اساس معنای کد شناسایی‌شده (کد خطا مثل AUTH-003 و context تابع) از نو بنویس. یک ماژول errors در بالای main.rs (بعد از mod db;) اضافه کن با ثابت‌های &str زیر (متن دقیق را بر اساس معنای هر خطا خودت تولید کن، مثال‌ها را می‌توانی به‌کار ببری یا بهتر کنی):
- APP_001: قفل فایل داده در دسترس نیست
- AUTH_001: وضعیت ورود در دسترس نیست
- AUTH_002: ابتدا وارد حساب کاربری شوید
- AUTH_003: نام کاربری یا رمز عبور نادرست است
- AUTH_004: اطلاعات ورود معتبر نیست
سپس در توابع conn، require_login، has_permission، require_permission، audit، login، logout، current_user این ثابت‌ها را به‌جای رشته‌های خام فعلی جایگزین کن.
این کار را برای تمام mojibake های کل main.rs (نه فقط بخش Auth) تکرار کن؛ در هر بخش بعدی این پرامپت، فهرست خطوط مربوط به همان بخش داده شده — همان الگو (ماژول errors با ثابت‌های مجزا به‌ازای هر پیشوند کد خطا مثل CONTACT_, PRODUCT_, INV_, TRE_, CHK_, RET_, REP_, API_, PLUGIN_, IMPORT_) را برای همه اعمال کن.

مشکل ۴ (طراحی - مستندسازی، بدون تغییر رفتار):
هیچ Command برای create_user، change_password یا مدیریت نقش‌ها در main.rs وجود ندارد؛ کاربران باید مستقیم در دیتابیس درج شوند. این را در docs/IMPLEMENTATION_STATUS.md به‌عنوان یک Known Gap ثبت کن (فقط مستندسازی، پیاده‌سازی کامل این Commandها را در این پرامپت انجام نده مگر جداگانه خواسته شود).

بعد از اتمام این بخش: کامیت با پیام "fix(db+auth): enforce per-connection pragmas, fix current_user active check, resolve mojibake in auth errors"

═══════════════════════════════════════════
دستورالعمل کلی برای بخش‌های بعدی (Contacts، Products/Inventory، Invoices، Treasury/Checks، Reports، Integrations/Plugins/API، Import/Export، Frontend) که در پیام‌های بعدی جزئیات هرکدام داده خواهد شد:
═══════════════════════════════════════════
برای هر بخش، طبق همین قالب عمل کن:
۱. فقط Queryها/توابع/فایل‌های همان بخش را تغییر بده؛ به بخش‌های دیگر دست نزن مگر وابستگی صریح اعلام شده باشد.
۲. اگر در همان بخش الگوی format!/string interpolation برای نام جدول یا فیلد دیدی، بررسی کن که آیا مقدار از ورودی کاربر می‌آید یا از یک enum/if-else داخلی کد؛ فقط در حالت اول باید validate_identifier (تعریف‌شده در security.rs ولی در کل main.rs هیچ‌جا استفاده نشده) واقعاً فراخوانی شود.
۳. هر تغییر باید تست واحد یا سناریوی تست دستی مشخص همراه داشته باشد.
۴. بعد از هر بخش یک کامیت جداگانه با پیام مشخص بزن؛ کامیت‌ها را مخلوط نکن.
۵. در پایان همه‌ی بخش‌ها، یک فایل docs/HARDENING_REPORT.md بساز که جدول زیر را داشته باشد: ستون‌های بخش، طراحی، عملکرد صحیح، امنیت، سرعت، عدم تداخل، تمیزی کد — با مقدار هرکدام یکی از (اصلاح‌شده / نیاز به بررسی بیشتر / بدون مشکل).
```

---

## بخش ۲: Contacts + Products (داده‌های پایه)

```
ادامه‌ی اصلاحات نرم‌افزار حسابداری «نوین پرداز» را روی ریپازیتوری گیت‌هاب اعمال کن. این بخش (Contacts + Products) را کامل تمام کن، تست‌های مشخص‌شده را اضافه/اجرا کن و در پایان یک کامیت مجزا بزن. به بخش قبلی (Auth/DB Connection) دست نزن مگر تابع conn() که در بخش قبل اصلاح شد و اینجا فقط استفاده می‌شود.

═══════════════════════════════════════════
بخش ۲: Contacts + Products (داده‌های پایه)
فایل هدف: apps/desktop-host/src-tauri/src/main.rs
توابع درگیر: create_contact (~خط ۳۳۸)، update_contact (~خط ۳۷۲)، delete_contact (~خط ۴۲۹)، create_product (~خط ۴۶۵)، update_product (~خط ۱۳۴۹)، delete_product (~خط ۱۴۱۰)
═══════════════════════════════════════════

مشکل ۱ (بحرانی - عملکرد صحیح / یکپارچگی داده):
در توابع create_contact، update_contact، create_product، update_product یک الگوی اشتباه تکرار شده: عملیات اصلی (INSERT یا UPDATE روی جدول contacts/products) مستقیماً روی اتصال c اجرا می‌شود و بلافاصله commit واقعی می‌گردد (auto-commit پیش‌فرض SQLite)، سپس یک تراکنش (tx) جداگانه فقط برای INSERT کردن رکورد audit_logs باز و commit می‌شود. یعنی INSERT/UPDATE اصلی و ثبت Audit Log در یک تراکنش اتمیک نیستند. اگر بین این دو مرحله (مثلاً به‌خاطر خرابی برنامه یا خطای audit) وقفه بیفتد، تغییر روی داده‌ی اصلی ثبت می‌شود ولی هیچ Audit Log ای برایش وجود نخواهد داشت — که برای یک نرم‌افزار حسابداری با نیاز به ردیابی کامل تغییرات (Audit Trail) قابل‌قبول نیست. تابع delete_contact و delete_product این الگو را درست پیاده کرده‌اند (عملیات اصلی داخل همان tx که audit هم در آن انجام می‌شود) - این دو تابع را به‌عنوان الگوی صحیح مرجع قرار بده.

اصلاح: در هر چهار تابع (create_contact، update_contact، create_product، update_product) کد را بازآرایی کن به این شکل: ابتدا یک تراکنش با c.transaction() باز کن، عملیات INSERT/UPDATE اصلی را روی همان tx (نه c) اجرا کن، سپس audit را روی همان tx فراخوانی کن، و در پایان فقط یک‌بار tx.commit() بزن. مثال برای create_contact:

قبل:
    c.execute("INSERT INTO contacts(...) VALUES(...)", params![...]).map_err(|e|format!("CONTACT-004: {e}"))?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    audit(&tx, &user, "contact.create", "contact", &id, None, Some(...))?;
    tx.commit().map_err(|e| e.to_string())?;

بعد:
    let tx = c.transaction().map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO contacts(...) VALUES(...)", params![...]).map_err(|e|format!("CONTACT-004: {e}"))?;
    audit(&tx, &user, "contact.create", "contact", &id, None, Some(...))?;
    tx.commit().map_err(|e| e.to_string())?;

همین تغییر را عیناً برای update_contact، create_product و update_product انجام بده (متغیر c باید mut بماند چون transaction() نیاز به &mut Connection دارد؛ این از قبل mut است، تغییری در امضای تابع لازم نیست).

تست: برای هر چهار تابع یک تست بنویس (یا سناریوی دستی) که: (۱) عملیات را با موفقیت انجام دهد و بعد چک کند هم رکورد اصلی هم رکورد audit_logits با یک commit واحد ثبت شده؛ (۲) با تزریق خطای مصنوعی در audit (مثلاً با یک entity_id بیش از حد طولانی که محدودیت طول دارد اگر وجود دارد، یا با موقتاً دستکاری تست) مطمئن شو که در صورت شکست audit، رکورد اصلی هم رول‌بک می‌شود و در دیتابیس باقی نمی‌ماند (قبل از این اصلاح این تست شکست می‌خورد چون INSERT/UPDATE اصلی از قبل commit شده بود).

مشکل ۲ (طراحی/عملکرد - عدم وجود محدودیت یکتایی روی Contacts):
جدول contacts (تعریف در db/mod.rs) هیچ UNIQUE constraint ندارد، نه روی mobile و نه روی ترکیب name+company_id، برخلاف جدول products که UNIQUE(company_id,sku) و یک Unique Index شرطی روی barcode دارد. یعنی در حال حاضر می‌توان یک مشتری/تأمین‌کننده را با نام و موبایل کاملاً یکسان چندین بار ثبت کرد که در عمل باعث پراکندگی حساب‌های تجاری و اشتباه در گزارش بدهکاران/بستانکاران می‌شود.

اصلاح: در db/mod.rs یک Migration جدید اضافه کن (به همان سبک Migrationهای موجود در فایل - با column_exists/index_exists مشابه الگوهای دیگر migrate) که یک Unique Index شرطی ایجاد کند:
CREATE UNIQUE INDEX IF NOT EXISTS idx_contacts_company_mobile_unique ON contacts(company_id,mobile) WHERE mobile IS NOT NULL AND mobile <> '';
سپس در create_contact یک بررسی خطای UNIQUE مشابه الگوی موجود در update_product اضافه کن:
    let result = tx.execute("INSERT INTO contacts(...) VALUES(...)", params![...]);
    if let Err(e) = result {
        return Err(if e.to_string().contains("UNIQUE") {
            "CONTACT-008: این شماره موبایل قبلاً برای شخص دیگری ثبت شده است".to_string()
        } else {
            format!("CONTACT-004: {e}")
        });
    }
همین بررسی خطای UNIQUE را به update_contact هم اضافه کن (با کد خطای مجزا مثلاً CONTACT-009).

تست: یک Contact با موبایل "09120000000" بساز، سپس تلاش کن Contact دیگری با همان موبایل (company_id یکسان) بسازی - باید خطای CONTACT-008 بگیری، نه خطای خام SQLite. سپس با company_id متفاوت همان موبایل را دوباره امتحان کن - باید موفق شود (چون Unique Index شرطی per-company است). همچنین دو Contact با mobile=NULL بساز و مطمئن شو خطا نمی‌گیری (چون شرط IS NOT NULL AND <> '' این حالت را استثنا کرده).

مشکل ۳ (تمیزی کد - mojibake):
تمام رشته‌های خطای فارسی مربوط به این بخش را طبق همان الگوی بخش قبل (ماژول errors با ثابت‌ها) بازنویسی کن. کدهای خطا: CONTACT-001 تا CONTACT-009، PRODUCT-001 تا PRODUCT-008. ثابت‌ها را با پیشوند CONTACT_ و PRODUCT_ در همان ماژول errors (که در بخش ۱ ساخته شد) اضافه کن، مثلاً:
- CONTACT_001: نام شخص الزامی است
- CONTACT_002: نوع شخص نامعتبر است
- CONTACT_003: شرکت فعال یافت نشد
- CONTACT_004: خطا در ثبت شخص
- CONTACT_005: شخص یافت نشد
- CONTACT_006: خطا در ویرایش شخص
- CONTACT_007: خطا در حذف شخص
- CONTACT_008: این شماره موبایل قبلاً برای شخص دیگری ثبت شده است
- CONTACT_009: این شماره موبایل قبلاً برای شخص دیگری ثبت شده است
- PRODUCT_001: کد کالا، نام و واحد الزامی است
- PRODUCT_002: مقادیر قیمت و حداقل موجودی نمی‌توانند منفی باشند
- PRODUCT_003: شرکت فعال یافت نشد
- PRODUCT_004: خطا در ثبت/یافتن کالا
- PRODUCT_005: SKU یا بارکد تکراری است
- PRODUCT_006: خطا در ویرایش کالا
- PRODUCT_007: این کالا سابقه موجودی دارد و قابل حذف نیست
- PRODUCT_008: خطا در حذف کالا
متن دقیق را بر اساس context هر خطا خودت نهایی کن؛ سپس در تمام توابع این بخش رشته‌های خام فعلی را با این ثابت‌ها جایگزین کن.

مشکل ۴ (سرعت/طراحی - تولید ID با ریسک برخورد):
شناسه‌های contact و product با format!("contact-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()) ساخته می‌شوند. اگر دو Command از دو Thread/Invoke همزمان (که در Tauri ممکن است رخ دهد) در یک نانوثانیه اجرا شوند - بعید ولی از نظر تئوری ممکن - یا اگر ساعت سیستم عقب برود، امکان برخورد ID وجود دارد؛ ضمناً unwrap_or_default() در صورت خطای ساعت سیستم مقدار 0 برمی‌گرداند که باعث ID تکراری «contact-0» برای همه‌ی رکوردهای بعدی در آن نشست می‌شود.

اصلاح: یک تابع کمکی gen_id(prefix: &str) -> String در نزدیکی audit() در main.rs اضافه کن که از crate uuid (نسخه v4) استفاده کند به‌جای timestamp:
    fn gen_id(prefix: &str) -> String {
        format!("{prefix}-{}", uuid::Uuid::new_v4())
    }
اگر crate uuid در Cargo.toml موجود نیست، آن را با feature "v4" اضافه کن. سپس در create_contact و create_product (و در ادامه‌ی بخش‌های بعدی هر جا از همین الگوی timestamp_nanos برای تولید ID استفاده شده) فراخوانی را به gen_id("contact") و gen_id("product") تغییر بده. توجه: خطوطی که timestamp_nanos را برای بخش‌های خط (line id مثل "{id}-line-{i}") استفاده می‌کنند را در این بخش تغییر نده؛ فقط ID های سطح رکورد اصلی (contact, product) در همین بخش اصلاح شوند.

تست: یک تست واحد بنویس که gen_id("contact") را ۱۰۰۰ بار پشت‌سرهم در یک حلقه فراخوانی کند و با HashSet بررسی کند هیچ مقدار تکراری تولید نشده.

بعد از اتمام این بخش: کامیت با پیام "fix(contacts+products): atomic audit transactions, unique contact mobile constraint, collision-safe IDs, resolve mojibake"
```

---

## بخش ۳: Inventory / Warehouses

```
ادامه‌ی اصلاحات نرم‌افزار حسابداری «نوین پرداز» را روی ریپازیتوری گیت‌هاب اعمال کن. این بخش (Inventory/Warehouses) را کامل تمام کن، تست‌های مشخص‌شده را اضافه/اجرا کن و در پایان یک کامیت مجزا بزن. به بخش‌های قبلی (Auth/DB Connection، Contacts/Products) دست نزن مگر ابزارهای مشترکی که در آن بخش‌ها ساخته شد (ماژول errors، تابع gen_id) و اینجا فقط استفاده می‌شوند.

═══════════════════════════════════════════
بخش ۳: Inventory / Warehouses
فایل هدف: apps/desktop-host/src-tauri/src/main.rs
توابع درگیر: reserve_inventory (~۷۴۶)، release_inventory (~۸۰۴)، create_inventory_transfer_order (~۱۱۱۲)، receive_inventory_transfer (~۱۲۰۴)، inventory_move (~۱۲۵۹، تابع مشترک پشت receive_stock/issue_stock)، transfer_stock (~۱۴۷۴)، adjust_stock (~۱۵۵۰)
═══════════════════════════════════════════

مشکل ۱ (بحرانی - طراحی/عملکرد صحیح - دو مسیر موازی و ناسازگار برای انتقال بین انبار):
در کد دو سیستم کاملاً جدا برای جابه‌جایی کالا بین دو انبار وجود دارد که هم‌پوشانی کامل دارند:
(الف) transfer_stock (خط ۱۴۷۴): انتقال فوری و یک‌مرحله‌ای - موجودی مبدا کم و مقصد زیاد می‌شود در همان لحظه.
(ب) create_inventory_transfer_order + receive_inventory_transfer (خط ۱۱۱۲ و ۱۲۰۴): انتقال دومرحله‌ای با فیلد in_transit_quantity - ابتدا از مبدا کم و به in_transit اضافه می‌شود، بعد در مرحله‌ی دریافت به مقصد اضافه می‌شود.
این دو مسیر مستقل، دو تابع محاسبه‌ی بهای تمام‌شده (cost) متفاوت دارند و اگر کاربر/فرانت‌اند به‌اشتباه یکی را برای برخی انتقال‌ها و دیگری را برای بقیه استفاده کند، گزارش موجودی و بهای تمام‌شده ناهماهنگ می‌شود. ضمناً transfer_stock هیچ چک is_active=1 روی هیچ‌کدام از دو انبار انجام نمی‌دهد، درحالی‌که create_inventory_transfer_order هر دو طرف را با is_active=1 چک می‌کند - این ناهماهنگی است.

اصلاح: 
۱) در transfer_stock چک is_active=1 را برای هر دو انبار اضافه کن (مشابه create_inventory_transfer_order):
تغییر کوئری‌های company و dest_company از:
"SELECT company_id FROM warehouses WHERE id=?1"
به:
"SELECT company_id FROM warehouses WHERE id=?1 AND is_active=1"
با پیام خطای مناسب اگر انبار غیرفعال یا ناموجود بود.
۲) در docs/ARCHITECTURE.md یک بخش «Inventory Transfer: دو مسیر موجود» اضافه کن که توضیح دهد transfer_stock برای جابه‌جایی فوری داخل یک شرکت است و create_inventory_transfer_order/receive_inventory_transfer برای انتقال دومرحله‌ای با ردیابی "در حال حمل" است؛ اگر این دو مسیر واقعاً باید برای دو سناریوی متفاوت محصول وجود داشته باشند مستندشان کن، در غیر این صورت (اگر یکی زائد است) آن را به‌عنوان TODO برای Deprecation علامت بزن اما در همین Commit چیزی را حذف نکن (حذف یک مسیر فعال بدون تایید محصول می‌تواند Regression بسازد).

تست: برای transfer_stock بعد از اصلاح، یک انبار مقصد را is_active=0 کن و انتقال را امتحان کن - باید خطا بگیری (قبل از اصلاح موفق می‌شد).

مشکل ۲ (بحرانی - عملکرد صحیح - مقایسه‌ی نادرست اعداد اعشاری):
در adjust_stock (خط ۱۵۵۰) خط زیر وجود دارد:
    let delta = new_quantity - old;
    if delta.abs() < f64::EPSILON {
        return Err("INV-013: ...");
    }
f64::EPSILON مقدار ثابتی حدود 2.22e-16 است که فقط برای مقایسه‌ی اعداد نزدیک به 1.0 مناسب است، نه برای مقادیر موجودی با دامنه‌ی دلخواه (مثلاً موجودی ۱۰٬۰۰۰ واحدی). این تقریباً معادل مقایسه‌ی == 0.0 دقیق است و خطاهای انباشته‌ی اعشاری واقعی (مثلاً due to چند عملیات ضرب/تقسیم قبلی روی همان مقدار) را که ممکن است اختلاف ۰٫۰۰۰۰۰۱ ایجاد کنند، تشخیص نمی‌دهد و ممکن است یک تعدیل بی‌معنی (نویز اعشاری) به‌اشتباه به‌عنوان تغییر واقعی موجودی ثبت شود.

اصلاح: به‌جای f64::EPSILON یک آستانه‌ی نسبی و معنادار برای دامنه‌ی موجودی تعریف کن، مثلاً یک ثابت سراسری نزدیک بالای main.rs:
    const QUANTITY_EPSILON: f64 = 1e-6;
و در adjust_stock:
    if delta.abs() < QUANTITY_EPSILON {
        return Err(errors::INV_013.to_string());
    }
این مقدار (1e-6) را طوری انتخاب کن که کوچک‌تر از هر واحد اندازه‌گیری معنادار (کیلوگرم، متر، عدد) باشد ولی بزرگ‌تر از نویز اعشاری معمول محاسبات f64.

تست: سناریویی بساز که old=100.0 و new_quantity=100.0000001 باشد (اختلاف کوچک‌تر از ۱e-6) - باید خطای INV-013 بگیری. سپس old=100.0 و new_quantity=100.00001 (اختلاف بزرگ‌تر از ۱e-6) - باید موفق شود و یک inventory_movements با quantity=0.00001 ثبت شود.

مشکل ۳ (متوسط - تمیزی کد/طراحی - تولید ID با کد fragile برای جلوگیری از تصادم):
در transfer_stock دو شناسه به این شکل ساخته می‌شوند:
    let out_id = format!("movement-transfer-out-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default());
    let in_id = format!("movement-transfer-in-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() + 1);
اضافه‌کردن دستی +1 به in_id یک راه‌حل موقتی و شکننده برای جلوگیری از تصادم زمانی است که فقط با شانس کار می‌کند (اگر unwrap_or_default به‌خاطر خطای ساعت سیستم صفر برگرداند، هر دو شناسه قابل پیش‌بینی و در تئوری در معرض ریسک برخورد با سایر رکوردهای همان نشست خواهند بود، هرچند چون پیشوندشان متفاوت است تصادم فعلی بعید است، ولی الگو ناپایدار است).

اصلاح: تمام موارد format!("...-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()) که برای شناسه‌ی سطح رکورد اصلی (نه خطوط فرعی مثل "{id}-line-{i}") در این بخش استفاده شده‌اند - شامل reserve_inventory (id رزرو)، create_inventory_lot، create_inventory_count، inventory_move (id حرکت انبار)، transfer_stock (out_id و in_id)، create_inventory_transfer_order (id سفارش انتقال) - را با تابع gen_id که در بخش قبل (Contacts/Products) ساخته شد جایگزین کن:
    let out_id = gen_id("movement-transfer-out");
    let in_id = gen_id("movement-transfer-in");
و مشابه برای بقیه.

تست: تستی بنویس که transfer_stock را در یک حلقه‌ی سریع (بدون تاخیر مصنوعی) ۵۰ بار پشت‌سرهم فراخوانی کند و مطمئن شود همه‌ی رکوردهای inventory_movements تولیدشده ID یکتا دارند.

مشکل ۴ (تمیزی کد - mojibake):
تمام رشته‌های خطای این بخش را طبق الگوی ماژول errors بازنویسی کن. کدهای خطا: INV-001 تا INV-013، INV-110 تا INV-147. متن دقیق را بر اساس معنای هر کد (که در کد اطراف مشخص است، مثلاً INV-004 در inventory_move یعنی «موجودی کافی نیست» ولی همین کد INV-004 در adjust_stock یعنی «انبار یافت نشد» - این دو کاربرد متفاوت از یک کد خطای یکسان (INV-004) اشتباه است و باید در همین اصلاح یکی از آنها به کد جدید تغییر کند تا کدهای خطا در کل main.rs یکتا و غیرمبهم باشند؛ کد جدید پیشنهادی برای مورد adjust_stock: INV-014).

مشکل ۵ (طراحی - مستندسازی):
هیچ Command ای برای «لغو» یک Inventory Transfer Order در وضعیت in_transit وجود ندارد (فقط create و receive وجود دارد) - یعنی اگر یک انتقال دومرحله‌ای اشتباهاً ثبت شود، موجودی برای همیشه در حالت in_transit_quantity گیر می‌کند و هیچ مسیری برای بازگرداندن آن به مبدا نیست. این را در docs/IMPLEMENTATION_STATUS.md به‌عنوان یک Known Gap ثبت کن (پیاده‌سازی کامل یک Command جدید cancel_inventory_transfer_order را در این پرامپت انجام نده مگر جداگانه خواسته شود، چون نیاز به تصمیم محصول در مورد قوانین لغو دارد).

بعد از اتمام این بخش: کامیت با پیام "fix(inventory): consistent warehouse active checks, correct float epsilon, collision-safe IDs, resolve duplicate/mojibake error codes"
```

---

## بخش ۴: Invoices (فروش/خرید + ثبت سند حسابداری مرتبط)

```
ادامه‌ی اصلاحات نرم‌افزار حسابداری «نوین پرداز» را روی ریپازیتوری گیت‌هاب اعمال کن. این بخش (Invoices - فروش/خرید) را کامل تمام کن، تست‌های مشخص‌شده را اضافه/اجرا کن و در پایان یک کامیت مجزا بزن.

⚠️ توجه مهم به ترتیب اجرا: مشکل ۱ در همین بخش مستقیماً به اصلاحی که در «بخش ۱ (Auth/DB Connection)» انجام شد وابسته است (فعال‌سازی foreign_keys=ON روی هر اتصال). قبل از این‌که آن اصلاح merge شده باشد، این باگ اینجا خاموش بود؛ بعد از آن اصلاح، اولین فاکتور posted برای هر شرکت غیر از company-demo با خطای Foreign Key Constraint شکست می‌خورد مگر این‌که مشکل ۱ همین بخش هم اعمال شود. این دو اصلاح باید در یک توالی منطقی (این بخش بعد از بخش ۱) دیده شوند، نه مستقل.

═══════════════════════════════════════════
بخش ۴: Invoices (فروش/خرید + ثبت سند حسابداری مرتبط)
فایل هدف: apps/desktop-host/src-tauri/src/main.rs
توابع درگیر: create_invoice_common (~۲۳۷۰)، post_invoice (~۲۵۶۳، پشت post_sales_invoice/post_purchase_invoice)، invoice_total (~۲۳۵۹)، active_context (~۲۳۴۵)
═══════════════════════════════════════════

مشکل ۱ (بحرانی - عملکرد صحیح/امنیت داده - حساب‌های حسابداری hardcoded که برای شرکت‌های واقعی تضمین‌شده وجود ندارند):
در post_invoice (خط ۲۶۵۳-۲۶۵۵) کدهای حساب زیر مستقیم در کد hardcode شده‌اند:
    let cash_acc = "acc-1101".to_string();
    let party_acc = if sale { "acc-1201" } else { "acc-2101" }.to_string();
    let main_acc = if sale { "acc-4100" } else { "acc-5100" }.to_string();
این کدهای حساب فقط برای company-demo در db/mod.rs (خطوط ۴۱۷، ۴۹۶-۴۹۹) Seed شده‌اند. برای هر شرکت واقعی که کاربر در برنامه می‌سازد، این حساب‌ها ممکن است با این دقیق همین کد وجود نداشته باشند (شرکت می‌تواند چارت حساب دلخواه خودش را بسازد). چون journal_lines.account_id یک REFERENCES accounts(id) دارد و بعد از اصلاح Foreign Key در بخش ۱، این محدودیت واقعاً اعمال می‌شود، اولین تلاش برای Post کردن فاکتور در هر شرکت واقعی (غیر دمو) با خطای Foreign Key شکست می‌خورد یا (اگر Foreign Key به هر دلیلی موقتاً غیرفعال بماند) سند حسابداری با حساب نامعتبر/ناموجود ثبت می‌شود که ترازنامه را خراب می‌کند.

اصلاح: به‌جای hardcode کردن کد حساب، یک مکانیزم Account Mapping قابل‌تنظیم اضافه کن:
۱) یک جدول جدید در db/mod.rs به Migration اضافه کن:
    CREATE TABLE IF NOT EXISTS account_mappings(
      company_id TEXT NOT NULL,
      mapping_key TEXT NOT NULL,
      account_id TEXT NOT NULL REFERENCES accounts(id),
      PRIMARY KEY(company_id, mapping_key)
    );
با کلیدهای استاندارد: 'cash_default', 'ar_default' (حساب دریافتنی/مشتری پیش‌فرض)، 'ap_default' (حساب پرداختنی/تأمین‌کننده پیش‌فرض)، 'sales_revenue_default'، 'cogs_default'.
۲) برای company-demo مقادیر پیش‌فرض فعلی (acc-1101، acc-1201، acc-2101، acc-4100، acc-5100) را به همین ترتیب در account_mappings درج کن (INSERT OR IGNORE، مطابق سبک بقیه‌ی Migration ها در فایل).
۳) یک تابع کمکی در main.rs اضافه کن:
    fn get_account_mapping(tx: &rusqlite::Transaction<'_>, company: &str, key: &str) -> Result<String, String> {
        tx.query_row(
            "SELECT account_id FROM account_mappings WHERE company_id=?1 AND mapping_key=?2",
            params![company, key],
            |r| r.get(0),
        ).map_err(|_| format!("ACC-020: نگاشت حساب '{key}' برای این شرکت تنظیم نشده است؛ ابتدا از بخش تنظیمات حسابداری آن را مشخص کنید"))
    }
۴) در post_invoice خطوط hardcode را با فراخوانی این تابع جایگزین کن:
    let cash_acc = get_account_mapping(&tx, &row.0, "cash_default")?;
    let party_acc = get_account_mapping(&tx, &row.0, if sale {"ar_default"} else {"ap_default"})?;
    let main_acc = get_account_mapping(&tx, &row.0, if sale {"sales_revenue_default"} else {"cogs_default"})?;
۵) یک Command جدید برای مدیریت این Mapping اضافه کن (get/set)، مثلاً:
    #[tauri::command]
    fn get_account_mappings(state: State<AppState>) -> Result<Vec<(String,String)>, String> { ... }
    #[tauri::command]
    fn set_account_mapping(state: State<AppState>, mapping_key: String, account_id: String) -> Result<(), String> { ... }
که فقط با permission مناسب (مثلاً "accounting.settings.edit") قابل تغییر باشند و account_id را قبل از ذخیره با یک Query در جدول accounts (متعلق به همان company) اعتبارسنجی کنند.
۶) هر دو Command جدید را به generate_handler! در main.rs اضافه کن.
۷) در apps/desktop-ui یک صفحه یا بخش تنظیمات ساده برای نگاشت این ۵ کلید اضافه کن (فرم ساده با Dropdown از list_accounts موجود).

تست: یک شرکت جدید بدون هیچ account_mappings بساز، یک فاکتور فروش برایش ایجاد و پست کن - باید خطای ACC-020 بگیری (نه خطای خام Foreign Key). سپس account_mappings را برای آن شرکت با set_account_mapping تنظیم کن و دوباره فاکتور را پست کن - باید موفق شود و journal_lines با account_id های درست ثبت شوند. یک تست هم برای company-demo بنویس که مطمئن شود بعد از migration مقادیر پیش‌فرض درست seed شده‌اند و فاکتورهای دمو مثل قبل کار می‌کنند (Regression Test).

مشکل ۲ (بحرانی - عملکرد صحیح - تخصیص نادرست مالیات/تخفیف در سند حسابداری):
در post_invoice، کل total فاکتور (که شامل subtotal - discount + tax است) به‌صورت یک عدد واحد بین party_acc و main_acc تقسیم می‌شود:
    let lines = if sale {
        vec![(party_acc.clone(), row.4, 0), (main_acc, 0, row.4)]
    } else {
        vec![(main_acc, row.4, 0), (party_acc.clone(), 0, row.4)]
    };
یعنی مالیات (tax) و تخفیف (discount) هرگز به حساب‌های جداگانه (مثل «مالیات بر ارزش افزوده پرداختنی/دریافتنی» یا «تخفیفات فروش») واریز نمی‌شوند، بلکه در دل حساب فروش/خرید اصلی گم می‌شوند. این باعث می‌شود گزارش سود و زیان و اظهارنامه مالیاتی درست از دیتابیس قابل استخراج نباشد.

اصلاح: 
۱) دو کلید جدید به account_mappings اضافه کن: 'tax_payable_default' (برای فروش - مالیات دریافتنی از مشتری) و 'tax_receivable_default' (برای خرید - مالیات پرداختنی به تأمین‌کننده)، و 'sales_discount_default'/'purchase_discount_default'.
۲) در post_invoice، به‌جای دو خط ساده، سطرهای سند را بر اساس subtotal، discount و tax واقعی فاکتور (که در جدول ذخیره شده‌اند - row فعلی total را دارد ولی subtotal/discount/tax را هم باید از همان SELECT اضافه کنی) به‌صورت جداگانه بساز:
برای فروش مثلاً:
    خط ۱: بدهکار party_acc به مبلغ total
    خط ۲: بستانکار main_acc (فروش) به مبلغ subtotal-discount
    خط ۳: بستانکار tax_acc به مبلغ tax (اگر tax > 0)
    خط ۴: بدهکار discount_acc به مبلغ discount (اگر discount > 0، به‌عنوان کاهش درآمد)
مطمئن شو مجموع بدهکار و بستانکار همیشه برابر می‌ماند (چک debit==credit را قبل از insert نگه‌دار، مثل کد فعلی).
۳) اگر tax یا discount صفر بود، آن خط را اصلاً insert نکن (برای جلوگیری از خطوط صفر بی‌معنی در دفتر).

تست: یک فاکتور فروش با subtotal=1000000، discount=50000، tax=90000 (total=1040000) بساز و پست کن. بعد از پست، journal_lines مربوطه را بخوان و مطمئن شو: مجموع debit == مجموع credit == 1040000، یک خط credit به حساب فروش با مبلغ 950000، یک خط credit به حساب مالیات با مبلغ 90000، و یک خط debit به حساب تخفیف با مبلغ 50000 وجود دارد.

مشکل ۳ (بحرانی - عملکرد صحیح - ناهماهنگی در محاسبه‌ی موجودی قابل‌فروش هنگام Post):
در post_invoice (خط داخل حلقه‌ی items)، هنگام فروش فقط current (کل موجودی) با q مقایسه می‌شود:
    let current:f64=tx.query_row("SELECT COALESCE(quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",...);
    if sale && current < *q {
        return Err("DOC-013: ...");
    }
این برخلاف inventory_move و transfer_stock است که همیشه quantity-reserved_quantity (موجودی قابل‌فروش واقعی، نه کل موجودی فیزیکی) را چک می‌کنند. یعنی اگر مقداری از موجودی از طریق reserve_inventory برای یک سفارش دیگر رزرو شده باشد، Post کردن این فاکتور می‌تواند موجودی را به زیر سطح رزروشده ببرد و تعهد آن رزرو را نقض کند.

اصلاح: کوئری current را به شکل زیر تغییر بده تا reserved_quantity را هم کم کند:
    let current:f64=tx.query_row("SELECT COALESCE(quantity-reserved_quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",params![pid,wid],|r|r.get(0)).unwrap_or(0.0);
توجه: این باعث می‌شود newq (موجودی جدید بعد از فروش) دیگر مستقیماً current-q نباشد؛ باید quantity واقعی (نه current قابل‌فروش) را جدا بخوانی برای محاسبه‌ی newq تا موجودی رزروشده به‌اشتباه از quantity کم نشود. یعنی دو مقدار جدا نگه دار:
    let raw_qty:f64=tx.query_row("SELECT COALESCE(quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",params![pid,wid],|r|r.get(0)).unwrap_or(0.0);
    let reserved:f64=tx.query_row("SELECT COALESCE(reserved_quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",params![pid,wid],|r|r.get(0)).unwrap_or(0.0);
    let available = raw_qty - reserved;
    if sale && available < *q {
        return Err(errors::DOC_013.to_string());
    }
    let newq = if sale { raw_qty - *q } else { raw_qty + *q };

تست: یک محصول با quantity=100 در یک انبار بساز. reserve_inventory را برای ۹۰ واحد از آن فرا بخوان (reserved_quantity=90، available=10). یک فاکتور فروش برای ۵۰ واحد از همان محصول/انبار بساز و پست کن - باید خطای DOC-013 بگیری (چون available=10 < 50)، درحالی‌که قبل از این اصلاح چون current=100 >= 50 بود، فاکتور با موفقیت پست می‌شد و موجودی رزروشده را نقض می‌کرد.

مشکل ۴ (متوسط - طراحی/تمیزی کد - عدم آزادسازی/بستن رزرو مرتبط هنگام Post):
اگر یک فاکتور فروش برای موجودی رزروشده از طریق reserve_inventory ایجاد شده باشد (مثلاً reference_type='sales_invoice' با reference_id متناظر)، هنگام Post شدن فاکتور هیچ کدی وجود ندارد که آن رکورد inventory_reservations را status='released' کند و reserved_quantity متناظر را کم کند - یعنی حتی بعد از فروش واقعی، موجودی همچنان به‌عنوان "رزروشده" علامت‌گذاری باقی می‌ماند (double counting: هم از quantity کم شده هم در reserved_quantity هنوز حساب می‌شود).

اصلاح: در post_invoice بعد از بخش موجودی، یک Query اضافه کن که رزروهای مرتبط با این فاکتور را پیدا و آزاد کند:
    let mut rst = tx.prepare("SELECT id, quantity FROM inventory_reservations WHERE reference_type='invoice' AND reference_id=?1 AND status='reserved'").map_err(|e|e.to_string())?;
    let reservations: Vec<(String,f64)> = rst.query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e|e.to_string())?.filter_map(Result::ok).collect();
    drop(rst);
    for (rid, rqty) in reservations {
        tx.execute("UPDATE inventory_reservations SET status='released',released_at=CURRENT_TIMESTAMP WHERE id=?1", params![rid]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE inventory_balances SET reserved_quantity=MAX(0,reserved_quantity-?) WHERE product_id IN (SELECT product_id FROM inventory_reservations WHERE id=?1) AND warehouse_id=?2", params![rqty, rid, wid]).map_err(|e|e.to_string())?;
    }
(اگر Command هایی که reference_id فاکتور را روی reserve_inventory ست می‌کنند از قبل با مقدار id فاکتور مطابقت ندارند، این را با تیم محصول/فرانت‌اند هماهنگ کن که هنگام رزرو موجودی برای یک فاکتور در حال ساخت، reference_type دقیقاً 'invoice' و reference_id دقیقاً همان id فاکتور ست شود).

تست: یک رزرو با reference_type='invoice' و reference_id برابر با id یک فاکتور فروش draft بساز. فاکتور را پست کن. بعد از پست، بررسی کن status رزرو 'released' شده و reserved_quantity محصول در آن انبار کاهش یافته.

مشکل ۵ (تمیزی کد - mojibake):
تمام رشته‌های خطای این بخش را طبق الگوی ماژول errors بازنویسی کن. کدهای خطا: DOC-001 تا DOC-013، ACC-002 (در این بخش دوباره تکرار شده - مطمئن شو با نسخه‌ی قبلی در بخش قبلی کد (ACC-001 تا ACC-006 در create_journal_internal) در تناقض معنایی نیست، چون همان کد ACC-002 در دو تابع مختلف به‌کار رفته با پیام مشابه - این یکی قابل‌قبول است چون معنای یکسان دارد ("جمع بدهکار با بستانکار برابر نیست")، فقط رشته‌ی متن را یکسان و از ثابت مشترک errors::ACC_002 در هر دو نقطه استفاده کن به‌جای تکرار رشته).

بعد از اتمام این بخش: کامیت با پیام "fix(invoices): configurable account mappings, split tax/discount posting, honor reserved quantity on post, release reservations on posting"
```

---

## بخش ۵: Treasury / Checks

```
ادامه‌ی اصلاحات نرم‌افزار حسابداری «نوین پرداز» را روی ریپازیتوری گیت‌هاب اعمال کن. این بخش (Treasury/Checks) را کامل تمام کن، تست‌های مشخص‌شده را اضافه/اجرا کن و در پایان یک کامیت مجزا بزن.

⚠️ وابستگی به بخش قبل: این بخش هم مثل بخش Invoices از حساب‌های hardcoded استفاده می‌کند و باید از همان مکانیزم account_mappings که در بخش Invoices ساخته شد استفاده کند، نه یک راه‌حل جدید و جدا.

═══════════════════════════════════════════
بخش ۵: Treasury / Checks
فایل هدف: apps/desktop-host/src-tauri/src/main.rs
توابع درگیر: create_treasury_account (~۲۹۱۱)، update_treasury_account (~۲۹۶۶)، create_check (~۳۳۵۳)، update_check_status (~۳۴۳۲)
═══════════════════════════════════════════

مشکل ۱ (بحرانی - عملکرد صحیح/یکپارچگی حسابداری - چک برگشتی بدون هیچ ثبت مالی هنگام برگشت از حالت «نزد بانک/انتقالی»):
در update_check_status (خط ۳۴۳۲)، ماشین‌حالت مجاز (خط ۳۴۵۲) این انتقال‌ها را قبول می‌کند:
    ("deposited", "bounced") | ("transferred", "bounced")
یعنی چکی که وضعیت "deposited" (نزد بانک) یا "transferred" (انتقالی) دارد و هنوز "cleared" (وصول‌شده) نشده، می‌تواند مستقیم به "bounced" برود. اما در کد، فقط شاخه‌ی `else if new_status == "bounced" && old == "cleared"` (خط ~۳۵۲۱) منطق واقعی برگشت چک (بازگرداندن سند حسابداری وصول، ثبت treasury_transactions معکوس) را دارد. برای دو حالت دیگر (deposited→bounced و transferred→bounced)، کد به شاخه‌ی `else` نهایی (خط ~۳۵۵۶) می‌افتد که فقط این خط را اجرا می‌کند:
    tx.execute("UPDATE checks SET status=?1 WHERE id=?2", params![new_status, check_id]).map_err(|e| e.to_string())?;
یعنی هیچ سند حسابداری، هیچ treasury_transaction و هیچ اثر مالی برای برگشت‌خوردن یک چک که هنوز وصول نشده ثبت نمی‌شود - درحالی‌که در واقعیت حسابداری، برگشت‌خوردن یک چک (حتی قبل از وصول قطعی) باید اثر مالی داشته باشد: اگر چک دریافتی (received) بود، باید بدهی مشتری دوباره فعال شود (اگر قبلاً به‌عنوان تسویه در نظر گرفته شده بود) یا حداقل یک رکورد پیگیری مالی ثبت شود؛ اگر چک پرداختی (issued) بود، مشابه.

اصلاح: شاخه‌ی bounced را طوری بازنویسی کن که برای هر دو حالت (چک قبلاً cleared شده، یا چک هنوز cleared نشده) رفتار درست و مجزا داشته باشد:

```rust
} else if new_status == "bounced" {
    if old == "cleared" {
        // منطق فعلی: معکوس‌کردن سند وصول (بدون تغییر - همان کد موجود خط ۳۵۲۱ تا ۳۵۵۵)
        ...same as existing code...
    } else {
        // چک هنوز cleared نشده (deposited یا transferred بود) - باید یک سند اطلاع‌رسانی/پیگیری ثبت شود
        // تا برگشت چک در دفاتر و در وضعیت حساب طرف مقابل قابل ردیابی باشد
        let jid = format!("journal-check-bounce-pending-{}", check_id);
        let n = next_journal_number(&tx, &row.2, &row.6)?;
        let party_acc = get_account_mapping(&tx, &row.2, if row.1 == "received" {"ar_default"} else {"ap_default"})?;
        let bounce_tracking_acc = get_account_mapping(&tx, &row.2, "check_bounce_tracking_default")?;
        let (debit, credit) = if row.1 == "received" {
            (party_acc.as_str(), bounce_tracking_acc.as_str())
        } else {
            (bounce_tracking_acc.as_str(), party_acc.as_str())
        };
        tx.execute("INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,source_id,created_by) VALUES(?,?,?,?,?,?, 'posted','check_bounce_pending',?,?)",params![jid,row.2,row.6,n,row.7,"برگشت چک قبل از وصول",check_id,user]).map_err(|e|format!("CHK-019: {e}"))?;
        tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{jid}-debit"),jid,debit,row.3,0,"برگشت چک قبل از وصول"]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{jid}-credit"),jid,credit,0,row.3,"برگشت چک قبل از وصول"]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE checks SET status='bounced' WHERE id=?1", params![check_id]).map_err(|e|e.to_string())?;
    }
} else {
    tx.execute("UPDATE checks SET status=?1 WHERE id=?2", params![new_status, check_id]).map_err(|e|e.to_string())?;
}
```

یک کلید جدید 'check_bounce_tracking_default' به account_mappings (که در بخش Invoices ساخته شد) اضافه کن و برای company-demo هم یک مقدار پیش‌فرض seed کن (پیشنهاد: یک حساب انتظامی/کنترلی جدید مثل «چک‌های برگشتی در انتظار پیگیری»).

تست: یک چک دریافتی بساز، وضعیتش را به "deposited" ببر، سپس به "bounced" ببر - قبل از اصلاح: بررسی کن هیچ journal_entries ای برای این تغییر ثبت نشده (باگ). بعد از اصلاح: بررسی کن یک journal_entries با source_type='check_bounce_pending' و journal_lines متناظر با مبلغ درست چک ثبت شده است. همچنین تست کن مسیر قبلی (cleared→bounced) هنوز دقیقاً مثل قبل کار می‌کند (Regression Test برای منطق موجود در خط ۳۵۲۱).

مشکل ۲ (بحرانی - عملکرد صحیح - حساب‌های hardcoded مشابه بخش Invoices):
در update_check_status، شاخه‌ی "cleared" (خط ~۳۴۸۹-۳۴۹۰) از همان حساب‌های hardcode شده استفاده می‌کند:
    let offset_account = if row.1 == "received" { "acc-1201" } else { "acc-2101" };
این باید از همان مکانیزم account_mappings استفاده کند که در بخش Invoices ساخته شد (کلیدهای 'ar_default' و 'ap_default').

اصلاح:
    let offset_account = get_account_mapping(&tx, &row.2, if row.1 == "received" {"ar_default"} else {"ap_default"})?;
همین تغییر را در ادامه‌ی همان تابع (بخش bounced از حالت cleared، اگر مشابه hardcode دیگری در آن مسیر بود) هم اعمال کن؛ کد فعلی آن مسیر account_id ها را از خود journal_lines سند اصلی می‌خواند (نه hardcode)، پس نیازی به تغییر آن‌جا نیست - فقط مطمئن شو.

تست: یک چک دریافتی را cleared کن، journal_lines حاصل را بخوان و مطمئن شو account_id برای طرف مقابل دقیقاً برابر مقداری است که در account_mappings برای کلید ar_default تنظیم شده (نه رشته‌ی ثابت acc-1201).

مشکل ۳ (متوسط - امنیت/عملکرد صحیح - عدم بررسی فعال‌بودن حساب مرتبط با treasury_account هنگام Clear):
در شاخه‌ی "cleared"، treasury_account (که همان linked_account_id از treasury_accounts است) خوانده می‌شود ولی فعال‌بودن آن در جدول accounts چک نمی‌شود؛ فقط treasury_accounts.is_active چک شده. اگر حساب حسابداری مرتبط (accounts.is_active) غیرفعال شده باشد ولی خود treasury_account فعال مانده باشد، سند حسابداری با یک حساب غیرفعال ثبت می‌شود.

اصلاح: بعد از خواندن treasury_account، یک بررسی اضافه کن:
    let acc_active: i64 = tx.query_row("SELECT COUNT(*) FROM accounts WHERE id=?1 AND is_active=1", params![treasury_account], |r| r.get(0)).unwrap_or(0);
    if acc_active == 0 {
        return Err("CHK-020: حساب حسابداری مرتبط با این حساب خزانه غیرفعال است".to_string());
    }

تست: یک treasury_account با linked_account_id به یک حساب فعال بساز، آن حساب حسابداری را is_active=0 کن، سپس یک چک با آن treasury_account را cleared کن - باید خطای CHK-020 بگیری.

مشکل ۴ (تمیزی کد - کد خطای تکراری با معنای متفاوت):
کد خطای CHK-004 دوبار با معنای کاملاً متفاوت استفاده شده: یک‌بار در create_check برای «تاریخ سررسید قبل از تاریخ صدور» (خط ~۳۳۷۰) و یک‌بار برای «شخص معتبر نیست» (خط ~۳۳۸۳). این ابهام‌آفرین است و در لاگ/پشتیبانی قابل ردیابی نیست کدام خطا رخ داده.

اصلاح: کد دوم (شخص معتبر نیست) را به CHK-021 تغییر بده. سپس تمام کدهای خطای این بخش (CHK-001 تا CHK-021) را طبق الگوی ماژول errors بازنویسی کن، مثال:
- CHK_001: نوع چک نامعتبر است
- CHK_002: شماره چک الزامی است
- CHK_003: مبلغ چک باید بیشتر از صفر باشد
- CHK_004: تاریخ سررسید نمی‌تواند قبل از تاریخ صدور باشد
- CHK_021: شخص معتبر نیست
(بقیه‌ی کدها را بر اساس معنای موجود در کد نهایی کن)

تست: بعد از تغییر، برای create_check با party_id نامعتبر مطمئن شو خطای CHK-021 (نه CHK-004) برگردانده می‌شود.

مشکل ۵ (طراحی - عدم اعتبارسنجی فرمت شماره حساب/شبا):
create_treasury_account و update_treasury_account هیچ اعتبارسنجی روی فرمت account_number یا iban انجام نمی‌دهند (هر رشته‌ی دلخواه پذیرفته می‌شود). برای یک نرم‌افزار حسابداری واقعی که ممکن است IBAN را در گزارش‌های بانکی/انتقال استفاده کند، این ریسک ورود داده‌ی نامعتبر را بالا می‌برد.

اصلاح: یک تابع کمکی validate_iban ساده اضافه کن که حداقل فرمت پایه‌ی IBAN ایران (پیشوند IR و طول ۲۶ کاراکتر) را چک کند:
    fn validate_iban(iban: &str) -> Result<(), String> {
        let cleaned: String = iban.chars().filter(|c| !c.is_whitespace()).collect();
        if !cleaned.is_empty() && (!cleaned.starts_with("IR") || cleaned.len() != 26) {
            return Err("TRE-008: فرمت شماره شبا نامعتبر است (باید با IR شروع شود و ۲۶ کاراکتر باشد)".to_string());
        }
        Ok(())
    }
این را در create_treasury_account و update_treasury_account، قبل از INSERT/UPDATE، روی iban فراخوانی کن (فقط اگر iban برابر None یا رشته خالی نبود).

تست: یک treasury_account با iban="12345" بساز - باید خطای TRE-008 بگیری. با iban="IR820540102680020817909002" (۲۶ کاراکتر، با IR شروع می‌شود) بساز - باید موفق شود. با iban=None بساز - باید موفق شود (چون شرط !cleaned.is_empty() این حالت را رد می‌کند).

بعد از اتمام این بخش: کامیت با پیام "fix(treasury+checks): record financial impact of pre-clearing bounces, use configurable account mappings, validate IBAN format, resolve duplicate error codes"
```

---

## بخش ۶: Reports (تراز آزمایشی، دفتر معین، بدهکاران/بستانکاران، KPI داشبورد، صورت سود و زیان)

```
ادامه‌ی اصلاحات نرم‌افزار حسابداری «نوین پرداز» را روی ریپازیتوری گیت‌هاب اعمال کن. این بخش (Reports) را کامل تمام کن، تست‌های مشخص‌شده را اضافه/اجرا کن و در پایان یک کامیت مجزا بزن.

⚠️ وابستگی به بخش‌های قبل: مشکل ۱ در همین بخش باید از همان مکانیزم account_mappings استفاده کند که در بخش Invoices ساخته شد (نه راه‌حل مستقل).

⚠️ توجه: main.rs از قبل هم get_profit_loss و هم get_financial_statement (با statement='income_statement') را دارد. Command جدیدی به نام get_income_statement نساز؛ به‌جای آن هر دو تابع موجود را طبق مشکل‌های ۱ و ۴ زیر اصلاح کن.

═══════════════════════════════════════════
بخش ۶: Reports (تراز آزمایشی، دفتر معین، بدهکاران/بستانکاران، KPI داشبورد، صورت سود و زیان)
فایل هدف: apps/desktop-host/src-tauri/src/main.rs
توابع درگیر: get_trial_balance (~۴۰۹۰)، get_account_ledger (~۴۱۱۸)، get_party_balances (~۴۱۹۶ به بعد)، close_fiscal_year (~۴۲۵۹)، active_company (~۴۴۰۸)، get_dashboard_kpis (~۴۴۱۴)، get_party_balances_for_company (~۴۴۳۳)، get_profit_loss (~۴۷۱۱)، get_financial_statement (~۴۵۸۹)
═══════════════════════════════════════════

مشکل ۱ (بحرانی - عملکرد صحیح - محاسبه‌ی سود ناخالص با کد حساب hardcoded در get_dashboard_kpis):
در get_dashboard_kpis (خط ~۴۴۱۹) محاسبه‌ی gross_profit این‌طور است:
    ...WHERE jl.account_id IN (SELECT id FROM accounts WHERE code IN ('4100','5100'))
یعنی سود ناخالص فقط برای شرکتی درست محاسبه می‌شود که دقیقاً کدهای حساب '4100' (فروش) و '5100' (بهای تمام‌شده) را در چارت حساب خودش داشته باشد.

اصلاح: به‌جای فیلتر روی code، از account_mappings (که در بخش Invoices ساخته شد، کلیدهای 'sales_revenue_default' و 'cogs_default') استفاده کن:
    let (company, fy) = active_company(&state, &c)?;
    let sales_acc: Option<String> = c.query_row("SELECT account_id FROM account_mappings WHERE company_id=?1 AND mapping_key='sales_revenue_default'", params![company], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
    let cogs_acc: Option<String> = c.query_row("SELECT account_id FROM account_mappings WHERE company_id=?1 AND mapping_key='cogs_default'", params![company], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
    let gross_profit: i64 = match (&sales_acc, &cogs_acc) {
        (Some(s), Some(cg)) => c.query_row(
            "SELECT COALESCE(SUM(CASE WHEN jl.account_id=?3 THEN jl.credit-jl.debit ELSE 0 END),0) - COALESCE(SUM(CASE WHEN jl.account_id=?4 THEN jl.debit-jl.credit ELSE 0 END),0) FROM journal_entries je JOIN journal_lines jl ON jl.journal_id=je.id WHERE je.company_id=?1 AND je.fiscal_year_id=?2 AND je.status='posted' AND jl.account_id IN (?3,?4)",
            params![company, fy, s, cg],
            |r| r.get(0)
        ).unwrap_or(0),
        _ => 0, // نگاشت تنظیم نشده - سود ناخالص قابل‌محاسبه نیست
    };
اگر account_mappings تنظیم نشده بود (match به شاخه‌ی _ برود)، در DashboardKpi یک فیلد جدید gross_profit_available: bool اضافه کن که false باشد تا فرانت‌اند بتواند به‌جای عدد گمراه‌کننده‌ی صفر، پیام «نگاشت حساب تنظیم نشده» نشان دهد.

تست: برای company-demo (که account_mappings پیش‌فرض دارد) بررسی کن gross_profit مثل قبل محاسبه می‌شود (Regression). برای یک شرکت جدید بدون account_mappings، بررسی کن gross_profit_available=false برمی‌گردد و gross_profit صفر است (نه یک خطای panic یا مقدار نامعتبر).

مشکل ۲ (متوسط - سرعت/تمیزی کد - باز کردن دو اتصال جدا برای یک عملیات):
الگوی زیر در ۹ نقطه از main.rs تکرار شده (از جمله در همین بخش: get_account_ledger خط ۴۱۲۴، get_party_balances خط ۴۲۰۰):
    let user = require_permission(&state, &conn(&state)?, "...")?;
    let mut c = conn(&state)?;
این خط یک اتصال SQLite کاملاً جدا فقط برای چک‌کردن Permission باز می‌کند (که بلافاصله drop می‌شود چون هیچ متغیری آن را نگه نمی‌دارد)، سپس یک اتصال دوم و جدا برای عملیات اصلی باز می‌شود.

اصلاح: در تمام ۹ محل (لیست دقیق خطوط: ۱۷۸۵، ۳۰۲۳، ۳۰۷۷، ۳۱۵۰، ۳۲۱۶، ۳۲۴۷، ۳۳۳۰، ۴۱۲۴، ۴۲۰۰) الگو را به شکل یکسان‌شده تغییر بده:
قبل (مثال از get_account_ledger):
    let user = require_permission(&state, &conn(&state)?, "reporting.view")?;
    let mut c = conn(&state)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
بعد:
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "reporting.view")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
همین تغییر را برای سایر ۸ محل با همان امضای عمومی انجام بده (بعضی فقط c غیرقابل‌تغییر لازم دارند نه tx - در آن موارد فقط ترتیب دو خط اول را عوض کن، ساختار بقیه‌ی تابع را دست نزن).

تست: بعد از تغییر، cargo build را اجرا کن و مطمئن شو کامپایل بدون خطا انجام می‌شود. یک تست دستی برای یکی از این ۹ تابع (مثلاً get_account_ledger) اجرا کن و مطمئن شو خروجی قبل و بعد از تغییر یکسان است.

مشکل ۳ (طراحی - مستندسازی - عدم وجود سند اختتامیه در بستن سال مالی):
close_fiscal_year (خط ۴۲۵۹) فقط فیلد is_closed را روی fiscal_years به ۱ تغییر می‌دهد؛ هیچ سند اختتامیه (Closing Entry) که حساب‌های درآمد و هزینه را صفر کرده و به حساب سود/زیان انباشته منتقل کند، ایجاد نمی‌شود. همچنین هیچ Command برای بازگشایی (Reopen) یک سال مالی بسته‌شده وجود ندارد.

اصلاح: این را در docs/IMPLEMENTATION_STATUS.md به‌عنوان یک Known Gap با جزئیات ثبت کن (پیاده‌سازی کامل سند اختتامیه و Reopen را در این پرامپت انجام نده چون نیاز به تصمیم محصول در مورد نگاشت حساب سود/زیان انباشته و قوانین مجاز Reopen دارد).

مشکل ۴ (بحرانی - عملکرد صحیح - get_profit_loss با متدولوژی ناسازگار با بقیه‌ی گزارش‌ها):
get_profit_loss (خط ~۴۷۱۱) دو مشکل دارد:
۱) hardcode کردن a.code='5100' برای COGS - همان مشکل حساب‌های hardcoded.
۲) متغیر revenue مستقیماً از SUM(total) در sales_invoices خوانده می‌شود که total شامل tax است (total = subtotal - discount + tax طبق invoice_total در بخش Invoices) - یعنی مالیات دریافتی از مشتری به‌اشتباه به‌عنوان بخشی از «درآمد» شرکت حساب می‌شود که از نظر حسابداری غلط است (مالیات بر ارزش افزوده بدهی شرکت به دولت است، نه درآمد).

اصلاح:
۱) به‌جای a.code='5100'، از get_account_mapping (تابعی که در بخش Invoices ساخته شد) برای کلید 'cogs_default' استفاده کن (چون get_profit_loss فقط Connection عادی دارد نه Transaction، یک نسخه‌ی get_account_mapping که Connection عادی هم بپذیرد اضافه کن یا با &c.transaction موقت این تابع را صدا بزن).
۲) revenue را به‌جای SUM(total)، از SUM(subtotal-discount) محاسبه کن (که مالیات را ندارد):
    let revenue:i64=c.query_row("SELECT COALESCE(SUM(subtotal-discount),0) FROM sales_invoices WHERE company_id=?1 AND fiscal_year_id=?2 AND status='posted'",params![company,fy],|r|r.get(0)).unwrap_or(0);
همین اصلاح را برای sales_returns هم اعمال کن (بررسی کن جدول sales_returns چه ستون‌هایی دارد و مشابه رفتار کن).
۳) اگر account_mappings برای 'cogs_default' تنظیم نشده بود، به‌جای مقدار پیش‌فرض صفر بی‌صدا، یک فیلد cogs_available: bool در ProfitLoss اضافه کن (مشابه gross_profit_available در مشکل ۱).

تست: برای company-demo مطمئن شو بعد از اصلاح، net_income از get_profit_loss با net_income از get_financial_statement('income_statement') در یک بازه‌ی زمانی یکسان قابل‌مقایسه و بدون تناقض علامت است. یک تست جدا: یک فاکتور با subtotal=1000000, discount=0, tax=90000 بساز و پست کن، سپس get_profit_loss را فرا بخوان و مطمئن شو revenue=1000000 است نه 1090000.

مشکل ۵ (بحرانی - عملکرد صحیح - get_financial_statement با فیلتر پیشوند کد حساب hardcoded):
در get_financial_statement (خط ~۴۵۸۹)، فیلتر گزارش سود و زیان بر اساس پیشوند کد حساب کار می‌کند:
    let (filter, title) = if statement == "balance_sheet" {
        ("substr(a.code,1,1) IN ('1','2','3')", "ترازنامه")
    } else {
        ("substr(a.code,1,1) IN ('4','5','6')", "صورت سود و زیان")
    };
این هم یک فرض hardcoded است (فرض می‌کند هر شرکتی حتماً حساب‌های ترازنامه را با پیشوند ۱/۲/۳ و حساب‌های سود/زیان را با پیشوند ۴/۵/۶ کدگذاری کرده).

اصلاح: به‌جای فیلتر بر اساس پیشوند کد، از ستون a.nature استفاده کن:
    let (filter, title) = if statement == "balance_sheet" {
        ("a.nature IN ('asset','liability','equity')", "ترازنامه")
    } else {
        ("a.nature IN ('revenue','expense')", "صورت سود و زیان")
    };
قبل از این تغییر، مقدار دقیق ستون nature را در db/mod.rs بررسی کن تا مطمئن شوی این مقادیر دقیقاً با آنچه در دیتابیس واقعی ذخیره می‌شود مطابقت دارد؛ اگر مقادیر متفاوتی استفاده شده، فیلتر را با همان مقدار دقیق تطبیق بده.

تست: برای company-demo، قبل و بعد از تغییر get_financial_statement('income_statement') را فرا بخوان و مطمئن شو مجموعه‌ی خطوط قبل و بعد یکسان است (Regression Test). برای یک شرکت فرضی با یک حساب درآمد با کد '9001' و nature='revenue'، بررسی کن بعد از اصلاح این حساب در گزارش سود و زیان ظاهر می‌شود (قبل از اصلاح ظاهر نمی‌شد).

مشکل ۶ (تمیزی کد - mojibake):
تمام رشته‌های خطای این بخش را طبق الگوی ماژول errors بازنویسی کن. کدهای خطا: RPT-001، RPT-002، RPT-010، REPORT-001، FY-002، FY-003.

بعد از اتمام این بخش: کامیت با پیام "fix(reports): configurable gross-profit/COGS/income-statement accounts, unify db connection pattern, resolve mojibake"
```

---

## بخش ۷: Integrations / Plugins / API Profiles + Import

```
ادامه‌ی اصلاحات نرم‌افزار حسابداری «نوین پرداز» را روی ریپازیتوری گیت‌هاب اعمال کن. این بخش (Integrations/Plugins/API Profiles + Import) را کامل تمام کن، تست‌های مشخص‌شده را اضافه/اجرا کن و در پایان یک کامیت مجزا بزن.

═══════════════════════════════════════════
بخش ۷: Integrations / Plugins / API Profiles + Import
فایل هدف: apps/desktop-host/src-tauri/src/main.rs
توابع درگیر: execute_api_request (~۵۶۰۵)، register_plugin (~۵۱۷۷)، execute_plugin (~۵۳۱۹)، import_data (~۲۱۷۸)
═══════════════════════════════════════════

مشکل ۱ (بحرانی - امنیت - SSRF از طریق Redirect در execute_api_request):
در execute_api_request، بعد از عبور موفق از بررسی Allowlist روی host مقصد اولیه، کلاینت reqwest به این شکل ساخته می‌شود:
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(p.4 as u64))
        .build()
        .map_err(...)?;
هیچ تنظیمی برای غیرفعال‌کردن یا محدودکردن Redirect وجود ندارد. رفتار پیش‌فرض reqwest دنبال‌کردن تا ۱۰ Redirect است. یعنی حتی اگر host اولیه در Allowlist باشد، اگر آن سرور یک پاسخ ۳۰۱/۳۰۲ به یک host کاملاً متفاوت (خارج از Allowlist، از جمله آدرس‌های داخلی شبکه یا Cloud Metadata Endpoint مثل 169.254.169.254) برگرداند، reqwest بدون بررسی مجدد Allowlist آن را دنبال می‌کند - این یک مسیر کلاسیک SSRF-via-redirect است.

اصلاح: در ساخت Client، سیاست Redirect را صراحتاً غیرفعال کن و اگر پاسخ Redirect بود، آن را دستی و با بررسی مجدد Allowlist مدیریت کن (حداکثر با یک یا دو مرحله‌ی دنبال‌کردن دستی):
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(p.4 as u64))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("API-019: خطا در ساخت Client انجام نشد: {e}"))?;
سپس بعد از دریافت resp، اگر status بین 300 تا 399 بود، هدر Location را بخوان، آن را resolve کن، host جدید را دوباره با همان allowed_domains چک کن، و فقط در صورت عبور از Allowlist یک درخواست جدید بفرست (حداکثر ۲ بار، برای جلوگیری از حلقه‌ی بی‌نهایت). اگر host جدید در Allowlist نبود، خطای صریح برگردان:
    "API-027: مقصد Redirect خارج از Allowlist است و دنبال نشد"

تست: یک Mock Server محلی بساز که با ۳۰۲ به یک آدرس خارج از Allowlist Redirect کند. profile را با آن Mock Server در Allowlist بساز و execute_api_request را فرا بخوان - باید خطای API-027 بگیری، نه این‌که درخواست به مقصد Redirect بی‌صدا ارسال شود.

مشکل ۲ (بحرانی - امنیت - مقایسه‌ی Case-Sensitive روی Hostname در Allowlist):
خط بررسی Allowlist:
    if !p.2.split(',').map(str::trim).any(|d| d == host) {
مقایسه‌ی d == host به‌صورت دقیق حروف بزرگ/کوچک انجام می‌شود، درحالی‌که نام دامنه Case-Insensitive است. کاربری که به تنظیم base_url دسترسی دارد می‌تواند با نوشتن دامنه با حروف متفاوت این بررسی را دور بزند.

اصلاح:
    let host_lower = host.to_lowercase();
    if !p.2.split(',').map(|d| d.trim().to_lowercase()).any(|d| d == host_lower) {
        return Err(errors::API_017.to_string());
    }

تست: یک api_profile با allowed_domains="example.com" بساز، سپس base_url را با "https://EXAMPLE.com/api" بساز و execute_api_request را فرا بخوان - بعد از اصلاح باید موفق تشخیص داده شود.

مشکل ۳ (متوسط - امنیت - عدم اعتبارسنجی IP خصوصی/داخلی در allowed_domains):
سیستم Allowlist فقط بر اساس نام دامنه کار می‌کند و هیچ بررسی نمی‌کند که آیا دامنه‌ی تنظیم‌شده به یک آدرس IP خصوصی/داخلی (127.0.0.1، 169.254.169.254، 10.x.x.x، 192.168.x.x) Resolve می‌شود یا نه.

اصلاح: این را به‌عنوان یک محدودیت شناخته‌شده در docs/ARCHITECTURE.md مستند کن، با این توضیح که create_api_profile و execute_api_request باید فقط به کاربران قابل‌اعتماد داده شود، و اگر در آینده این قابلیت به کاربران کم‌اعتمادتر داده شد، باید یک DNS Resolver با فیلتر IP خصوصی (رد کردن RFC 1918 و Link-Local) اضافه شود. پیاده‌سازی کامل این فیلتر را در همین پرامپت انجام نده، فقط محدودیت را مستند کن.

مشکل ۴ (کم - طراحی/امنیت - محدودیت اجرای Plugin روی ویندوز):
در register_plugin، بعد از کپی فایل اجرایی Plugin، محدودیت مجوز اجرا فقط برای Unix تنظیم می‌شود (#[cfg(unix)] با set_mode(0o700)). برای ویندوز (پلتفرم اصلی این نرم‌افزار طبق BUILD_WINDOWS.md) هیچ محدودیت معادلی اعمال نمی‌شود.

اصلاح: این را در docs/ARCHITECTURE.md به‌عنوان یک محدودیت شناخته‌شده مستند کن (کنترل ACL دقیق در ویندوز نیاز به crate جداگانه دارد که پیاده‌سازی کامل آن را در همین پرامپت انجام نده). حداقل بررسی کن که plugin_root() در یک مسیر Per-User (نه مسیر مشترک سیستمی) قرار دارد و کامنت توضیحی اضافه کن.

مشکل ۵ (تمیزی کد - mojibake):
تمام رشته‌های خطای این بخش را طبق الگوی ماژول errors بازنویسی کن. کدهای خطا: API-001 تا API-027، PLUGIN-001 تا PLUGIN-026، IMPORT-001 تا IMPORT-011.

بعد از اتمام این بخش: کامیت با پیام "fix(integrations): prevent SSRF via redirects, case-insensitive domain allowlist, document private-IP and Windows-ACL gaps, resolve mojibake"
```

---

## بخش ۸: Frontend (apps/desktop-ui)

```
ادامه‌ی اصلاحات نرم‌افزار حسابداری «نوین پرداز» را روی ریپازیتوری گیت‌هاب اعمال کن. این بخش (Frontend - apps/desktop-ui) را کامل تمام کن، تست‌های مشخص‌شده را اضافه/اجرا کن و در پایان یک کامیت مجزا بزن.

═══════════════════════════════════════════
بخش ۸: Frontend (apps/desktop-ui/src)
فایل‌های هدف: App.tsx، store/appStore.ts، pages/Treasury.tsx (و الگوی مشابه در سایر صفحات)، api.ts
═══════════════════════════════════════════

مشکل ۱ (بحرانی - امنیت - ورود خودکار با اعتبار hardcoded در حالت DEMO_BUILD):
در App.tsx (خط ~۵۸)، اگر VITE_DEMO_MODE برابر 'true' باشد، برنامه بدون هیچ تعامل کاربر به‌صورت خودکار این خط را اجرا می‌کند:
    const loggedIn = await login('admin', 'demo')
یعنی نام کاربری admin و رمز عبور demo مستقیم در کد Frontend hardcode شده است.

اصلاح:
۱) در پیکربندی Build، یک هشدار صریح اضافه کن که VITE_DEMO_MODE هرگز نباید true باشد مگر برای Build های Demo عمومی که عمداً و آگاهانه با داده‌ی نمایشی توزیع می‌شوند.
۲) یک بررسی ایمنی اضافه کن که اگر VITE_DEMO_MODE=true بود ولی برنامه یک نشانه‌ی Production (مثلاً VITE_APP_ENV !== 'demo-public') داشت، ورود خودکار انجام نشود:
    useEffect(()=>{
        let alive=true
        const boot=async()=>{
            try{
                if(DEMO_BUILD){
                    if(import.meta.env.VITE_APP_ENV && import.meta.env.VITE_APP_ENV !== 'demo-public'){
                        console.warn('DEMO_BUILD فعال است ولی VITE_APP_ENV با حالت دمو عمومی مطابقت ندارد؛ ورود خودکار غیرفعال شد.')
                    } else {
                        const loggedIn=await login('admin','demo')
                        const status=await getDemoStatus()
                        if(alive){setUser(loggedIn);setAuthenticated(true);setDemo(status)}
                    }
                }
            }catch{if(alive)setBootError('راه‌اندازی برنامه انجام نشد')}
            finally{if(alive)setBooting(false)}
        }
        boot()
        return()=>{alive=false}
    },[DEMO_BUILD])
۳) در README.md یا docs/BUILD_WINDOWS.md یک بخش هشدار امنیتی اضافه کن که مشخص کند رمز admin/demo باید در هر استقرار واقعی فوراً تغییر یابد.

تست: با VITE_DEMO_MODE=true و بدون VITE_APP_ENV، بررسی کن رفتار فعلی (ورود خودکار) حفظ می‌شود (Regression). با VITE_DEMO_MODE=true و VITE_APP_ENV=production، بررسی کن ورود خودکار انجام نمی‌شود.

مشکل ۲ (متوسط - طراحی/امنیت - منوی سایدبار بدون توجه به Permission واقعی کاربر):
در App.tsx، آرایه‌ی menu (خط ۲۲-۳۳) به‌صورت ثابت و یکسان برای همه‌ی کاربران رندر می‌شود، بدون توجه به Permission واقعی کاربر.

اصلاح:
۱) در App.tsx یک useEffect اضافه کن که بعد از لاگین موفق، لیست Permission های کاربر را از list_permissions بگیرد:
    const [permissions, setPermissions] = useState<string[]>([])
    useEffect(()=>{
        if(!authenticated) return
        let alive = true
        listPermissions().then(p => { if(alive) setPermissions(p.map(x=>x.name)) }).catch(()=>{})
        return ()=>{alive=false}
    }, [authenticated])
(نیاز به اضافه‌کردن تابع جدید در api.ts: export const listPermissions=()=>api<Permission[]>('list_permissions') با type Permission={id:string,name:string}.)
۲) هر آیتم منو را با یک permission اختیاری مشخص کن (حداقل موارد بحرانی: treasury، checks، integrations، data-tools):
    {id:'treasury',label:'خزانه',icon:'wallet',children:[...],requiredPermission:'treasury.account.view'},
    {id:'checks',label:'چک‌ها',icon:'check',children:[...],requiredPermission:'treasury.check.view'},
    {id:'integrations',label:'اتصالات و افزونه‌ها',icon:'settings',requiredPermission:'integrations.execute'},
۳) در رندر منو، آیتم‌هایی که requiredPermission دارند و کاربر آن را ندارد فیلتر کن:
    const visibleMenu = menu.filter(item => !item.requiredPermission || permissions.includes(item.requiredPermission))

تست: یک کاربر تست بدون Permission "treasury.account.view" بساز، وارد شو و مطمئن شو آیتم «خزانه» در سایدبار نمایش داده نمی‌شود. برای کاربر admin با همه‌ی Permission ها، مطمئن شو منو کامل نمایش داده می‌شود (Regression).

مشکل ۳ (متوسط - تمیزی کد - داده‌ی ساختگی/بی‌ربط در store/appStore.ts):
فایل store/appStore.ts شامل یک Zustand Store با داده‌ی کاملاً ساختگی (seedInvoices، seedProducts) است که هیچ ارتباطی با بک‌اند واقعی ندارد. تنها استفاده‌ی واقعی این Store، فیلدهای dark و toggleTheme در components/Topbar.tsx است.

اصلاح: store/appStore.ts را به این شکل ساده‌سازی کن:
    import {create} from 'zustand';
    type State={dark:boolean;toggleTheme:()=>void};
    export const useAppStore=create<State>((set)=>({dark:false,toggleTheme:()=>set(s=>({dark:!s.dark}))}));
تایپ‌های Invoice و Product و داده‌های seedInvoices/seedProducts/addInvoice را کامل حذف کن.

توجه مهم قبل از حذف: با grep -rn "useAppStore" apps/desktop-ui/src بررسی کن که هیچ کامپوننت دیگری از فیلدهای invoices/products/addInvoice استفاده نمی‌کند؛ اگر جایی استفاده می‌شود، آن کامپوننت را به api.ts واقعی وصل کن به‌جای حذف صرف فیلد.

تست: بعد از تغییر، npm run build را اجرا کن و مطمئن شو خطای کامپایل TypeScript رخ نمی‌دهد. Topbar را در برنامه باز کن و مطمئن شو دکمه‌ی تغییر تم هنوز کار می‌کند (Regression).

مشکل ۴ (متوسط - عملکرد صحیح/UX - نمایش خطای خام و غیرقابل‌فهم به کاربر در تمام صفحات):
الگوی زیر در چندین صفحه (نمونه: pages/Treasury.tsx، pages/Checks.tsx) تکرار شده:
    catch(e){setError(String(e))}
String(e) روی خطای برگشتی از invoke تائوری ممکن است چیزی مثل "Error: CHK-020: ..." یا "[object Object]" تولید کند.

اصلاح: یک تابع کمکی مشترک در apps/desktop-ui/src/lib/errors.ts بساز:
    export function parseApiError(e: unknown): string {
        if (typeof e === 'string') return e
        if (e instanceof Error) return e.message
        if (e && typeof e === 'object' && 'message' in e) return String((e as any).message)
        return 'خطای غیرمنتظره‌ای رخ داد. لطفاً دوباره تلاش کنید.'
    }
سپس در تمام صفحاتی که از الگوی catch(e){setError(String(e))} استفاده می‌کنند (با grep -rn "setError(String(e))" apps/desktop-ui/src فهرست دقیق را استخراج کن)، الگو را به شکل زیر تغییر بده:
    import {parseApiError} from '../lib/errors'
    catch(e){setError(parseApiError(e))}

تست: یک خطای عمدی از بک‌اند بگیر (مثلاً treasury_account با نام خالی) و مطمئن شو متن کامل و خوانای فارسی خطا در UI نمایش داده می‌شود.

مشکل ۵ (کم - طراحی/UX - ورودی آزاد متنی برای شناسه‌ی حساب به‌جای انتخاب از لیست):
در Treasury.tsx، فیلد «حساب مقابل» یک <input> متنی آزاد است:
    <label>حساب مقابل<input name="offset" required placeholder="شناسه حساب"/></label>

اصلاح: این ورودی را به یک <select> تبدیل کن که از getAccounts (موجود در api.ts، wrapper واقعی list_accounts) پر می‌شود:
    const [accounts, setAccounts] = useState<Account[]>([])
    useEffect(()=>{ getAccounts().then(setAccounts).catch(()=>{}) }, [])
    <label>حساب مقابل<select name="offset" required>
        <option value="">انتخاب کنید...</option>
        {accounts.map(a => <option value={a.id} key={a.id}>{a.code} - {a.name}</option>)}
    </select></label>

تست: فرم ثبت تراکنش خزانه را باز کن و مطمئن شو Dropdown «حساب مقابل» لیست واقعی حساب‌های شرکت را نمایش می‌دهد. یک تراکنش با انتخاب یکی از حساب‌ها ثبت کن و مطمئن شو ثبت با موفقیت انجام می‌شود.

بعد از اتمام این بخش: کامیت با پیام "fix(frontend): guard demo auto-login, permission-aware sidebar, remove dead mock store, unify error display, account picker instead of free text"
```

---

## بخش ۹ (جدید): تکمیل ویژگی‌های ناقص/غایب — Manufacturing/BOM + مرکز تنظیمات واقعی

```
این بخش را فقط بعد از تکمیل کامل بخش‌های ۱ تا ۸ (که همگی قبلاً به این ایجنت داده شده‌اند) اجرا کن، چون از ابزارهای مشترک آن بخش‌ها (ماژول errors، تابع gen_id، مکانیزم account_mappings) استفاده می‌کند. این بخش دو شکاف واقعی بین منو/UI موجود و پیاده‌سازی واقعی را می‌بندد که در بررسی مقایسه‌ای بین رابط کاربری برنامه و کد واقعی کشف شد.

═══════════════════════════════════════════
بخش ۹-الف: تولید و فرمول تولید (Manufacturing / Bill of Materials)
═══════════════════════════════════════════

مشکل (بحرانی - عملکرد صحیح/طراحی - ویژگی در منو تبلیغ شده ولی هیچ پیاده‌سازی ندارد):
در apps/desktop-ui/src/App.tsx، زیرمنوی «انبار و کالا» شامل آیتم «تولید و فرمول» است:
    {id:'inventory',label:'انبار و کالا',icon:'package',children:['کالاها','موجودی انبار','انتقال بین انبارها','انبارگردانی','تولید و فرمول']}
اما این آیتم به هیچ صفحه یا فرم واقعی متصل نیست (کلیک روی آن صرفاً به همان صفحه‌ی عمومی AdvancedInventory.tsx می‌رود که هیچ UI برای تعریف فرمول تولید یا ثبت دستور تولید ندارد) و در main.rs (apps/desktop-host/src-tauri/src) هیچ Command ای برای فرمول ساخت (Bill of Materials)، دستور کار تولید (Production Order)، یا مصرف مواد اولیه/تحویل محصول نهایی وجود ندارد. تنها ارجاع به کلمه‌ی "manufacture" در main.rs فقط فیلد manufacture_date در inventory_lots (تاریخ تولید یک بچ/سری) است که کاملاً بی‌ربط به این ویژگی است.

اصلاح - Backend (main.rs و db/mod.rs):
۱) در db/mod.rs به Migration دو جدول جدید اضافه کن:
    CREATE TABLE IF NOT EXISTS bill_of_materials(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL,
      product_id TEXT NOT NULL REFERENCES products(id),
      name TEXT NOT NULL,
      output_quantity REAL NOT NULL DEFAULT 1,
      is_active INTEGER NOT NULL DEFAULT 1,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS bom_components(
      id TEXT PRIMARY KEY,
      bom_id TEXT NOT NULL REFERENCES bill_of_materials(id),
      component_product_id TEXT NOT NULL REFERENCES products(id),
      quantity REAL NOT NULL CHECK(quantity>0)
    );
    CREATE TABLE IF NOT EXISTS production_orders(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL,
      bom_id TEXT NOT NULL REFERENCES bill_of_materials(id),
      warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
      planned_quantity REAL NOT NULL CHECK(planned_quantity>0),
      status TEXT NOT NULL DEFAULT 'draft',
      order_date TEXT NOT NULL,
      completed_at TEXT,
      created_by TEXT NOT NULL,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
با ایندکس مناسب روی company_id در هر سه جدول (مطابق سبک بقیه‌ی جداول موجود در فایل).

۲) در main.rs این Command ها را اضافه کن (با همان الگوی امنیتی/تراکنشی که در بخش‌های قبلی این پروژه جا افتاد: require_login، require_permission با کلیدهای جدید "inventory.bom.manage" و "inventory.production.execute"، gen_id برای شناسه‌ها، تراکنش واحد برای هر عملیات، ماژول errors برای پیام‌ها با پیشوند BOM_ و PROD_):

    #[tauri::command]
    fn create_bom(state: State<AppState>, product_id: String, name: String, output_quantity: f64, components: Vec<(String, f64)>) -> Result<String, String>
    // اعتبارسنجی: product_id باید در products همان company موجود باشد؛ output_quantity>0؛ هر جزء (component) باید یک محصول معتبر دیگر (نه خودِ product_id، برای جلوگیری از حلقه‌ی خودارجاعی ساده) با quantity>0 باشد؛ همه در یک تراکنش با audit ثبت شود.

    #[tauri::command]
    fn list_boms(state: State<AppState>, product_id: Option<String>) -> Result<Vec<BomSummary>, String>

    #[tauri::command]
    fn get_bom_detail(state: State<AppState>, bom_id: String) -> Result<BomDetail, String>
    // شامل لیست کامل اجزا با نام/SKU هر Component

    #[tauri::command]
    fn create_production_order(state: State<AppState>, bom_id: String, warehouse_id: String, planned_quantity: f64, order_date: String) -> Result<String, String>
    // فقط وضعیت 'draft' می‌سازد؛ موجودی هنوز دست نمی‌خورد.

    #[tauri::command]
    fn complete_production_order(state: State<AppState>, order_id: String) -> Result<(), String>
    // این تابع باید داخل یک تراکنش واحد: (الف) برای هر Component در bom_components، quantity_needed = component.quantity * (planned_quantity/bom.output_quantity) را از inventory_balances همان warehouse کم کند (با همان الگوی reserve_inventory/inventory_move: چک موجودی کافی با در نظر گرفتن reserved_quantity، وگرنه خطای PROD-010)، هر کدام را در inventory_movements با reference_type='production_order' ثبت کند؛ (ب) planned_quantity از محصول نهایی (bom.product_id) را به inventory_balances همان warehouse اضافه کند و در inventory_movements ثبت کند؛ (ج) status سفارش را 'completed' و completed_at را ست کند؛ (د) audit بزند. اگر موجودی هرکدام از اجزا کافی نبود، کل تراکنش rollback شود و خطای دقیق (کدام Component کم است) برگردانده شود.

    #[tauri::command]
    fn cancel_production_order(state: State<AppState>, order_id: String) -> Result<(), String>
    // فقط برای status='draft' مجاز است (سفارش completed قابل لغو نیست چون قبلاً موجودی جابه‌جا شده)؛ status را 'cancelled' می‌کند.

    #[tauri::command]
    fn list_production_orders(state: State<AppState>, status: Option<String>) -> Result<Vec<ProductionOrderSummary>, String>

همه‌ی این Command ها را به generate_handler! در main.rs اضافه کن.

اصلاح - Frontend (apps/desktop-ui/src):
۱) یک فایل جدید apps/desktop-ui/src/pages/Manufacturing.tsx بساز که دو تب داشته باشد: «فرمول‌های تولید (BOM)» و «دستورهای تولید». تب اول: جدول BOM ها + فرمی برای ساخت BOM جدید (انتخاب محصول نهایی از products، انتخاب چند Component با مقدار از همان products، output_quantity). تب دوم: جدول Production Order ها با دکمه‌های «تکمیل» (فراخوانی complete_production_order) و «لغو» (فقط برای draft، فراخوانی cancel_production_order) + فرم ساخت سفارش جدید (انتخاب BOM، انبار، مقدار برنامه‌ریزی‌شده، تاریخ). از همان الگوی طراحی و کلاس‌های CSS موجود در DataPage.tsx/AdvancedInventory.tsx برای هماهنگی بصری استفاده کن.
۲) در api.ts، Wrapper های TypeScript برای هر Command جدید بالا اضافه کن (با تایپ‌های BomSummary، BomDetail، ProductionOrderSummary مطابق ساختار داده‌ی بازگشتی Rust).
۳) در App.tsx، آیتم منو «تولید و فرمول» را به یک صفحه‌ی واقعی متصل کن: زیرمنوی 'inventory' را طوری تغییر بده که کلیک روی «تولید و فرمول» مقدار page را به 'manufacturing' تنظیم کند (نه به 'inventory')، و در رندر اصلی یک شرط جدید اضافه کن: page==='manufacturing'?<Manufacturing/>: ... (در ادامه‌ی زنجیره‌ی شرط‌های موجود).

تست:
۱) یک BOM برای محصول نهایی X با دو Component (A مقدار ۲، B مقدار ۱) و output_quantity=1 بساز.
۲) موجودی اولیه A=100 و B=50 در یک انبار ثبت کن (با receive_stock موجود).
۳) یک Production Order برای planned_quantity=10 از همان BOM بساز (وضعیت باید draft بماند و موجودی نباید تغییر کند - این را با خواندن inventory_balances تایید کن).
۴) complete_production_order را فرا بخوان - بعد از آن باید: موجودی A دقیقاً ۸۰ (100-2*10)، موجودی B دقیقاً ۴۰ (50-1*10)، موجودی X دقیقاً ۱۰ (0+10) باشد؛ وضعیت سفارش 'completed' باشد.
۵) یک Production Order دیگر برای planned_quantity=1000 (بیش از موجودی موجود) بساز و complete_production_order را فرا بخوان - باید خطای PROD-010 بگیری و موجودی هیچ‌کدام از محصولات تغییر نکرده باشد (Rollback کامل).
۶) یک Production Order در وضعیت draft بساز و cancel_production_order را فرا بخوان - وضعیت باید 'cancelled' شود و موجودی هیچ محصولی نباید تغییر کرده باشد. سپس تلاش کن یک Production Order که قبلاً completed شده را cancel کنی - باید خطا بگیری.

بعد از اتمام این بخش: کامیت با پیام "feat(manufacturing): implement bill-of-materials and production orders end-to-end"

═══════════════════════════════════════════
بخش ۹-ب: تکمیل واقعی مرکز تنظیمات (SettingsCenter)
═══════════════════════════════════════════

مشکل (بحرانی - طراحی/UX - پوسته‌ی تنظیمات ساخته شده ولی اکثر گروه‌ها خالی و بدون عملکرد واقعی هستند):
در apps/desktop-ui/src/components/SettingsCenter.tsx، از ۱۲ گروه تنظیمات تعریف‌شده، فقط سه گروه (general، data، company) محتوای واقعی دارند. ۹ گروه دیگر (accounting، sales، inventory، treasury، printing، backup، integrations، appearance، security) این کد را دارند:
    {!['general','data','company'].includes(active)&&<div className="settings-grid">{['تنظیمات پایه','دسترسی کاربران','اعتبارسنجی','ثبت تغییرات','پیش‌فرض‌ها','کنترل عملیات'].map((x,i)=><div className="setting-card" key={i}><div><b>{x}</b><span>این بخش برای اتصال به تنظیمات واقعی ماژول آماده است.</span></div><span className="status pending">آماده توسعه</span></div>)}</div>}
یعنی کاربر وارد این تب‌ها می‌شود و کارت‌های خالی با متن «آماده توسعه» می‌بیند، بدون هیچ کنترل واقعی.

اصلاح - اولویت‌بندی‌شده (این ۹ گروه را طبق این ترتیب اولویت کامل کن؛ اگر فرصت محدود بود، حداقل سه گروه اول را کامل کن و بقیه را با یک TODO دقیق در docs/IMPLEMENTATION_STATUS.md مستند کن، نه با کارت خالی بی‌توضیح):

۱) گروه accounting (بالاترین اولویت - مستقیماً به بخش ۴/۵/۶ این پروژه وابسته است):
این گروه باید UI واقعی برای مدیریت account_mappings (که در بخش ۴ ساخته شد) باشد: یک جدول با ۱۰ ردیف ثابت (یکی به‌ازای هر mapping_key: cash_default، ar_default، ap_default، sales_revenue_default، cogs_default، tax_payable_default، tax_receivable_default، sales_discount_default، purchase_discount_default، check_bounce_tracking_default) که هرکدام یک برچسب فارسی خوانا و یک Dropdown برای انتخاب حساب از list_accounts دارد. مقدار فعلی هر mapping را از get_account_mappings بخوان و در Dropdown انتخاب‌شده نشان بده؛ با تغییر هر Dropdown بلافاصله set_account_mapping را صدا بزن. اگر مقداری تنظیم نشده بود، Dropdown را خالی و با یک نشانگر بصری (مثلاً حاشیه‌ی قرمز) نشان بده تا کاربر متوجه شود این نگاشت برای عملکرد صحیح فاکتور/چک/گزارش ضروری است.

۲) گروه treasury (خزانه و چک):
یک لینک/دکمه‌ی میانبر به صفحه‌ی Treasury.tsx و Checks.tsx موجود اضافه کن (این صفحات از قبل کامل و کاربردی هستند، فقط از داخل تنظیمات هم قابل دسترسی باشند) + یک تنظیم واقعی جدید: مدت‌زمان پیش‌فرض هشدار «نزدیک سررسید چک» (که در get_check_dashboard استفاده می‌شود - این تابع را در main.rs پیدا کن و ببین آیا این بازه‌ی زمانی در حال حاضر hardcode شده؛ اگر بله، آن را به یک تنظیم قابل‌ذخیره در یک جدول ساده company_settings(company_id, setting_key, setting_value) تبدیل کن، مشابه الگوی account_mappings).

۳) گروه security (امنیت و کاربران):
حداقل یک جدول فقط-خواندنی از کاربران فعلی (از یک Command جدید list_users که کاربران را با نقش نمایش دهد - این Command تا الان وجود نداشته، طبق یافته‌ی Known Gap بخش ۱ این پروژه) و لیست Permission های هرکدام (list_permissions موجود) نمایش بده. مدیریت کامل کاربر (ایجاد/ویرایش/تغییر رمز) را طبق تصمیم محصول به‌عنوان یک TODO مستند‌شده رها کن (این خودش یک ویژگی بزرگ جداست)، ولی حداقل این تب دیگر کاملاً خالی نباشد و اطلاعات واقعی نشان دهد.

۴) گروه‌های sales، inventory، printing، backup، integrations، appearance:
برای هرکدام، حداقل یک لینک میانبر واقعی و کاربردی به صفحه‌ی مرتبط موجود در برنامه اضافه کن (sales/inventory → میانبر به صفحات مرتبط؛ printing → میانبر به PrintTemplates.tsx؛ backup → توضیح route واقعی بکاپ‌گیری اگر در main.rs وجود دارد (با grep -n "backup" در main.rs بررسی کن)؛ integrations → میانبر به Integrations.tsx؛ appearance → همان کنترل تم dark/light که هم‌اکنون در گروه general هست را اینجا هم در دسترس بگذار یا از آنجا لینک بده). هدف این است که هیچ گروهی صرفاً کارت‌های بی‌معنی «آماده توسعه» نداشته باشد - یا محتوای واقعی دارد، یا حداقل یک مسیر واقعی به بخش مرتبط برنامه.

تست: برای گروه accounting، بعد از تنظیم یک mapping از طریق UI، یک فاکتور فروش بساز و پست کن و مطمئن شو از همان account_id که در UI انتخاب شد استفاده می‌شود (پل کامل بین UI تنظیمات و منطق واقعی Backend). برای گروه treasury، اگر تنظیم بازه‌ی هشدار پیاده شد، مقدار را تغییر بده و بررسی کن get_check_dashboard خروجی due_soon_count متفاوتی بر اساس بازه‌ی جدید برمی‌گرداند. برای بقیه‌ی گروه‌ها، به‌صورت دستی هر تب را باز کن و مطمئن شو هیچ‌کدام صرفاً متن «آماده توسعه» بدون هیچ محتوای مفید دیگر نشان نمی‌دهد.

بعد از اتمام این بخش: کامیت با پیام "feat(settings): connect accounting/treasury/security settings tabs to real backend, add shortcuts for remaining tabs"
```

---

## بخش ۱۰: بیلد نهایی، تست End-to-End و گزارش جامع

```
تمام بخش‌های اصلاحی که قبلاً روی این ریپازیتوری (نرم‌افزار حسابداری «نوین پرداز» - Tauri/Rust + React/TS) اجرا شد را جمع‌بندی و نهایی کن:

بخش‌های قبلی (باید همه قبل از این مرحله کامیت شده باشند؛ اگر هرکدام ناقص یا انجام‌نشده است، همین‌جا متوقف شو و آن را گزارش بده، وارد این مرحله نشو):
۱. Auth/Users + لایه اتصال دیتابیس
۲. Contacts + Products
۳. Inventory / Warehouses
۴. Invoices (فروش/خرید)
۵. Treasury / Checks
۶. Reports (شامل اصلاح get_dashboard_kpis، get_profit_loss، get_financial_statement)
۷. Integrations / Plugins / API Profiles + Import
۸. Frontend (apps/desktop-ui)
۹. Manufacturing/BOM + تکمیل مرکز تنظیمات

═══════════════════════════════════════════
مرحله ۱: بیلد و تست کامل پروژه
═══════════════════════════════════════════
۱) در ریشه‌ی apps/desktop-host/src-tauri اجرا کن: cargo build --release و cargo test. اگر خطای کامپایل یا شکست تست وجود دارد، آن‌ها را رفع کن قبل از رفتن به مرحله‌ی بعد - رفع خطا نباید هیچ‌کدام از اصلاح‌های ۹ بخش قبلی را نقض یا معکوس کند؛ اگر رفع یک خطای کامپایل نیازمند تغییر منطقی غیر از رفع خطای نحوی/تایپی ساده است، آن را به‌جای اصلاح خاموش، به‌عنوان یک یافته‌ی جدید در گزارش نهایی (مرحله ۳) ثبت کن.
۲) در ریشه‌ی apps/desktop-ui اجرا کن: npm run build (یا معادل vite build) و در صورت وجود اسکریپت تست (npm test)، آن را هم اجرا کن. خطاهای کامپایل TypeScript ناشی از تغییرات بخش ۸ و ۹ (مثلاً فیلدهای حذف‌شده از appStore، صفحه‌ی جدید Manufacturing.tsx) را رفع کن.
۳) اسکریپت‌های موجود در tools/ (hardening_audit.py، v12_audit.py، v14_inventory_audit.py، v15_reporting_audit.py، v17_audit.py) و scripts/commercial-hardening-tests.mjs را اجرا کن - این‌ها احتمالاً بررسی‌های خودکار از قبل موجود در پروژه هستند؛ اگر بعد از اصلاحات شکست می‌خورند، بررسی کن که آیا شکست به‌خاطر یک Regression واقعی از اصلاحات ماست یا چون این اسکریپت‌ها با فرض‌های قدیمی (مثلاً حساب‌های hardcoded acc-4100 که دیگر اجباری نیستند) نوشته شده‌اند؛ در حالت دوم خود اسکریپت تست را به‌روزرسانی کن تا با مکانیزم account_mappings جدید سازگار باشد، نه این‌که اصلاح درست را برای عبور از یک تست قدیمی معکوس کنی.

═══════════════════════════════════════════
مرحله ۲: تست رگرسیون سناریوی کامل end-to-end
═══════════════════════════════════════════
با استفاده از دیتابیس company-demo (یا یک دیتابیس تست تازه با migration کامل)، این جریان کامل حسابداری را به‌ترتیب اجرا و تایید کن که همه چیز از ابتدا تا انتها بدون خطای غیرمنتظره کار می‌کند:
۱. login با کاربر دمو
۲. ساخت یک Contact جدید (مشتری) و یک Product جدید
۳. تنظیم account_mappings برای یک شرکت تازه (اگر قبلاً تنظیم نشده) شامل تمام کلیدهای لازم: cash_default، ar_default، ap_default، sales_revenue_default، cogs_default، tax_payable_default، tax_receivable_default، sales_discount_default، purchase_discount_default، check_bounce_tracking_default — از طریق UI تنظیمات واقعی که در بخش ۹-ب ساخته شد
۴. دریافت موجودی اولیه از طریق receive_stock برای Product ساخته‌شده
۵. ساخت و Post کردن یک فاکتور فروش شامل تخفیف و مالیات برای همان Contact/Product - بررسی کن journal_lines حاصل دقیقاً به تفکیک فروش خالص/تخفیف/مالیات ثبت شده (طبق اصلاح بخش ۴)
۶. reserve_inventory برای بخشی از موجودی باقی‌مانده و بررسی این‌که یک فاکتور فروش دیگر که سعی می‌کند بیش از موجودی آزاد بفروشد با خطای DOC-013 رد می‌شود (طبق اصلاح بخش ۴)
۷. ساخت یک Check دریافتی برای تسویه‌ی همان فاکتور، انتقال وضعیت آن به deposited و سپس bounced (بدون رسیدن به cleared) - بررسی کن طبق اصلاح بخش ۵ یک journal_entries با source_type='check_bounce_pending' ثبت می‌شود
۸. اجرای get_trial_balance و get_financial_statement('income_statement') و بررسی این‌که مجموع بدهکار/بستانکار در تراز آزمایشی برابر است و نتایج دو گزارش تناقض علامت ندارند (طبق اصلاح بخش ۶)
۹. اجرای یک چرخه‌ی کامل تولید طبق بخش ۹-الف (ساخت BOM، ساخت Production Order، تکمیل آن) و بررسی صحت موجودی مواد اولیه/محصول نهایی بعد از تکمیل
۱۰. اجرای close_fiscal_year روی یک سال مالی تستی جدید (نه سال مالی دمو اصلی، تا داده‌ی دمو خراب نشود) و بررسی رفتار صحیح آن

هر انحراف از رفتار مورد انتظار در این جریان را به‌عنوان یک یافته‌ی باز (Open Issue) در گزارش نهایی مرحله ۳ ثبت کن؛ آن را در همین مرحله سرهم‌بندی اصلاح نکن مگر اصلاح بسیار کوچک و بدون‌ریسک باشد.

═══════════════════════════════════════════
مرحله ۳: تکمیل گزارش نهایی سخت‌گیری (Hardening Report)
═══════════════════════════════════════════
فایل docs/HARDENING_REPORT.md را بساز (اگر بخش ۶ آن را ساخته، تکمیل و نهایی کن) با ساختار زیر:

۱) جدول خلاصه‌ی کلی: ستون‌های «بخش»، «طراحی»، «عملکرد صحیح»، «امنیت»، «سرعت»، «عدم تداخل»، «تمیزی کد» - هر سلول یکی از مقادیر (اصلاح‌شده ✅ / نیاز به بررسی بیشتر ⚠️ / بدون مشکل ➖) را داشته باشد، برای هر ۹ بخش.

۲) برای هر بخش، یک زیربخش با:
   - فهرست مشکلات بحرانی که اصلاح شدند (با شماره‌ی کد خطا و شماره فایل/خط قبل از تغییر، برای قابلیت ردیابی در Git History)
   - فهرست شکاف‌های شناخته‌شده (Known Gaps) که عمداً در این دور اصلاح نشدند و نیاز به تصمیم محصول دارند (مثلاً: سند اختتامیه‌ی سال مالی، Command های مدیریت کاربر کامل، فیلتر IP خصوصی در Integrations، لغو Inventory Transfer Order در حال حمل، دو مسیر موازی transfer_stock/transfer_order، مدیریت کامل کاربران در تب امنیت تنظیمات)
   - ریسک باقی‌مانده‌ی مستند برای هر بخش

۳) یک بخش «وابستگی‌های بین بخشی» که مسیرهای وابستگی مهمی که در طول این پروژه کشف شد را فهرست کند، به‌خصوص:
   - فعال‌سازی foreign_keys در بخش ۱ که پیش‌نیاز کشف باگ حساب‌های hardcoded در بخش ۴ بود
   - مکانیزم account_mappings که در بخش ۴ ساخته شد و در بخش‌های ۵، ۶، ۷ و ۹-ب دوباره استفاده شد
   - تابع gen_id که در بخش ۲ ساخته شد و در بخش‌های ۳، ۹-الف و بعدی برای شناسه‌های سطح رکورد استفاده شد
   - ماژول errors که در بخش ۱ ساخته شد و در تمام بخش‌های بعدی برای رفع mojibake تکمیل شد

۴) یک بخش «یافته‌های باز از تست End-to-End مرحله ۲» با هر انحراف کشف‌شده در آن سناریو.

۵) در انتهای فایل، نسخه‌ی Git (commit hash نهایی بعد از تمام ۹ کامیت + کامیت این مرحله) و تاریخ تکمیل را ثبت کن.

بعد از تکمیل این گزارش، یک کامیت نهایی بزن با پیام: "chore(hardening): finalize build verification, end-to-end regression pass, and hardening report"
```
