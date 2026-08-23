/**
 * ادغام کلاس‌های Tailwind با حل تضاد.
 *
 * چرا لازم است: در `cn('p-2', condition && 'p-4')` بدون ادغام هوشمند، هر دو
 * کلاس در خروجی می‌مانند و کدام یک اثر بگذارد به ترتیب تولید CSS بستگی دارد
 * — یعنی نتیجه غیرقابل پیش‌بینی می‌شود. `tailwind-merge` کلاس متأخر را
 * برنده می‌کند، همان چیزی که نویسنده انتظار دارد.
 */
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
