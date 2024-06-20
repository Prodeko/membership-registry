import { QueryKey, useDeleteManyMembers, useGetMembersWithIds } from "@/lib/api";
import { useQueryClient } from "@tanstack/react-query";
import { FunctionComponent } from "react";
import { Button } from "../ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "../ui/dialog";
import { DialogClose } from "@radix-ui/react-dialog";

interface DeleteMembersModalProps {
  userIds: string[];
  onClose: () => void;
  disabled: boolean;
}

const DeleteMembersModal: FunctionComponent<DeleteMembersModalProps> = ({
  userIds,
  onClose,
  disabled,
}) => {
  const { mutate: deleteMultipleMembers } = useDeleteManyMembers();
  const { data: selectedMembers } = useGetMembersWithIds(userIds);
  const queryClient = useQueryClient();

  const handleSubmit = () => {
    if (userIds.length > 0) {
      deleteMultipleMembers(userIds, {
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: [QueryKey.MEMBERS_WITH_ROLES] });
          onClose();
        },
      });
    }
  };

  return (
    <Dialog>
      <DialogTrigger disabled={disabled}>
        <Button variant={"outline"} disabled={disabled}>
          Delete
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete members</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete members: {selectedMembers?.map((m) => m.email).join(", ")}
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col space-y-4">
          <DialogClose asChild>
            <Button onClick={handleSubmit}>Delete members</Button>
          </DialogClose>
        </div>
      </DialogContent>
    </Dialog>
  );
};

export default DeleteMembersModal;
