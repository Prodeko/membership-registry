import {
  useGetAllMembersWithRoles,
  useGetKeycloakSyncStatus,
  useGetMembersCount,
  useGetRoles,
} from "@/lib/api";
import { defaultFrom, defaultTo, stringsToOptions } from "@/lib/utils";
import React from "react";
import { RowSelectionState } from "@tanstack/react-table";
import { useSearchParams } from "react-router-dom";
import { DataTable } from "../ui/data-table";
import { DateRangePicker } from "../ui/date-range-picker";
import MultipleSelector, { Option } from "../ui/multiple-selector";
import { getColumns } from "./columns";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { TooltipProvider } from "../ui/tooltip";
import BulkCommandDock from "./BulkCommandDock";
import MemberDrawer from "./MemberDrawer";
import { MemberWithRoles } from "@/common/types";

type FilterState = {
  roles: Option[];
  validFrom: Date;
  validUntil: Date;
};

const defaultFilterState: FilterState = {
  roles: [],
  validFrom: defaultFrom,
  validUntil: defaultTo,
};

const Members: React.FC = () => {
  const { data: roles, isLoading, error } = useGetRoles();
  const { data: syncStatus } = useGetKeycloakSyncStatus();
  const [filterVisible, setFilterVisible] = React.useState(false);
  const [draft, setDraft] = React.useState<FilterState>(defaultFilterState);
  const [applied, setApplied] = React.useState<FilterState>(defaultFilterState);
  const [rowSelection, setRowSelection] = React.useState<RowSelectionState>({});
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
    valid_until: applied.validUntil,
    valid_from: applied.validFrom,
  };

  const { data: countData } = useGetMembersCount({ customFilters });

  const columns = React.useMemo(() => getColumns(syncStatus), [syncStatus]);

  const hasPendingChanges =
    JSON.stringify(draft.roles.map((r) => r.value).sort()) !==
      JSON.stringify(applied.roles.map((r) => r.value).sort()) ||
    draft.validFrom.getTime() !== applied.validFrom.getTime() ||
    draft.validUntil.getTime() !== applied.validUntil.getTime();

  const selectedIds = Object.entries(rowSelection)
    .filter(([, selected]) => selected)
    .map(([id]) => id);

  const clearSelection = () => setRowSelection({});

  const applyFilters = () => setApplied(draft);

  const clearAllFilters = () => {
    setDraft(defaultFilterState);
    setApplied(defaultFilterState);
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
                <span className="text-xs font-medium text-muted-foreground">
                  Valid
                </span>
                <DateRangePicker
                  onUpdate={({ range }) =>
                    setDraft((d) => ({
                      ...d,
                      validUntil: range.to ?? defaultTo,
                      validFrom: range.from,
                    }))
                  }
                  initialDateFrom={draft.validFrom}
                  initialDateTo={draft.validUntil}
                  locale="fi"
                  showCompare={false}
                />
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
              ...defaultFilterState,
              roles: stringsToOptions(filters?.roles ?? []),
              validFrom: filters?.valid_from
                ? new Date(filters.valid_from as string)
                : defaultFrom,
              validUntil: filters?.valid_until
                ? new Date(filters.valid_until as string)
                : defaultTo,
            };
            setDraft(next);
            setApplied(next);
          }}
          getRowId={(row) => row.user_id}
          rowSelection={rowSelection}
          onRowSelectionChange={setRowSelection}
          enableSavedFilters={true}
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
