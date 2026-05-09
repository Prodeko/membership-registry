import { useEffect, useState } from "react";
import {
  AttributeDefinition,
  CreateAttributeDefinition,
  EditableBy,
  UpdateAttributeDefinition,
} from "@/common/types";
import { Button } from "../ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../ui/dialog";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Textarea } from "../ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import { Switch } from "../ui/switch";

type Mode = "create" | "edit";

interface Props {
  open: boolean;
  mode: Mode;
  initial?: AttributeDefinition;
  onClose: () => void;
  onSubmit: (
    body: CreateAttributeDefinition | UpdateAttributeDefinition,
  ) => void;
  submitting: boolean;
}

const NAME_RE = /^[a-z0-9]+(-[a-z0-9]+)*$/;

const AttributeFormModal = ({
  open,
  mode,
  initial,
  onClose,
  onSubmit,
  submitting,
}: Props) => {
  const [name, setName] = useState(initial?.name ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [allowedValuesText, setAllowedValuesText] = useState(
    initial?.allowed_values?.join(", ") ?? "",
  );
  const [syncToKeycloak, setSyncToKeycloak] = useState(
    initial?.sync_to_keycloak ?? true,
  );
  const [editableBy, setEditableBy] = useState<EditableBy>(
    initial?.editable_by ?? "admin",
  );
  const [nameError, setNameError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setName(initial?.name ?? "");
      setDescription(initial?.description ?? "");
      setAllowedValuesText(initial?.allowed_values?.join(", ") ?? "");
      setSyncToKeycloak(initial?.sync_to_keycloak ?? true);
      setEditableBy(initial?.editable_by ?? "admin");
      setNameError(null);
    }
  }, [open, initial]);

  const handleSubmit = () => {
    const allowed_values = allowedValuesText
      .split(",")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);

    if (mode === "create") {
      if (!NAME_RE.test(name)) {
        setNameError(
          "Use lowercase letters, digits and hyphens only. No leading/trailing hyphen.",
        );
        return;
      }
      onSubmit({
        name,
        description: description || null,
        allowed_values: allowed_values.length > 0 ? allowed_values : null,
        sync_to_keycloak: syncToKeycloak,
        editable_by: editableBy,
      } satisfies CreateAttributeDefinition);
    } else {
      onSubmit({
        description: description || null,
        allowed_values: allowed_values.length > 0 ? allowed_values : null,
        sync_to_keycloak: syncToKeycloak,
        editable_by: editableBy,
      } satisfies UpdateAttributeDefinition);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>
            {mode === "create" ? "New attribute" : `Edit ${initial?.name}`}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="attr-name">Name</Label>
            <Input
              id="attr-name"
              placeholder="xq-year"
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                setNameError(null);
              }}
              disabled={mode === "edit"}
            />
            {mode === "create" && (
              <p className="text-xs text-muted-foreground">
                Lowercase letters, digits, and hyphens. Becomes the JWT claim
                name.
              </p>
            )}
            {nameError && (
              <p className="text-xs text-destructive">{nameError}</p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="attr-description">Description</Label>
            <Textarea
              id="attr-description"
              placeholder="Year of study"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="attr-allowed">Allowed values</Label>
            <Input
              id="attr-allowed"
              placeholder="I, II, III, IV, V"
              value={allowedValuesText}
              onChange={(e) => setAllowedValuesText(e.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              Comma-separated. Leave empty for free text.
            </p>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <Label htmlFor="attr-sync">Sync to Keycloak</Label>
              <p className="text-xs text-muted-foreground">
                When enabled, value flows to consumer apps via JWT claim.
              </p>
            </div>
            <Switch
              id="attr-sync"
              checked={syncToKeycloak}
              onCheckedChange={setSyncToKeycloak}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="attr-editable">Editable by</Label>
            <Select
              value={editableBy}
              onValueChange={(v) => setEditableBy(v as EditableBy)}
            >
              <SelectTrigger id="attr-editable">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="admin">Admin only</SelectItem>
                <SelectItem value="user">User only</SelectItem>
                <SelectItem value="both">Admin and user</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={submitting}>
            {submitting ? "Saving..." : mode === "create" ? "Create" : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default AttributeFormModal;
