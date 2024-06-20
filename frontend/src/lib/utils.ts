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