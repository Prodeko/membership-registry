import { ApplicationTargetableRole } from "@/common/types";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import MultipleSelector, { Option } from "@/components/ui/multiple-selector";
import { Switch } from "@/components/ui/switch";
import {
  QueryKey,
  useGetAttributeDefinitions,
  useUpdateTargetableRole,
} from "@/lib/api";
import { describeError, stringsToOptions } from "@/lib/utils";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { toast } from "sonner";

interface Props {
  role: ApplicationTargetableRole | null;
  onClose: () => void;
}

const EditTargetableRolesModal = ({ role, onClose }: Props) => {
  const [active, setActive] = useState<boolean>(role?.active ?? true);
  const [formAttributes, setFormAttributes] = useState<string[]>(
    role?.form_attributes ?? [],
  );
  const { data: attributeDefs } = useGetAttributeDefinitions();
  const { mutate: updateTargetableRole, isPending } = useUpdateTargetableRole();
  const queryClient = useQueryClient();

  // Reset local state whenever a different row is opened. Without this the
  // dialog would surface the previous row's values for one frame.
  useEffect(() => {
    if (role) {
      setActive(role.active);
      setFormAttributes(role.form_attributes);
    }
  }, [role]);

  if (!role) return null;

  const initialAttrs = role.form_attributes;
  const attrsChanged =
    formAttributes.length !== initialAttrs.length ||
    formAttributes.some((v, i) => v !== initialAttrs[i]);

  const handleSubmit = () => {
    // Send Patch::Leave (omit) for unchanged fields so partial updates are
    // possible and concurrent writers don't clobber each other's changes.
    const body: Parameters<typeof updateTargetableRole>[0] = {
      role_name: role.role_name,
      valid_until: role.valid_until,
    };
    if (active !== role.active) body.active = active;
    if (attrsChanged) body.form_attributes = formAttributes;

    updateTargetableRole(body, {
      onSuccess: () => {
        toast.success(`Updated ${role.role_name}`);
        queryClient.invalidateQueries({
          queryKey: [QueryKey.TARGETABLE_ROLES],
        });
        onClose();
      },
      onError: (e) => {
        toast.error(`Failed to update: ${describeError(e)}`);
      },
    });
  };

  return (
    <Dialog open={!!role} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            Edit {role.role_name} ({role.valid_until})
          </DialogTitle>
          <DialogDescription>
            Role name and valid_until form the primary key and cannot be
            changed. Delete and recreate to rename.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <Label htmlFor="targetable-active">Active</Label>
            <Switch
              id="targetable-active"
              checked={active}
              onCheckedChange={setActive}
            />
          </div>

          <div className="space-y-1">
            <Label>Application form attributes</Label>
            <MultipleSelector
              value={stringsToOptions(formAttributes)}
              options={stringsToOptions(
                attributeDefs?.map((d) => d.name) ?? [],
              )}
              onChange={(selected: Option[]) =>
                setFormAttributes(selected.map((s) => s.value))
              }
              placeholder="Attributes shown on the application form"
            />
            <p className="text-xs text-muted-foreground">
              Order is preserved as the applicant&apos;s field order.
            </p>
          </div>
        </div>

        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline">Cancel</Button>
          </DialogClose>
          <Button onClick={handleSubmit} disabled={isPending}>
            {isPending ? "Saving..." : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default EditTargetableRolesModal;
