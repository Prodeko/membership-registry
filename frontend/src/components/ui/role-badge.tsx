import { capitalizeFirstLetter } from "@/lib/utils";
import { Badge } from "./badge";

const RoleBadge = ({ role }: { role: string }) => {
  return (
    <Badge variant={"outline"}>{capitalizeFirstLetter(role)}</Badge>
  );
}

export default RoleBadge;