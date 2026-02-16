import { AuditLogEntryWithActor } from "@/common/types";
import { ColumnDef } from "@tanstack/react-table";
import { DataTableColumnHeader } from "../ui/column-header";

export const columns: ColumnDef<AuditLogEntryWithActor>[] = [
  {
    accessorKey: "created_at",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Timestamp" />
    ),
    cell: ({ row }) => {
      const date = new Date(row.original.created_at);
      return (
        <span className="whitespace-nowrap text-sm">
          {date.toLocaleString()}
        </span>
      );
    },
  },
  {
    accessorKey: "actor_name",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Actor" />
    ),
    cell: ({ row }) => {
      return (
        <span className="text-sm">{row.original.actor_name ?? "System"}</span>
      );
    },
  },
  {
    accessorKey: "action",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Action" />
    ),
    cell: ({ row }) => {
      return <code className="text-sm">{row.original.action}</code>;
    },
  },
  {
    accessorKey: "entity_type",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Entity Type" />
    ),
    cell: ({ row }) => {
      return <span className="text-sm">{row.original.entity_type}</span>;
    },
  },
  {
    accessorKey: "entity_id",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Entity ID" />
    ),
    cell: ({ row }) => {
      return (
        <span className="text-sm font-mono break-all">
          {row.original.entity_id}
        </span>
      );
    },
  },
  {
    accessorKey: "details",
    header: ({ column }) => (
      <DataTableColumnHeader column={column} title="Details" />
    ),
    cell: ({ row }) => {
      const details = row.original.details;
      if (!details) return <span className="text-muted-foreground">-</span>;
      return (
        <pre className="text-xs whitespace-pre-wrap break-all">
          {JSON.stringify(details, null, 2)}
        </pre>
      );
    },
    enableSorting: false,
  },
];
