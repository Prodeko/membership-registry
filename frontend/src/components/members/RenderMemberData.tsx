import { Member } from "@/common/types";

const RenderMemberData = ({
  member,
  variant,
}: {
  member: Member;
  variant: "enduser" | "admin";
}) => {
  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-4">
        <div>Full name:</div>
        <div>{member.full_name}</div>
        {variant === "admin" && (
          <>
            <div>User id:</div>
            <div>{member.user_id}</div>
          </>
        )}
        <div>Email:</div>
        <div>{member.email}</div>
        <div>Home municipality:</div>
        <div>{member.home_municipality}</div>
        <div>Has accepted policies:</div>
        <div>{member.has_accepted_policies ? "True" : "False"}</div>
      </div>
    </div>
  );
};

export default RenderMemberData;
