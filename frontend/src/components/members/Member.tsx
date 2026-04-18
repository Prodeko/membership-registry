import { useGetMember, useGetMemberRoles } from "@/lib/api";
import { useNavigate, useParams } from "react-router-dom";
import { Pencil } from "lucide-react";
import { Card } from "../ui/card";
import AddRolesModal from "./AddRolesModal";
import { Button } from "../ui/button";
import DeleteMembersModal from "./DeleteMembersModal";
import RoleBadge from "../ui/role-badge";
import RenderMemberData from "./RenderMemberData";

const Member: React.FC = () => {
  const { id: userId } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const {
    data: member,
    isLoading: isMemberLoading,
    error: memberError,
  } = useGetMember(userId!);

  const {
    data: roles,
    isLoading: isRolesLoading,
    error: rolesError,
    refetch: refetchRoles,
  } = useGetMemberRoles(userId!);

  if (isMemberLoading || isRolesLoading) {
    return <div>Loading...</div>;
  }

  if (memberError || rolesError) {
    return (
      <div>
        Error: {memberError?.message ?? ""}, {rolesError?.message ?? ""}
      </div>
    );
  }

  if (!member) {
    return <div>Member not found</div>;
  }

  return (
    <div className="flex justify-center align-middle p-14">
      <Card className="p-8 space-y-6">
        <div className="flex justify-between items-center">
          <h1 className="text-2xl font-bold">
            {member.first_name} {member.last_name}
          </h1>
          <div className="flex gap-1 shrink-0">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => navigate(`/members/${userId}/edit`)}
              aria-label="Edit member"
            >
              <Pencil className="h-4 w-4" />
            </Button>
            <DeleteMembersModal
              userIds={[member.user_id]}
              onClose={() => navigate("/members")}
              disabled={false}
              iconMode
            />
          </div>
        </div>
        <RenderMemberData member={member} variant="admin" />
        <div className="space-y-3">
          <h2 className="text-2xl space-x-4">
            <span>Roles</span>{" "}
            <AddRolesModal
              userIds={[member.user_id]}
              disabled={false}
              onClose={refetchRoles}
            />{" "}
          </h2>
          <ul>
            {roles?.length
              ? roles.map((role) => (
                  <li
                    key={role.role_name}
                    className="space-y-2 grid grid-cols-2"
                  >
                    <span>
                      <RoleBadge role={role.role_name} />
                    </span>
                    <span>
                      {new Date(role.valid_from).toLocaleDateString()} -{" "}
                      {role.valid_until
                        ? new Date(role.valid_until).toLocaleDateString()
                        : "N/A"}
                    </span>
                  </li>
                ))
              : "No roles"}
          </ul>
        </div>
      </Card>
    </div>
  );
};

export default Member;
