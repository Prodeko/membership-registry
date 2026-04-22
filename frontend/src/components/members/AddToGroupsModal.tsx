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
import { useAssignRoleGroup, useGetRoleGroups } from "@/lib/api";

interface Props {
  userIds: string[];
  onClose: () => void;
  trigger: React.ReactNode;
}

export default function AddToGroupsModal({ userIds, onClose, trigger }: Props) {
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const { data: groups, isLoading } = useGetRoleGroups();
  const assign = useAssignRoleGroup();

  const toggle = (id: string) =>
    setSelected((prev) =>
      prev.includes(id) ? prev.filter((g) => g !== id) : [...prev, id],
    );

  const handleSubmit = async () => {
    if (selected.length === 0) return;
    const today = new Date().toISOString().split("T")[0];
    const pairs = selected.flatMap((groupId) =>
      userIds.map((userId) => ({ groupId, userId, validFrom: today })),
    );
    try {
      await Promise.all(pairs.map((p) => assign.mutateAsync(p)));
      toast.success(
        `Added ${userIds.length} member(s) to ${selected.length} group(s)`,
      );
      setSelected([]);
      setOpen(false);
      onClose();
    } catch {
      toast.error("Failed to assign some members to groups");
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
          <DialogTitle>Add {userIds.length} member(s) to groups</DialogTitle>
        </DialogHeader>
        {isLoading ? (
          <p className="text-sm text-muted-foreground">Loading groups…</p>
        ) : !groups || groups.length === 0 ? (
          <p className="text-sm text-muted-foreground">No groups available.</p>
        ) : (
          <div className="space-y-2 max-h-64 overflow-y-auto py-1">
            {groups.map((group) => (
              <label
                key={group.id}
                className="flex items-center gap-2 cursor-pointer select-none"
              >
                <Checkbox
                  checked={selected.includes(group.id)}
                  onCheckedChange={() => toggle(group.id)}
                />
                <span className="text-sm">{group.name}</span>
              </label>
            ))}
          </div>
        )}
        <DialogFooter>
          <Button
            onClick={handleSubmit}
            disabled={selected.length === 0 || assign.isPending}
          >
            Add to groups
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
