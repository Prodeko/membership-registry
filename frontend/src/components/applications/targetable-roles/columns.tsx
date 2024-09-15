import { ColumnDef } from "@tanstack/react-table";
import {
  CopyIcon,
  DollarSignIcon,
  FileIcon,
  MoreHorizontal,
} from "lucide-react";
import { Link } from "react-router-dom";
import { Button } from "../../ui/button";
import { Checkbox } from "../../ui/checkbox";
import { DataTableColumnHeader } from "../../ui/column-header";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "../../ui/dropdown-menu";
import { ApplicationTargetableRole } from "@/common/types";
import RoleBadge from "@/components/ui/role-badge";

export const columns: ColumnDef<ApplicationTargetableRole>[] = [
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
    accessorKey: "role_name",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Role Name" />
    ),
  },
  {
    accessorKey: "active",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Active" />
    ),
  },
  {
    accessorKey: "valid_until",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Valid Until" />
    ),
    cell: ({ row }) => {
      const role = row.original as ApplicationTargetableRole;
      return role.valid_until
        ? new Date(role.valid_until).toLocaleDateString()
        : "Never";
    },
  },
  {
    accessorKey: "payment_link",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Payment Link" />
    ),
  },
  {
    accessorKey: "optional_roles",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Optional Roles" />
    ),
    cell: ({ row }) => {
      const role = row.original as ApplicationTargetableRole;
      return (
        <div className="flex flex-wrap gap-1">
          {role.optional_roles?.map((roleName) => (
            <RoleBadge key={roleName} role={roleName} />
          ))}
        </div>
      );
    },
  },
  {
    accessorKey: "actions",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Actions" />
    ),
    cell: ({ row }) => {
      const role = row.original as ApplicationTargetableRole;
      return (
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
              onClick={() => {
                navigator.clipboard.writeText(role.role_name);
              }}
              className="flex items-center justify-between"
            >
              Copy role name
              <CopyIcon className="w-4 h-4 ml-2" />
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem className="flex items-center justify-between">
              <Link to={`/roles/${role.role_name}`}>View role</Link>
              <FileIcon className="w-4 h-4 ml-2" />
            </DropdownMenuItem>
            {role.payment_link && (
              <DropdownMenuItem className="flex items-center justify-between">
                {/* TODO fix the url */}
                <a href={`/roles/${role.role_name}`}>View payment link in Stripe</a>
                <DollarSignIcon className="w-4 h-4 ml-2" />
              </DropdownMenuItem>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      );
    },
  },
];
