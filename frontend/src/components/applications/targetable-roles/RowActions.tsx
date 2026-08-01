import { ApplicationTargetableRole } from "@/common/types";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { QueryKey, useDeleteTargetableRole } from "@/lib/api";
import { useQueryClient } from "@tanstack/react-query";
import {
  Copy as CopyIcon,
  DollarSign as DollarSignIcon,
  File as FileIcon,
  MoreHorizontal,
  Pencil as PencilIcon,
  Trash as TrashIcon,
} from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";
import EditTargetableRolesModal from "./EditTargetableRolesModal";

const RowActions = ({ role }: { role: ApplicationTargetableRole }) => {
  const { mutate: deleteTargetableRole } = useDeleteTargetableRole();
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState(false);

  return (
    <>
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
          <DropdownMenuItem
            className="flex items-center justify-between"
            onClick={() => setEditing(true)}
          >
            Edit
            <PencilIcon className="w-4 h-4 ml-2" />
          </DropdownMenuItem>
          <DropdownMenuItem className="flex items-center justify-between">
            <Link to={`/roles/${role.role_name}`}>View role</Link>
            <FileIcon className="w-4 h-4 ml-2" />
          </DropdownMenuItem>
          {role.payment_link && (
            <DropdownMenuItem className="flex items-center justify-between">
              {/* TODO fix the url */}
              <a href={`/roles/${role.role_name}`}>
                View payment link in Stripe
              </a>
              <DollarSignIcon className="w-4 h-4 ml-2" />
            </DropdownMenuItem>
          )}
          <DropdownMenuItem
            className="flex items-center justify-between"
            onClick={() => {
              if (
                window.confirm(
                  `Are you sure you want to delete the role "${role.role_name}"?`,
                )
              ) {
                deleteTargetableRole(
                  { ...role },
                  {
                    onSuccess: () => {
                      queryClient.invalidateQueries({
                        queryKey: [QueryKey.TARGETABLE_ROLES],
                      });
                    },
                  },
                );
              }
            }}
          >
            Delete role
            <TrashIcon className="w-4 h-4 ml-2" />
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <EditTargetableRolesModal
        role={editing ? role : null}
        onClose={() => setEditing(false)}
      />
    </>
  );
};

export default RowActions;
