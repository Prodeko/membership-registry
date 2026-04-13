import { ApplicationWithMember } from "@/common/types";
import { ColumnDef } from "@tanstack/react-table";
import { Checkbox } from "../ui/checkbox";
import { DataTableColumnHeader } from "../ui/column-header";
import {
  QueryKey,
  useDeleteApplication,
  useSetApplicationStatus,
} from "@/lib/api";
import { useQueryClient } from "@tanstack/react-query";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "../ui/dropdown-menu";
import {
  MoreHorizontal,
  Copy as CopyIcon,
  User as UserIcon,
  Trash as TrashIcon,
  File as FileIcon,
  Check as CheckIcon,
  Ban as BanIcon,
  DollarSign as DollarSignIcon,
} from "lucide-react";
import { Button } from "../ui/button";
import { Link } from "react-router-dom";
import { capitalizeFirstLetter } from "@/lib/utils";

export const columns: ColumnDef<ApplicationWithMember>[] = [
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
    accessorKey: "application_id",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Application ID" />
    ),
  },
  {
    accessorKey: "user_id",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="User ID" />
    ),
  },
  {
    accessorKey: "full_name",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Full name" />
    ),
    cell: ({ row }) => {
      const application = row.original;
      return (
        <Link
          to={`/applications/${application.application_id}`}
          className="flex items-center"
        >
          {application.full_name ?? "N/A"}
        </Link>
      );
    },
  },
  {
    accessorKey: "email",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Email" />
    ),
  },
  {
    accessorKey: "role_name",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Role" />
    ),
  },
  {
    accessorKey: "status",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Status" />
    ),
    cell: ({ row }) => {
      const application = row.original;
      return (
        <span>
          {application.status
            ? capitalizeFirstLetter(application.status)
            : "N/A"}
        </span>
      );
    },
  },
  {
    accessorKey: "valid_until",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Valid until" />
    ),
    cell: ({ row }) => {
      const application = row.original;
      return (
        <span>
          {application.valid_until
            ? new Date(application.valid_until).toLocaleDateString()
            : "N/A"}
        </span>
      );
    },
  },
  {
    accessorKey: "created_at",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Created at" />
    ),
    cell: ({ row }) => {
      const application = row.original;
      return (
        <span>
          {new Date(application.created_at).toLocaleDateString()}
          {" at "}
          {new Date(application.created_at).toLocaleTimeString()}
        </span>
      );
    },
  },
  {
    accessorKey: "actions",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Actions" />
    ),
    cell: ({ row }) => {
      const application = row.original;
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const { mutate: deleteApplication } = useDeleteApplication();
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const { mutate: updateApplicationStatus } = useSetApplicationStatus();
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
              onClick={() => navigator.clipboard.writeText(application.user_id)}
              className="flex items-center justify-between"
            >
              Copy user ID
              <CopyIcon className="w-4 h-4 ml-2" />
            </DropdownMenuItem>
            {application.stripe_payment_id && (
              <DropdownMenuItem className="flex items-center justify-between">
                <Link
                  to={`https://dashboard.stripe.com/payments/${application.stripe_payment_id}`}
                  target="_blank"
                >
                  View payment
                </Link>
                <DollarSignIcon className="w-4 h-4 ml-2" />
              </DropdownMenuItem>
            )}
            {(application.status === "pending" ||
              application.status === "unpaid") && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onClick={async () => {
                    updateApplicationStatus(
                      { id: application.application_id, action: "approve" },
                      {
                        onSuccess: () => {
                          queryClient.invalidateQueries({
                            queryKey: [QueryKey.APPLICATIONS],
                          });
                        },
                      },
                    );
                  }}
                  className="flex items-center justify-between"
                >
                  Approve application
                  <CheckIcon className="w-4 h-4 ml-2" />
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={async () => {
                    updateApplicationStatus(
                      { id: application.application_id, action: "reject" },
                      {
                        onSuccess: () => {
                          queryClient.invalidateQueries({
                            queryKey: [QueryKey.APPLICATIONS],
                          });
                        },
                      },
                    );
                  }}
                  className="flex items-center justify-between"
                >
                  Reject application
                  <BanIcon className="w-4 h-4 ml-2" />
                </DropdownMenuItem>
              </>
            )}
            <DropdownMenuSeparator />
            <DropdownMenuItem className="flex items-center justify-between">
              <Link to={`/applications/${application.application_id}`}>
                View application
              </Link>
              <FileIcon className="w-4 h-4 ml-2" />
            </DropdownMenuItem>
            <DropdownMenuItem className="flex items-center justify-between">
              <Link to={`/members/${application.user_id}`}>View member</Link>
              <UserIcon className="w-4 h-4 ml-2" />
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={async () => {
                deleteApplication(application.application_id, {
                  onSuccess: () => {
                    queryClient.invalidateQueries({
                      queryKey: [QueryKey.APPLICATIONS],
                    });
                  },
                });
              }}
              className="flex items-center justify-between"
            >
              Delete application
              <TrashIcon className="w-4 h-4 ml-2" />
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      );
    },
  },
];
