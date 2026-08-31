# تحلیل شکاف: مقایسه‌ی ریپازیتوری AccSoftware با نرم‌افزار مرجع «نوین پرداز نسخه ۸.۵.۳»

## روش بررسی

۱۶ اسکرین‌شات ارسالی از نرم‌افزار حسابداری تجاری/قدیمی «نوین پرداز» (نسخه ۸.۵.۳، Windows Desktop) به‌عنوان **مشخصات/الزامات محصول** در نظر گرفته شدند. کد واقعی ریپازیتوری `itsrza/AccSoftware` (Tauri/Rust + React) مستقیماً از GitHub خوانده و با این مشخصات مقایسه شد. هر یافته‌ی زیر بر اساس بررسی مستقیم کد (نه حدس) است.

⚠️ یادآوری مهم: طبق بررسی قبلی، **هیچ‌کدام از ۹ بخش پرامپت‌های اصلاحی قبلی (فایل AccSoftware-Status-and-Prompts.md) هنوز روی ریپازیتوری اجرا نشده‌اند.** پرامپت زیر (بخش ۱۱) باید **بعد از تکمیل کامل بخش‌های ۱ تا ۱۰ آن فایل** اجرا شود، چون به ابزارهای مشترک ساخته‌شده در آن‌ها (ماژول errors، gen_id، account_mappings) وابسته است.

## خلاصه‌ی شکاف‌های کشف‌شده

| ویژگی در نرم‌افزار مرجع | وضعیت در ریپازیتوری فعلی |
|---|---|
| سطوح قیمت چندگانه کالا (جزئی/کلی/همکار ۱-۳/نمایشگاه/فصلی) | ❌ کاملاً غایب — فقط sale_price و purchase_price تکی |
| فرمول تولید (BOM) به‌عنوان تب داخل فرم کالا | ❌ کاملاً غایب (نه Backend نه Frontend) |
| مشخصات فنی سفارشی/قابل‌تنظیم کالا (۸ فیلد پویا) | ❌ غایب |
| چند واحدی برای کالا | ❌ غایب — فقط یک unit ثابت |
| اطلاعات مالیاتی/بارکد/ارزی/فروشگاه اینترنتی در فرم کالا | ❌ غایب |
| تنظیمات دوره‌ی مالی پیشرفته (بازه‌ی مجاز سررسید اقساط، حداکثر بازه‌ی تحویل کالا) | ❌ غایب — فقط close_fiscal_year ساده |
| **اتصال به سامانه مودیان (تکلیف قانونی صورتحساب الکترونیکی ایران)** | ❌ کاملاً غایب — نه Backend نه UI |
| کد اقتصادی/شناسه ملی/کد ثبت شرکت برای اشخاص | ❌ غایب در create_contact |
| فیلد بازاریاب (Salesperson/Marketer) روی فاکتور و شخص | ❌ کاملاً غایب |
| گروه‌بندی اشخاص (بدهکاران تجاری/بستانکاران/همکاران/سایت) + نمایش مانده‌ی زنده در لیست | ⚠️ فقط is_customer/is_supplier boolean ساده، بدون گروه‌بندی و بدون ستون مانده در لیست |
| سطح قیمت خرید بر اساس شرایط پرداخت (نقدی/۱ماهه/۳ماهه/آخرین خرید/میانگین) | ❌ غایب |
| تخفیف و مالیات/عوارض جدا در سطح خط فاکتور + بازاریاب | ⚠️ فقط سطح کل فاکتور (طبق بخش ۴ پرامپت‌های قبلی نیز مستند شده) |
| پرداخت با کارتخوان (POS) در فاکتور | ❌ غایب |
| دسته‌چک (Checkbook) مدیریت مستقل از چک‌های تکی | ❌ غایب |
| چک‌های وارده/صادره ابتدای دوره (Opening Balance Checks) | ❌ غایب |
| عملیات کامل چک: باطل کردن (Void) جدا از برگشت خوردن (Bounce) | ❌ غایب — state machine فعلی فقط issued→deposited→collected→bounced→cancelled دارد، بدون واگذاری/نقد کردن/عودت به‌عنوان اکشن‌های جدا |
| دفتر اقساط (Installment Ledger) و پرداخت وام (Loan Repayment) | ❌ کاملاً غایب |
| پخش مویرگی (Direct/Route Distribution) | ❌ کاملاً غایب — این یک ویژگی تخصصی FMCG است؛ باید با تیم محصول بررسی شود آیا واقعاً برای دامنه‌ی فعلی محصول لازم است |
| بازسازی اطلاعات (Data Repair/Rebuild) | ❌ غایب |
| لاگ عملکرد کاربران - نمایش UI (خود جدول audit_logs موجود است ولی هیچ Command خواندنی برایش نیست) | ⚠️ داده موجود، مصرف‌کننده (UI/Command) غایب |
| پشتیبان‌گیری/بازگردانی اطلاعات | ✅ **Backend کامل موجود است** (backup_database، restore_database، list_backups، verify_backup_file در main.rs) — فقط در Frontend به SettingsCenter وصل نشده (طبق بخش ۹-ب پرامپت‌های قبلی) |
| گزارش‌ساز پویا | ✅ موجود (ReportBuilder.tsx) — انطباق خوب |
| صورت‌های مالی | ✅ تا حدی موجود (get_financial_statement، get_profit_loss) با باگ‌هایی که در بخش ۶ پرامپت‌های قبلی مستند شد |
| حساب‌های بانکی/صندوق | ✅ انطباق مفهومی خوب با Treasury.tsx موجود |

