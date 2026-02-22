import {
  QueryKey,
  useAddMultipleRolesToMembers,
  useGetMembersWithIds,
  useGetRoles,
} from "@/lib/api";
import { defaultFrom, defaultTo, stringsToOptions } from "@/lib/utils";
import { useQueryClient } from "@tanstack/react-query";
import { FunctionComponent, useState } from "react";
import { DateRange } from "react-day-picker";
import { Button } from "../ui/button";
import { DateRangePicker } from "../ui/date-range-picker";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "../ui/dialog";
import MultipleSelector from "../ui/multiple-selector";

interface AddRolesModalProps {
  userIds: string[];
  onClose: () => void;
  disabled: boolean;
}

const AddRolesModal: FunctionComponent<AddRolesModalProps> = ({
  userIds,
  onClose,
  disabled,
}) => {
  const [selectedRoles, setSelectedRoles] = useState<string[]>([]);
  const [selectedDateRange, setSelectedDateRange] = useState<DateRange | null>({
    from: defaultFrom,
    to: defaultTo,
  });
  const { data: roles } = useGetRoles();
  const { mutate: addMultipleRolesToMembers } = useAddMultipleRolesToMembers();
  const { data: selectedMembers } = useGetMembersWithIds(userIds);
  const queryClient = useQueryClient();
  const options = stringsToOptions(roles?.map((r) => r.name) ?? []);

  const handleSubmit = () => {
    console.log(selectedDateRange);
    console.log(selectedRoles);

    if (
      selectedDateRange?.to &&
      selectedDateRange?.from &&
      selectedRoles.length > 0 &&
      userIds.length > 0
    ) {
      addMultipleRolesToMembers(
        {
          userIds,
          roleNames: selectedRoles,
          validFrom: selectedDateRange.from,
          validUntil: selectedDateRange.to,
        },
        {
          onSuccess: () => {
            queryClient.invalidateQueries({
              queryKey: [QueryKey.MEMBERS_WITH_ROLES],
            });
            onClose();
          },
        },
      );
    }
  };

  return (
    <Dialog>
      <DialogTrigger disabled={disabled}>
        <Button variant={"outline"} disabled={disabled}>
          Add roles
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add roles</DialogTitle>
          <DialogDescription>
            Select the roles you want to add to the selected members and the
            dates of the validity. Selected members:{" "}
            {selectedMembers?.map((m) => m.email).join(", ")}
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col space-y-4">
          <MultipleSelector
            options={options}
            onChange={(roles) => setSelectedRoles(roles.map((r) => r.value))}
          />
          <DateRangePicker
            onUpdate={({ range }) => {
              console.log(range);
              return setSelectedDateRange(range);
            }}
            disabled={selectedRoles.length === 0}
            showCompare={false}
            align="center"
            initialDateFrom={defaultFrom}
            initialDateTo={defaultTo}
          />
          <DialogClose asChild>
            <Button onClick={handleSubmit}>Add roles</Button>
          </DialogClose>
        </div>
      </DialogContent>
    </Dialog>
  );
};

export default AddRolesModal;
