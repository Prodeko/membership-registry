import { Badge } from "./badge";

const RoleBadge = ({ role }: { role: string }) => {
  return (
    <Badge variant={"outline"}>{role}</Badge>
  );
}

export default RoleBadge;