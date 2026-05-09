import { ColumnDef } from "@tanstack/react-table";
import { Checkbox } from "../../ui/checkbox";
import { DataTableColumnHeader } from "../../ui/column-header";
import { ApplicationTargetableRole } from "@/common/types";
import RoleBadge from "@/components/ui/role-badge";
import RowActions from "./RowActions";

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
    cell: ({ row }) => (
      <RowActions role={row.original as ApplicationTargetableRole} />
    ),
  },
];
