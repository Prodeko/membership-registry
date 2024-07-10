import { QueryKey, useCreateTargetableRole, useGetRoles } from "@/lib/api";
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { DatePicker } from "../ui/date-picker";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "../ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import { Button } from "../ui/button";
import { Link } from "react-router-dom";

const CreateTargetableRolesModal = () => {
  const [selectedRole, setSelectedRole] = useState<string>("");
  const [selectedValidUntil, setSelectedValidUntil] = useState<
    Date | undefined
  >(undefined);
  const { data: roles } = useGetRoles();

  const { mutate: createTargetableRole } = useCreateTargetableRole();

  const queryClient = useQueryClient();

  const handleSubmit = () => {
    if (selectedRole && selectedValidUntil) {
      createTargetableRole(
        {
          role_name: selectedRole,
          valid_until: selectedValidUntil,
          active: true,
        },
        {
          onSuccess: () => {
            queryClient.invalidateQueries({
              queryKey: [QueryKey.TARGETABLE_ROLES],
            });
          },
        }
      );
    }
  };

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button>Create Application Targetable Role</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create Application Targetable Role</DialogTitle>
          <DialogClose />
        </DialogHeader>
        <DialogDescription>
          Add a new role that can be targeted by applications. Add more roles{" "}
          <Link to={"/roles"}>here</Link>
        </DialogDescription>
        <div className="flex justify-between">
          <Select onValueChange={setSelectedRole}>
            <SelectTrigger>
              <SelectValue placeholder="Select role" />
            </SelectTrigger>
            <SelectContent>
              {roles?.map((role) => (
                <SelectItem key={role.name} value={role.name}>
                  {role.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <DatePicker
            onSelect={(day: Date | undefined) => setSelectedValidUntil(day)}
            title="Valid until"
          />
        </div>
        <DialogClose asChild>
          <Button onClick={handleSubmit}>Create</Button>
        </DialogClose>
      </DialogContent>
    </Dialog>
  );
};

export default CreateTargetableRolesModal;
