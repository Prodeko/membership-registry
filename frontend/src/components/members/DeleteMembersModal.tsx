import {
  QueryKey,
  useDeleteManyMembers,
  useGetMembersWithIds,
} from "@/lib/api";
import { useQueryClient } from "@tanstack/react-query";
import { FunctionComponent, ReactNode } from "react";
import { Trash2 } from "lucide-react";
import { Button } from "../ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "../ui/dialog";
import { DialogClose } from "@radix-ui/react-dialog";

interface DeleteMembersModalProps {
  userIds: string[];
  onClose: () => void;
  disabled: boolean;
  iconMode?: boolean;
  trigger?: ReactNode;
}

const DeleteMembersModal: FunctionComponent<DeleteMembersModalProps> = ({
  userIds,
  onClose,
  disabled,
  iconMode = false,
  trigger,
}) => {
  const { mutate: deleteMultipleMembers } = useDeleteManyMembers();
  const { data: selectedMembers } = useGetMembersWithIds(userIds);
  const queryClient = useQueryClient();

  const handleSubmit = () => {
    if (userIds.length > 0) {
      deleteMultipleMembers(userIds, {
        onSuccess: () => {
          queryClient.invalidateQueries({
            queryKey: [QueryKey.MEMBERS_WITH_ROLES],
          });
          onClose();
        },
      });
    }
  };

  const defaultTrigger = iconMode ? (
    <Button variant="ghost" size="icon" disabled={disabled}>
      <Trash2 className="h-4 w-4" />
    </Button>
  ) : (
    <Button variant="outline" disabled={disabled}>
      Delete
    </Button>
  );

  return (
    <Dialog>
      <DialogTrigger asChild disabled={disabled}>
        {trigger ?? defaultTrigger}
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete members</DialogTitle>
          <DialogDescription className="space-y-2">
            <span className="block">
              This will permanently remove the following members from the
              registry and revoke their Keycloak access:
            </span>
            <span className="block font-medium text-foreground">
              {selectedMembers?.map((m) => m.email).join(", ")}
            </span>
            <span className="block text-destructive font-medium">
              This cannot be undone.
            </span>
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline">Cancel</Button>
          </DialogClose>
          <DialogClose asChild>
            <Button variant="destructive" onClick={handleSubmit}>
              <Trash2 className="mr-2 h-4 w-4" />
              Delete {userIds.length} member{userIds.length === 1 ? "" : "s"}
            </Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default DeleteMembersModal;
