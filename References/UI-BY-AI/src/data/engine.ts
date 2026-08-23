// ---------------------------------------------------------------------------
// Novin Pardaz — realistic accounting data engine
// Seeded & deterministic. Relationships are accounting-consistent:
//   receipts settle sales  ->  receivables = sales - receipts
//   payments settle purchases -> payables = purchases - payments
//   profit = Σ(sale margin) - expenses
// ---------------------------------------------------------------------------

import {
  addDays,
  DAY,
  dayKey,
  faDateShort,
  faMonth,
  jParts,
  previousRange,
  startOfDay,
  type DateRange,
} from "../lib/format";

// --------- seed -------------------------------------------------------------
function mulberry32(seed: number) {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// --------- types ------------------------------------------------------------
export type TxType = "sale" | "purchase" | "receipt" | "payment" | "expense" | "transfer";
export type TxStatus = "settled" | "pending" | "due" | "cancelled";
export type CheckStatus = "inHand" | "cashed" | "bounced";

export interface Branch { id: string; name: string }
export interface Account { id: string; name: string; bank: string }
export interface Category { id: string; name: string }
export interface AppUser { id: string; name: string; role: string }
export interface Customer { id: string; name: string; city: string }
export interface Product {
  id: string; name: string; categoryId: string; price: number; cost: number;
  stock: number; sold: number;
}

export interface Tx {
  id: string;
  date: number; // epoch ms
  doc: string;
  type: TxType;
  status: TxStatus;
  partyId: string;
  partyName: string;
  amount: number;
  cost: number; // only for sales
  branchId: string;
  accountId: string;
  userId: string;
  categoryId: string; // product category or expense category
  qty: number;
  productId: string;
  parentId?: string; // receipt -> sale, payment -> purchase
  dueDate?: number;
}

export interface Check {
  id: string; partyName: string; amount: number; dueDate: number;
  status: CheckStatus; sayyad: string;
}

export interface Filters {
  range: DateRange;
  branchId: string;
  accountId: string;
  categoryId: string;
  txType: string;
  userId: string;
}

// --------- static master data ------------------------------------------------
export const BRANCHES: Branch[] = [
  { id: "hq", name: "شعبه مرکزی" },
  { id: "saadat", name: "شعبه سعادت‌آباد" },
  { id: "isfahan", name: "شعبه اصفهان" },
  { id: "mashhad", name: "شعبه مشهد" },
];

export const ACCOUNTS: Account[] = [
  { id: "mellat", name: "ملت — جاری ۵۲۳۷", bank: "بانک ملت" },
  { id: "saman", name: "سامان — ۸۱۰۴۵", bank: "بانک سامان" },
  { id: "pasargad", name: "پاسارگاد — ۹۰۱۱۲", bank: "بانک پاسارگاد" },
  { id: "cash", name: "صندوق مرکزی", bank: "نقدی" },
];

export const EXPENSE_CATEGORIES: Category[] = [
  { id: "salary", name: "حقوق و دستمزد" },
  { id: "rent", name: "اجاره" },
  { id: "purchase", name: "خرید مصرفی" },
  { id: "transport", name: "حمل‌ونقل" },
  { id: "ads", name: "تبلیغات" },
  { id: "bills", name: "قبوض" },
  { id: "other", name: "سایر" },
];

export const PRODUCT_CATEGORIES: Category[] = [
  { id: "hardware", name: "سخت‌افزار" },
  { id: "mobile-acc", name: "جانبی موبایل" },
  { id: "pc-acc", name: "جانبی کامپیوتر" },
  { id: "network", name: "تجهیزات شبکه" },
  { id: "office", name: "تجهیزات اداری" },
  { id: "av", name: "صوتی و تصویری" },
];

export const USERS: AppUser[] = [
  { id: "admin", name: "مدیر سیستم", role: "مدیر" },
  { id: "hoseini", name: "علی حسینی", role: "حسابدار" },
  { id: "karimi", name: "مریم کریمی", role: "صندوق‌دار" },
  { id: "naderi", name: "سارا نادری", role: "فروشنده" },
];

const CUSTOMER_NAMES: [string, string][] = [
  ["شرکت بازرگانی آفتاب‌گردان", "تهران"], ["فروشگاه زنجیره‌ای مهر و ماه", "تهران"],
  ["آقای محمد رضایی", "اصفهان"], ["سرکار خانم سارا احمدی", "شیراز"],
  ["شرکت پخش آریا", "تهران"], ["فروشگاه دیجیتال پارس", "مشهد"],
  ["آقای حسن کاظمی", "تبریز"], ["موسسه فرهنگی نگارستان", "تهران"],
  ["شرکت مهندسی سازه‌پرداز", "کرج"], ["فروشگاه لوازم‌التحریر قلم", "قم"],
  ["سرکار خانم نیلوفر موسوی", "تهران"], ["آقای رضا صادقی", "اهواز"],
  ["شرکت تجاری دریای خزر", "رشت"], ["فروشگاه اینترنتی کالاچین", "تهران"],
  ["آقای بهنام شریفی", "کرمان"], ["مدرسه اندیشه نو", "تهران"],
  ["شرکت داروسازی سلامت‌یار", "تهران"], ["فروشگاه موبایل همراه", "اصفهان"],
  ["سرکار خانم مریم عزیزی", "مشهد"], ["آقای فرهاد جلالی", "یزد"],
  ["رستوران سنتی باغ ایرانی", "تهران"], ["شرکت تبلیغاتی رنگین‌کمان", "تهران"],
  ["فروشگاه اسپرت ورزش", "کرج"], ["آقای مجید قاسمی", "اردبیل"],
  ["سرکار خانم الهه رستمی", "تهران"], ["کلینیک تخصصی سلامت", "شیراز"],
  ["شرکت چاپ و نشر آوا", "تهران"], ["فروشگاه عکاسی لنز", "تهران"],
  ["آقای امیر توکلی", "زنجان"], ["کافی‌شاپ رقص نور", "اصفهان"],
  ["شرکت حمل‌ونقل سپید", "بندرعباس"], ["فروشگاه کتاب دانش", "تهران"],
  ["سرکار خانم فرشیّده کاظم‌نژاد", "قم"], ["آقای سعید مرادی", "ساری"],
  ["شرکت صنعتی فولاد غرب", "کرمانشاه"], ["آموزشگاه زبان گفتمان", "تهران"],
  ["فروشگاه پوشاک مروارید", "تهران"], ["آقای پویا نیک‌بخت", "همدان"],
  ["شرکت گردشگری سفر سبز", "تهران"], ["سرکار خانم درسا امینی", "بابل"],
  ["فروشگاه زیورآلات سیمین", "تهران"], ["آقای کاوه رفیعی", "گرگان"],
  ["شرکت معماری عمران‌ساز", "تهران"], ["داروخانه شفا", "اراک"],
  ["سرکار خانم ترانه رفیعی", "تهران"], ["آقای بابک قنبری", "سمنان"],
  ["فروشگاه کفش نگین", "مشهد"], ["شرکت خدماتی نگین‌پاک", "تهران"],
  ["آقای آرش ملکی", "یاسوج"], ["مرکز آموزش علمی تدبیر", "تهران"],
];

const PRODUCTS: [string, string, number][] = [
  // name, categoryId, basePrice (toman)
  ["لپ‌تاپ ۱۵ اینچ مدل X200", "hardware", 42_500_000],
  ["لپ‌تاپ ۱۳ اینچ اولترابوک", "hardware", 56_000_000],
  ["کامپیوتر اداری Core i5", "hardware", 28_900_000],
  ["مینی‌کامپیوتر صنعتی", "hardware", 19_800_000],
  ["مانیتور ۲۷ اینچ IPS", "hardware", 11_400_000],
  ["مانیتور ۲۴ اینچ اداری", "hardware", 7_300_000],
  ["گوشی هوشمند مدل A54", "hardware", 18_200_000],
  ["تبلت ۱۰ اینچ", "hardware", 13_700_000],
  ["هارد اکسترنال ۱ ترابایت", "hardware", 3_650_000],
  ["حافظه SSD یک ترابایت", "hardware", 4_950_000],
  ["رم ۱۶ گیگ DDR4", "hardware", 3_200_000],
  ["پاوربانک ۲۰٬۰۰۰ میلی‌آمپر", "mobile-acc", 1_280_000],
  ["شارژر سریع ۶۵ وات", "mobile-acc", 690_000],
  ["کابل Type-C دو متری", "mobile-acc", 240_000],
  ["هندزفری بلوتوثی Pro", "mobile-acc", 1_950_000],
  ["قاب محافظ ژله‌ای", "mobile-acc", 180_000],
  ["ماوس بی‌سیم سایلنت", "pc-acc", 720_000],
  ["کیبورد مکانیکال RGB", "pc-acc", 3_400_000],
  ["هاب USB-C شش‌پورت", "pc-acc", 1_150_000],
  ["وب‌کم Full HD", "pc-acc", 2_300_000],
  ["میکروفون استودیویی USB", "pc-acc", 4_100_000],
  ["روتر بی‌سیم AX3000", "network", 4_600_000],
  ["سوییچ ۸ پورت گیگابیت", "network", 3_100_000],
  ["نقطه دسترسی سقفی", "network", 5_800_000],
  ["پرینتر لیزری تک‌کاره", "office", 9_700_000],
  ["پرینتر چندکاره جوهرافشان", "office", 7_900_000],
  ["دستگاه کارتخوان سیار", "office", 4_300_000],
  ["کاغذ A4 بسته ۵۰۰ برگی", "office", 390_000],
  ["اسپیکر دسکتاپ استریو", "av", 2_750_000],
  ["ساندبار خانگی", "av", 6_600_000],
];

// Jalali-month seasonality (Farvardin .. Esfand)
const SEASON = [0.62, 0.78, 0.92, 1.0, 1.06, 1.04, 0.96, 1.02, 0.95, 1.08, 1.18, 1.42];

// --------- generation ---------------------------------------------------------
export interface Database {
  txs: Tx[];
  customers: Customer[];
  products: Product[];
  checks: Check[];
  lastUpdate: Date;
}

let cache: Database | null = null;

export function getDatabase(): Database {
  if (cache) return cache;
  const rnd = mulberry32(20260521);
  const R = (min: number, max: number) => min + rnd() * (max - min);
  const RI = (min: number, max: number) => Math.floor(R(min, max + 1));
  const pick = <T,>(arr: readonly T[]): T => arr[Math.floor(rnd() * arr.length)];

  const now = new Date();
  const today = startOfDay(now);
  const DAYS = 400;

  const customers: Customer[] = CUSTOMER_NAMES.map(([name, city], i) => ({
    id: `c${i}`,
    name,
    city,
  }));

  const products: Product[] = PRODUCTS.map(([name, categoryId, price], i) => ({
    id: `p${i}`,
    name,
    categoryId,
    price: Math.round(price * R(0.96, 1.05)),
    cost: 0,
    stock: RI(30, 120),
    sold: 0,
  }));
  products.forEach((p) => (p.cost = Math.round(p.price * R(0.6, 0.73))));

  const branchW = [0.46, 0.24, 0.17, 0.13];
  const pickBranch = () => {
    const r = rnd();
    let acc = 0;
    for (let i = 0; i < branchW.length; i++) {
      acc += branchW[i];
      if (r <= acc) return BRANCHES[i].id;
    }
    return "hq";
  };
  const pickAccount = () => pick(ACCOUNTS).id;
  const pickUser = () => pick(USERS).id;

  const txs: Tx[] = [];
  let nSale = 0, nPurchase = 0, nReceipt = 0, nPayment = 0, nExpense = 0, nTransfer = 0, nId = 0;
  const nid = () => `t${++nId}`;

  const push = (t: Omit<Tx, "id">) => {
    txs.push({ id: nid(), ...t });
  };

  for (let d = DAYS; d >= 0; d--) {
    const date = addDays(today, -d);
    const { jm } = jParts(date);
    const season = SEASON[jm - 1];
    const dow = date.getDay(); // 5 = Friday in JS (Fri=5)
    const dowF = dow === 5 ? 0.42 : dow === 4 ? 1.35 : dow === 0 ? 1.1 : 1;
    const daySales = Math.max(1, Math.round(R(1, 5) * season * dowF * (d === DAYS ? 0.2 : 1)));

    // ---- sales (+ linked receipts)
    for (let s = 0; s < daySales; s++) {
      const product = pick(products);
      const qty = RI(1, product.price > 20_000_000 ? 4 : product.price > 5_000_000 ? 9 : 36);
      const unit = Math.round(product.price * R(0.98, 1.06));
      const amount = unit * qty;
      const cost = product.cost * qty;
      const customer = pick(customers);
      const branchId = pickBranch();
      const userId = pickUser();
      const t = date.getTime() + RI(9, 20) * 3_600_000;
      const stR = rnd();
      const status: TxStatus = stR < 0.9 ? "settled" : stR < 0.96 ? "pending" : stR < 0.985 ? "due" : "cancelled";
      product.sold += qty;

      nSale++;
      const saleId = nid();
      txs.push({
        id: saleId,
        date: t,
        doc: `ف-${1200 + nSale}`,
        type: "sale",
        status,
        partyId: customer.id,
        partyName: customer.name,
        amount,
        cost,
        branchId,
        accountId: pickAccount(),
        userId,
        categoryId: product.categoryId,
        qty,
        productId: product.id,
        dueDate: startOfDay(addDays(date, 45)).getTime(),
      });

      // receipt settles this sale (only not-cancelled ones can be paid)
      if (status !== "cancelled") {
        const r = rnd();
        if (r < 0.81) {
          const payDate = addDays(date, RI(0, 16));
          if (payDate.getTime() <= now.getTime()) {
            nReceipt++;
            push({
              date: payDate.getTime() + RI(9, 20) * 3_600_000,
              doc: `د-${4100 + nReceipt}`,
              type: "receipt",
              status: "settled",
              partyId: customer.id,
              partyName: customer.name,
              amount: r < 0.72 ? amount : Math.round(amount * 0.5),
              cost: 0,
              branchId,
              accountId: pickAccount(),
              userId,
              categoryId: product.categoryId,
              qty: 0,
              productId: product.id,
              parentId: saleId,
            });
          }
        }
      }
    }

    // ---- purchases (restock) roughly every 1-2 days
    if (d % 2 === 0 || rnd() < 0.4) {
      const product = pick(products);
      const qty = RI(10, product.price > 10_000_000 ? 22 : 130);
      const amount = Math.round(product.cost * qty * R(1, 1.04));
      const vendor = pick(customers);
      const branchId = pickBranch();
      nPurchase++;
      const pId = nid();
      const t = date.getTime() + RI(9, 19) * 3_600_000;
      txs.push({
        id: pId,
        date: t,
        doc: `خ-${700 + nPurchase}`,
        type: "purchase",
        status: rnd() < 0.93 ? "settled" : "pending",
        partyId: vendor.id,
        partyName: `تامین‌کننده — ${vendor.name}`,
        amount,
        cost: 0,
        branchId,
        accountId: pickAccount(),
        userId: pickUser(),
        categoryId: product.categoryId,
        qty,
        productId: product.id,
        dueDate: startOfDay(addDays(date, 60)).getTime(),
      });
      product.stock += qty;

      if (rnd() < 0.86) {
        const payDate = addDays(date, RI(0, 22));
        if (payDate.getTime() <= now.getTime()) {
          nPayment++;
          push({
            date: payDate.getTime() + RI(9, 19) * 3_600_000,
            doc: `پ-${9000 + nPayment}`,
            type: "payment",
            status: "settled",
            partyId: vendor.id,
            partyName: `تامین‌کننده — ${vendor.name}`,
            amount,
            cost: 0,
            branchId,
            accountId: pickAccount(),
            userId: pickUser(),
            categoryId: product.categoryId,
            qty: 0,
            productId: product.id,
            parentId: pId,
          });
        }
      }
    }

    // ---- transfers between accounts (weekly)
    if (d % 6 === 0) {
      nTransfer++;
      push({
        date: date.getTime() + 3_600_000 * 11,
        doc: `ن-${300 + nTransfer}`,
        type: "transfer",
        status: "settled",
        partyId: "internal",
        partyName: "انتقال بین حساب‌ها",
        amount: RI(8, 60) * 1_000_000,
        cost: 0,
        branchId: pickBranch(),
        accountId: pickAccount(),
        userId: "admin",
        categoryId: "other",
        qty: 0,
        productId: "",
      });
    }

    // ---- expenses
    const addExpense = (cat: string, amount: number, branchId: string, t: number) => {
      nExpense++;
      push({
        date: t,
        doc: `ه-${500 + nExpense}`,
        type: "expense",
        status: "settled",
        partyId: "expense",
        partyName: EXPENSE_CATEGORIES.find((c) => c.id === cat)!.name,
        amount,
        cost: 0,
        branchId,
        accountId: pickAccount(),
        userId: pickUser(),
        categoryId: cat,
        qty: 0,
        productId: "",
      });
    };
    const { jd } = jParts(date);
    const tMid = date.getTime() + 3_600_000 * 17;
    if (jd === 1) {
      BRANCHES.forEach((b, i) => {
        addExpense("salary", RI(11, 19) * 5_850_000 * (i === 0 ? 2.2 : 1), b.id, tMid);
        addExpense("rent", (i === 0 ? 48 : RI(16, 26)) * 1_000_000, b.id, tMid);
        addExpense("bills", RI(45, 110) * 100_000, b.id, tMid);
      });
    }
    if (d % 5 === 2) addExpense("transport", RI(24, 68) * 100_000, pickBranch(), tMid);
    if (d % 12 === 4) addExpense("ads", RI(90, 230) * 100_000, "hq", tMid);
    if (d % 4 === 1) addExpense("purchase", RI(30, 95) * 100_000, pickBranch(), tMid);
    if (d % 21 === 7) addExpense("other", RI(55, 160) * 100_000, "hq", tMid);

    // guarantee visible activity for «امروز» / «دیروز» presets
    if (d <= 1) {
      const gCust = pick(customers);
      const gProd = pick(products);
      const gTime = Math.min(
        date.getTime() + RI(8, 19) * 3_600_000 + RI(0, 3000) * 1000,
        now.getTime() - 300_000,
      );
      const gBranch = pickBranch();
      nReceipt++;
      push({
        date: gTime,
        doc: `د-${4100 + nReceipt}`,
        type: "receipt",
        status: "settled",
        partyId: gCust.id,
        partyName: gCust.name,
        amount: Math.round(RI(12, 68)) * 1_000_000,
        cost: 0,
        branchId: gBranch,
        accountId: pickAccount(),
        userId: pickUser(),
        categoryId: gProd.categoryId,
        qty: 0,
        productId: gProd.id,
      });
      const gVend = pick(customers);
      nPayment++;
      push({
        date: Math.max(date.getTime() + 3_600_000 * 9, gTime - 3_600_000 * RI(2, 5)),
        doc: `پ-${9000 + nPayment}`,
        type: "payment",
        status: "settled",
        partyId: gVend.id,
        partyName: `تامین‌کننده — ${gVend.name}`,
        amount: Math.round(RI(8, 42)) * 1_000_000,
        cost: 0,
        branchId: gBranch,
        accountId: pickAccount(),
        userId: pickUser(),
        categoryId: gProd.categoryId,
        qty: 0,
        productId: gProd.id,
      });
      if (d === 0) addExpense("transport", RI(32, 74) * 100_000, gBranch, gTime - 3_600_000);
    }
  }

  // stock consistency
  products.forEach((p) => (p.stock = Math.max(2, p.stock - p.sold)));

  // ---- checks
  const checks: Check[] = [];
  for (let i = 0; i < 14; i++) {
    const st = rnd();
    checks.push({
      id: `ch${i}`,
      partyName: pick(customers).name,
      amount: RI(6, 95) * 1_000_000,
      dueDate: startOfDay(addDays(today, RI(-9, 24))).getTime(),
      status: st < 0.55 ? "inHand" : st < 0.85 ? "cashed" : "bounced",
      sayyad: `۱۲۸${RI(10_000, 99_999)}`,
    });
  }

  txs.sort((a, b) => b.date - a.date);
  cache = { txs, customers, products, checks, lastUpdate: now };
  return cache;
}

// --------- labels -------------------------------------------------------------
export const TX_TYPE_LABEL: Record<TxType, string> = {
  sale: "فروش",
  purchase: "خرید",
  receipt: "دریافت",
  payment: "پرداخت",
  expense: "هزینه",
  transfer: "انتقال وجه",
};

export const TX_STATUS_LABEL: Record<TxStatus, string> = {
  settled: "تکمیل شده",
  pending: "در انتظار",
  due: "سررسید شده",
  cancelled: "لغو شده",
};

export const CHECK_STATUS_LABEL: Record<CheckStatus, string> = {
  inHand: "دریافتنی",
  cashed: "وصول شده",
  bounced: "برگشتی",
};

// --------- filtering & aggregation --------------------------------------------
function matchesDims(t: Tx, f: Filters): boolean {
  if (f.branchId !== "all" && t.branchId !== f.branchId) return false;
  if (f.accountId !== "all" && t.accountId !== f.accountId) return false;
  if (f.userId !== "all" && t.userId !== f.userId) return false;
  if (f.categoryId !== "all" && t.categoryId !== f.categoryId) return false;
  if (f.txType !== "all" && t.type !== f.txType) return false;
  return true;
}

const inRange = (t: Tx, from: number, to: number) => t.date >= from && t.date <= to;

export interface KPISet {
  sales: number; profit: number; receipts: number; payments: number;
  receivables: number; payables: number; inventory: number; invoices: number;
}

function sumKPIs(txs: Tx[], from: number, to: number): Omit<KPISet, "receivables" | "payables" | "inventory"> {
  let sales = 0, margin = 0, receipts = 0, payments = 0, expenses = 0, invoices = 0;
  for (const t of txs) {
    if (!inRange(t, from, to)) continue;
    switch (t.type) {
      case "sale":
        if (t.status !== "cancelled") {
          sales += t.amount;
          margin += t.amount - t.cost;
          invoices++;
        }
        break;
      case "receipt":
        receipts += t.amount;
        break;
      case "payment":
        payments += t.amount;
        break;
      case "expense":
        expenses += t.amount;
        break;
    }
  }
  return { sales, profit: margin - expenses, receipts, payments: payments + expenses, invoices };
}

export interface Bucket {
  key: string; label: string;
  sales: number; purchases: number; expenses: number; profit: number;
  receipts: number; out: number; net: number;
}

export type Granularity = "hour" | "day" | "week" | "month";

export function autoGranularity(from: number, to: number): Granularity {
  const days = Math.round((to - from) / DAY);
  if (days <= 3) return "hour";
  if (days <= 62) return "day";
  if (days <= 240) return "week";
  return "month";
}

export function bucketize(txs: Tx[], from: number, to: number, g: Granularity): Bucket[] {
  const map = new Map<string, Bucket>();
  const order: string[] = [];
  const getBucket = (d: Date): { key: string; label: string } => {
    if (g === "hour") {
      const h = Math.floor(d.getHours() / 4) * 4;
      return {
        key: `${dayKey(d)}-${h}`,
        label: `${faDateShort(d)} · ${new Intl.NumberFormat("fa-IR").format(h)} تا ${new Intl.NumberFormat("fa-IR").format(h + 4)}`,
      };
    }
    if (g === "day") return { key: dayKey(d), label: faDateShort(d) };
    if (g === "week") {
      const offset = (d.getDay() + 1) % 7;
      const sat = startOfDay(addDays(d, -offset));
      return { key: `w${dayKey(sat)}`, label: faDateShort(sat) };
    }
    return { key: `m${jParts(d).jy}-${jParts(d).jm}`, label: faMonth(d) };
  };

  const touch = (d: Date): Bucket => {
    const { key, label } = getBucket(d);
    let b = map.get(key);
    if (!b) {
      b = { key, label, sales: 0, purchases: 0, expenses: 0, profit: 0, receipts: 0, out: 0, net: 0 };
      map.set(key, b);
      order.push(key);
    }
    return b;
  };

  for (const t of txs) {
    if (!inRange(t, from, to)) continue;
    const b = touch(new Date(t.date));
    switch (t.type) {
      case "sale":
        if (t.status !== "cancelled") {
          b.sales += t.amount;
          b.profit += t.amount - t.cost;
        }
        break;
      case "purchase":
        b.purchases += t.amount;
        break;
      case "expense":
        b.expenses += t.amount;
        b.profit -= t.amount;
        b.out += t.amount;
        break;
      case "receipt":
        b.receipts += t.amount;
        break;
      case "payment":
        b.out += t.amount;
        break;
    }
    b.net = b.receipts - b.out;
  }
  return order.map((k) => map.get(k)!);
}

// receivables / payables status buckets
export interface AgingBucket { label: string; amount: number; count: number; tone: "ok" | "warn" | "bad" | "done" }
export interface Aging {
  total: number;
  buckets: AgingBucket[];
}

function buildAging(
  txs: Tx[],
  kind: "sale" | "purchase",
  settleType: "receipt" | "payment",
  dims: Filters,
  asOf: number,
): Aging {
  const settled = new Map<string, number>();
  for (const t of txs) {
    if (t.type === settleType && t.parentId && t.date <= asOf && matchesDims(t, dims)) {
      settled.set(t.parentId, (settled.get(t.parentId) ?? 0) + t.amount);
    }
  }
  let current = 0, near = 0, over = 0, done = 0;
  let cCurrent = 0, cNear = 0, cOver = 0, cDone = 0;
  const nearLimit = asOf + 7 * DAY;
  for (const t of txs) {
    if (t.type !== kind || t.status === "cancelled" || t.date > asOf || !matchesDims(t, dims)) continue;
    const paid = settled.get(t.id) ?? 0;
    const open = t.amount - paid;
    if (open <= 500) {
      done += t.amount;
      cDone++;
      continue;
    }
    const due = t.dueDate ?? t.date + 45 * DAY;
    if (due < asOf) {
      over += open; cOver++;
    } else if (due <= nearLimit) {
      near += open; cNear++;
    } else {
      current += open; cCurrent++;
    }
  }
  return {
    total: current + near + over,
    buckets: [
      { label: "جاری", amount: current, count: cCurrent, tone: "ok" },
      { label: "سررسید نزدیک", amount: near, count: cNear, tone: "warn" },
      { label: "سررسید گذشته", amount: over, count: cOver, tone: "bad" },
      { label: "تسویه شده", amount: done, count: cDone, tone: "done" },
    ],
  };
}

export interface Derived {
  filtered: Tx[];
  kpis: KPISet;
  prev: KPISet;
  trend: Bucket[];
  granularity: Granularity;
  expenses: { name: string; value: number; id: string }[];
  topProducts: { id: string; name: string; qty: number; revenue: number }[];
  topCustomers: {
    id: string; name: string; count: number; amount: number; balance: number; last: number;
  }[];
  receivables: Aging;
  payables: Aging;
}

export function derive(db: Database, f: Filters): Derived {
  const from = f.range.from.getTime();
  const to = f.range.to.getTime();

  const scope = db.txs.filter((t) => matchesDims(t, f));
  const filtered = scope.filter((t) => inRange(t, from, to));

  const cur = sumKPIs(scope, from, to);
  const pr = previousRange(f.range);
  const prev = sumKPIs(scope, pr.from.getTime(), pr.to.getTime());

  const { receivables, payables } = agings(db, f, to);
  const inventory = inventoryValue(db, f, to);

  const kpis: KPISet = { ...cur, receivables, payables, inventory };
  const prevSnap = snapshotBefore(db, f, pr.to.getTime());

  const granularity = autoGranularity(from, to);
  const trend = bucketize(db.txs.filter((t) => matchesDims(t, f)), from, to, granularity);

  // expense breakdown
  const exp = new Map<string, number>();
  for (const t of filtered) {
    if (t.type === "expense") exp.set(t.categoryId, (exp.get(t.categoryId) ?? 0) + t.amount);
  }
  const expenses = EXPENSE_CATEGORIES.map((c) => ({
    id: c.id,
    name: c.name,
    value: exp.get(c.id) ?? 0,
  })).filter((e) => e.value > 0);

  // top products from filtered sales
  const prod = new Map<string, { qty: number; revenue: number }>();
  for (const t of filtered) {
    if (t.type === "sale" && t.status !== "cancelled" && t.productId) {
      const p = prod.get(t.productId) ?? { qty: 0, revenue: 0 };
      p.qty += t.qty;
      p.revenue += t.amount;
      prod.set(t.productId, p);
    }
  }
  const topProducts = [...prod.entries()]
    .map(([id, v]) => ({ id, name: db.products.find((p) => p.id === id)?.name ?? "—", ...v }))
    .sort((a, b) => b.revenue - a.revenue)
    .slice(0, 8);

  // top customers
  const cust = new Map<string, { count: number; amount: number; balance: number; last: number }>();
  for (const t of db.txs) {
    if (!matchesDims(t, f) || !inRange(t, from, to)) continue;
    if (t.type === "sale" && t.status !== "cancelled") {
      const c = cust.get(t.partyId) ?? { count: 0, amount: 0, balance: 0, last: 0 };
      c.count++;
      c.amount += t.amount;
      c.balance += t.amount;
      c.last = Math.max(c.last, t.date);
      cust.set(t.partyId, c);
    } else if (t.type === "receipt") {
      const c = cust.get(t.partyId) ?? { count: 0, amount: 0, balance: 0, last: 0 };
      c.balance -= t.amount;
      c.last = Math.max(c.last, t.date);
      cust.set(t.partyId, c);
    }
  }
  const topCustomers = [...cust.entries()]
    .map(([id, v]) => ({ id, name: db.customers.find((c) => c.id === id)?.name ?? vName(id), ...v }))
    .filter((c) => c.amount > 0)
    .sort((a, b) => b.amount - a.amount)
    .slice(0, 6);

  return {
    filtered,
    kpis,
    prev: { ...prev, ...prevSnap },
    trend,
    granularity,
    expenses,
    topProducts,
    topCustomers,
    receivables: agings(db, f, to).r,
    payables: agings(db, f, to).p,
  };
}

const vName = (id: string) => (id === "expense" ? "هزینه" : id === "internal" ? "داخلی" : "—");

function agings(db: Database, f: Filters, asOf: number) {
  const dims = { ...f, txType: "all" };
  const r = buildAging(db.txs, "sale", "receipt", dims, asOf);
  const p = buildAging(db.txs, "purchase", "payment", dims, asOf);
  return { receivables: r.total, payables: p.total, r, p };
}

function inventoryValue(db: Database, f: Filters, asOf: number): number {
  // opening stock value, scaled when the scope is narrowed
  let value = 620_000_000 * (f.branchId !== "all" ? 0.27 : 1) * (f.categoryId !== "all" ? 0.15 : 1);
  const dims = { ...f, txType: "all" };
  for (const t of db.txs) {
    if (t.date > asOf || !matchesDims(t, dims)) continue;
    if (t.type === "purchase") value += t.amount;
    else if (t.type === "sale" && t.status !== "cancelled") value -= t.cost;
  }
  return Math.max(0, value);
}

function snapshotBefore(db: Database, f: Filters, asOf: number) {
  const { receivables, payables } = agings(db, f, asOf);
  return {
    receivables,
    payables,
    inventory: inventoryValue(db, f, asOf),
  };
}

// change helper
export function pctChange(cur: number, prev: number): number | null {
  if (!prev) return cur > 0 ? 100 : null;
  return ((cur - prev) / Math.abs(prev)) * 100;
}
