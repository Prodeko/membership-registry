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
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";
import { cn } from "@/lib/utils";

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
      <div onClick={(e) => e.stopPropagation()}>
        <Checkbox
          checked={row.getIsSelected()}
          onCheckedChange={(value) => row.toggleSelected(!!value)}
          aria-label="Select row"
        />
      </div>
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
    cell: ({ row }) => <span className="font-medium">{row.original.first_name}</span>,
  },
  {
    accessorKey: "last_name",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Last Name" />
    ),
    cell: ({ row }) => <span className="font-medium">{row.original.last_name}</span>,
  },
  {
    accessorKey: "email",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Email" />
    ),
    cell: ({ row }) => (
      <span className="text-muted-foreground text-sm">{row.original.email}</span>
    ),
  },
  {
    accessorKey: "group_names",
    header: "Groups",
    cell: ({ row }) => {
      const groups = row.original.group_names ?? [];
      return (
        <div className="flex flex-wrap gap-1">
          {groups.length === 0 ? (
            <span className="text-xs text-muted-foreground">—</span>
          ) : (
            groups.map((g) => (
              <Badge key={g} variant="secondary" className="text-xs font-medium">
                {g}
              </Badge>
            ))
          )}
        </div>
      );
    },
    enableSorting: false,
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
    header: () => <span className="text-xs text-muted-foreground">KC</span>,
    cell: ({ row }) => {
      if (!syncStatus) {
        return (
          <span className="inline-block w-2.5 h-2.5 rounded-full bg-muted-foreground/30" />
        );
      }
      const status = syncStatus.statuses[row.original.user_id];
      const isSync = !status;
      const dotColor = isSync ? "bg-green-500" : "bg-red-500";
      const ringColor = isSync ? "ring-green-200" : "ring-red-200";
      const label = isSync ? "In sync" : "Mismatch";
      const detailParts = status
        ? [
            status.missing_in_keycloak.length > 0 &&
              `Missing in KC: ${status.missing_in_keycloak.join(", ")}`,
            status.expired_in_keycloak.length > 0 &&
              `Expired in KC: ${status.expired_in_keycloak.join(", ")}`,
            status.unmanaged_in_keycloak.length > 0 &&
              `Unmanaged: ${status.unmanaged_in_keycloak.join(", ")}`,
          ].filter(Boolean)
        : [];

      return (
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="inline-flex items-center justify-center w-5 h-5 cursor-default">
              <span
                className={cn(
                  "w-2.5 h-2.5 rounded-full ring-2 ring-offset-1",
                  dotColor,
                  ringColor,
                )}
              />
            </span>
          </TooltipTrigger>
          <TooltipContent side="bottom" className="max-w-xs text-xs">
            <p className="font-medium">{label}</p>
            {detailParts.length > 0 && (
              <p className="text-muted-foreground mt-0.5">
                {detailParts.join(" · ")}
              </p>
            )}
          </TooltipContent>
        </Tooltip>
      );
    },
    enableSorting: false,
    size: 48,
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
          <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
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
