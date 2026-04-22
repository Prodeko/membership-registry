import { toast } from "sonner";
import { DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { useDeleteRoleGroup } from "@/lib/api";
import { RoleGroup } from "@/common/types";

export default function DeleteGroupMenuItem({ group }: { group: RoleGroup }) {
  const deleteMutation = useDeleteRoleGroup();
  return (
    <DropdownMenuItem
      className="text-destructive focus:text-destructive"
      onClick={() => {
        if (
          !confirm(
            `Delete role group "${group.name}"? This will remove it from Keycloak too.`,
          )
        )
          return;
        deleteMutation.mutate(group.id, {
          onSuccess: () => toast.success(`Role group "${group.name}" deleted`),
          onError: () => toast.error("Failed to delete role group"),
        });
      }}
      disabled={deleteMutation.isPending}
    >
      Delete
    </DropdownMenuItem>
  );
}