---

## نحوه‌ی استفاده

این پرامپت (بخش ۱۱) را **بعد از تکمیل کامل بخش‌های ۱ تا ۱۰ فایل AccSoftware-Status-and-Prompts.md** به همان ایجنت بده. حجم این بخش زیاد است؛ اگر ایجنت محدودیت زمانی/توکنی دارد، می‌توانی زیربخش‌های ۱۱-الف تا ۱۱-ح را در پیام‌های جداگانه و پشت‌سرهم بدهی (هرکدام مستقل قابل‌اجراست چون از ابزارهای مشترک بخش‌های قبلی استفاده می‌کند، نه از هم).

---

## بخش ۱۱: بستن شکاف بین ریپازیتوری و نرم‌افزار مرجع «نوین پرداز ۸.۵.۳»

```
این بخش را فقط بعد از تکمیل کامل بخش‌های ۱ تا ۱۰ (که قبلاً به این ایجنت داده شده) اجرا کن. این بخش، شکاف‌های ویژگی بین ریپازیتوری فعلی و نرم‌افزار مرجع تجاری «نوین پرداز نسخه ۸.۵.۳» (که مشخصات محصول از آن استخراج شده) را می‌بندد. هر زیربخش را جدا تمام و کامیت کن.

═══════════════════════════════════════════
بخش ۱۱-الف: سطوح قیمت چندگانه‌ی کالا (Price Levels)
═══════════════════════════════════════════

مشکل (بحرانی - عملکرد صحیح/طراحی - نبود کامل سیستم چندسطحی قیمت‌گذاری):
در نرم‌افزار مرجع، هر کالا حداقل ۷ سطح قیمت مستقل دارد: جزئی (خرده‌فروشی)، کلی (عمده)، همکار درجه ۱/۲/۳ (سطوح نمایندگی)، نمایشگاه، فصلی. در ریپازیتوری فعلی، جدول products فقط دو ستون sale_price و purchase_price دارد - یعنی امکان فروش با قیمت متفاوت به گروه‌های مختلف مشتری وجود ندارد.

اصلاح - Backend:
۱) در db/mod.rs جدول جدید اضافه کن:
    CREATE TABLE IF NOT EXISTS product_price_levels(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL,
      code TEXT NOT NULL,
      name TEXT NOT NULL,
      is_active INTEGER NOT NULL DEFAULT 1,
      UNIQUE(company_id, code)
    );
    CREATE TABLE IF NOT EXISTS product_prices(
      id TEXT PRIMARY KEY,
      product_id TEXT NOT NULL REFERENCES products(id),
      price_level_id TEXT NOT NULL REFERENCES product_price_levels(id),
      price INTEGER NOT NULL DEFAULT 0,
      UNIQUE(product_id, price_level_id)
    );
برای company-demo، ۷ سطح پیش‌فرض (retail، wholesale، dealer1، dealer2، dealer3، showroom، seasonal) با نام‌های فارسی (جزئی، کلی، همکار درجه۱، همکار درجه۲، همکار درجه۳، نمایشگاه، فصلی) seed کن.
۲) Command های جدید: create_price_level، list_price_levels، set_product_price(product_id, price_level_id, price)، get_product_prices(product_id).
۳) در create_contact یک ستون default_price_level_id (nullable, REFERENCES product_price_levels) اضافه کن تا هر مشتری سطح قیمت پیش‌فرض خودش را داشته باشد.
۴) در create_invoice_common، هنگام افزودن هر قلم به فاکتور فروش، اگر contact یک default_price_level_id داشت، قیمت پیش‌فرض خط را از product_prices همان سطح بخوان (نه از products.sale_price ثابت)؛ اگر برای آن سطح قیمتی ثبت نشده بود، به products.sale_price برگرد (Fallback).

اصلاح - Frontend:
۱) در DataPage.tsx (برای kind='products')، یک تب/بخش «قیمت‌ها» به فرم افزودن/ویرایش کالا اضافه کن که جدولی از تمام price_level ها را با یک input عددی برای هرکدام نشان دهد.
۲) در فرم Contact، یک Dropdown برای انتخاب default_price_level_id اضافه کن.

تست: یک کالا با قیمت جزئی=100000 و قیمت همکار۱=80000 بساز. یک Contact با default_price_level_id=همکار۱ بساز. یک فاکتور فروش برای آن Contact و آن کالا بساز - قیمت پیش‌فرض خط باید 80000 باشد. یک Contact دیگر بدون default_price_level_id بساز - قیمت پیش‌فرض باید 100000 (fallback) باشد.

بعد از اتمام: کامیت با پیام "feat(pricing): implement multi-tier product price levels"

═══════════════════════════════════════════
بخش ۱۱-ب: اتصال به سامانه مودیان (الزام قانونی صورتحساب الکترونیکی ایران)
═══════════════════════════════════════════

مشکل (بحرانی - قانونی/طراحی - نبود کامل اتصال به سامانه مودیان مالیاتی):
نرم‌افزار مرجع یک بخش تنظیمات کامل برای اتصال به «سامانه مودیان» (tp.tax.gov.ir) دارد: امضای دیجیتال صورتحساب با کلید نرم‌افزاری یا سخت‌افزاری، ارسال خودکار فاکتور به سرور مالیاتی، دریافت شناسه یکتای مالیاتی. این یک الزام قانونی برای کسب‌وکارهای ایرانی است و در ریپازیتوری فعلی هیچ ردی از آن نیست.

⚠️ توجه: پیاده‌سازی کامل امضای دیجیتال و ارتباط واقعی با tp.tax.gov.ir یک کار پیچیده و حساس (شامل مدیریت کلید خصوصی/عمومی) است که نباید سرسری انجام شود. در این پرامپت فقط **زیرساخت و رابط کاربری** را بساز؛ برای منطق واقعی امضا و ارسال، به‌عنوان یک Known Gap با جزئیات کامل در docs/IMPLEMENTATION_STATUS.md مستند کن که نیاز به بررسی جداگانه با متخصص امنیت/حقوقی دارد.

اصلاح - Backend:
۱) در db/mod.rs جدول تنظیمات مودیان اضافه کن:
    CREATE TABLE IF NOT EXISTS tax_authority_settings(
      company_id TEXT PRIMARY KEY,
      enabled INTEGER NOT NULL DEFAULT 0,
      server_url TEXT,
      memory_bank_id TEXT,
      key_type TEXT CHECK(key_type IN ('software','hardware')),
      invoice_prefix_type TEXT,
      sms_template TEXT
    );
۲) Command های زیر را با پیاده‌سازی حداقلی (Placeholder امن، نه ارتباط واقعی) اضافه کن:
    get_tax_authority_settings، set_tax_authority_settings (ذخیره‌ی url، نوع کلید، فعال/غیرفعال).
    submit_invoice_to_tax_authority(invoice_id) - این تابع فعلاً فقط باید بررسی کند enabled=1 هست یا نه؛ اگر بله، خطای صریح "TAX-001: این قابلیت هنوز به‌طور کامل پیاده‌سازی نشده - نیازمند بررسی امنیتی/حقوقی جداگانه است" برگرداند (تا کاربر گمراه نشود که این ویژگی فعال است درحالی‌که نیست).
۳) هیچ کلید خصوصی/عمومی واقعی را در این پرامپت پیاده‌سازی، تولید یا ذخیره نکن.

اصلاح - Frontend:
یک تب جدید «سامانه مودیان» در SettingsCenter.tsx (زیر گروه accounting یا یک گروه جدید tax_authority) اضافه کن که فرم تنظیمات (URL سرور، نوع کلید، فعال/غیرفعال) را نشان دهد و یک پیام هشدار واضح نمایش دهد: «این بخش در حال توسعه است؛ ارسال خودکار صورتحساب به سامانه مودیان هنوز فعال نیست.»

تست: تنظیمات را با enabled=true ذخیره کن، سپس submit_invoice_to_tax_authority را روی یک فاکتور فرا بخوان - باید خطای TAX-001 بگیری (نه یک تلاش واقعی برای اتصال به سرور مالیاتی که با شکست امنیتی مواجه شود).

بعد از اتمام: کامیت با پیام "feat(tax-authority): add settings scaffold for Modian tax integration (implementation deferred - see IMPLEMENTATION_STATUS.md)"

═══════════════════════════════════════════
بخش ۱۱-ج: اطلاعات تکمیلی اشخاص (کد اقتصادی، بازاریاب، گروه‌بندی)
═══════════════════════════════════════════

مشکل (متوسط - طراحی/عملکرد صحیح - فیلدهای ضروری تجاری در فرم شخص غایب‌اند):
فرم create_contact/update_contact فعلی فقط name، kind، mobile، is_customer، is_supplier دارد. نرم‌افزار مرجع این فیلدهای اضافی را دارد: کد ملی/کد اقتصادی/شماره ثبت شرکت (برای صورتحساب رسمی B2B ضروری است)، بازاریاب مرتبط (Salesperson)، و گروه‌بندی اشخاص (بدهکاران تجاری، بستانکاران تجاری، همکاران، سایت - به‌جای فقط دو boolean ساده).

اصلاح - Backend:
۱) در db/mod.rs به جدول contacts این ستون‌ها را اضافه کن (با ALTER TABLE در Migration، چک column_exists قبل از افزودن، مطابق سبک موجود فایل):
    economic_code TEXT
    national_id TEXT
    company_registration_number TEXT
    salesperson_id TEXT REFERENCES contacts(id)  -- بازاریاب هم یک نوع Contact با نقش خاص است، یا اگر ترجیح می‌دهی جدول جداگانه‌ی salespersons بساز
    contact_group TEXT DEFAULT 'other' -- 'trade_debtor'|'trade_creditor'|'partner'|'site'|'other'
۲) create_contact و update_contact را طوری به‌روزرسانی کن که این فیلدهای اختیاری را هم بپذیرند و ذخیره کنند (بدون شکستن امضای فعلی برای فراخوانی‌های موجود - از Option<String> با مقدار پیش‌فرض None استفاده کن).
۳) یک Command list_contact_groups (یا یک Enum ثابت در همان تابع) اضافه کن تا Frontend بتواند مقادیر مجاز contact_group را بگیرد.

اصلاح - Frontend:
۱) در DataPage.tsx (kind='contacts')، فیلدهای جدید (کد اقتصادی، کد ملی/شناسه ثبت، بازاریاب - با Dropdown از سایر Contact ها، گروه) را به فرم اضافه کن.
۲) در جدول لیست اشخاص، یک ستون «گروه» و یک ستون «مانده‌ی حساب» (از یک Command جدید get_contact_balance(contact_id) که بر اساس journal_lines مرتبط با آن Contact محاسبه می‌شود - مشابه منطق get_party_balances موجود، فقط برای یک Contact واحد) اضافه کن.

تست: یک Contact با economic_code، contact_group='trade_debtor' بساز و از طریق get_contacts بخوان - مقادیر باید درست برگردند. یک فاکتور فروش نسیه برای آن Contact پست کن و get_contact_balance را فرا بخوان - باید مانده‌ی بدهکار درست را نشان دهد.

بعد از اتمام: کامیت با پیام "feat(contacts): add economic code, salesperson link, grouping, and live balance"

═══════════════════════════════════════════
بخش ۱۱-د: بازاریاب/تخفیف و مالیات سطح خط + پرداخت با کارتخوان روی فاکتور
═══════════════════════════════════════════

مشکل (متوسط - عملکرد صحیح - محدودیت‌های فاکتور فروش نسبت به نرم‌افزار مرجع):
فاکتور فروش مرجع، تخفیف و مالیات/عوارض را در سطح هر خط کالا (نه فقط کل فاکتور) پشتیبانی می‌کند، فیلد بازاریاب دارد، و امکان ثبت پرداخت با کارتخوان (POS) و تأیید تراکنش کارتخوان را دارد.

⚠️ وابستگی: تخفیف/مالیات سطح کل فاکتور از قبل در بخش ۴ پرامپت‌های قبلی (create_invoice_common/post_invoice) اصلاح شده بود. این بخش آن را به سطح خط توسعه می‌دهد - اصلاح‌های بخش ۴ را دوباره پیاده نکن، روی آن‌ها بساز.

اصلاح - Backend:
۱) جدول invoice_lines (یا هر نام معادل موجود برای اقلام فاکتور - نام دقیق را در main.rs/db/mod.rs پیدا کن) را بررسی کن؛ اگر ستون‌های line_discount و line_tax ندارد، با Migration اضافه کن.
۲) در create_invoice_common، هنگام محاسبه‌ی جمع فاکتور، تخفیف و مالیات سطح خط را هم لحاظ کن (جمع کل = SUM((quantity*unit_price - line_discount) * (1+line_tax_rate)) + مالیات/تخفیف سطح فاکتور که در بخش ۴ اضافه شد).
۳) یک ستون salesperson_id (REFERENCES contacts(id)) به جدول فاکتور اضافه کن.
۴) برای POS: یک جدول ساده pos_transactions(id, invoice_id, terminal_reference, amount, confirmed_at) اضافه کن و یک Command confirm_pos_payment(invoice_id, terminal_reference, amount) که فقط رکورد را ثبت می‌کند (این پرامپت به یکپارچه‌سازی واقعی با سخت‌افزار کارتخوان نمی‌پردازد - آن یک Known Gap جداست که باید مستند شود، مشابه سامانه مودیان).

اصلاح - Frontend:
در Invoices.tsx، برای هر خط کالا دو فیلد ورودی تخفیف و مالیات اضافه کن؛ یک Dropdown انتخاب بازاریاب در سطح کل فاکتور اضافه کن؛ یک دکمه‌ی «تأیید پرداخت کارتخوان» که confirm_pos_payment را صدا می‌زند.

تست: یک فاکتور با یک خط (quantity=2, unit_price=100000, line_discount=10000, line_tax_rate=0.09) بساز و پست کن - جمع آن خط باید (2*100000-10000)*1.09 = 207100 باشد. confirm_pos_payment را برای همان فاکتور با یک terminal_reference فرا بخوان و بررسی کن رکورد pos_transactions ثبت شده.

بعد از اتمام: کامیت با پیام "feat(invoices): line-level discount/tax, salesperson field, POS payment confirmation scaffold"

═══════════════════════════════════════════
بخش ۱۱-ه: تکمیل چرخه‌ی چک (دسته‌چک، مانده‌ی ابتدای دوره، باطل‌کردن)
═══════════════════════════════════════════

مشکل (متوسط - عملکرد صحیح/طراحی - چرخه‌ی چک ناقص نسبت به نرم‌افزار مرجع):
نرم‌افزار مرجع سه قابلیت دارد که در ریپازیتوری غایب است: (۱) مدیریت دسته‌چک (Checkbook) به‌عنوان یک موجودیت مستقل که چک‌های صادره از آن شماره‌گذاری می‌شوند، (۲) ثبت چک‌های ابتدای دوره (Opening Balance Checks - چک‌هایی که از دوره‌ی مالی قبل به دوره‌ی جاری منتقل شده‌اند)، (۳) اکشن «باطل کردن» (Void) به‌عنوان یک وضعیت مجزا از «برگشت خوردن» (Bounced) - در حال حاضر create_check state machine فقط "cancelled" را از وضعیت "registered" مجاز می‌داند (خط ۳۴۶۳)، نه از سایر وضعیت‌ها، که یعنی چک واگذارشده/نزد بانک را نمی‌توان باطل کرد، فقط می‌توان برگرداند.

اصلاح - Backend:
۱) جدول checkbooks(id, company_id, treasury_account_id REFERENCES treasury_accounts(id), series_prefix, start_number, end_number, is_active) اضافه کن. Command های create_checkbook، list_checkbooks.
۲) در create_check یک checkbook_id اختیاری اضافه کن (اگر چک صادره/issued بود)؛ اگر checkbook_id داده شد، شماره چک را از بازه‌ی start_number تا end_number همان دسته‌چک اعتبارسنجی کن.
۳) یک ستون is_opening_balance (INTEGER DEFAULT 0) به جدول checks اضافه کن؛ یک Command جدید create_opening_balance_check که چک را مستقیم با status='deposited' یا 'transferred' (بدون سند حسابداری جدید، چون قرار است مانده‌ی موجود دوره‌ی قبل باشد) ثبت کند - فقط با Permission "treasury.check.opening_balance".
۴) ماشین‌حالت update_check_status را گسترش بده تا "cancelled" از وضعیت‌های "deposited" و "transferred" هم مجاز باشد (نه فقط از "registered")، با این تفاوت مهم نسبت به "bounced": باطل‌کردن یعنی چک اصلاً معتبر نبوده (مثلاً پاره شده) و باید هیچ اثر مالی نداشته باشد اگر چک هنوز cleared نشده (این با bounced که در بخش ۵ پرامپت‌های قبلی یک سند پیگیری می‌سازد فرق دارد - cancelled/void نباید سند بسازد، چون فرض بر این است که چک اصلاً به گردش مالی وارد نشده). اگر چک از حالت "cleared" باطل شود، باید مشابه بخش ۵ (bounced از cleared) سند معکوس بسازد.

اصلاح - Frontend:
۱) در Checks.tsx یک دکمه‌ی «باطل کردن» جدا از چرخه‌ی «مرحله بعد» فعلی اضافه کن که مستقیم به‌جای advance، وضعیت 'cancelled' را صدا بزند (با تأیید Modal چون غیرقابل‌بازگشت است).
۲) یک صفحه یا Modal برای مدیریت دسته‌چک‌ها (لیست + ساخت جدید) اضافه کن.
۳) یک فرم برای ثبت چک ابتدای دوره اضافه کن (احتمالاً در بخش تنظیمات دوره‌ی مالی، مشابه گروه company در SettingsCenter).

تست: یک checkbook با start_number=1000، end_number=1010 بساز. یک چک صادره با checkbook_id آن و check_number=1005 بساز - باید موفق شود. یک چک دیگر با check_number=2000 (خارج از بازه) بساز - باید خطا بگیری. یک چک را به deposited ببر سپس cancelled کن - باید بدون سند حسابداری جدید status آن cancelled شود. یک چک دیگر را تا cleared ببر سپس cancelled کن - باید یک سند معکوس (مشابه منطق bounced-from-cleared در بخش ۵) ساخته شود.

بعد از اتمام: کامیت با پیام "feat(checks): checkbook management, opening-balance checks, void distinct from bounce"

═══════════════════════════════════════════
بخش ۱۱-و: دفتر اقساط و پرداخت وام (Installment Ledger)
═══════════════════════════════════════════

مشکل (متوسط - طراحی - عدم پشتیبانی از فروش/خرید اقساطی):
نرم‌افزار مرجع «دفتر اقساط» و «پرداخت وام» به‌عنوان بخش‌های مجزا دارد که امکان تعریف قسط‌بندی برای فاکتورهای نسیه و پیگیری بازپرداخت را می‌دهد. در ریپازیتوری فعلی هیچ مفهوم قسط یا وام وجود ندارد.

اصلاح - Backend:
۱) جدول installment_plans(id, company_id, reference_type, reference_id, total_amount, installment_count, start_date, created_by) و installment_items(id, plan_id, due_date, amount, status DEFAULT 'pending' CHECK(status IN('pending','paid','overdue')), paid_at) اضافه کن.
۲) Command create_installment_plan(reference_type, reference_id, total_amount, installment_count, start_date, interval_days) که installment_count قسط مساوی (یا نزدیک به مساوی، باقیمانده‌ی تقسیم را در آخرین قسط بگذار) با فاصله‌ی interval_days از start_date می‌سازد.
۳) Command pay_installment(item_id, treasury_account_id, payment_date) که یک تراکنش خزانه (با استفاده از منطق create_treasury_transaction موجود) برای مبلغ آن قسط ثبت می‌کند و status آن قسط را 'paid' می‌کند.
۴) Command list_installment_plans و get_installment_plan_detail.
۵) یک Command list_overdue_installments که installment_items با status='pending' و due_date < امروز را برمی‌گرداند (برای هشدار سررسید).

اصلاح - Frontend:
یک صفحه‌ی جدید InstallmentLedger.tsx بساز که لیست طرح‌های اقساط، جزئیات هر طرح (جدول اقساط با وضعیت رنگی: سررسیدشده قرمز، پرداخت‌شده سبز، در انتظار زرد)، و دکمه‌ی «ثبت پرداخت» برای هر قسط را نشان دهد.

تست: یک فاکتور فروش نسیه ۱۰۰۰۰۰۰ تومانی بساز. یک installment_plan برای آن با installment_count=4 و interval_days=30 بساز - باید ۴ قسط ۲۵۰۰۰۰ تومانی با فاصله‌ی ۳۰ روز ایجاد شود. یک قسط را pay_installment کن - باید treasury_transaction ثبت شود و status آن قسط 'paid' شود. list_overdue_installments را با یک قسط با due_date گذشته فرا بخوان - باید آن قسط را برگرداند.

بعد از اتمام: کامیت با پیام "feat(installments): implement installment plans and payment tracking"

═══════════════════════════════════════════
بخش ۱۱-ز: نمایش لاگ عملکرد کاربران (Audit Log Viewer)
═══════════════════════════════════════════

مشکل (متوسط - طراحی/امنیت - داده‌ی Audit موجود ولی بدون مصرف‌کننده):
جدول audit_logs در main.rs از قبل توسط تابع audit() به‌طور کامل پر می‌شود (هر عملیات create/update/delete در بخش‌های قبلی این پروژه یک رکورد آنجا ثبت می‌کند)، اما هیچ Command خواندنی برای این جدول وجود ندارد - یعنی این داده‌ی ارزشمند امنیتی/حسابرسی هرگز در دسترس کاربر یا مدیر سیستم قرار نمی‌گیرد، برخلاف نرم‌افزار مرجع که «لاگ عملکرد کاربران» را به‌عنوان یک ویژگی مدیریتی مجزا دارد.

اصلاح - Backend:
یک Command list_audit_logs(from_date: Option<String>, to_date: Option<String>, user_id: Option<String>, entity_type: Option<String>, limit: Option<i64>) اضافه کن که با Permission "system.audit.view" محافظت شود و رکوردهای audit_logs را فیلترشده و صفحه‌بندی‌شده (پیش‌فرض ۲۰۰ رکورد آخر) برگرداند، مرتب بر اساس created_at نزولی.

اصلاح - Frontend:
یک صفحه‌ی جدید AuditLog.tsx بساز با فیلترهای بازه‌ی تاریخ، کاربر، نوع موجودیت (contact/product/invoice/check/...)، و جدولی که action، entity_type، entity_id، user، زمان را نشان دهد. این صفحه را به منوی «امکانات» یا یک گروه امنیتی در SettingsCenter وصل کن.

تست: چند عملیات مختلف (ساخت Contact، ویرایش Product، حذف چیزی) انجام بده، سپس list_audit_logs را بدون فیلتر فرا بخوان - باید همه‌ی این رکوردها با ترتیب زمانی درست برگردند. با entity_type='contact' فیلتر کن - فقط رکوردهای مرتبط با Contact باید برگردند.

بعد از اتمام: کامیت با پیام "feat(audit): expose audit log viewer for user activity tracking"

═══════════════════════════════════════════
بخش ۱۱-ح: اتصال Frontend به قابلیت‌های موجود ولی وصل‌نشده‌ی Backend (Backup/Restore)
═══════════════════════════════════════════

مشکل (کم - طراحی - قابلیت موجود در Backend ولی بدون UI):
برخلاف تصور اولیه، main.rs از قبل Command های کامل backup_database، restore_database، list_backups و verify_backup_file را دارد. اما SettingsCenter.tsx (طبق بخش ۹-ب پرامپت‌های قبلی) گروه «backup» را همچنان با کارت‌های «آماده توسعه» نشان می‌دهد. اگر بخش ۹-ب پرامپت‌های قبلی قبلاً این گروه را متصل کرده، این زیربخش را نادیده بگیر (فقط بررسی کن)؛ اگر هنوز متصل نشده، آن را همین‌جا کامل کن.

اصلاح - Frontend:
در گروه backup در SettingsCenter.tsx، یک لیست از list_backups (نام، تاریخ، حجم فایل)، دکمه‌ی «تهیه‌ی نسخه‌ی پشتیبان جدید» (فراخوانی backup_database)، و دکمه‌ی «بازگردانی» برای هر نسخه (فراخوانی restore_database با تأیید Modal دو مرحله‌ای چون این عملیات مخرب و غیرقابل‌بازگشت است) اضافه کن.

تست: backup_database را از UI فرا بخوان - باید یک فایل پشتیبان جدید در لیست ظاهر شود. list_backups را بررسی کن که نتیجه با آنچه UI نشان می‌دهد یکسان است.

بعد از اتمام: کامیت با پیام "feat(settings): wire backup/restore UI to existing backend commands"

═══════════════════════════════════════════
دستورالعمل نهایی این بخش
═══════════════════════════════════════════
بعد از اتمام تمام زیربخش‌های ۱۱-الف تا ۱۱-ح، فایل docs/HARDENING_REPORT.md را باز کن و یک بخش جدید «بخش ۱۱: بستن شکاف با نرم‌افزار مرجع» اضافه کن که برای هر زیربخش (الف تا ح) وضعیت (تکمیل‌شده/بخشی تکمیل‌شده/مستندسازی‌شده به‌عنوان Known Gap) و فایل‌های تغییریافته را فهرست کند. به‌خصوص برای بخش‌های ۱۱-ب (سامانه مودیان) و بخش پرداخت کارتخوان در ۱۱-د، به‌وضوح ذکر کن که این‌ها فقط Scaffold/زیرساخت هستند و پیاده‌سازی واقعی (امضای دیجیتال، ارتباط با درگاه کارتخوان فیزیکی) نیاز به کار تخصصی جداگانه دارد که در این دور انجام نشده.

هیچ‌کدام از این ۸ زیربخش را با فرض «چون در نرم‌افزار مرجع هست پس باید دقیقاً همان‌طور پیاده شود» جلو نبر اگر با معماری فعلی (Rust/Tauri/SQLite در برابر معماری قدیمی Delphi/Windows) در تضاد اساسی بود؛ در چنین مواردی، معادل منطقی و قابل‌اجرا در معماری فعلی را پیاده کن و تفاوت را در گزارش توضیح بده.
```
