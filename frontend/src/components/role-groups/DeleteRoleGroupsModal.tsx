import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { useDeleteRoleGroup } from "@/lib/api";

interface Props {
  groupIds: string[];
  groupNames: string[];
  onClose: () => void;
  trigger: React.ReactNode;
}

export default function DeleteRoleGroupsModal({
  groupIds,
  groupNames,
  onClose,
  trigger,
}: Props) {
  const [open, setOpen] = useState(false);
  const deleteMutation = useDeleteRoleGroup();

  const handleDelete = async () => {
    try {
      await Promise.all(groupIds.map((id) => deleteMutation.mutateAsync(id)));
      toast.success(`Deleted ${groupIds.length} group(s)`);
      setOpen(false);
      onClose();
    } catch {
      toast.error("Failed to delete some groups");
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>{trigger}</DialogTrigger>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle>Delete {groupIds.length} group(s)?</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          This will permanently delete the following groups and remove them from
          Keycloak:
        </p>
        <ul className="text-sm space-y-1 pl-4 list-disc">
          {groupNames.map((n) => (
            <li key={n}>{n}</li>
          ))}
        </ul>
        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={handleDelete}
            disabled={deleteMutation.isPending}
          >
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
