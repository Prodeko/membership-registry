import { MemberWithRoles } from "@/common/types";
import { ColumnDef } from "@tanstack/react-table";
import { DataTableColumnHeader } from "../ui/column-header";

export const columns: ColumnDef<MemberWithRoles>[] = [
  {
    accessorKey: "user_id",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="User ID" />
    ),
  },
  {
    accessorKey: "first_name",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="First Name" />
    ),
  },
  {
    accessorKey: "last_name",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Last Name" />
    ),
  },
  {
    accessorKey: "email",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Email" />
    ),
  },
  {
    accessorKey: "role_names",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Current roles" />
    ),
    cell: ({ row }) => {
      console.log(row.original.role_names);
      return (
        <div className="flex flex-wrap">
          {row.original.role_names?.map((role) => (
            <span key={role} className="px-2 py-1 bg-gray-200 rounded-full whitespace-nowrap">
              {role}
            </span>
          ))}
        </div>
      );
    }
  },
  {
    accessorKey: "home_municipality",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Home Municipality" />
    ),
  },
  {
    accessorKey: "has_accepted_policies",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Has Accepted Policies" />
    ),
  },
];
