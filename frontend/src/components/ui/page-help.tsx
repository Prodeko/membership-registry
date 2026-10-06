import { ChevronDown, Info } from "lucide-react";
import { ReactNode, useState } from "react";
import { cn } from "@/lib/utils";

interface Props {
  /** Remembers per page whether the viewer closed the box. */
  id: string;
  title?: string;
  children: ReactNode;
}

const storageKey = (id: string) => `page-help:${id}`;

// Open until the viewer closes it, so a new admin sees how the page works.
// Storage can be unavailable (private windows); then it just starts open.
const readOpen = (id: string) => {
  try {
    return localStorage.getItem(storageKey(id)) !== "closed";
  } catch {
    return true;
  }
};

/** A collapsible box explaining how an admin page works. */
const PageHelp = ({ id, title = "How this page works", children }: Props) => {
  const [open, setOpen] = useState(() => readOpen(id));

  const toggle = () => {
    const next = !open;
    setOpen(next);
    try {
      localStorage.setItem(storageKey(id), next ? "open" : "closed");
    } catch {
      // Not remembered; the box still toggles.
    }
  };

  return (
    <div className="rounded-md border bg-muted/40 text-sm text-muted-foreground">
      <button
        type="button"
        onClick={toggle}
        aria-expanded={open}
        className="flex w-full items-center gap-2 px-4 py-3 text-left font-medium text-foreground"
      >
        <Info className="h-4 w-4 shrink-0" />
        <span className="flex-1">{title}</span>
        <ChevronDown
          className={cn("h-4 w-4 transition-transform", open && "rotate-180")}
        />
      </button>
      {open && (
        <div className="space-y-2 px-4 pb-4 [&_ul]:list-disc [&_ul]:space-y-1 [&_ul]:pl-5">
          {children}
        </div>
      )}
    </div>
  );
};

export default PageHelp;
