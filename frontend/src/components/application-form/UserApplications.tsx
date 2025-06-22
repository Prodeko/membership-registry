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
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";
import { InfoCircledIcon } from "@radix-ui/react-icons";
import InfoTooltip from "../ui/info-tooltip";

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
          <span>
            Existing applications
            <InfoTooltip>
              <p>
                This section shows the applications you have submitted for role
                membeships.
              </p>
              <p>
                If you want to delete an application, please contact{" "}
                <a
                  href="mailto:mediakeisari@prodeko.org"
                  className="text-blue-500 hover:underline"
                >
                  mediakeisari@prodeko.org
                </a>
              </p>
            </InfoTooltip>
          </span>
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
                  {role?.payment_link && app.status === "unpaid" && (
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
