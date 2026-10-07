import { Trash2 as TrashIcon } from "lucide-react";
import { toast } from "sonner";
import { DropdownMenuItem } from "../ui/dropdown-menu";
import { useDeleteRole } from "@/lib/api";
import { describeError } from "@/lib/utils";
import { RoleStats } from "@/common/types";

/// Only roles nobody holds or has held can be deleted; the backend enforces
/// this too, so membership history is never lost.
export default function DeleteRoleMenuItem({ role }: { role: RoleStats }) {
  const deleteMutation = useDeleteRole();
  const hasMembers = (role.member_count ?? 0) > 0;
  return (
    <DropdownMenuItem
      className="flex items-center justify-between text-destructive focus:text-destructive"
      disabled={hasMembers || deleteMutation.isPending}
      title={
        hasMembers
          ? "Remove every membership of this role, including expired ones, before deleting it"
          : undefined
      }
      onClick={() => {
        if (
          !confirm(
            `Delete role "${role.name}"? This will remove it from Keycloak too.`,
          )
        )
          return;
        deleteMutation.mutate(role.name, {
          onSuccess: () => toast.success(`Role "${role.name}" deleted`),
          onError: (e) =>
            toast.error(`Failed to delete role: ${describeError(e)}`),
        });
      }}
    >
      {hasMembers ? "Delete role (has members)" : "Delete role"}
      <TrashIcon className="w-4 h-4 ml-2" />
    </DropdownMenuItem>
  );
}
