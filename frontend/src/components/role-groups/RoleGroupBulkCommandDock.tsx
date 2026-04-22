import { Plus, Trash2, X } from "lucide-react";
import { RoleGroup } from "@/common/types";
import AddRolesToGroupsModal from "./AddRolesToGroupsModal";
import DeleteRoleGroupsModal from "./DeleteRoleGroupsModal";

interface Props {
  count: number;
  selectedGroupIds: string[];
  groups: RoleGroup[];
  onClear: () => void;
}

export default function RoleGroupBulkCommandDock({
  count,
  selectedGroupIds,
  groups,
  onClear,
}: Props) {
  if (count === 0) return null;

  const selectedGroups = groups.filter((g) => selectedGroupIds.includes(g.id));
  const selectedNames = selectedGroups.map((g) => g.name);

  return (
    <div
      style={{ animation: "dockIn 0.22s cubic-bezier(0.34,1.4,0.64,1)" }}
      className="fixed bottom-7 left-1/2 -translate-x-1/2 z-50 flex items-center gap-2.5
        bg-[hsl(222,45%,11%)] border border-[hsl(222,40%,18%)] rounded-2xl px-3.5 py-2.5
        shadow-2xl whitespace-nowrap"
    >
      <span className="bg-primary text-primary-foreground rounded-full px-2.5 py-0.5 text-xs font-bold">
        {count}
      </span>
      <span className="text-sm font-medium text-slate-300">
        {count === 1 ? "group selected" : "groups selected"}
      </span>
      <div className="w-px h-5 bg-white/10 mx-0.5" />

      <AddRolesToGroupsModal
        groupIds={selectedGroupIds}
        groups={selectedGroups}
        onClose={onClear}
        trigger={
          <button
            type="button"
            className="inline-flex items-center gap-1.5 bg-[hsl(218,80%,30%)] text-[hsl(210,60%,90%)]
              border border-[hsl(218,60%,38%)] rounded-lg px-3 py-1.5 text-sm font-medium cursor-pointer
              hover:bg-[hsl(218,80%,35%)] transition-colors"
          >
            <Plus className="h-3.5 w-3.5" /> Add roles
          </button>
        }
      />

      <DeleteRoleGroupsModal
        groupIds={selectedGroupIds}
        groupNames={selectedNames}
        onClose={onClear}
        trigger={
          <button
            type="button"
            className="inline-flex items-center gap-1.5 bg-red-500/15 text-red-300
              border border-red-500/25 rounded-lg px-3 py-1.5 text-sm font-medium cursor-pointer
              hover:bg-red-500/25 transition-colors"
          >
            <Trash2 className="h-3.5 w-3.5" /> Delete
          </button>
        }
      />

      <button
        type="button"
        onClick={onClear}
        className="text-slate-500 hover:text-slate-300 transition-colors px-1 ml-1"
        aria-label="Clear selection"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}
