import { Option } from "@/components/ui/multiple-selector";
import { type ClassValue, clsx } from "clsx"
import React from "react";
import { useEffect } from "react";
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}


export function useDebounce<T>(value: T, delay?: number): T {
  const [debouncedValue, setDebouncedValue] = React.useState<T>(value);

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedValue(value), delay || 500);

    return () => {
      clearTimeout(timer);
    };
  }, [value, delay]);

  return debouncedValue;
}

export function capitalizeFirstLetter(string: string) {
  return string.charAt(0).toUpperCase() + string.slice(1).toLowerCase();
}

export function getDateAsString(date: Date | undefined) {
  if (!date) {
    return undefined;
  }

  const year = date.getFullYear();
  const month = (date.getMonth() + 1).toString().padStart(2, '0'); // Months are zero-indexed, so add 1
  const day = date.getDate().toString().padStart(2, '0');

  return `${year}-${month}-${day}`;
}

export function kebabCaseToTitleCase(str: string): string {
  return str
    .split('-')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join(' ');
}


export function confirmAnd(action: () => void, message: string) {
  if (window.confirm(message)) {
    action();
  }
}

export function stringsToOptions(strings: string[]): Option[] {
  return strings.map((string) => ({
    label: capitalizeFirstLetter(string),
    value: string,
  }));
}

export const downloadCsv = (blob: BlobPart, filename: string) => {
  const url = window.URL.createObjectURL(new Blob([blob]));
  const link = document.createElement('a');
  link.href = url;
  link.setAttribute('download', filename);
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
};

export const defaultFrom = new Date(new Date().getFullYear(), 0, 1, 0, 0, 0, 0);
export const defaultTo = new Date(new Date().getFullYear(), 11, 31, 23, 59, 59, 999);
  