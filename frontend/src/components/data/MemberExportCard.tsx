import { useState } from "react";
import { useExportAllMembers, useGetAttributeDefinitions } from "@/lib/api";
import { Button } from "../ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../ui/card";
import { Checkbox } from "../ui/checkbox";

// Member fields, in export order. Keys match the backend's
// `MemberExportColumn` and the member import's column names.
const FIELDS: { key: string; label: string }[] = [
  { key: "user_id", label: "User ID" },
  { key: "first_name", label: "First name" },
  { key: "last_name", label: "Last name" },
  { key: "full_name", label: "Full name" },
  { key: "home_municipality", label: "Home municipality" },
  { key: "language", label: "Language" },
  { key: "email_notifications", label: "Email notifications" },
  { key: "email", label: "Email" },
  { key: "role_names", label: "Roles" },
  { key: "groups", label: "Groups" },
];

// The columns the export had before they were selectable.
const DEFAULT_COLUMNS = [
  "user_id",
  "first_name",
  "last_name",
  "full_name",
  "home_municipality",
  "email_notifications",
  "email",
  "role_names",
];

// Columns the member import reads; attributes are added to this.
const IMPORT_FIELDS = [
  "email",
  "first_name",
  "last_name",
  "home_municipality",
  "language",
  "email_notifications",
];

const attributeKey = (name: string) => `attribute:${name}`;

const MemberExportCard = () => {
  const { mutate, isPending } = useExportAllMembers();
  const { data: definitions } = useGetAttributeDefinitions();
  const attributeKeys = (definitions ?? []).map((d) => attributeKey(d.name));
  // The import refuses member-only attributes, so Import format leaves them out.
  const importableAttributeKeys = (definitions ?? [])
    .filter((d) => d.editable_by !== "user")
    .map((d) => attributeKey(d.name));
  const [selected, setSelected] = useState<Set<string>>(
    () => new Set(DEFAULT_COLUMNS),
  );

  const toggle = (key: string, on: boolean) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (on) next.add(key);
      else next.delete(key);
      return next;
    });

  // Export in a fixed order regardless of the order boxes were ticked.
  const columns = [
    ...FIELDS.map((f) => f.key).filter((k) => selected.has(k)),
    ...attributeKeys.filter((k) => selected.has(k)),
  ];

  const option = (key: string, label: string, mono = false) => (
    <label key={key} className="flex items-center gap-2 text-sm">
      <Checkbox
        checked={selected.has(key)}
        onCheckedChange={(c) => toggle(key, c === true)}
      />
      <span className={mono ? "font-mono" : undefined}>{label}</span>
    </label>
  );

  return (
    <Card className="flex flex-col">
      <CardHeader>
        <CardTitle>Members</CardTitle>
        <CardDescription className="text-base">
          Every member, one row per member. Choose the columns to include.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-1 flex-col gap-4 text-base">
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={() => setSelected(new Set(DEFAULT_COLUMNS))}
          >
            Default
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() =>
              setSelected(
                new Set([...IMPORT_FIELDS, ...importableAttributeKeys]),
              )
            }
          >
            Import format
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() =>
              setSelected(
                new Set([...FIELDS.map((f) => f.key), ...attributeKeys]),
              )
            }
          >
            All
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setSelected(new Set())}
          >
            None
          </Button>
        </div>

        <div className="space-y-2">
          <p className="text-sm font-medium">Member fields</p>
          <div className="grid grid-cols-2 gap-2">
            {FIELDS.map((f) => option(f.key, f.label))}
          </div>
        </div>

        {definitions && definitions.length > 0 && (
          <div className="space-y-2">
            <p className="text-sm font-medium">Attributes</p>
            <div className="grid grid-cols-2 gap-2">
              {definitions.map((d) =>
                option(attributeKey(d.name), d.name, true),
              )}
            </div>
          </div>
        )}

        <ul className="list-disc space-y-1 pl-5">
          <li>
            CSV file (comma-separated). Each attribute is its own column, named
            after the attribute; several values are separated with{" "}
            <code>;</code>.
          </li>
          <li>
            <strong>Roles</strong> lists every role the member has had,
            including ended ones; <strong>Groups</strong> lists their role
            groups.
          </li>
          <li>
            <strong>Import format</strong> picks the columns the member import
            reads, so the file can be edited and imported back. Attributes only
            members can edit are left out, as the import can&apos;t set them.
          </li>
        </ul>

        <div className="mt-auto">
          <Button
            onClick={() => mutate(columns)}
            disabled={isPending || columns.length === 0}
          >
            {isPending
              ? "Exporting..."
              : `Export CSV (${columns.length} columns)`}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
};

export default MemberExportCard;
