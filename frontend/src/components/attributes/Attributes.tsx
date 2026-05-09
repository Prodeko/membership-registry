import { useState } from "react";
import { toast } from "sonner";
import {
  AttributeDefinition,
  CreateAttributeDefinition,
  UpdateAttributeDefinition,
} from "@/common/types";
import {
  useCreateAttributeDefinition,
  useDeleteAttributeDefinition,
  useGetAttributeDefinitions,
  useGetAttributesSyncStatus,
  useSyncMissingAttributes,
  useUpdateAttributeDefinition,
} from "@/lib/api";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import { Badge } from "../ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "../ui/table";
import AttributeFormModal from "./AttributeFormModal";

const Attributes = () => {
  const { data: definitions, isLoading } = useGetAttributeDefinitions();
  const deleteMutation = useDeleteAttributeDefinition();
  const createMutation = useCreateAttributeDefinition();
  const updateMutation = useUpdateAttributeDefinition();

  const [createOpen, setCreateOpen] = useState(false);
  const [editing, setEditing] = useState<AttributeDefinition | null>(null);

  const handleCreate = (body: CreateAttributeDefinition) => {
    createMutation.mutate(body, {
      onSuccess: () => {
        toast.success(`Attribute ${body.name} created`);
        setCreateOpen(false);
      },
    });
  };

  const handleUpdate = (name: string, data: UpdateAttributeDefinition) => {
    updateMutation.mutate(
      { name, data },
      {
        onSuccess: () => {
          toast.success(`Attribute ${name} updated`);
          setEditing(null);
        },
      },
    );
  };

  const handleDelete = (def: AttributeDefinition) => {
    const confirmed = window.confirm(
      `Delete attribute "${def.name}"? This will also remove all member values for it${def.sync_to_keycloak ? " and remove the mapper from SSO tokens" : ""}.`,
    );
    if (!confirmed) return;
    deleteMutation.mutate(def.name, {
      onSuccess: () => toast.success(`Attribute ${def.name} deleted`),
    });
  };

  return (
    <div className="space-y-4">
      <div className="flex justify-between">
        <h1 className="text-4xl">Attributes</h1>
        <Button onClick={() => setCreateOpen(true)}>New attribute</Button>
      </div>

      <SyncStatusPanel />

      {isLoading ? (
        <div>Loading...</div>
      ) : (
        <Card>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Description</TableHead>
                <TableHead>Allowed values</TableHead>
                <TableHead>SSO</TableHead>
                <TableHead>Editable by</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {definitions?.length === 0 && (
                <TableRow>
                  <TableCell
                    colSpan={6}
                    className="text-center text-muted-foreground"
                  >
                    No attributes yet
                  </TableCell>
                </TableRow>
              )}
              {definitions?.map((def) => (
                <TableRow key={def.name}>
                  <TableCell className="font-mono">{def.name}</TableCell>
                  <TableCell>{def.description ?? ""}</TableCell>
                  <TableCell>
                    {def.allowed_values && def.allowed_values.length > 0 ? (
                      <div className="flex flex-wrap gap-1">
                        {def.allowed_values.map((v) => (
                          <Badge key={v} variant="secondary">
                            {v}
                          </Badge>
                        ))}
                      </div>
                    ) : (
                      <span className="text-muted-foreground">free text</span>
                    )}
                  </TableCell>
                  <TableCell>
                    <Badge
                      variant={def.sync_to_keycloak ? "default" : "outline"}
                    >
                      {def.sync_to_keycloak ? "SSO" : "Internal"}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline">{def.editable_by}</Badge>
                  </TableCell>
                  <TableCell className="text-right space-x-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setEditing(def)}
                    >
                      Edit
                    </Button>
                    <Button
                      variant="destructive"
                      size="sm"
                      onClick={() => handleDelete(def)}
                      disabled={deleteMutation.isPending}
                    >
                      Delete
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      )}

      <AttributeFormModal
        open={createOpen}
        mode="create"
        onClose={() => setCreateOpen(false)}
        onSubmit={(body) => handleCreate(body as CreateAttributeDefinition)}
        submitting={createMutation.isPending}
      />

      {editing && (
        <AttributeFormModal
          open={!!editing}
          mode="edit"
          initial={editing}
          onClose={() => setEditing(null)}
          onSubmit={(body) =>
            handleUpdate(editing.name, body as UpdateAttributeDefinition)
          }
          submitting={updateMutation.isPending}
        />
      )}
    </div>
  );
};

const SyncStatusPanel = () => {
  const { data: status, isLoading } = useGetAttributesSyncStatus();
  const syncMutation = useSyncMissingAttributes();

  if (isLoading) return null;
  if (!status) return null;

  const counts = status.entries.reduce(
    (acc, e) => {
      acc[e.type] += 1;
      return acc;
    },
    { registry_only: 0, keycloak_only: 0, value_mismatch: 0 },
  );
  const drift = status.entries.length;
  const inSync = drift === 0;

  const handleSync = () => {
    syncMutation.mutate(undefined, {
      onSuccess: (summary) => {
        toast.success(
          `Pushed ${summary.applied} attribute value(s) to Keycloak${summary.failed > 0 ? `, ${summary.failed} failed` : ""}`,
        );
      },
    });
  };

  return (
    <Card className="p-4 flex items-center justify-between">
      <div className="space-x-3">
        <Badge variant={inSync ? "default" : "destructive"}>
          {inSync ? "In sync" : `${drift} differences`}
        </Badge>
        {!inSync && (
          <span className="text-sm text-muted-foreground">
            Registry-only: {counts.registry_only} · Value mismatch:{" "}
            {counts.value_mismatch} · Keycloak-only: {counts.keycloak_only}
          </span>
        )}
      </div>
      <Button
        variant="outline"
        onClick={handleSync}
        disabled={syncMutation.isPending || inSync}
      >
        {syncMutation.isPending ? "Syncing..." : "Sync now"}
      </Button>
    </Card>
  );
};

export default Attributes;
