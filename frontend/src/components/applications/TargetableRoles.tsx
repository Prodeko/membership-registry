import { useGetTargetableRoles } from "@/lib/api";
import CreateTargetableRolesModal from "./CreateTargetableRolesModal";

const TargetableRoles = () => {
  const { data: targetableRoles, isLoading } = useGetTargetableRoles();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  return (
    <div>
      <div className="flex justify-between">
        <h1 className="text-4xl">Targetable Roles</h1>
        <CreateTargetableRolesModal />
      </div>
      <ul>
        {targetableRoles?.map((targetableRole) => (
          <div>
            {targetableRole.role_name}{" "}
            {targetableRole.valid_until?.toDateString()}
          </div>
        ))}
      </ul>
    </div>
  );
};

export default TargetableRoles;
