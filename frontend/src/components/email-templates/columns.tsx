import { ColumnDef } from "@tanstack/react-table";
import { MoreHorizontal, TrashIcon, PencilIcon } from "lucide-react";
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
import { EmailTemplate } from "@/common/types";
import { QueryKey, useDeleteEmailTemplate } from "@/lib/api";
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import EmailTemplateFormModal from "./EmailTemplateFormModal";

export const columns: ColumnDef<EmailTemplate>[] = [
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
    accessorKey: "name",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Name" />
    ),
  },
  {
    accessorKey: "subject",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Subject" />
    ),
  },
  {
    accessorKey: "actions",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Actions" />
    ),
    cell: ({ row }) => {
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const { mutate: deleteTemplate } = useDeleteEmailTemplate();
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const queryClient = useQueryClient();
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const [editOpen, setEditOpen] = useState(false);
      const template = row.original;

      return (
        <>
          <EmailTemplateFormModal
            template={template}
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
                      `Are you sure you want to delete "${template.name}"?`,
                    )
                  ) {
                    deleteTemplate(template.name, {
                      onSuccess: () => {
                        queryClient.invalidateQueries({
                          queryKey: [QueryKey.EMAIL_TEMPLATES],
                        });
                      },
                    });
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
