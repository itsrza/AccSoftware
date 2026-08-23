// ---------------------------------------------------------------------------
// Navigation information architecture (mirrors Novin Pardaz modules)
// ---------------------------------------------------------------------------
import {
  LayoutDashboard, ShoppingCart, ShoppingBag, Boxes, Tags, Zap, Users, UserCog,
  Vault, ScrollText, BookOpen, Rows3, BarChart3, Blocks, ArrowLeftRight, Printer,
  Settings, Activity, type LucideIcon,
} from "lucide-react";

export interface NavItem {
  id: string;
  label: string;
  icon: LucideIcon;
  children?: { id: string; label: string }[];
  desc?: string;
}

export interface NavGroup {
  id: string;
  label: string;
  items: NavItem[];
}

export const NAV_GROUPS: NavGroup[] = [
  {
    id: "main",
    label: "اصلی",
    items: [
      { id: "dashboard", label: "داشبورد", icon: LayoutDashboard, desc: "نمای کلی عملکرد مالی مجموعه" },
    ],
  },
  {
    id: "ops",
    label: "عملیات",
    items: [
      {
        id: "sales", label: "فروش", icon: ShoppingCart, desc: "مدیریت چرخه فروش",
        children: [
          { id: "sales-invoice", label: "فاکتور فروش" },
          { id: "sales-return", label: "برگشت از فروش" },
          { id: "sales-quote", label: "پیش‌فاکتورها" },
        ],
      },
      { id: "quick-invoice", label: "صدور فاکتور فروش", icon: Zap, desc: "صدور سریع فاکتور فروش به مشتریان" },
      { id: "purchase", label: "خرید", icon: ShoppingBag, desc: "فاکتورهای خرید و تامین کالا" },
      { id: "inventory", label: "انبار و کالا", icon: Boxes, desc: "موجودی، کاردکس و گردش کالا" },
      { id: "prices", label: "قیمت کالاها", icon: Tags, desc: "سطوح قیمت و به‌روزرسانی نرخ‌ها" },
    ],
  },
  {
    id: "finance",
    label: "مالی",
    items: [
      { id: "treasury", label: "خزانه", icon: Vault, desc: "دریافت‌ها، پرداخت‌ها و حساب‌های بانکی" },
      { id: "checks", label: "چک‌ها", icon: ScrollText, desc: "چک‌های دریافتنی و پرداختنی" },
      { id: "accounting", label: "حسابداری", icon: BookOpen, desc: "اسناد، دفاتر و ترازنامه" },
      { id: "simple-doc", label: "سند یکسطری", icon: Rows3, desc: "ثبت سریع سند حسابداری" },
    ],
  },
  {
    id: "people",
    label: "طرف‌های حساب",
    items: [
      { id: "persons", label: "اشخاص", icon: Users, desc: "مشتریان، تامین‌کنندگان و معرفین" },
      { id: "persons-admin", label: "مدیریت اشخاص", icon: UserCog, desc: "سطوح دسترسی و گروه‌بندی اشخاص" },
    ],
  },
  {
    id: "tools",
    label: "ابزارها",
    items: [
      { id: "reports", label: "گزارشات", icon: BarChart3, desc: "گزارش‌های مالی و عملیاتی" },
      { id: "integrations", label: "اتصالات و افزونه‌ها", icon: Blocks, desc: "اتصال به فروشگاه‌ساز، درگاه و سامانه مودیان" },
      { id: "io", label: "ورود و خروج اطلاعات", icon: ArrowLeftRight, desc: "انتقال داده از/به اکسل و نسخه‌های قبلی" },
      { id: "print-templates", label: "قالب‌های چاپ", icon: Printer, desc: "طراحی قالب فاکتور و اسناد" },
    ],
  },
];

export const BOTTOM_NAV: NavItem[] = [
  { id: "settings", label: "تنظیمات برنامه", icon: Settings, desc: "پیکربندی نرم‌افزار و سیاست‌ها" },
  { id: "system", label: "وضعیت سیستم", icon: Activity, desc: "سلامت سرویس‌ها و نسخه برنامه" },
];

// flat lookup
const flat = new Map<string, { title: string; groupLabel: string; desc?: string }>();
for (const g of NAV_GROUPS) {
  for (const item of g.items) {
    flat.set(item.id, { title: item.label, groupLabel: g.label, desc: item.desc });
    item.children?.forEach((c) =>
      flat.set(c.id, { title: c.label, groupLabel: item.label, desc: item.desc }),
    );
  }
}
for (const b of BOTTOM_NAV) flat.set(b.id, { title: b.label, groupLabel: "سیستم", desc: b.desc });

export function pageMeta(id: string) {
  return flat.get(id) ?? { title: "داشبورد", groupLabel: "اصلی", desc: "" };
}
