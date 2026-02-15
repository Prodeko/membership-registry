import { QueryKey, useCreateRole } from "@/lib/api";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "../ui/dialog";
import { Button } from "../ui/button";
import { useState } from "react";
import { Input } from "../ui/input";
import { useQueryClient } from "@tanstack/react-query";

const CreateRoleModal = () => {
  const [name, setName] = useState<string>("");
  const { mutate: createRole } = useCreateRole();
  const queryClient = useQueryClient();

  return (
    <Dialog>
      <DialogTrigger>
        <Button>Create Role</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create Role</DialogTitle>
          <DialogClose />
        </DialogHeader>
        <DialogDescription className="flex space-x-4">
          <Input value={name} onChange={(e) => setName(e.target.value)} />
          <DialogClose asChild>
            <Button
              onClick={() =>
                createRole(
                  { name },
                  {
                    onSuccess: () =>
                      queryClient.invalidateQueries({
                        queryKey: [QueryKey.ROLES],
                      }),
                  },
                )
              }
            >
              Create
            </Button>
          </DialogClose>
        </DialogDescription>
      </DialogContent>
    </Dialog>
  );
};

export default CreateRoleModal;
