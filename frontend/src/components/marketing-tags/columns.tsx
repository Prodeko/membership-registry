import { ColumnDef } from "@tanstack/react-table";
import {
  MoreHorizontal,
  Trash as TrashIcon,
  Pencil as PencilIcon,
} from "lucide-react";
import { Button } from "../ui/button";
import { Checkbox } from "../ui/checkbox";
import { DataTableColumnHeader } from "../ui/column-header";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "../ui/dropdown-menu";
import { MarketingTag } from "@/common/types";
import { useDeleteMarketingTag } from "@/lib/api";
import { useState } from "react";
import MarketingTagFormModal from "./MarketingTagFormModal";

export const columns: ColumnDef<MarketingTag>[] = [
  {
    id: "select",
    header: ({ table }) => (
      <Checkbox
        checked={
          table.getIsAllPageRowsSelected() ||
          (table.getIsSomePageRowsSelected() && "indeterminate")
        }
        onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        aria-label="Select all"
      />
    ),
    cell: ({ row }) => (
      <Checkbox
        checked={row.getIsSelected()}
        onCheckedChange={(value) => row.toggleSelected(!!value)}
        aria-label="Select row"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: "label",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Label (Mailchimp key)" />
    ),
  },
  {
    accessorKey: "name_en",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Name (EN)" />
    ),
  },
  {
    accessorKey: "name_fi",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Name (FI)" />
    ),
  },
  {
    accessorKey: "display_order",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Order" />
    ),
  },
  {
    accessorKey: "auto_apply",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Auto-apply" />
    ),
    cell: ({ row }) => (row.original.auto_apply ? "Yes" : "No"),
  },
  {
    accessorKey: "actions",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Actions" />
    ),
    cell: ({ row }) => {
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const { mutate: deleteTag } = useDeleteMarketingTag();
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const [editOpen, setEditOpen] = useState(false);
      const tag = row.original;

      return (
        <>
          <MarketingTagFormModal
            tag={tag}
            open={editOpen}
            onOpenChange={setEditOpen}
          />
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" className="h-8 w-8 p-0">
                <span className="sr-only">Open menu</span>
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuLabel>Actions</DropdownMenuLabel>
              <DropdownMenuItem
                className="flex items-center justify-between"
                onClick={() => setEditOpen(true)}
              >
                Edit
                <PencilIcon className="w-4 h-4 ml-2" />
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                className="flex items-center justify-between"
                onClick={() => {
                  if (
                    window.confirm(
                      `Delete marketing tag "${tag.label}"? This only removes it from the user-editable catalog; existing Mailchimp subscribers keep the tag.`,
                    )
                  ) {
                    deleteTag(tag.label);
                  }
                }}
              >
                Delete
                <TrashIcon className="w-4 h-4 ml-2" />
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </>
      );
    },
  },
];
