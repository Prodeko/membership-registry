import { useCreateSavedFilter } from "@/lib/api";
import {
  Dialog,
  DialogTrigger,
  DialogContent,
  DialogTitle,
  DialogClose,
  DialogDescription,
  DialogHeader,
} from "../ui/dialog";
import { useState } from "react";
import { Input } from "../ui/input";
import { NewSavedFilter } from "@/common/types";
import { Button } from "../ui/button";
import { Checkbox } from "../ui/checkbox";
import { Label } from "../ui/label";

interface Props {
  newSavedFilter: NewSavedFilter;
  refetchSavedFilters: () => void;
}

const CreateSavedFilterModal = ({
  newSavedFilter,
  refetchSavedFilters,
}: Props) => {
  const [name, setName] = useState<string>(newSavedFilter.name);
  const [visibleForAll, setVisibleForAll] = useState<boolean>(
    newSavedFilter.visible_for_all
  );

  const { mutate: createSavedFilter } = useCreateSavedFilter();

  const handleCreate = async () => {
    await createSavedFilter(
      {
        ...newSavedFilter,
        name,
        visible_for_all: visibleForAll,
      },
      {
        onSuccess: () => refetchSavedFilters(),
      }
    );
  };

  return (
    <Dialog>
      <DialogTrigger>
        <Button>Save filter</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create Saved Filter</DialogTitle>
          <DialogClose />
        </DialogHeader>
        <div className="flex flex-col space-y-4">
          <Input
            id="filter-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Filter name"
          />
          <p className="flex items-center space-x-2">
            <Checkbox
              id="visible-for-all"
              checked={visibleForAll}
              onCheckedChange={() =>
                setVisibleForAll((old) => {
                  console.log(old);
                  return !old;
                })
              }
              aria-label="Visible for all users"
            />
            <Label htmlFor="visible-for-all">Visible for all</Label>
          </p>
          <DialogClose asChild>
            <Button onClick={handleCreate}>Create</Button>
          </DialogClose>
        </div>
      </DialogContent>
    </Dialog>
  );
};

export default CreateSavedFilterModal;
