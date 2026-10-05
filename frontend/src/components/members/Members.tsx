import {
  useGetAllMembersWithRoles,
  useGetKeycloakSyncStatus,
  useGetMembersCount,
  useGetRoles,
} from "@/lib/api";
import { stringsToOptions } from "@/lib/utils";
import React from "react";
import { RowSelectionState } from "@tanstack/react-table";
import { useSearchParams } from "react-router";
import { DataTable } from "../ui/data-table";
import { DateRangePicker } from "../ui/date-range-picker";
import MultipleSelector, { Option } from "../ui/multiple-selector";
import { getColumns } from "./columns";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Switch } from "../ui/switch";
import { TooltipProvider } from "../ui/tooltip";
import BulkCommandDock from "./BulkCommandDock";
import MemberDrawer from "./MemberDrawer";
import { MemberWithRoles } from "@/common/types";

type FilterState = {
  roles: Option[];
  // When off, roles are listed regardless of their validity dates.
  dateFilterOn: boolean;
  validFrom: Date;
  validUntil: Date;
};

const startOfToday = () => {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  return d;
};

// "Current roles" means roles valid today. The backend only returns roles
// whose validity covers the whole filter range, so a wider default (such as
// the calendar year) hides every role that starts or ends inside it (#145).
const defaultFilterState = (): FilterState => ({
  roles: [],
  dateFilterOn: true,
  validFrom: startOfToday(),
  validUntil: startOfToday(),
});

const Members: React.FC = () => {
  const { data: roles, isLoading, error } = useGetRoles();
  const { data: syncStatus } = useGetKeycloakSyncStatus();
  const [filterVisible, setFilterVisible] = React.useState(false);
  const [draft, setDraft] = React.useState<FilterState>(defaultFilterState);
  const [applied, setApplied] = React.useState<FilterState>(defaultFilterState);
  const [rowSelection, setRowSelection] = React.useState<RowSelectionState>({});
  const [savedFilter, setSavedFilter] = React.useState<string | null>(null);
  const [searchParams, setSearchParams] = useSearchParams();
  const editUserId = searchParams.get("edit");

  const openDrawer = (member: MemberWithRoles) => {
    setSearchParams({ edit: member.user_id });
  };

  const closeDrawer = () => {
    setSearchParams({});
  };

  const customFilters = {
    roles: applied.roles.map((r) => r.value),
    valid_until: applied.dateFilterOn ? applied.validUntil : undefined,
    valid_from: applied.dateFilterOn ? applied.validFrom : undefined,
  };

  const { data: countData } = useGetMembersCount({ customFilters });

  const columns = React.useMemo(() => getColumns(syncStatus), [syncStatus]);

  const hasPendingChanges =
    JSON.stringify(draft.roles.map((r) => r.value).sort()) !==
      JSON.stringify(applied.roles.map((r) => r.value).sort()) ||
    draft.dateFilterOn !== applied.dateFilterOn ||
    draft.validFrom.getTime() !== applied.validFrom.getTime() ||
    draft.validUntil.getTime() !== applied.validUntil.getTime();

  const selectedIds = Object.entries(rowSelection)
    .filter(([, selected]) => selected)
    .map(([id]) => id);

  const clearSelection = () => setRowSelection({});

  // Changing the filters by hand means the saved filter no longer applies.
  const applyFilters = () => {
    setApplied(draft);
    setSavedFilter(null);
  };

  const resetFilters = () => {
    const next = defaultFilterState();
    setDraft(next);
    setApplied(next);
  };

  const clearAllFilters = () => {
    resetFilters();
    setSavedFilter(null);
  };

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  return (
    <TooltipProvider>
      <div className="space-y-4">
        {/* Heading + count badge */}
        <div className="flex items-center gap-3">
          <h1 className="text-4xl font-bold tracking-tight">Members</h1>
          <Badge
            variant="secondary"
            className="text-sm font-semibold rounded-full px-3"
          >
            {countData?.total ?? "—"}
          </Badge>
        </div>

        <DataTable
          modelName="members"
          columns={columns}
          useFetchData={useGetAllMembersWithRoles}
          useCount={useGetMembersCount}
          searchColumn="first_name"
          initialColumnVisibility={{
            user_id: false,
            home_municipality: false,
          }}
          filterVisible={filterVisible}
          setFilterVisible={setFilterVisible}
          filterContent={
            <div className="flex items-center gap-4 flex-wrap px-3 py-2.5 bg-muted rounded-lg">
              <div className="flex items-center gap-2">
                <span className="text-xs font-medium text-muted-foreground">
                  Role
                </span>
                <MultipleSelector
                  options={stringsToOptions(roles?.map((r) => r.name) ?? [])}
                  onChange={(rs) => setDraft((d) => ({ ...d, roles: rs }))}
                  value={draft.roles}
                  placeholder="All roles"
                  className="w-48 bg-background"
                />
              </div>

              <div className="flex items-center gap-2">
                <Switch
                  id="members-date-filter"
                  checked={draft.dateFilterOn}
                  onCheckedChange={(on) =>
                    setDraft((d) => ({ ...d, dateFilterOn: on }))
                  }
                />
                <label
                  htmlFor="members-date-filter"
                  className="text-xs font-medium text-muted-foreground"
                >
                  {draft.dateFilterOn ? "Valid" : "Any validity"}
                </label>
                {draft.dateFilterOn && (
                  <DateRangePicker
                    onUpdate={({ range }) =>
                      setDraft((d) => ({
                        ...d,
                        validUntil: range.to ?? range.from,
                        validFrom: range.from,
                      }))
                    }
                    initialDateFrom={draft.validFrom}
                    initialDateTo={draft.validUntil}
                    locale="fi"
                    showCompare={false}
                  />
                )}
              </div>

              <div className="flex items-center gap-2 ml-auto">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={!hasPendingChanges}
                  onClick={applyFilters}
                >
                  Apply filters
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-muted-foreground text-xs"
                  onClick={clearAllFilters}
                >
                  Clear filters
                </Button>
              </div>
            </div>
          }
          customFilters={customFilters}
          setCustomFilters={(filters) => {
            const next: FilterState = {
              roles: stringsToOptions(filters?.roles ?? []),
              // A filter saved with the date filter off has no dates.
              dateFilterOn: Boolean(filters?.valid_from),
              validFrom: filters?.valid_from
                ? new Date(filters.valid_from as string)
                : startOfToday(),
              validUntil: filters?.valid_until
                ? new Date(filters.valid_until as string)
                : startOfToday(),
            };
            setDraft(next);
            setApplied(next);
          }}
          getRowId={(row) => row.user_id}
          rowSelection={rowSelection}
          onRowSelectionChange={setRowSelection}
          enableSavedFilters={true}
          selectedSavedFilter={savedFilter}
          onSelectedSavedFilterChange={setSavedFilter}
          resetCustomFilters={resetFilters}
          onRowClick={openDrawer}
        />

        <BulkCommandDock
          count={selectedIds.length}
          selectedIds={selectedIds}
          onClear={clearSelection}
        />

        {editUserId && (
          <MemberDrawer userId={editUserId} onClose={closeDrawer} />
        )}
      </div>
    </TooltipProvider>
  );
};

export default Members;
