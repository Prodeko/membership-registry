import { MemberWithRoles } from "@/common/types";
import { QueryKey, useDeleteManyMembers, useDeleteMember, useGetAllMembers, useGetRoles } from "@/lib/api";
import { capitalizeFirstLetter, confirmAnd } from "@/lib/utils";
import { useQueryClient } from "@tanstack/react-query";
import React from "react";
import { DataTable } from "../ui/data-table";
import { DateRangePicker } from "../ui/date-range-picker";
import MultipleSelector, { Option } from "../ui/multiple-selector";
import { columns } from "./columns";

const Members: React.FC = () => {
  const { data, isLoading, error } = useGetRoles();
  const { mutate: deleteMembersMutation} = useDeleteManyMembers();
  const queryClient = useQueryClient();
  const [selectedRoles, setSelectedRoles] = React.useState<Option[]>([]);
  const [selectedValidFrom, setSelectedValidFrom] = React.useState<
    Date | undefined
  >(undefined);
  const [selectedValidUntil, setSelectedValidUntil] = React.useState<
    Date | undefined
  >(undefined);

  const onRoleChange = (selectedRoles: Option[]) => {
    setSelectedRoles(selectedRoles);
  };

  const deleteManyMembers = (members: MemberWithRoles[]) => {
    deleteMembersMutation(members.map((member) => member.user_id), {
      onSuccess: () => {
        queryClient.invalidateQueries({queryKey: [QueryKey.MEMBERS]});
      },
    });
  };

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  const options: Option[] =
    data?.map((role) => ({
      label: capitalizeFirstLetter(role.name),
      value: role.name,
    })) ?? [];

  return (
    <div>
      <h1>Members</h1>
      <div className="flex">
        <MultipleSelector options={options} onChange={onRoleChange} />
        <DateRangePicker
          onUpdate={({ range }) => {
            setSelectedValidUntil(range.to);
            setSelectedValidFrom(range.from);
          }}
          locale="fi"
          showCompare={false}
          disabled={selectedRoles.length === 0}
        />
      </div>
      <DataTable
        columns={columns}
        useFetchData={useGetAllMembers}
        initialColumnVisibility={{
          user_id: false,
          has_accepted_policies: false,
          home_municipality: false,
        }}
        customFilters={{
          roles: selectedRoles.map((role) => role.value),
          valid_until: selectedValidUntil,
          valid_from: selectedValidFrom,
        }}
        multipleRowActions={[
            { label: "Delete", onClick: (rows) => confirmAnd(() => deleteManyMembers(rows), `Delete members: ${rows.map((row) => row.email).join(", ")}?`) },
            { label: "Add roles", onClick: () => console.log("Add roles") },
            { label: "Remove roles", onClick: () => console.log("Remove roles") },
        ]}
      />
    </div>
  );
};

export default Members;
