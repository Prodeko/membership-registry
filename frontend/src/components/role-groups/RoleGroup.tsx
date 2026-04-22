import { useState } from "react";
import { useParams, Link } from "react-router-dom";
import {
  useGetRoleGroup,
  useGetRoleGroupMembers,
  useGetRoles,
  useSetRoleGroupRoles,
  useAssignRoleGroup,
  useRemoveRoleGroupAssignment,
  useUpdateRoleGroup,
} from "@/lib/api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

export default function RoleGroup() {
  const { id = "" } = useParams<{ id: string }>();

  const { data: group, isLoading: isGroupLoading } = useGetRoleGroup(id);
  const { data: members, isLoading: isMembersLoading } =
    useGetRoleGroupMembers(id);
  const { data: allRoles } = useGetRoles();

  const updateGroupMutation = useUpdateRoleGroup();
  const setRolesMutation = useSetRoleGroupRoles();
  const assignMutation = useAssignRoleGroup();
  const removeMutation = useRemoveRoleGroupAssignment();

  // Edit group meta
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [editMetaOpen, setEditMetaOpen] = useState(false);

  // Edit roles
  const [editRolesOpen, setEditRolesOpen] = useState(false);
  const [selectedRoles, setSelectedRoles] = useState<string[]>([]);

  // Assign member
  const [assignOpen, setAssignOpen] = useState(false);
  const [assignUserId, setAssignUserId] = useState("");
  const [assignValidFrom, setAssignValidFrom] = useState("");
  const [assignValidUntil, setAssignValidUntil] = useState("");

  if (isGroupLoading || isMembersLoading) return <p>Loading...</p>;
  if (!group) return <p>Group not found</p>;

  const handleOpenEditMeta = () => {
    setEditName(group.name);
    setEditDescription(group.description ?? "");
    setEditMetaOpen(true);
  };

  const handleSaveMeta = (e: React.FormEvent) => {
    e.preventDefault();
    updateGroupMutation.mutate(
      {
        id,
        name: editName.trim(),
        description: editDescription.trim() || undefined,
      },
      {
        onSuccess: () => {
          toast.success("Group updated");
          setEditMetaOpen(false);
        },
        onError: () => toast.error("Failed to update group"),
      },
    );
  };

  const handleOpenEditRoles = () => {
    setSelectedRoles([...group.role_names]);
    setEditRolesOpen(true);
  };

  const toggleRole = (name: string) => {
    setSelectedRoles((prev) =>
      prev.includes(name) ? prev.filter((r) => r !== name) : [...prev, name],
    );
  };

  const handleSaveRoles = (e: React.FormEvent) => {
    e.preventDefault();
    setRolesMutation.mutate(
      { id, role_names: selectedRoles },
      {
        onSuccess: () => {
          toast.success("Roles updated");
          setEditRolesOpen(false);
        },
        onError: () => toast.error("Failed to update roles"),
      },
    );
  };

  const handleAssign = (e: React.FormEvent) => {
    e.preventDefault();
    if (!assignUserId.trim()) return;
    assignMutation.mutate(
      {
        groupId: id,
        userId: assignUserId.trim(),
        validFrom: assignValidFrom || undefined,
        validUntil: assignValidUntil || undefined,
      },
      {
        onSuccess: () => {
          toast.success("Member assigned to group");
          setAssignOpen(false);
          setAssignUserId("");
          setAssignValidFrom("");
          setAssignValidUntil("");
        },
        onError: () => toast.error("Failed to assign member"),
      },
    );
  };

  const handleRemove = (userId: string, validFrom: string) => {
    removeMutation.mutate(
      { groupId: id, userId, validFrom },
      {
        onSuccess: () => toast.success("Assignment removed"),
        onError: () => toast.error("Failed to remove assignment"),
      },
    );
  };

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
            <Link to="/role-groups" className="hover:underline">
              Role Groups
            </Link>
            <span>/</span>
            <span>{group.name}</span>
          </div>
          <h1 className="text-4xl">{group.name}</h1>
          {group.description && (
            <p className="mt-1 text-muted-foreground">{group.description}</p>
          )}
        </div>

        {/* Edit meta dialog */}
        <Dialog open={editMetaOpen} onOpenChange={setEditMetaOpen}>
          <DialogTrigger asChild>
            <Button variant="outline" onClick={handleOpenEditMeta}>
              Edit
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Edit group</DialogTitle>
            </DialogHeader>
            <form onSubmit={handleSaveMeta} className="space-y-4">
              <div className="space-y-1">
                <Label>Name</Label>
                <Input
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  required
                />
              </div>
              <div className="space-y-1">
                <Label>Description</Label>
                <Input
                  value={editDescription}
                  onChange={(e) => setEditDescription(e.target.value)}
                />
              </div>
              <div className="flex justify-end gap-2">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => setEditMetaOpen(false)}
                >
                  Cancel
                </Button>
                <Button type="submit" disabled={updateGroupMutation.isPending}>
                  Save
                </Button>
              </div>
            </form>
          </DialogContent>
        </Dialog>
      </div>

      {/* Roles section */}
      <section>
        <div className="flex items-center justify-between mb-2">
          <h2 className="text-xl font-semibold">Contained roles</h2>
          {/* Edit roles dialog */}
          <Dialog open={editRolesOpen} onOpenChange={setEditRolesOpen}>
            <DialogTrigger asChild>
              <Button variant="outline" size="sm" onClick={handleOpenEditRoles}>
                Edit roles
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Set roles for {group.name}</DialogTitle>
              </DialogHeader>
              <form onSubmit={handleSaveRoles} className="space-y-4">
                <div className="max-h-64 overflow-y-auto space-y-2 border rounded p-2">
                  {allRoles?.map((role) => (
                    <div key={role.name} className="flex items-center gap-2">
                      <Checkbox
                        id={`edit-role-${role.name}`}
                        checked={selectedRoles.includes(role.name)}
                        onCheckedChange={() => toggleRole(role.name)}
                      />
                      <label
                        htmlFor={`edit-role-${role.name}`}
                        className="text-sm cursor-pointer"
                      >
                        {role.name}
                      </label>
                    </div>
                  ))}
                </div>
                <div className="flex justify-end gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => setEditRolesOpen(false)}
                  >
                    Cancel
                  </Button>
                  <Button type="submit" disabled={setRolesMutation.isPending}>
                    Save
                  </Button>
                </div>
              </form>
            </DialogContent>
          </Dialog>
        </div>
        <div className="flex flex-wrap gap-2">
          {group.role_names.length === 0 ? (
            <p className="text-muted-foreground text-sm">No roles assigned.</p>
          ) : (
            group.role_names.map((r) => (
              <Badge key={r} variant="secondary">
                {r}
              </Badge>
            ))
          )}
        </div>
      </section>

      {/* Members section */}
      <section>
        <div className="flex items-center justify-between mb-2">
          <h2 className="text-xl font-semibold">Members</h2>
          {/* Assign member dialog */}
          <Dialog open={assignOpen} onOpenChange={setAssignOpen}>
            <DialogTrigger asChild>
              <Button size="sm">Assign member</Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Assign member to {group.name}</DialogTitle>
              </DialogHeader>
              <form onSubmit={handleAssign} className="space-y-4">
                <div className="space-y-1">
                  <Label>User ID</Label>
                  <Input
                    value={assignUserId}
                    onChange={(e) => setAssignUserId(e.target.value)}
                    placeholder="UUID"
                    required
                  />
                </div>
                <div className="space-y-1">
                  <Label>Valid from (optional)</Label>
                  <Input
                    type="date"
                    value={assignValidFrom}
                    onChange={(e) => setAssignValidFrom(e.target.value)}
                  />
                </div>
                <div className="space-y-1">
                  <Label>Valid until (optional)</Label>
                  <Input
                    type="date"
                    value={assignValidUntil}
                    onChange={(e) => setAssignValidUntil(e.target.value)}
                  />
                </div>
                <div className="flex justify-end gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => setAssignOpen(false)}
                  >
                    Cancel
                  </Button>
                  <Button type="submit" disabled={assignMutation.isPending}>
                    Assign
                  </Button>
                </div>
              </form>
            </DialogContent>
          </Dialog>
        </div>

        {!members || members.length === 0 ? (
          <p className="text-muted-foreground text-sm">No members assigned.</p>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>User ID</TableHead>
                <TableHead>Valid from</TableHead>
                <TableHead>Valid until</TableHead>
                <TableHead />
              </TableRow>
            </TableHeader>
            <TableBody>
              {members.map((m) => (
                <TableRow key={`${m.user_id}-${m.valid_from}`}>
                  <TableCell>
                    <Link
                      to={`/members/${m.user_id}`}
                      className="font-mono text-sm hover:underline"
                    >
                      {m.user_id}
                    </Link>
                  </TableCell>
                  <TableCell>{m.valid_from}</TableCell>
                  <TableCell>{m.valid_until ?? "—"}</TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-destructive hover:text-destructive"
                      onClick={() => handleRemove(m.user_id, m.valid_from)}
                      disabled={removeMutation.isPending}
                    >
                      Remove
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </section>
    </div>
  );
}
