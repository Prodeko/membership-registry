import { useGetRoles } from "@/lib/api";
import CreateRoleModal from "./CreateRoleModal";

const Roles = () => {
  const { data: roles, isLoading } = useGetRoles();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  return (
    <div>
      <div className="flex justify-between">
        <h1 className="text-4xl">Roles</h1>
        <CreateRoleModal />
      </div>
      {roles?.map((role) => (
        <div>{role.name}</div>
      ))}
    </div>
  );
};

export default Roles;
