import {
  useGetAllMembersWithRoles,
  useGetKeycloakSyncStatus,
  useGetRoles,
} from "@/lib/api";
import { defaultFrom, defaultTo, stringsToOptions } from "@/lib/utils";
import React from "react";
import { DataTable } from "../ui/data-table";
import { DateRangePicker } from "../ui/date-range-picker";
import MultipleSelector, { Option } from "../ui/multiple-selector";
import AddRolesModal from "./AddRolesModal";
import DeleteMembersModal from "./DeleteMembersModal";
import { getColumns } from "./columns";
import { Button } from "../ui/button";

const Members: React.FC = () => {
  const { data: roles, isLoading, error } = useGetRoles();
  const { data: syncStatus } = useGetKeycloakSyncStatus();
  const [selectedRoles, setSelectedRoles] = React.useState<Option[]>([]);
  const [filterVisible, setFilterVisible] = React.useState(false);
  const [selectedValidFrom, setSelectedValidFrom] = React.useState<Date>(
    new Date(),
  );
  const [selectedValidUntil, setSelectedValidUntil] = React.useState<Date>(
    new Date(),
  );

  const columns = React.useMemo(() => getColumns(syncStatus), [syncStatus]);

  const onRoleChange = (selectedRoles: Option[]) => {
    setSelectedRoles(selectedRoles);
  };

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex space-x-8">
        <h1 className="text-4xl">Members</h1>
        <Button variant={"outline"} onClick={() => setFilterVisible((e) => !e)}>
          {filterVisible ? "Hide filters" : "Show filters"}
        </Button>
      </div>
      {filterVisible && (
        <div className="flex space-x-2 flex-wrap">
          <MultipleSelector
            options={stringsToOptions(roles?.map((r) => r.name) ?? [])}
            onChange={onRoleChange}
            value={selectedRoles}
            placeholder="Filter by role"
          />
          <DateRangePicker
            onUpdate={({ range }) => {
              setSelectedValidUntil(range.to ?? defaultTo);
              setSelectedValidFrom(range.from);
            }}
            initialDateFrom={selectedValidFrom}
            initialDateTo={selectedValidUntil}
            locale="fi"
            showCompare={false}
            disabled={selectedRoles.length === 0}
          />
        </div>
      )}
      <DataTable
        modelName="members"
        columns={columns}
        useFetchData={useGetAllMembersWithRoles}
        searchColumn="first_name"
        initialColumnVisibility={{
          user_id: false,
          home_municipality: false,
        }}
        filterVisible={filterVisible}
        customFilters={{
          roles: selectedRoles.map((role) => role.value),
          valid_until: selectedValidUntil,
          valid_from: selectedValidFrom,
        }}
        setCustomFilters={(filters) => {
          setSelectedRoles(stringsToOptions(filters.roles ?? []));
          setSelectedValidFrom(filters?.valid_from ?? defaultFrom);
          setSelectedValidUntil(filters?.valid_until ?? defaultTo);
        }}
        multipleRowActionElements={[
          (table, ids) => (
            <AddRolesModal
              userIds={ids}
              onClose={() => table.setRowSelection({})}
              disabled={ids.length === 0}
            />
          ),
          (table, ids) => (
            <DeleteMembersModal
              userIds={ids}
              onClose={() => table.setRowSelection({})}
              disabled={ids.length === 0}
            />
          ),
        ]}
        getRowId={(row) => row.user_id}
      />
    </div>
  );
};

export default Members;
