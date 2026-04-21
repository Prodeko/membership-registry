import {
  QueryKey,
  useAddMultipleRolesToMembers,
  useGetMembersWithIds,
  useGetRoles,
} from "@/lib/api";
import { defaultFrom, defaultTo, stringsToOptions } from "@/lib/utils";
import { useQueryClient } from "@tanstack/react-query";
import { FunctionComponent, ReactNode, useState } from "react";
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
import RoleBadge from "../ui/role-badge";

interface AddRolesModalProps {
  userIds: string[];
  onClose: () => void;
  disabled: boolean;
  trigger?: ReactNode;
}

const AddRolesModal: FunctionComponent<AddRolesModalProps> = ({
  userIds,
  onClose,
  disabled,
  trigger,
}) => {
  const [open, setOpen] = useState(false);
  const [step, setStep] = useState<1 | 2>(1);
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

  const resetState = () => {
    setStep(1);
    setSelectedRoles([]);
    setSelectedDateRange({ from: defaultFrom, to: defaultTo });
  };

  const handleOpenChange = (next: boolean) => {
    setOpen(next);
    if (!next) resetState();
  };

  const handleSubmit = () => {
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
            setOpen(false);
            resetState();
            onClose();
          },
        },
      );
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild disabled={disabled}>
        {trigger ?? (
          <Button variant="outline" disabled={disabled}>
            Add roles
          </Button>
        )}
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <div className="flex items-start justify-between">
            <DialogTitle>Add roles</DialogTitle>
            <span className="text-xs text-muted-foreground">
              Step {step} of 2
            </span>
          </div>
          <DialogDescription>
            {userIds.length} member{userIds.length === 1 ? "" : "s"} selected
            {selectedMembers && selectedMembers.length > 0
              ? `: ${selectedMembers
                  .slice(0, 3)
                  .map((m) => m.email)
                  .join(", ")}${selectedMembers.length > 3 ? ", …" : ""}`
              : null}
          </DialogDescription>
        </DialogHeader>

        {step === 1 ? (
          <div className="flex flex-col space-y-4">
            <div>
              <label className="text-sm font-medium mb-2 block">
                Select roles
              </label>
              <MultipleSelector
                options={options}
                onChange={(rs) => setSelectedRoles(rs.map((r) => r.value))}
                value={stringsToOptions(selectedRoles)}
              />
            </div>
            <div className="flex justify-end gap-2">
              <DialogClose asChild>
                <Button variant="outline">Cancel</Button>
              </DialogClose>
              <Button
                onClick={() => setStep(2)}
                disabled={selectedRoles.length === 0}
              >
                Next: Set dates →
              </Button>
            </div>
          </div>
        ) : (
          <div className="flex flex-col space-y-4">
            <div>
              <label className="text-xs text-muted-foreground mb-1.5 block">
                Assigning
              </label>
              <div className="flex flex-wrap gap-1">
                {selectedRoles.map((r) => (
                  <RoleBadge key={r} role={r} />
                ))}
              </div>
            </div>
            <div>
              <label className="text-sm font-medium mb-2 block">
                Validity period
              </label>
              <DateRangePicker
                onUpdate={({ range }) => setSelectedDateRange(range)}
                showCompare={false}
                align="center"
                initialDateFrom={selectedDateRange?.from ?? defaultFrom}
                initialDateTo={selectedDateRange?.to ?? defaultTo}
              />
            </div>
            <div className="flex items-center justify-between">
              <button
                type="button"
                onClick={() => setStep(1)}
                className="text-sm text-muted-foreground hover:text-foreground"
              >
                ← Back
              </button>
              <Button onClick={handleSubmit}>Add roles</Button>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
};

export default AddRolesModal;
