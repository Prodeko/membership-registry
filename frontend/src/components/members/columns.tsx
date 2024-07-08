import { MemberWithRoles } from "@/common/types";
import { ColumnDef } from "@tanstack/react-table";
import { DataTableColumnHeader } from "../ui/column-header";
import { Checkbox } from "@/components/ui/checkbox";
import { DeleteIcon, MoreHorizontal, TrashIcon, UserIcon } from "lucide-react";
import { Button } from "../ui/button";
import { CopyIcon } from "@radix-ui/react-icons";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "../ui/dropdown-menu";
import { QueryKey, useDeleteMember } from "@/lib/api";
import { useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";

export const columns: ColumnDef<MemberWithRoles>[] = [
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
    cell: ({ row }) => {
      return (
        <Link to={`/members/${row.original.user_id}`} className="flex items-center">
          {row.original.first_name}
        </Link>
      );
    }
  },
  {
    accessorKey: "last_name",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Last Name" />
    ),
    cell: ({ row }) => {
      return (
        <Link to={`/members/${row.original.user_id}`} className="flex items-center">
          {row.original.last_name}
        </Link>
      );
    }
  },
  {
    accessorKey: "email",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Email" />
    ),
    cell: ({ row }) => {
      return (
        <Link to={`/members/${row.original.user_id}`} className="flex items-center">
          {row.original.email}
        </Link>
      );
    }
  },
  {
    accessorKey: "role_names",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Current roles" />
    ),
    cell: ({ row }) => {
      return (
        <div className="flex flex-wrap">
          {row.original.role_names?.map((role) => (
            <span
              key={role}
              className="px-2 py-1 bg-gray-200 rounded-full whitespace-nowrap"
            >
              {role}
            </span>
          ))}
        </div>
      );
    },
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
  {
    id: "actions",
    cell: ({ row }) => {
      const member = row.original;
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const { mutate: deleteMember } = useDeleteMember();
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const queryClient = useQueryClient();

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
              onClick={() => navigator.clipboard.writeText(member.user_id)}
              className="flex items-center justify-between"
            >
              Copy user ID
              <CopyIcon className="w-4 h-4 ml-2" />
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem className="flex items-center justify-between">
              View user
              <UserIcon className="w-4 h-4 ml-2" />
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={async () => {
                deleteMember(member.user_id, {
                  onSuccess: () => {
                    queryClient.invalidateQueries({queryKey: [QueryKey.MEMBERS_WITH_ROLES]});
                  },
                })
              }}
              className="flex items-center justify-between"
            >
              Delete user
              <TrashIcon className="w-4 h-4 ml-2" />
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      );
    },
  },
];
