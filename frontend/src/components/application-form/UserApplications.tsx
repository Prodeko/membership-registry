import { useGetTargetableRoles, useGetUserApplications } from "@/lib/api";
import { getDateAsString } from "@/lib/utils";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "../ui/accordion";
import { Badge } from "../ui/badge";
import { Separator } from "../ui/separator";

const UserApplications = () => {
  const { data: userApplications } = useGetUserApplications();
  const { data: targetableRoles } = useGetTargetableRoles();

  if (!userApplications || !targetableRoles) {
    return null;
  }

  return (
    <Accordion type="single" collapsible>
      <AccordionItem value="item-1">
        <AccordionTrigger className="font-bold">
          <span>Existing applications</span>
        </AccordionTrigger>
        <AccordionContent className="space-y-4">
          {userApplications.map((app) => {
            const role = targetableRoles.find(
              (role) =>
                role.role_name === app.role_name &&
                role.valid_until.getTime() === app.valid_until.getTime()
            );
            return (
              <>
                <Separator />
                <div
                  key={app.application_id}
                  className="flex justify-between w-full"
                >
                  <div>
                    <h4 className="text-md font-semibold">
                      {app.role_name} - Role valid until:{" "}
                      {getDateAsString(app.valid_until)}
                    </h4>
                    <div className="text-sm">
                      Status:
                      <Badge
                        variant={
                          app.status === "approved"
                            ? "default"
                            : app.status === "rejected"
                            ? "destructive"
                            : "secondary"
                        }
                        className="ml-2"
                      >
                        {app.status}
                      </Badge>
                    </div>
                  </div>
                  {role?.payment_link && !app.stripe_payment_id && (
                    <Badge
                      variant={"secondary"}
                      className="text-sm align-center"
                    >
                      <a
                        href={role.payment_link}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-blue-500 hover:underline"
                      >
                        Pay the fee
                      </a>
                    </Badge>
                  )}
                </div>
              </>
            );
          })}
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  );
};

export default UserApplications;
