import { useGetRole, useGetRoleMembers } from "@/lib/api";
import { useNavigate, useParams, Link } from "react-router-dom";
import { Card } from "../ui/card";
import { Button } from "../ui/button";
import RoleBadge from "../ui/role-badge";

const Role = () => {
  const { id: roleName = "" } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const {
    data: role,
    isLoading: isRoleLoading,
    error: roleError,
  } = useGetRole(roleName, { enabled: roleName.length > 0 });

  const {
    data: members,
    isLoading: isMembersLoading,
    error: membersError,
  } = useGetRoleMembers(roleName, { enabled: roleName.length > 0 });

  if (isRoleLoading || isMembersLoading) {
    return <div>Loading...</div>;
  }

  if (roleError || membersError) {
    return (
      <div>
        Error: {roleError?.message ?? ""}, {membersError?.message ?? ""}
      </div>
    );
  }

  if (!role) {
    return <div>Role not found</div>;
  }

  return (
    <div className="p-8 space-y-6 max-w-4xl mx-auto">
      <Button variant="outline" onClick={() => navigate("/roles")}>
        Back to roles
      </Button>

      <Card className="p-8 space-y-6">
        <div className="space-y-2">
          <h1 className="text-3xl font-bold">{role.name}</h1>
          <div className="flex items-center gap-3">
            <RoleBadge role={role.name} />
            {role.color && (
              <span
                className="inline-block w-4 h-4 rounded-full border"
                style={{ backgroundColor: role.color }}
              />
            )}
          </div>
          {role.description && (
            <p className="text-muted-foreground">{role.description}</p>
          )}
        </div>

        <div className="space-y-3">
          <h2 className="text-xl font-semibold">
            Members ({members?.length ?? 0})
          </h2>
          {members?.length ? (
            <div className="rounded-md border">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b bg-muted/50">
                    <th className="text-left p-3 font-medium">Name</th>
                    <th className="text-left p-3 font-medium">Email</th>
                    <th className="text-left p-3 font-medium">Municipality</th>
                  </tr>
                </thead>
                <tbody>
                  {members.map((member) => (
                    <tr key={member.user_id} className="border-b last:border-0">
                      <td className="p-3">
                        <Link
                          to={`/members/${member.user_id}`}
                          className="text-blue-600 hover:underline"
                        >
                          {member.full_name ?? "N/A"}
                        </Link>
                      </td>
                      <td className="p-3">{member.email ?? "N/A"}</td>
                      <td className="p-3">
                        {member.home_municipality ?? "N/A"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <p className="text-muted-foreground">No members</p>
          )}
        </div>
      </Card>
    </div>
  );
};

export default Role;
