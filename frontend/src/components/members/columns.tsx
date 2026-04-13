import { KeycloakSyncStatusMap, MemberWithRoles } from "@/common/types";
import { Checkbox } from "@/components/ui/checkbox";
import { QueryKey, useDeleteMember } from "@/lib/api";
import { CopyIcon } from "@radix-ui/react-icons";
import { useQueryClient } from "@tanstack/react-query";
import { ColumnDef } from "@tanstack/react-table";
import {
  MoreHorizontal,
  Trash as TrashIcon,
  User as UserIcon,
} from "lucide-react";
import { Link } from "react-router-dom";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { DataTableColumnHeader } from "../ui/column-header";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "../ui/dropdown-menu";
import RoleBadge from "../ui/role-badge";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "../ui/tooltip";

export const getColumns = (
  syncStatus?: KeycloakSyncStatusMap,
): ColumnDef<MemberWithRoles>[] => [
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
        <Link
          to={`/members/${row.original.user_id}`}
          className="flex items-center"
        >
          {row.original.first_name}
        </Link>
      );
    },
  },
  {
    accessorKey: "last_name",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Last Name" />
    ),
    cell: ({ row }) => {
      return (
        <Link
          to={`/members/${row.original.user_id}`}
          className="flex items-center"
        >
          {row.original.last_name}
        </Link>
      );
    },
  },
  {
    accessorKey: "email",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Email" />
    ),
    cell: ({ row }) => {
      return (
        <Link
          to={`/members/${row.original.user_id}`}
          className="flex items-center"
        >
          {row.original.email}
        </Link>
      );
    },
  },
  {
    accessorKey: "role_names",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Current roles" />
    ),
    cell: ({ row }) => {
      return (
        <div className="flex flex-wrap">
          {row.original.role_names?.map((role) =>
            role ? <RoleBadge key={role} role={role} /> : null,
          )}
        </div>
      );
    },
  },
  {
    id: "kc_sync",
    header: () => <span className="text-xs">KC Status</span>,
    cell: ({ row }) => {
      if (!syncStatus) {
        return <span className="text-xs text-muted-foreground">...</span>;
      }
      const status = syncStatus.statuses[row.original.user_id];
      if (!status) {
        return (
          <Badge
            variant="outline"
            className="text-xs bg-green-50 text-green-700 border-green-200"
          >
            In sync
          </Badge>
        );
      }
      return (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger className="cursor-help">
              <Badge
                variant="outline"
                className="text-xs bg-red-50 text-red-700 border-red-200"
              >
                Mismatch
              </Badge>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="max-w-xs">
              <div className="space-y-1 text-xs">
                {status.extra_in_keycloak.length > 0 && (
                  <div>
                    <span className="font-medium">Extra in KC:</span>{" "}
                    {status.extra_in_keycloak.join(", ")}
                  </div>
                )}
                {status.missing_in_keycloak.length > 0 && (
                  <div>
                    <span className="font-medium">Missing in KC:</span>{" "}
                    {status.missing_in_keycloak.join(", ")}
                  </div>
                )}
              </div>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      );
    },
    enableSorting: false,
  },
  {
    accessorKey: "home_municipality",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Home Municipality" />
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
            <DropdownMenuItem asChild>
              <Link
                to={`/members/${member.user_id}`}
                className="flex items-center justify-between w-full"
              >
                View user
                <UserIcon className="w-4 h-4 ml-2" />
              </Link>
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={async () => {
                deleteMember(member.user_id, {
                  onSuccess: () => {
                    queryClient.invalidateQueries({
                      queryKey: [QueryKey.MEMBERS_WITH_ROLES],
                    });
                  },
                });
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
