import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCreateRoleGroup, useGetRoles } from "@/lib/api";
import { toast } from "sonner";
import { Checkbox } from "@/components/ui/checkbox";

export default function CreateRoleGroupModal() {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [selectedRoles, setSelectedRoles] = useState<string[]>([]);

  const { data: roles } = useGetRoles();
  const createMutation = useCreateRoleGroup();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    createMutation.mutate(
      {
        name: name.trim(),
        description: description.trim() || undefined,
        role_names: selectedRoles,
      },
      {
        onSuccess: () => {
          toast.success(`Role group "${name}" created`);
          setOpen(false);
          setName("");
          setDescription("");
          setSelectedRoles([]);
        },
        onError: () => {
          toast.error("Failed to create role group");
        },
      },
    );
  };

  const toggleRole = (roleName: string) => {
    setSelectedRoles((prev) =>
      prev.includes(roleName)
        ? prev.filter((r) => r !== roleName)
        : [...prev, roleName],
    );
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>Create group</Button>
      </DialogTrigger>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Create role group</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-1">
            <Label htmlFor="group-name">Name</Label>
            <Input
              id="group-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. hallitus"
              required
            />
          </div>
          <div className="space-y-1">
            <Label htmlFor="group-description">Description (optional)</Label>
            <Input
              id="group-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Brief description"
            />
          </div>
          {roles && roles.length > 0 && (
            <div className="space-y-2">
              <Label>Roles</Label>
              <div className="max-h-48 overflow-y-auto space-y-2 border rounded p-2">
                {roles.map((role) => (
                  <div key={role.name} className="flex items-center gap-2">
                    <Checkbox
                      id={`role-${role.name}`}
                      checked={selectedRoles.includes(role.name)}
                      onCheckedChange={() => toggleRole(role.name)}
                    />
                    <label
                      htmlFor={`role-${role.name}`}
                      className="text-sm cursor-pointer"
                    >
                      {role.name}
                    </label>
                  </div>
                ))}
              </div>
            </div>
          )}
          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => setOpen(false)}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={createMutation.isPending || !name.trim()}
            >
              {createMutation.isPending ? "Creating..." : "Create"}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
