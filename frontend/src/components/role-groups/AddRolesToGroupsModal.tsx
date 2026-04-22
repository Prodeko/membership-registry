import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { useGetRoles, useSetRoleGroupRoles } from "@/lib/api";
import { RoleGroup } from "@/common/types";

interface Props {
  groupIds: string[];
  groups: RoleGroup[];
  onClose: () => void;
  trigger: React.ReactNode;
}

export default function AddRolesToGroupsModal({
  groupIds,
  groups,
  onClose,
  trigger,
}: Props) {
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const { data: roles, isLoading } = useGetRoles();
  const setRoles = useSetRoleGroupRoles();

  const toggle = (name: string) =>
    setSelected((prev) =>
      prev.includes(name) ? prev.filter((r) => r !== name) : [...prev, name],
    );

  const handleSubmit = async () => {
    if (selected.length === 0) return;
    const targets = groups.filter((g) => groupIds.includes(g.id));
    try {
      await Promise.all(
        targets.map((g) => {
          const merged = Array.from(new Set([...g.role_names, ...selected]));
          return setRoles.mutateAsync({ id: g.id, role_names: merged });
        }),
      );
      toast.success(
        `Added ${selected.length} role(s) to ${targets.length} group(s)`,
      );
      setSelected([]);
      setOpen(false);
      onClose();
    } catch {
      toast.error("Failed to add roles to some groups");
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(v) => {
        setOpen(v);
        if (!v) setSelected([]);
      }}
    >
      <DialogTrigger asChild>{trigger}</DialogTrigger>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle>Add roles to {groupIds.length} group(s)</DialogTitle>
        </DialogHeader>
        {isLoading ? (
          <p className="text-sm text-muted-foreground">Loading roles…</p>
        ) : (
          <div className="space-y-2 max-h-64 overflow-y-auto py-1">
            {roles?.map((role) => (
              <label
                key={role.name}
                className="flex items-center gap-2 cursor-pointer select-none"
              >
                <Checkbox
                  checked={selected.includes(role.name)}
                  onCheckedChange={() => toggle(role.name)}
                />
                <span className="text-sm">{role.name}</span>
              </label>
            ))}
          </div>
        )}
        <DialogFooter>
          <Button
            onClick={handleSubmit}
            disabled={selected.length === 0 || setRoles.isPending}
          >
            Add roles
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
