import { useGetAllMembers, useGetRoles } from "@/lib/api";
import { capitalizeFirstLetter } from "@/lib/utils";
import React from "react";
import { DataTable } from "../ui/data-table";
import { DateRangePicker } from "../ui/date-range-picker";
import MultipleSelector, { Option } from "../ui/multiple-selector";
import { columns } from "./columns";

const Members: React.FC = () => {
  const { data, isLoading, error } = useGetRoles();
  const [selectedRoles, setSelectedRoles] = React.useState<Option[]>([]);
  const [selectedValidFrom, setSelectedValidFrom] = React.useState<Date | undefined>(undefined);
  const [selectedValidUntil, setSelectedValidUntil] = React.useState<Date | undefined>(undefined);


  const onRoleChange = (selectedRoles: Option[]) => {
    setSelectedRoles(selectedRoles);
  }

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  const options: Option[] = data?.map((role) => ({
    label: capitalizeFirstLetter(role.name),
    value: role.name,
  })) ?? []; 

  return (
    <div>
      <h1>Members</h1>
      <div className="flex">
        <MultipleSelector options={options} onChange={onRoleChange}/>
        <DateRangePicker
          onUpdate={({range}) => {
            setSelectedValidUntil(range.to);
            setSelectedValidFrom(range.from);
          }}
          locale="fi"
          showCompare={false}
        />
      </div>
      <DataTable
        columns={columns}
        useFetchData={useGetAllMembers}
        initialColumnVisibility={{
          user_id: false,
          has_accepted_policies: false,
        }}
        customFilters={{
          roles: selectedRoles.map((role) => role.value),
          valid_until: selectedValidUntil,
          valid_from: selectedValidFrom,
        }}
      />
    </div>
  );
};

export default Members;
